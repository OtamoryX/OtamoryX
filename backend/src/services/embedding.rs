use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Url};
use serde_json::{json, Value};
use sqlx::{Pool, Sqlite};
use std::{collections::HashMap, net::IpAddr, sync::LazyLock, time::Duration};
use tokio::{
    sync::Mutex,
    time::{sleep_until, Instant},
};

use crate::models::{
    EmbeddingAuthMode, EmbeddingProvider, EmbeddingSettings, EMBEDDING_SETTINGS_VERSION,
};

const SETTINGS_KEY: &str = "ai_embedding_settings";
const API_KEY_SETTINGS_KEY: &str = "ai_embedding_api_key";

static EMBEDDING_REQUEST_STARTS: LazyLock<Mutex<HashMap<String, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub async fn load_embedding_settings(pool: &Pool<Sqlite>) -> Result<EmbeddingSettings> {
    let stored = sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(SETTINGS_KEY)
        .fetch_optional(pool)
        .await?;
    let mut settings = match stored {
        Some(raw) => serde_json::from_str::<EmbeddingSettings>(&raw)
            .context("stored embedding settings are invalid")?,
        None => EmbeddingSettings::default(),
    };

    normalize_settings(&mut settings);
    settings.api_key = sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(API_KEY_SETTINGS_KEY)
        .fetch_optional(pool)
        .await?;
    settings.api_key_configured = configured_api_key(&settings).is_some();
    Ok(settings)
}

pub async fn save_embedding_settings(
    pool: &Pool<Sqlite>,
    mut settings: EmbeddingSettings,
) -> Result<()> {
    if settings.settings_version != EMBEDDING_SETTINGS_VERSION {
        return Err(anyhow!(
            "unsupported embedding settings version {}; expected {}",
            settings.settings_version,
            EMBEDDING_SETTINGS_VERSION
        ));
    }
    let stored = load_embedding_settings(pool).await?;
    normalize_settings(&mut settings);

    let submitted_key = settings
        .api_key
        .take()
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty());
    let effective_key = submitted_key.or_else(|| reusable_api_key(&stored, &settings));

    validate_embedding_settings(&settings, effective_key.as_deref())?;
    settings.api_key_configured = false;
    let stored_json = serde_json::to_string(&settings)?;
    let mut transaction = pool.begin().await?;
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(SETTINGS_KEY)
    .bind(stored_json)
    .execute(&mut *transaction)
    .await?;

    if settings.auth_mode == EmbeddingAuthMode::Bearer {
        if let Some(api_key) = effective_key {
            sqlx::query(
                "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            )
            .bind(API_KEY_SETTINGS_KEY)
            .bind(api_key)
            .execute(&mut *transaction)
            .await?;
        }
    } else {
        sqlx::query("DELETE FROM settings WHERE key = ?")
            .bind(API_KEY_SETTINGS_KEY)
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    Ok(())
}

pub fn embedding_settings_for_connection_test(
    stored: &EmbeddingSettings,
    mut provided: EmbeddingSettings,
) -> Result<EmbeddingSettings> {
    if provided.settings_version != EMBEDDING_SETTINGS_VERSION {
        return Err(anyhow!(
            "unsupported embedding settings version {}; expected {}",
            provided.settings_version,
            EMBEDDING_SETTINGS_VERSION
        ));
    }
    normalize_settings(&mut provided);
    let provided_key = provided
        .api_key
        .take()
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty());
    provided.api_key = provided_key.or_else(|| reusable_api_key(stored, &provided));
    provided.api_key_configured = configured_api_key(&provided).is_some();
    validate_embedding_settings(&provided, provided.api_key.as_deref())?;
    Ok(provided)
}

pub async fn test_embedding_connection(settings: &EmbeddingSettings) -> Result<()> {
    let embeddings =
        generate_embeddings(settings, &["embedding connection test".to_string()]).await?;
    if embeddings.len() != 1 || embeddings[0].is_empty() {
        return Err(anyhow!("embedding provider returned an empty test vector"));
    }
    Ok(())
}

