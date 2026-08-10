use anyhow::{anyhow, Context, Result};
use chrono::{Duration as ChronoDuration, Utc};
use reqwest::Client;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};
use std::time::Duration;
use tracing::warn;
use uuid::Uuid;

use crate::models::AISettings;

const SETTINGS_KEY: &str = "ai_settings";
const API_KEY_SETTINGS_KEY: &str = "ai_connection_api_key";
const TITLE_TRANSLATION_JOB: &str = "title_translation";
const MAX_AI_WORKERS: usize = 16;

#[derive(Debug, Clone, Copy, Default)]
pub struct BackfillResult {
    pub queued: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone)]
struct ClaimedJob {
    id: String,
    archive_id: String,
    source_hash: Option<String>,
}

pub async fn load_ai_settings(pool: &Pool<Sqlite>) -> Result<AISettings> {
    let mut settings: AISettings =
        sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
            .bind(SETTINGS_KEY)
            .fetch_optional(pool)
            .await?
            .map(|raw| deserialize_stored_settings(&raw))
            .unwrap_or_default();

    let stored_key = sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(API_KEY_SETTINGS_KEY)
        .fetch_optional(pool)
        .await?;
    settings.connection.api_key = stored_key;
    settings.connection.api_key_configured = configured_api_key(&settings).is_some();
    Ok(settings)
}

fn deserialize_stored_settings(raw: &str) -> AISettings {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return AISettings::default();
    };
    if value.get("connection").is_some() {
        return serde_json::from_value(value).unwrap_or_default();
    }

    // Preserve the subset of the original flat settings schema that still has a destination in
    // the shared AI configuration. The old scheduler fields had no executing implementation.
    let mut settings = AISettings::default();
    settings.features.auto_tagging.enabled = value
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    settings.features.auto_tagging.auto_apply_threshold = value
        .get("auto_apply_threshold")
        .or_else(|| value.get("autoApplyThreshold"))
        .and_then(Value::as_f64)
        .filter(|threshold| (0.0..=1.0).contains(threshold))
        .map(|threshold| threshold as f32)
        .unwrap_or(settings.features.auto_tagging.auto_apply_threshold);

    let limits = value
        .get("resource_limits")
        .or_else(|| value.get("resourceLimits"));
    if let Some(limits) = limits {
        settings.execution.max_concurrent_tasks = limits
            .get("max_concurrent_tasks")
            .or_else(|| limits.get("maxConcurrentTasks"))
            .and_then(Value::as_u64)
            .filter(|count| (1..=MAX_AI_WORKERS as u64).contains(count))
            .map(|count| count as usize)
            .unwrap_or(settings.execution.max_concurrent_tasks);
        settings.execution.timeout_seconds = limits
            .get("timeout_seconds")
            .or_else(|| limits.get("timeoutSeconds"))
            .and_then(Value::as_u64)
            .filter(|timeout| (5..=1_800).contains(timeout))
            .unwrap_or(settings.execution.timeout_seconds);
        settings.execution.max_retries = limits
            .get("max_retries")
            .or_else(|| limits.get("maxRetries"))
            .and_then(Value::as_u64)
            .filter(|retries| *retries <= 10)
            .map(|retries| retries as u32)
            .unwrap_or(settings.execution.max_retries);
    }
    settings
}