/// Generate one vector for each input using the configured provider. This is the transport
/// boundary used by the future tag-clustering worker as well as the settings smoke test.
pub async fn generate_embeddings(
    settings: &EmbeddingSettings,
    inputs: &[String],
) -> Result<Vec<Vec<f32>>> {
    if inputs.is_empty() {
        return Err(anyhow!("embedding input must contain at least one item"));
    }
    validate_embedding_settings(settings, settings.api_key.as_deref())?;
    let endpoint = embedding_endpoint(settings)?;
    let client = Client::builder()
        .timeout(request_timeout(settings))
        .build()
        .context("failed to build embedding client")?;
    reserve_embedding_request_start(settings).await;

    let request = authenticated_post(&client, &endpoint, settings)?
        .json(&embedding_payload(settings, inputs));
    let response = request
        .send()
        .await
        .context("embedding provider request failed")?;
    let status = response.status();
    let body = response
        .text()
        .await
        .context("failed to read embedding provider response")?;
    if !status.is_success() {
        return Err(anyhow!(
            "embedding provider returned HTTP {}: {}",
            status,
            compact_error_body(&body)
        ));
    }
    let body: Value = serde_json::from_str(&body).context("invalid embedding response envelope")?;
    parse_embedding_response(settings, &body, inputs.len())
}

pub fn embedding_endpoint(settings: &EmbeddingSettings) -> Result<String> {
    let base = settings.base_url.trim();
    if base.chars().any(char::is_whitespace) {
        return Err(anyhow!("embedding base URL must not contain whitespace"));
    }
    let mut url = Url::parse(base).context("embedding base URL is invalid")?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(anyhow!("embedding base URL must use http:// or https://"));
    }
    if url.host_str().is_none() {
        return Err(anyhow!("embedding base URL must contain a host"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(anyhow!(
            "embedding base URL must not contain user information"
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(anyhow!(
            "embedding base URL must not contain a query or fragment"
        ));
    }

    let path = url.path().trim_end_matches('/');
    let endpoint_path = match settings.provider {
        EmbeddingProvider::Ollama => {
            if path.ends_with("/api/embed") || path == "/api/embed" {
                path.to_string()
            } else if path.ends_with("/api") || path == "/api" {
                format!("{path}/embed")
            } else {
                format!("{path}/api/embed")
            }
        }
        EmbeddingProvider::OpenaiCompatible => {
            if path.ends_with("/embeddings") || path == "/embeddings" {
                path.to_string()
            } else {
                format!("{path}/embeddings")
            }
        }
    };
    url.set_path(if endpoint_path.is_empty() {
        "/"
    } else {
        &endpoint_path
    });
    Ok(url.to_string())
}

pub fn embedding_payload(settings: &EmbeddingSettings, inputs: &[String]) -> Value {
    let input = Value::Array(inputs.iter().cloned().map(Value::String).collect());
    match settings.provider {
        EmbeddingProvider::Ollama => json!({
            "model": settings.model,
            "input": input,
            "truncate": true,
        }),
        EmbeddingProvider::OpenaiCompatible => {
            let mut payload = json!({
                "model": settings.model,
                "input": input,
                "encoding_format": "float",
            });
            if let Some(dimensions) = settings.dimensions {
                payload["dimensions"] = Value::from(dimensions);
            }
            payload
        }
    }
}

fn parse_embedding_response(
    settings: &EmbeddingSettings,
    body: &Value,
    expected_count: usize,
) -> Result<Vec<Vec<f32>>> {
    if let Some(error) = body.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .or_else(|| error.as_str())
            .unwrap_or("unknown embedding provider error");
        return Err(anyhow!("embedding provider returned an error: {message}"));
    }

    let values = match settings.provider {
        EmbeddingProvider::Ollama => body
            .get("embeddings")
            .ok_or_else(|| anyhow!("Ollama response did not contain embeddings"))?,
        EmbeddingProvider::OpenaiCompatible => body
            .get("data")
            .ok_or_else(|| anyhow!("OpenAI-compatible response did not contain data"))?,
    };
    let values = values
        .as_array()
        .ok_or_else(|| anyhow!("embedding response vectors must be an array"))?;
    if values.len() != expected_count {
        return Err(anyhow!(
            "embedding provider returned {} vectors for {} inputs",
            values.len(),
            expected_count
        ));
    }

    let expected_dimension = match settings.provider {
        EmbeddingProvider::Ollama => None,
        EmbeddingProvider::OpenaiCompatible => settings.dimensions.map(|value| value as usize),
    };
    let mut vectors: Vec<Option<Vec<f32>>> = (0..expected_count).map(|_| None).collect();
    for (position, value) in values.iter().enumerate() {
        let target = match settings.provider {
            EmbeddingProvider::Ollama => position,
            EmbeddingProvider::OpenaiCompatible => {
                let index = value
                    .get("index")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| anyhow!("OpenAI-compatible data item has an invalid index"))?;
                usize::try_from(index)
                    .map_err(|_| anyhow!("OpenAI-compatible embedding index is too large"))?
            }
        };
        if target >= expected_count {
            return Err(anyhow!(
                "embedding response index {} is outside the input range",
                target
            ));
        }
        if vectors[target].is_some() {
            return Err(anyhow!(
                "embedding response contains duplicate index {}",
                target
            ));
        }
        let vector = match settings.provider {
            EmbeddingProvider::Ollama => value,
            EmbeddingProvider::OpenaiCompatible => value
                .get("embedding")
                .ok_or_else(|| anyhow!("OpenAI-compatible data item did not contain embedding"))?,
        };
        let vector = vector
            .as_array()
            .ok_or_else(|| anyhow!("embedding vector must be an array"))?;
        if vector.is_empty() {
            return Err(anyhow!("embedding provider returned an empty vector"));
        }
        if let Some(expected_dimension) = expected_dimension {
            if vector.len() != expected_dimension {
                return Err(anyhow!(
                    "embedding provider returned dimension {}, expected {}",
                    vector.len(),
                    expected_dimension
                ));
            }
        }
        let vector = vector
            .iter()
            .map(|value| {
                let value = value
                    .as_f64()
                    .ok_or_else(|| anyhow!("embedding vector contains a non-number"))?;
                if !value.is_finite() {
                    return Err(anyhow!("embedding vector contains a non-finite number"));
                }
                Ok(value as f32)
            })
            .collect::<Result<Vec<_>>>()?;
        vectors[target] = Some(vector);
    }
    let vectors = vectors
        .into_iter()
        .enumerate()
        .map(|(index, vector)| {
            vector.ok_or_else(|| anyhow!("embedding response is missing index {}", index))
        })
        .collect::<Result<Vec<_>>>()?;
    let dimension = vectors
        .first()
        .map(Vec::len)
        .ok_or_else(|| anyhow!("embedding response did not contain vectors"))?;
    if vectors.iter().any(|vector| vector.len() != dimension) {
        return Err(anyhow!(
            "embedding provider returned inconsistent dimensions"
        ));
    }
    Ok(vectors)
}

fn authenticated_post(
    client: &Client,
    endpoint: &str,
    settings: &EmbeddingSettings,
) -> Result<reqwest::RequestBuilder> {
    let request = client.post(endpoint);
    match settings.auth_mode {
        EmbeddingAuthMode::None => Ok(request),
        EmbeddingAuthMode::Bearer => configured_api_key(settings)
            .map(|key| request.bearer_auth(key))
            .ok_or_else(|| anyhow!("no embedding API key is configured")),
    }
}

fn configured_api_key(settings: &EmbeddingSettings) -> Option<String> {
    if settings.auth_mode == EmbeddingAuthMode::None {
        return None;
    }
    settings
        .api_key
        .clone()
        .filter(|key| !key.trim().is_empty())
}

fn reusable_api_key(stored: &EmbeddingSettings, provided: &EmbeddingSettings) -> Option<String> {
    if stored.auth_mode != EmbeddingAuthMode::Bearer
        || provided.auth_mode != EmbeddingAuthMode::Bearer
        || stored.provider != provided.provider
    {
        return None;
    }
    let stored_endpoint = embedding_endpoint(stored).ok()?;
    let provided_endpoint = embedding_endpoint(provided).ok()?;
    (stored_endpoint == provided_endpoint)
        .then(|| stored.api_key.clone())
        .flatten()
        .filter(|key| !key.trim().is_empty())
}

fn validate_embedding_settings(
    settings: &EmbeddingSettings,
    effective_api_key: Option<&str>,
) -> Result<()> {
    let endpoint = embedding_endpoint(settings)?;
    if settings.auth_mode == EmbeddingAuthMode::Bearer {
        let endpoint_url = Url::parse(&endpoint).context("embedding endpoint is invalid")?;
        if endpoint_url.scheme() == "http" && !endpoint_url.host_str().is_some_and(is_loopback_host)
        {
            return Err(anyhow!(
                "Bearer embedding authentication requires HTTPS unless the host is loopback"
            ));
        }
    }
    if settings.model.trim().is_empty() {
        return Err(anyhow!("embedding model must not be empty"));
    }
    if !(5..=3_600).contains(&settings.timeout_seconds) {
        return Err(anyhow!(
            "embedding timeoutSeconds must be between 5 and 3600"
        ));
    }
    if settings.request_interval_seconds > 3_600 {
        return Err(anyhow!(
            "embedding requestIntervalSeconds must be between 0 and 3600"
        ));
    }
    if settings
        .dimensions
        .is_some_and(|dimensions| !(1..=65_536).contains(&dimensions))
    {
        return Err(anyhow!("embedding dimensions must be between 1 and 65536"));
    }
    if settings.auth_mode == EmbeddingAuthMode::Bearer
        && effective_api_key.is_none_or(|key| key.trim().is_empty())
    {
        return Err(anyhow!(
            "embedding API key is required for Bearer authentication"
        ));
    }
    Ok(())
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .map(|address| address.is_loopback())
            .unwrap_or(false)
}

fn normalize_settings(settings: &mut EmbeddingSettings) {
    settings.base_url = settings.base_url.trim().to_string();
    settings.model = settings.model.trim().to_string();
    settings.settings_version = EMBEDDING_SETTINGS_VERSION;
    if settings.dimensions == Some(0) {
        settings.dimensions = None;
    }
}

fn request_timeout(settings: &EmbeddingSettings) -> Duration {
    Duration::from_secs(settings.timeout_seconds.clamp(5, 3_600))
}

async fn reserve_embedding_request_start(settings: &EmbeddingSettings) {
    let interval = Duration::from_secs(settings.request_interval_seconds);
    if interval.is_zero() {
        return;
    }
    let model_key = format!(
        "{}:{}:{}",
        match settings.provider {
            EmbeddingProvider::Ollama => "ollama",
            EmbeddingProvider::OpenaiCompatible => "openaiCompatible",
        },
        settings.base_url.trim_end_matches('/'),
        settings.model
    );
    let reserved_start = {
        let mut starts = EMBEDDING_REQUEST_STARTS.lock().await;
        let now = Instant::now();
        let reserved_start = starts
            .get(&model_key)
            .copied()
            .filter(|scheduled| *scheduled > now)
            .unwrap_or(now);
        starts.insert(model_key, reserved_start + interval);
        reserved_start
    };
    sleep_until(reserved_start).await;
}

fn compact_error_body(body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        return "empty response".to_string();
    }
    let mut compact = body.chars().take(512).collect::<String>();
    if body.chars().count() > 512 {
        compact.push_str("...");
    }
    compact
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::State, http::HeaderMap, routing::post, Json, Router};
    use std::sync::Arc;
    use tokio::{net::TcpListener, sync::Mutex, task::JoinHandle};

    static MOCK_SERVER_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn ollama_settings() -> EmbeddingSettings {
        EmbeddingSettings {
            model: "test-embedding".to_string(),
            ..EmbeddingSettings::default()
        }
    }

    fn openai_settings() -> EmbeddingSettings {
        EmbeddingSettings {
            provider: EmbeddingProvider::OpenaiCompatible,
            base_url: "https://example.test/v1".to_string(),
            model: "embedding-model".to_string(),
            ..EmbeddingSettings::default()
        }
    }

    #[test]
    fn default_settings_do_not_select_a_model() {
        let settings = EmbeddingSettings::default();
        assert!(settings.model.is_empty());
        assert!(validate_embedding_settings(&settings, None).is_err());
    }

    #[derive(Clone)]
    struct MockState {
        request: Arc<Mutex<Option<(HeaderMap, Value)>>>,
        response: Value,
    }

    async fn capture_embedding_request(
        State(state): State<MockState>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        *state.request.lock().await = Some((headers, body));
        Json(state.response)
    }

    async fn spawn_mock_server(
        path: &'static str,
        response: Value,
    ) -> (
        String,
        Arc<Mutex<Option<(HeaderMap, Value)>>>,
        JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let request = Arc::new(Mutex::new(None));
        let state = MockState {
            request: request.clone(),
            response,
        };
        let router = Router::new()
            .route(path, post(capture_embedding_request))
            .with_state(state);
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        (format!("http://{address}"), request, server)
    }

    #[test]
    fn builds_provider_specific_endpoints() {
        assert_eq!(
            embedding_endpoint(&ollama_settings()).unwrap(),
            "http://localhost:11434/api/embed"
        );
        assert_eq!(
            embedding_endpoint(&EmbeddingSettings {
                base_url: "http://localhost:11434/api".to_string(),
                ..ollama_settings()
            })
            .unwrap(),
            "http://localhost:11434/api/embed"
        );
        assert_eq!(
            embedding_endpoint(&openai_settings()).unwrap(),
            "https://example.test/v1/embeddings"
        );
    }

    #[test]
    fn builds_ollama_and_openai_payloads() {
        let inputs = vec!["one".to_string(), "two".to_string()];
        let ollama = embedding_payload(&ollama_settings(), &inputs);
        assert_eq!(ollama["model"], "test-embedding");
        assert_eq!(ollama["input"], json!(["one", "two"]));
        assert_eq!(ollama["truncate"], true);

        let mut openai_settings = openai_settings();
        openai_settings.dimensions = Some(768);
        let openai = embedding_payload(&openai_settings, &inputs);
        assert_eq!(openai["encoding_format"], "float");
        assert_eq!(openai["dimensions"], 768);
    }

    #[test]
    fn parses_provider_specific_responses() {
        let ollama = ollama_settings();
        let body = json!({"embeddings": [[0.1, -0.2], [0.3, 0.4]]});
        assert_eq!(
            parse_embedding_response(&ollama, &body, 2).unwrap(),
            vec![vec![0.1, -0.2], vec![0.3, 0.4]]
        );

        let openai = openai_settings();
        let body = json!({"data": [
            {"index": 0, "embedding": [0.1, 0.2]},
            {"index": 1, "embedding": [0.3, 0.4]}
        ]});
        assert_eq!(
            parse_embedding_response(&openai, &body, 2).unwrap(),
            vec![vec![0.1, 0.2], vec![0.3, 0.4]]
        );
    }

    #[test]
    fn reorders_openai_embeddings_by_index() {
        let settings = openai_settings();
        let body = json!({"data": [
            {"index": 1, "embedding": [0.3, 0.4]},
            {"index": 0, "embedding": [0.1, 0.2]}
        ]});
        assert_eq!(
            parse_embedding_response(&settings, &body, 2).unwrap(),
            vec![vec![0.1, 0.2], vec![0.3, 0.4]]
        );
    }

    #[test]
    fn rejects_invalid_openai_embedding_indexes() {
        let settings = openai_settings();
        let duplicate = json!({"data": [
            {"index": 0, "embedding": [0.1, 0.2]},
            {"index": 0, "embedding": [0.3, 0.4]}
        ]});
        assert!(parse_embedding_response(&settings, &duplicate, 2).is_err());

        let missing = json!({"data": [
            {"index": 0, "embedding": [0.1, 0.2]},
            {"index": 2, "embedding": [0.3, 0.4]}
        ]});
        assert!(parse_embedding_response(&settings, &missing, 2).is_err());
    }

    #[tokio::test]
    async fn sends_native_ollama_request_to_embed_endpoint() {
        let _server_guard = MOCK_SERVER_LOCK.lock().await;
        let (base_url, request, server) =
            spawn_mock_server("/api/embed", json!({"embeddings": [[0.1, 0.2]]})).await;
        let settings = EmbeddingSettings {
            base_url,
            request_interval_seconds: 0,
            ..ollama_settings()
        };
        let inputs = vec!["one".to_string()];
        let vectors = generate_embeddings(&settings, &inputs).await.unwrap();
        assert_eq!(vectors, vec![vec![0.1, 0.2]]);
        let (_, body) = request.lock().await.clone().unwrap();
        assert_eq!(body["model"], "test-embedding");
        assert_eq!(body["input"], json!(["one"]));
        assert_eq!(body["truncate"], true);
        server.abort();
    }

    #[tokio::test]
    async fn sends_openai_compatible_request_and_bearer_key() {
        let _server_guard = MOCK_SERVER_LOCK.lock().await;
        let (base_url, request, server) = spawn_mock_server(
            "/v1/embeddings",
            json!({"data": [{"index": 0, "embedding": [0.3, 0.4]}]}),
        )
        .await;
        let settings = EmbeddingSettings {
            base_url: format!("{base_url}/v1"),
            auth_mode: EmbeddingAuthMode::Bearer,
            api_key: Some("test-key".to_string()),
            dimensions: Some(2),
            request_interval_seconds: 0,
            ..openai_settings()
        };
        let inputs = vec!["two".to_string()];
        let vectors = generate_embeddings(&settings, &inputs).await.unwrap();
        assert_eq!(vectors, vec![vec![0.3, 0.4]]);
        let (headers, body) = request.lock().await.clone().unwrap();
        assert_eq!(
            headers
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some("Bearer test-key")
        );
        assert_eq!(body["model"], "embedding-model");
        assert_eq!(body["input"], json!(["two"]));
        assert_eq!(body["dimensions"], 2);
        assert_eq!(body["encoding_format"], "float");
        server.abort();
    }

    #[test]
    fn preserves_existing_key_for_connection_tests() {
        let mut stored = ollama_settings();
        stored.auth_mode = EmbeddingAuthMode::Bearer;
        stored.api_key = Some("secret".to_string());
        let provided = EmbeddingSettings {
            auth_mode: EmbeddingAuthMode::Bearer,
            ..ollama_settings()
        };
        let effective = embedding_settings_for_connection_test(&stored, provided).unwrap();
        assert_eq!(effective.api_key.as_deref(), Some("secret"));
        assert!(effective.api_key_configured);
    }

    #[test]
    fn does_not_reuse_key_after_endpoint_changes() {
        let mut stored = ollama_settings();
        stored.auth_mode = EmbeddingAuthMode::Bearer;
        stored.api_key = Some("secret".to_string());
        let provided = EmbeddingSettings {
            base_url: "http://localhost:11435".to_string(),
            auth_mode: EmbeddingAuthMode::Bearer,
            ..ollama_settings()
        };
        assert!(embedding_settings_for_connection_test(&stored, provided).is_err());
    }

    #[test]
    fn bearer_auth_requires_https_for_remote_hosts() {
        let settings = EmbeddingSettings {
            base_url: "http://embedding.example/v1".to_string(),
            provider: EmbeddingProvider::OpenaiCompatible,
            auth_mode: EmbeddingAuthMode::Bearer,
            api_key: Some("secret".to_string()),
            ..EmbeddingSettings::default()
        };
        let error = validate_embedding_settings(&settings, Some("secret")).unwrap_err();
        assert!(error.to_string().contains("requires HTTPS"));
    }

    #[test]
    fn rejects_base_url_query_and_fragment() {
        let settings = EmbeddingSettings {
            base_url: "http://localhost:11434?tenant=one".to_string(),
            ..EmbeddingSettings::default()
        };
        assert!(embedding_endpoint(&settings).is_err());
    }
}