pub async fn save_ai_settings(pool: &Pool<Sqlite>, mut settings: AISettings) -> Result<()> {
    validate_settings(&settings)?;
    let submitted_key = settings
        .connection
        .api_key
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let stored_json = serde_json::to_string(&settings)?;
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(SETTINGS_KEY)
    .bind(stored_json)
    .execute(pool)
    .await?;

    // Persist API keys independently so ordinary settings reads and responses cannot expose them.
    if let Some(api_key) = submitted_key {
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(API_KEY_SETTINGS_KEY)
        .bind(api_key)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub fn settings_for_response(mut settings: AISettings) -> AISettings {
    settings.connection.api_key_configured = configured_api_key(&settings).is_some();
    settings.connection.api_key = None;
    settings
}

pub async fn enqueue_title_translation(pool: &Pool<Sqlite>, archive_id: &str) -> Result<bool> {
    let settings = load_ai_settings(pool).await?;
    enqueue_title_translation_with_settings(pool, archive_id, &settings).await
}

async fn enqueue_title_translation_with_settings(
    pool: &Pool<Sqlite>,
    archive_id: &str,
    settings: &AISettings,
) -> Result<bool> {
    let feature = &settings.features.title_translation;
    if !feature.enabled {
        return Ok(false);
    }
    let row = sqlx::query(
        "SELECT title, subtitle, subtitle_language, subtitle_source_hash FROM archives WHERE id = ? LIMIT 1",
    )
    .bind(archive_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("archive `{archive_id}` not found"))?;
    let title: String = row.get("title");
    let subtitle: Option<String> = row.get("subtitle");
    let subtitle_language: Option<String> = row.get("subtitle_language");
    let active_hash: Option<String> = row.get("subtitle_source_hash");
    let source_hash = title_hash(&title);

    if feature.skip_if_target_language
        && title_looks_like_target_language(&title, &feature.target_language)
    {
        return Ok(false);
    }
    if active_hash.as_deref() == Some(&source_hash)
        && subtitle.is_some()
        && subtitle_language.as_deref() == Some(feature.target_language.as_str())
    {
        return Ok(false);
    }
    if !feature.retranslate_on_title_change && subtitle.is_some() {
        return Ok(false);
    }

    // A subtitle belongs to both a source title and a target language. Do not show a stale
    // translation while the current title or language is waiting in the AI queue.
    if subtitle.is_some() {
        sqlx::query(
            "UPDATE archives SET subtitle = NULL, subtitle_language = NULL, subtitle_source_hash = NULL, updated_at = ? WHERE id = ?",
        )
        .bind(Utc::now())
        .bind(archive_id)
        .execute(pool)
        .await?;
    }

    let dedupe_key = format!("{TITLE_TRANSLATION_JOB}:{archive_id}:{source_hash}");
    let now = Utc::now();
    let translation_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO archive_title_translations (
            id, archive_id, source_title, source_hash, target_language, status, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, 'pending', ?, ?)
        ON CONFLICT(archive_id, target_language, source_hash) DO UPDATE SET
            status = CASE WHEN archive_title_translations.status = 'completed' THEN 'completed' ELSE 'pending' END,
            last_error = NULL,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(translation_id)
    .bind(archive_id)
    .bind(&title)
    .bind(&source_hash)
    .bind(&feature.target_language)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    let result = sqlx::query(
        r#"
        INSERT OR IGNORE INTO ai_processing_queue (
            id, archive_id, status, priority, attempts, job_type, payload, source_hash, dedupe_key,
            created_at, next_run_at
        ) VALUES (?, ?, 'pending', 0, 0, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(archive_id)
    .bind(TITLE_TRANSLATION_JOB)
    .bind(json!({ "targetLanguage": feature.target_language }).to_string())
    .bind(source_hash)
    .bind(dedupe_key)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn enqueue_title_translation_backfill(pool: &Pool<Sqlite>) -> Result<BackfillResult> {
    let settings = load_ai_settings(pool).await?;
    if !settings.features.title_translation.enabled {
        return Err(anyhow!("Title translation is disabled"));
    }
    let ids = sqlx::query_scalar::<_, String>("SELECT id FROM archives ORDER BY created_at ASC")
        .fetch_all(pool)
        .await?;
    let mut result = BackfillResult::default();
    for archive_id in ids {
        if enqueue_title_translation_with_settings(pool, &archive_id, &settings).await? {
            result.queued += 1;
        } else {
            result.skipped += 1;
        }
    }
    Ok(result)
}

pub fn spawn_ai_worker(pool: Pool<Sqlite>) {
    for slot in 0..MAX_AI_WORKERS {
        let supervisor_pool = pool.clone();
        tokio::spawn(async move {
            loop {
                let worker_pool = supervisor_pool.clone();
                match tokio::spawn(async move { run_ai_worker(worker_pool, slot).await }).await {
                    Ok(()) => warn!(slot, "AI worker stopped unexpectedly; restarting"),
                    Err(err) => {
                        tracing::error!(slot, error = %err, "AI worker panicked; restarting")
                    }
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }
}

async fn run_ai_worker(pool: Pool<Sqlite>, slot: usize) {
    loop {
        let worker_limit = load_ai_settings(&pool)
            .await
            .map(|settings| {
                settings
                    .execution
                    .max_concurrent_tasks
                    .clamp(1, MAX_AI_WORKERS)
            })
            .unwrap_or(1);
        if slot >= worker_limit {
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        match process_next_job(&pool).await {
            Ok(true) => continue,
            Ok(false) => tokio::time::sleep(Duration::from_secs(2)).await,
            Err(err) => {
                warn!("AI worker iteration failed: {err:#}");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

/// Runs one job at most. Public to make the queue behavior testable without a background worker.
pub async fn process_next_job(pool: &Pool<Sqlite>) -> Result<bool> {
    release_expired_leases(pool).await?;
    let Some(job) = claim_next_job(pool).await? else {
        return Ok(false);
    };
    let settings = load_ai_settings(pool).await?;
    if !settings.features.title_translation.enabled {
        // Do not consume jobs while the feature is disabled; it can be re-enabled later.
        sqlx::query("UPDATE ai_processing_queue SET status = 'pending', started_at = NULL, lease_expires_at = NULL WHERE id = ?")
            .bind(&job.id)
            .execute(pool)
            .await?;
        return Ok(false);
    }

    match process_title_translation_job(pool, &settings, &job).await {
        Ok(()) => complete_job(pool, &job.id).await?,
        Err(err) => {
            fail_or_retry_job(
                pool,
                &settings,
                &job.id,
                &job.archive_id,
                job.source_hash.as_deref(),
                &err.to_string(),
            )
            .await?
        }
    }
    Ok(true)
}

async fn release_expired_leases(pool: &Pool<Sqlite>) -> Result<()> {
    sqlx::query(
        "UPDATE ai_processing_queue SET status = 'pending', started_at = NULL, lease_expires_at = NULL \
         WHERE status = 'processing' AND lease_expires_at IS NOT NULL \
           AND julianday(lease_expires_at) < julianday('now')",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn claim_next_job(pool: &Pool<Sqlite>) -> Result<Option<ClaimedJob>> {
    let row = sqlx::query(
        r#"
        SELECT id, archive_id, source_hash
        FROM ai_processing_queue
        WHERE status = 'pending' AND job_type = ?
          AND (next_run_at IS NULL OR julianday(next_run_at) <= julianday('now'))
        ORDER BY priority DESC, created_at ASC
        LIMIT 1
        "#,
    )
    .bind(TITLE_TRANSLATION_JOB)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else { return Ok(None) };
    let job = ClaimedJob {
        id: row.get("id"),
        archive_id: row.get("archive_id"),
        source_hash: row.try_get("source_hash")?,
    };
    let lease_expires_at = Utc::now() + ChronoDuration::minutes(10);
    let claimed = sqlx::query(
        "UPDATE ai_processing_queue SET status = 'processing', attempts = attempts + 1, started_at = CURRENT_TIMESTAMP, lease_expires_at = ? WHERE id = ? AND status = 'pending'",
    )
    .bind(lease_expires_at)
    .bind(&job.id)
    .execute(pool)
    .await?;
    Ok((claimed.rows_affected() == 1).then_some(job))
}

async fn process_title_translation_job(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    job: &ClaimedJob,
) -> Result<()> {
    let source_hash = job
        .source_hash
        .as_deref()
        .ok_or_else(|| anyhow!("title translation job has no source hash"))?;
    let row = sqlx::query("SELECT title FROM archives WHERE id = ? LIMIT 1")
        .bind(&job.archive_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| anyhow!("archive deleted before translation"))?;
    let title: String = row.get("title");
    if title_hash(&title) != source_hash {
        return Err(anyhow!("archive title changed before translation"));
    }

    let translated = translate_title(settings, &title).await?;
    if translated.trim().is_empty() || translated.len() > 1_000 {
        return Err(anyhow!("model returned an invalid translated title"));
    }
    let feature = &settings.features.title_translation;
    let now = Utc::now();
    sqlx::query(
        r#"
        UPDATE archive_title_translations
        SET translated_title = ?, status = 'completed', provider = ?, model = ?, last_error = NULL,
            completed_at = ?, updated_at = ?
        WHERE archive_id = ? AND source_hash = ? AND target_language = ?
        "#,
    )
    .bind(&translated)
    .bind(&settings.connection.provider)
    .bind(&settings.connection.model)
    .bind(now)
    .bind(now)
    .bind(&job.archive_id)
    .bind(source_hash)
    .bind(&feature.target_language)
    .execute(pool)
    .await?;
    let updated = sqlx::query(
        "UPDATE archives SET subtitle = ?, subtitle_language = ?, subtitle_source_hash = ?, updated_at = ? WHERE id = ? AND title = ?",
    )
    .bind(translated)
    .bind(&feature.target_language)
    .bind(source_hash)
    .bind(now)
    .bind(&job.archive_id)
    .bind(&title)
    .execute(pool)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(anyhow!("archive title changed while writing translation"));
    }
    Ok(())
}

async fn complete_job(pool: &Pool<Sqlite>, job_id: &str) -> Result<()> {
    sqlx::query(
        "UPDATE ai_processing_queue SET status = 'completed', completed_at = CURRENT_TIMESTAMP, lease_expires_at = NULL, last_error = NULL WHERE id = ?",
    )
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn fail_or_retry_job(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    job_id: &str,
    archive_id: &str,
    source_hash: Option<&str>,
    error: &str,
) -> Result<()> {
    let attempts: i64 = sqlx::query_scalar("SELECT attempts FROM ai_processing_queue WHERE id = ?")
        .bind(job_id)
        .fetch_one(pool)
        .await?;
    let final_failure = attempts >= settings.execution.max_retries.max(1) as i64;
    let status = if final_failure { "failed" } else { "pending" };
    let retry_at = Utc::now() + ChronoDuration::seconds((attempts * 10).min(300));
    sqlx::query(
        "UPDATE ai_processing_queue SET status = ?, last_error = ?, next_run_at = ?, completed_at = CASE WHEN ? = 'failed' THEN CURRENT_TIMESTAMP ELSE NULL END, lease_expires_at = NULL WHERE id = ?",
    )
    .bind(status)
    .bind(error)
    .bind(retry_at)
    .bind(status)
    .bind(job_id)
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE archive_title_translations SET status = ?, last_error = ?, updated_at = CURRENT_TIMESTAMP WHERE archive_id = ? AND source_hash = ?",
    )
    .bind(status)
    .bind(error)
    .bind(archive_id)
    .bind(source_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn test_connection(settings: &AISettings) -> Result<()> {
    let key = configured_api_key(settings).ok_or_else(|| anyhow!("No AI API key is configured"))?;
    let endpoint = chat_completions_endpoint(&settings.connection.base_url)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(
            settings.execution.timeout_seconds.clamp(5, 300),
        ))
        .build()?;
    let response = client
        .post(endpoint)
        .bearer_auth(key)
        .json(&json!({
            "model": settings.connection.model,
            "temperature": 0,
            "max_tokens": 4,
            "messages": [{ "role": "user", "content": "Reply exactly: OK" }]
        }))
        .send()
        .await
        .context("AI connection request failed")?;
    if !response.status().is_success() {
        return Err(anyhow!("AI provider returned HTTP {}", response.status()));
    }
    Ok(())
}

async fn translate_title(settings: &AISettings, title: &str) -> Result<String> {
    let key = configured_api_key(settings).ok_or_else(|| anyhow!("No AI API key is configured"))?;
    let endpoint = chat_completions_endpoint(&settings.connection.base_url)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(
            settings.execution.timeout_seconds.clamp(5, 300),
        ))
        .build()?;
    let target = &settings.features.title_translation.target_language;
    let prompt = format!(
        "Translate this comic title into {target}. Return only the translated title. Preserve volume/chapter numbers, bracketed content, proper names, author/circle names, and rating markers. Do not add explanations.\\n\\nTitle: {title}"
    );
    let response = client
        .post(endpoint)
        .bearer_auth(key)
        .json(&json!({
            "model": settings.connection.model,
            "temperature": 0.1,
            "messages": [
                { "role": "system", "content": "You translate comic metadata faithfully and return only the title." },
                { "role": "user", "content": prompt }
            ]
        }))
        .send()
        .await
        .context("AI title translation request failed")?;
    if !response.status().is_success() {
        return Err(anyhow!("AI provider returned HTTP {}", response.status()));
    }
    let body: Value = response
        .json()
        .await
        .context("Invalid AI provider response")?;
    let content = body
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("AI provider response has no assistant content"))?;
    normalize_model_title(content)
}

fn configured_api_key(settings: &AISettings) -> Option<String> {
    std::env::var("AI_PROVIDER_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            settings
                .connection
                .api_key
                .clone()
                .filter(|value| !value.trim().is_empty())
        })
}

fn chat_completions_endpoint(base_url: &str) -> Result<String> {
    let base = base_url.trim().trim_end_matches('/');
    if !(base.starts_with("https://") || base.starts_with("http://")) {
        return Err(anyhow!("AI base URL must use http:// or https://"));
    }
    Ok(if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{base}/chat/completions")
    })
}

fn normalize_model_title(content: &str) -> Result<String> {
    let stripped = content.trim().trim_matches('`').trim();
    if let Ok(value) = serde_json::from_str::<Value>(stripped) {
        if let Some(title) = value.get("title").and_then(Value::as_str) {
            return normalize_model_title(title);
        }
    }
    let single_line = stripped.lines().next().unwrap_or("").trim();
    if single_line.is_empty() {
        return Err(anyhow!("AI provider returned an empty title"));
    }
    Ok(single_line.to_string())
}

fn validate_settings(settings: &AISettings) -> Result<()> {
    if settings.connection.provider != "openaiCompatible" {
        return Err(anyhow!(
            "Only the openaiCompatible provider is currently supported"
        ));
    }
    chat_completions_endpoint(&settings.connection.base_url)?;
    if settings.connection.model.trim().is_empty() {
        return Err(anyhow!("AI model must not be empty"));
    }
    if settings
        .features
        .title_translation
        .target_language
        .trim()
        .is_empty()
    {
        return Err(anyhow!(
            "Title translation target language must not be empty"
        ));
    }
    if settings.execution.max_concurrent_tasks == 0 || settings.execution.max_concurrent_tasks > 16
    {
        return Err(anyhow!("AI maxConcurrentTasks must be between 1 and 16"));
    }
    Ok(())
}

pub fn title_hash(title: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(title.trim().as_bytes());
    format!("{:x}", hasher.finalize())
}

fn title_looks_like_target_language(title: &str, language: &str) -> bool {
    let letters: Vec<char> = title.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return false;
    }
    let language = language.to_ascii_lowercase();
    if language.starts_with("zh") {
        return letters
            .iter()
            .filter(|c| matches!(**c as u32, 0x4E00..=0x9FFF))
            .count()
            * 2
            >= letters.len();
    }
    if language.starts_with("ja") {
        return letters
            .iter()
            .filter(|c| matches!(**c as u32, 0x3040..=0x30FF | 0x4E00..=0x9FFF))
            .count()
            * 2
            >= letters.len();
    }
    if language.starts_with("ko") {
        return letters
            .iter()
            .filter(|c| matches!(**c as u32, 0xAC00..=0xD7AF))
            .count()
            * 2
            >= letters.len();
    }
    language.starts_with("en") && letters.iter().all(|c| c.is_ascii())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn hashes_trimmed_title_and_detects_target_scripts() {
        assert_eq!(title_hash(" title "), title_hash("title"));
        assert!(title_looks_like_target_language("中文标题 Vol. 1", "zh-CN"));
        assert!(!title_looks_like_target_language("English title", "zh-CN"));
    }

    #[test]
    fn normalizes_plain_or_json_model_responses() {
        assert_eq!(
            normalize_model_title("  translated title  ").unwrap(),
            "translated title"
        );
        assert_eq!(
            normalize_model_title(r#"{"title":"译名"}"#).unwrap(),
            "译名"
        );
        assert!(chat_completions_endpoint("example.com").is_err());
    }

    #[test]
    fn settings_responses_never_serialize_api_keys() {
        let mut settings = AISettings::default();
        settings.connection.api_key = Some("secret-value".to_string());
        settings.connection.api_key_configured = true;
        let response = serde_json::to_string(&settings_for_response(settings)).unwrap();
        assert!(!response.contains("secret-value"));
        assert!(response.contains("apiKeyConfigured"));
    }

    #[test]
    fn preserves_legacy_ai_settings_when_reading_the_new_schema() {
        let settings = deserialize_stored_settings(
            r#"{
                "enabled": true,
                "auto_apply_threshold": 0.65,
                "resource_limits": {
                    "max_concurrent_tasks": 4,
                    "timeout_seconds": 180,
                    "max_retries": 5
                }
            }"#,
        );
        assert!(settings.features.auto_tagging.enabled);
        assert_eq!(settings.features.auto_tagging.auto_apply_threshold, 0.65);
        assert_eq!(settings.execution.max_concurrent_tasks, 4);
        assert_eq!(settings.execution.timeout_seconds, 180);
        assert_eq!(settings.execution.max_retries, 5);
    }

    #[tokio::test]
    async fn queues_a_single_job_for_an_unchanged_title_without_network() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for statement in [
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at DATETIME)",
            "CREATE TABLE archives (id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT, subtitle_source_hash TEXT, updated_at DATETIME)",
            "CREATE TABLE archive_title_translations (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, source_title TEXT NOT NULL, source_hash TEXT NOT NULL, target_language TEXT NOT NULL, translated_title TEXT, status TEXT NOT NULL, provider TEXT, model TEXT, last_error TEXT, created_at DATETIME, updated_at DATETIME, completed_at DATETIME, UNIQUE(archive_id, target_language, source_hash))",
            "CREATE TABLE ai_processing_queue (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, status TEXT NOT NULL, priority INTEGER NOT NULL, attempts INTEGER NOT NULL, last_error TEXT, created_at DATETIME, started_at DATETIME, completed_at DATETIME, job_type TEXT NOT NULL, payload TEXT, source_hash TEXT, dedupe_key TEXT, next_run_at DATETIME, lease_expires_at DATETIME)",
            "CREATE UNIQUE INDEX ai_jobs_active_dedupe ON ai_processing_queue (dedupe_key) WHERE dedupe_key IS NOT NULL AND status IN ('pending', 'processing')",
        ] {
            sqlx::query(statement).execute(&pool).await.unwrap();
        }
        let mut settings = AISettings::default();
        settings.features.title_translation.enabled = true;
        save_ai_settings(&pool, settings).await.unwrap();
        sqlx::query("INSERT INTO archives (id, title) VALUES ('archive-1', 'Original title')")
            .execute(&pool)
            .await
            .unwrap();

        assert!(enqueue_title_translation(&pool, "archive-1").await.unwrap());
        assert!(!enqueue_title_translation(&pool, "archive-1").await.unwrap());
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ai_processing_queue")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn malformed_title_job_does_not_terminate_queue_processing() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for statement in [
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at DATETIME)",
            "CREATE TABLE archives (id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT, subtitle_source_hash TEXT, updated_at DATETIME)",
            "CREATE TABLE archive_title_translations (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, source_title TEXT NOT NULL, source_hash TEXT NOT NULL, target_language TEXT NOT NULL, translated_title TEXT, status TEXT NOT NULL, provider TEXT, model TEXT, last_error TEXT, created_at DATETIME, updated_at DATETIME, completed_at DATETIME, UNIQUE(archive_id, target_language, source_hash))",
            "CREATE TABLE ai_processing_queue (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, status TEXT NOT NULL, priority INTEGER NOT NULL, attempts INTEGER NOT NULL, last_error TEXT, created_at DATETIME, started_at DATETIME, completed_at DATETIME, job_type TEXT NOT NULL, payload TEXT, source_hash TEXT, dedupe_key TEXT, next_run_at DATETIME, lease_expires_at DATETIME)",
        ] {
            sqlx::query(statement).execute(&pool).await.unwrap();
        }
        let mut settings = AISettings::default();
        settings.features.title_translation.enabled = true;
        settings.execution.max_retries = 1;
        save_ai_settings(&pool, settings).await.unwrap();
        sqlx::query("INSERT INTO ai_processing_queue (id, archive_id, status, priority, attempts, job_type, source_hash) VALUES ('malformed-job', 'archive-1', 'pending', 0, 0, 'title_translation', NULL)")
            .execute(&pool)
            .await
            .unwrap();

        assert!(process_next_job(&pool).await.unwrap());
        let status: String =
            sqlx::query_scalar("SELECT status FROM ai_processing_queue WHERE id = 'malformed-job'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "failed");
    }

    #[tokio::test]
    async fn claims_ready_rfc3339_job_without_claiming_future_job() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE ai_processing_queue (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, status TEXT NOT NULL, priority INTEGER NOT NULL, attempts INTEGER NOT NULL, started_at DATETIME, job_type TEXT NOT NULL, source_hash TEXT, created_at DATETIME, next_run_at DATETIME, lease_expires_at DATETIME)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let today = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let tomorrow = today + ChronoDuration::days(1);
        for (id, priority, next_run_at) in [("ready", 0, today), ("future", 10, tomorrow)] {
            sqlx::query(
                "INSERT INTO ai_processing_queue (id, archive_id, status, priority, attempts, job_type, source_hash, created_at, next_run_at) VALUES (?, ?, 'pending', ?, 0, 'title_translation', 'hash', CURRENT_TIMESTAMP, ?)",
            )
            .bind(id)
            .bind(id)
            .bind(priority)
            .bind(next_run_at)
            .execute(&pool)
            .await
            .unwrap();
        }

        let claimed = claim_next_job(&pool).await.unwrap().unwrap();

        assert_eq!(claimed.id, "ready");
        let future_status: String =
            sqlx::query_scalar("SELECT status FROM ai_processing_queue WHERE id = 'future'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(future_status, "pending");
    }

    #[tokio::test]
    async fn releases_expired_rfc3339_lease_without_releasing_future_lease() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE ai_processing_queue (id TEXT PRIMARY KEY, status TEXT NOT NULL, started_at DATETIME, lease_expires_at DATETIME)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let today = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let tomorrow = today + ChronoDuration::days(1);
        for (id, lease_expires_at) in [("expired", today), ("future", tomorrow)] {
            sqlx::query(
                "INSERT INTO ai_processing_queue (id, status, started_at, lease_expires_at) VALUES (?, 'processing', CURRENT_TIMESTAMP, ?)",
            )
            .bind(id)
            .bind(lease_expires_at)
            .execute(&pool)
            .await
            .unwrap();
        }

        release_expired_leases(&pool).await.unwrap();

        let statuses: Vec<(String, String)> =
            sqlx::query_as("SELECT id, status FROM ai_processing_queue ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(
            statuses,
            vec![
                ("expired".to_string(), "pending".to_string()),
                ("future".to_string(), "processing".to_string()),
            ]
        );
    }
}
