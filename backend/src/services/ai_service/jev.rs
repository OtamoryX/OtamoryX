//! Dedicated OpenRouter Alpha Decisions transport for observing tag relations.
//!
//! JEV is a typed decision endpoint, not an OpenAI-compatible Chat Completions model. The
//! request builder therefore owns its protocol and sends only the bounded tag pair metadata.

use super::*;
use crate::models::AISettings;
use crate::services::recommendations::semantic_edges::{
    combine_bidirectional_choices, parse_choice_answers, upsert_tag_semantic_edge, TagRelationPair,
    TagSemanticEdgeWrite,
};
use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

const CHOICE_TASK: &str = "tag_pair_relation";
const CHOICE_SAME_MEANING: &str =
    "The two tag labels have the same meaning for recommendation identity and can replace each other.";
const CHOICE_RELATED: &str =
    "The labels are related but describe different non-replaceable concepts, attributes, or forms.";
const CHOICE_BROADER: &str =
    "One label is broader or narrower than the other, so replacing it changes scope.";
const CHOICE_UNRELATED: &str =
    "The labels are not semantically related for this tag relation decision.";
const CHOICE_UNCERTAIN: &str =
    "The two tag names and namespaces are insufficient to make a reliable relation decision.";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TagRelationBatchPayload {
    pairs: Vec<TagRelationPair>,
}

#[derive(Debug, Default)]
struct JEVResponse {
    body: Value,
    provider: Option<String>,
    model: Option<String>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    total_tokens: Option<i64>,
    cost_usd: Option<f64>,
    latency_ms: i64,
}

#[derive(Debug, Default, serde::Deserialize)]
struct UsageRecord {
    input_tokens: Option<i64>,
    prompt_tokens: Option<i64>,
    output_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    total_tokens: Option<i64>,
    cost: Option<f64>,
    total_cost: Option<f64>,
}

/// Enqueues only newly selected, bounded candidates. The caller is responsible for selecting
/// candidates from deterministic co-occurrence data; this function never scans the tag table.
pub async fn enqueue_tag_relation_jev_candidates(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    candidates: &[TagRelationPair],
) -> Result<BackfillResult> {
    let config = &settings.features.recommendations.tag_relation;
    if !config.enabled || config.transport != "openrouterAlphaDecisions" {
        return Ok(BackfillResult::default());
    }
    let Some(profile_id) = select_relation_profile_id(settings) else {
        return Ok(BackfillResult::default());
    };
    let selected = settings_for_profile(settings, Some(&profile_id))?;
    if !tag_relation_profile_is_compatible(&selected) {
        // A missing key is a deliberate no-op. It must not create queue work that can never
        // issue a request, and it keeps the default local installation completely idle.
        return Ok(BackfillResult {
            queued: 0,
            skipped: candidates.len(),
        });
    }

    let mut unique = BTreeMap::new();
    for candidate in candidates.iter().take(config.max_pairs_per_trigger) {
        let pair = candidate.clone().canonicalize()?;
        unique.entry(pair.pair_id.clone()).or_insert(pair);
    }
    let pairs = filter_uncached_pairs(
        pool,
        unique.into_values().collect::<Vec<_>>(),
        &profile_id,
        config,
    )
    .await?;
    if pairs.is_empty() {
        return Ok(BackfillResult::default());
    }

    let batch_size = config.batch_size.clamp(1, 4);
    let mut queued = 0;
    for batch in pairs.chunks(batch_size) {
        let payload = TagRelationBatchPayload {
            pairs: batch.to_vec(),
        };
        let serialized = serde_json::to_string(&payload)?;
        let dedupe_key = tag_relation_dedupe_key(
            batch,
            &profile_id,
            &config.protocol_version,
            &config.prompt_version,
            &config.schema_version,
            &config.model,
        );
        let source_hash = sha256_hex(serialized.as_bytes());
        if enqueue_pipeline_job(
            pool,
            None,
            &source_hash,
            TAG_RELATION_JEV_JOB,
            &serialized,
            "llm",
            Some(&profile_id),
            0,
            &dedupe_key,
            ActiveQueueConflict::Ignore,
        )
        .await?
        {
            queued += 1;
        }
    }
    Ok(BackfillResult {
        queued,
        skipped: candidates.len().saturating_sub(queued),
    })
}

fn select_relation_profile_id(settings: &AISettings) -> Option<String> {
    let configured = settings
        .features
        .recommendations
        .tag_relation
        .profile_id
        .trim();
    if configured != "" && configured != "auto" {
        return settings
            .profiles
            .iter()
            .find(|profile| {
                profile.id == configured
                    && profile.enabled
                    && profile.connection.provider == "openaiCompatible"
                    && profile
                        .connection
                        .api_key
                        .as_deref()
                        .is_some_and(|key| !key.trim().is_empty())
            })
            .map(|profile| profile.id.clone());
    }
    settings
        .profiles
        .iter()
        .find(|profile| {
            profile.id == settings.active_profile_id
                && tag_relation_profile_is_compatible_profile(profile)
        })
        .or_else(|| {
            settings
                .profiles
                .iter()
                .find(|profile| tag_relation_profile_is_compatible_profile(profile))
        })
        .map(|profile| profile.id.clone())
}

fn tag_relation_profile_is_compatible_profile(
    profile: &crate::models::AIConnectionProfile,
) -> bool {
    profile.enabled
        && profile.connection.provider == "openaiCompatible"
        && profile
            .connection
            .api_key
            .as_deref()
            .is_some_and(|key| !key.trim().is_empty())
}

pub(super) fn tag_relation_profile_is_compatible(settings: &AISettings) -> bool {
    settings
        .profiles
        .iter()
        .find(|profile| profile.id == settings.active_profile_id)
        .is_some_and(tag_relation_profile_is_compatible_profile)
}

async fn filter_uncached_pairs(
    pool: &Pool<Sqlite>,
    pairs: Vec<TagRelationPair>,
    profile_id: &str,
    config: &crate::models::AITagRelationSettings,
) -> Result<Vec<TagRelationPair>> {
    if pairs.is_empty() {
        return Ok(pairs);
    }

    // Keep the lookup bounded to the candidate batch. Eight bind values per pair keeps each
    // SQLite statement below its parameter limit while avoiding an all-table scan.
    const LOOKUP_BATCH_SIZE: usize = 100;
    let mut cached_pairs = BTreeSet::new();
    for batch in pairs.chunks(LOOKUP_BATCH_SIZE) {
        let clauses = batch
            .iter()
            .map(|_| {
                "(tag_a_id = ? AND tag_b_id = ? AND pair_input_hash = ? AND \
                 candidate_algorithm_version = ? AND protocol_version = ? AND \
                 prompt_version = ? AND schema_version = ? AND profile_id = ?)"
            })
            .collect::<Vec<_>>()
            .join(" OR ");
        let query = format!(
            "SELECT tag_a_id, tag_b_id FROM tag_semantic_edges \
             WHERE status IN ('observing', 'uncertain', 'failed') AND ({clauses})"
        );
        let mut request = sqlx::query(&query);
        for pair in batch {
            request = request
                .bind(&pair.tag_a.id)
                .bind(&pair.tag_b.id)
                .bind(&pair.pair_input_hash)
                .bind(&config.candidate_algorithm_version)
                .bind(&config.protocol_version)
                .bind(&config.prompt_version)
                .bind(&config.schema_version)
                .bind(profile_id);
        }
        for row in request.fetch_all(pool).await? {
            cached_pairs.insert((
                row.try_get::<String, _>("tag_a_id")?,
                row.try_get::<String, _>("tag_b_id")?,
            ));
        }
    }

    Ok(pairs
        .into_iter()
        .filter(|pair| !cached_pairs.contains(&(pair.tag_a.id.clone(), pair.tag_b.id.clone())))
        .collect())
}

fn tag_relation_dedupe_key(
    pairs: &[TagRelationPair],
    profile_id: &str,
    protocol_version: &str,
    prompt_version: &str,
    schema_version: &str,
    model: &str,
) -> String {
    let mut values = pairs
        .iter()
        .map(|pair| pair.pair_input_hash.as_str())
        .collect::<Vec<_>>();
    values.sort_unstable();
    format!(
        "jev:{profile_id}:{model}:{protocol_version}:{prompt_version}:{schema_version}:{}",
        values.join(",")
    )
}

fn sha256_hex(input: &[u8]) -> String {
    Sha256::digest(input)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) async fn process_tag_relation_jev_job(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    job: &ClaimedJob,
    request_context: &AIRequestContext,
) -> Result<()> {
    let config = &settings.features.recommendations.tag_relation;
    if !config.enabled {
        return Err(anyhow!("JEV tag relation lane is disabled"));
    }
    let payload = job
        .payload
        .as_deref()
        .ok_or_else(|| anyhow!("JEV tag relation job has no payload"))?;
    let mut batch: TagRelationBatchPayload =
        serde_json::from_str(payload).context("JEV tag relation payload is invalid JSON")?;
    if batch.pairs.is_empty() || batch.pairs.len() > config.batch_size.max(1).min(4) {
        return Err(anyhow!(
            "JEV tag relation payload has an invalid batch size"
        ));
    }
    for pair in &mut batch.pairs {
        let canonical = pair.clone().canonicalize()?;
        if canonical.pair_id != pair.pair_id || canonical.pair_input_hash != pair.pair_input_hash {
            return Err(anyhow!("JEV tag relation payload is not canonical"));
        }
        *pair = canonical;
    }

    let forward =
        request_choice_batch(settings, config, &batch.pairs, false, request_context).await?;
    if settings.connection.request_interval_seconds > 0 {
        tokio::time::sleep(Duration::from_secs(
            settings.connection.request_interval_seconds,
        ))
        .await;
    }
    let reverse =
        request_choice_batch(settings, config, &batch.pairs, true, request_context).await?;
    let expected_ids = batch
        .pairs
        .iter()
        .map(|pair| pair.pair_id.clone())
        .collect::<Vec<_>>();
    let forward_answers = match parse_choice_answers(&forward.body, &expected_ids) {
        Ok(answers) => answers,
        Err(error) => {
            mark_batch_failed(pool, &batch.pairs, settings, job, &error.to_string()).await?;
            return Ok(());
        }
    };
    let reverse_answers = match parse_choice_answers(&reverse.body, &expected_ids) {
        Ok(answers) => answers,
        Err(error) => {
            mark_batch_failed(pool, &batch.pairs, settings, job, &error.to_string()).await?;
            return Ok(());
        }
    };

    for pair in &batch.pairs {
        let forward_answer = forward_answers
            .get(&pair.pair_id)
            .expect("validated pair id must be present");
        let reverse_answer = reverse_answers
            .get(&pair.pair_id)
            .expect("validated pair id must be present");
        let decision =
            combine_bidirectional_choices(forward_answer, reverse_answer, config.min_confidence);
        upsert_tag_semantic_edge(
            pool,
            &TagSemanticEdgeWrite {
                pair: pair.clone(),
                decision,
                forward: forward_answer.clone(),
                reverse: reverse_answer.clone(),
                candidate_algorithm_version: config.candidate_algorithm_version.clone(),
                protocol_version: config.protocol_version.clone(),
                prompt_version: config.prompt_version.clone(),
                schema_version: config.schema_version.clone(),
                profile_id: Some(settings.active_profile_id.clone()),
                provider: forward
                    .provider
                    .clone()
                    .or_else(|| reverse.provider.clone()),
                model: forward.model.clone().or_else(|| reverse.model.clone()),
                input_tokens: sum_optional(forward.input_tokens, reverse.input_tokens),
                output_tokens: sum_optional(forward.output_tokens, reverse.output_tokens),
                total_tokens: sum_optional(forward.total_tokens, reverse.total_tokens),
                cost_usd: sum_optional_f64(forward.cost_usd, reverse.cost_usd),
                latency_ms: Some(forward.latency_ms.saturating_add(reverse.latency_ms)),
                attempts: 1,
                last_error: None,
            },
        )
        .await?;
    }
    Ok(())
}

async fn mark_batch_failed(
    pool: &Pool<Sqlite>,
    pairs: &[TagRelationPair],
    settings: &AISettings,
    job: &ClaimedJob,
    error: &str,
) -> Result<()> {
    for pair in pairs {
        crate::services::recommendations::semantic_edges::mark_tag_semantic_edge_failed(
            pool,
            pair,
            &settings.features.recommendations.tag_relation,
            Some(settings.active_profile_id.as_str()),
            error,
            job.id.as_str(),
        )
        .await?;
    }
    Ok(())
}

fn sum_optional(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.saturating_add(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn sum_optional_f64(left: Option<f64>, right: Option<f64>) -> Option<f64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left + right),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

async fn request_choice_batch(
    settings: &AISettings,
    config: &crate::models::AITagRelationSettings,
    pairs: &[TagRelationPair],
    reverse: bool,
    request_context: &AIRequestContext,
) -> Result<JEVResponse> {
    let endpoint = config.endpoint.trim();
    if !(endpoint.starts_with("https://") || endpoint.starts_with("http://")) {
        return Err(anyhow!(
            "JEV Alpha Decisions endpoint must use http:// or https://"
        ));
    }
    let outbound_pairs = pairs
        .iter()
        .map(|pair| {
            let (left, right) = if reverse {
                (&pair.tag_b, &pair.tag_a)
            } else {
                (&pair.tag_a, &pair.tag_b)
            };
            json!({
                "id": pair.pair_id,
                "left": {"namespace": left.namespace, "name": left.name},
                "right": {"namespace": right.namespace, "name": right.name},
            })
        })
        .collect::<Vec<_>>();
    let questions = pairs
        .iter()
        .map(|pair| {
            (
                pair.pair_id.clone(),
                json!({
                    "type": "choice",
                    "instructions": format!(
                        "Judge only the pair in state.pairs with id={}. Use only the two tag objects' namespace and name. Do not use counts or any other context. Select exactly one relation label from the provided criteria. Use uncertain when the names do not support a reliable decision.",
                        pair.pair_id
                    ),
                    "criteria": {
                        "same_meaning": CHOICE_SAME_MEANING,
                        "related_nonreplaceable": CHOICE_RELATED,
                        "broader_or_narrower": CHOICE_BROADER,
                        "unrelated": CHOICE_UNRELATED,
                        "uncertain": CHOICE_UNCERTAIN,
                    }
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let payload = json!({
        "model": config.model,
        "state": {"task": CHOICE_TASK, "pairs": outbound_pairs},
        "questions": questions,
    });

    let started = Instant::now();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(
            settings.connection.timeout_seconds.clamp(5, 3_600),
        ))
        .build()?;
    let request = apply_request_context(
        profile_authenticated_post(&client, endpoint, settings)?.json(&payload),
        Some(request_context),
    );
    let response = request.send().await.map_err(|error| {
        anyhow::Error::new(ProviderRequestError::unavailable(
            format!("JEV Alpha Decisions request failed: {error}"),
            None,
        ))
    })?;
    let status = response.status();
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<i64>().ok());
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        let message = format!("JEV Alpha Decisions returned HTTP {}", status);
        if status.as_u16() == 429 || status.is_server_error() {
            return Err(anyhow::Error::new(ProviderRequestError::unavailable(
                message,
                retry_after,
            )));
        }
        return Err(anyhow!("{message}"));
    }
    let body: Value = serde_json::from_str(&body).context("JEV response was not JSON")?;
    let usage = body
        .get("usage")
        .cloned()
        .and_then(|value| serde_json::from_value::<UsageRecord>(value).ok())
        .unwrap_or_default();
    let input_tokens = usage.input_tokens.or(usage.prompt_tokens);
    let output_tokens = usage.output_tokens.or(usage.completion_tokens);
    let total_tokens = usage
        .total_tokens
        .or_else(|| sum_optional(input_tokens, output_tokens));
    let cost_usd = body
        .get("cost")
        .and_then(Value::as_f64)
        .or(usage.cost)
        .or(usage.total_cost);
    Ok(JEVResponse {
        provider: body
            .get("provider")
            .and_then(Value::as_str)
            .map(str::to_string),
        model: body
            .get("model")
            .and_then(Value::as_str)
            .map(str::to_string),
        input_tokens,
        output_tokens,
        total_tokens,
        cost_usd,
        latency_ms: started.elapsed().as_millis().min(i64::MAX as u128) as i64,
        body,
    })
}

fn profile_authenticated_post(
    client: &reqwest::Client,
    endpoint: &str,
    settings: &AISettings,
) -> Result<reqwest::RequestBuilder> {
    let request = client.post(endpoint);
    match settings.connection.auth_mode {
        crate::models::AIAuthMode::None => Ok(request),
        crate::models::AIAuthMode::Bearer => settings
            .connection
            .api_key
            .as_deref()
            .filter(|key| !key.trim().is_empty())
            .map(|key| request.bearer_auth(key))
            .ok_or_else(|| anyhow!("No API key is configured on the selected AI profile")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::recommendations::semantic_edges::TagRelationTag;

    fn pair() -> TagRelationPair {
        TagRelationPair {
            pair_id: "tag-a:tag-b".to_string(),
            tag_a: TagRelationTag {
                id: "tag-a".to_string(),
                namespace: "general".to_string(),
                name: "romance".to_string(),
                support_count: 2,
            },
            tag_b: TagRelationTag {
                id: "tag-b".to_string(),
                namespace: "general".to_string(),
                name: "romantic".to_string(),
                support_count: 3,
            },
            pair_input_hash: "hash".to_string(),
        }
    }

    #[test]
    fn dedupe_key_changes_with_pair_input_and_versions() {
        let pair = pair().canonicalize().unwrap();
        let first = tag_relation_dedupe_key(
            std::slice::from_ref(&pair),
            "profile",
            "protocol-v1",
            "prompt-v1",
            "schema-v1",
            "jev",
        );
        let second = tag_relation_dedupe_key(
            std::slice::from_ref(&pair),
            "profile",
            "protocol-v2",
            "prompt-v1",
            "schema-v1",
            "jev",
        );
        assert_ne!(first, second);
    }

    #[test]
    fn outbound_payload_does_not_use_support_count() {
        let pair = pair().canonicalize().unwrap();
        let left = &pair.tag_a;
        let payload =
            json!({"id": pair.pair_id, "left": {"namespace": left.namespace, "name": left.name}});
        assert!(!payload.to_string().contains("support_count"));
        assert!(!payload.to_string().contains("archive"));
    }

    #[tokio::test]
    async fn matching_observed_semantic_rows_are_not_requeued() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::database::run_sqlite_migrations(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES
             ('tag-a', 'romance', 'general'), ('tag-b', 'romantic', 'general')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let config = crate::models::AITagRelationSettings::default();
        let canonical = pair().canonicalize().unwrap();
        sqlx::query(
            "INSERT INTO tag_semantic_edges
             (tag_a_id, tag_b_id, relation_kind, status, pair_input_hash,
              candidate_algorithm_version, protocol_version, prompt_version, schema_version,
              profile_id)
             VALUES (?, ?, 'same_meaning', 'observing', ?, ?, ?, ?, ?, ?)",
        )
        .bind(&canonical.tag_a.id)
        .bind(&canonical.tag_b.id)
        .bind(&canonical.pair_input_hash)
        .bind(&config.candidate_algorithm_version)
        .bind(&config.protocol_version)
        .bind(&config.prompt_version)
        .bind(&config.schema_version)
        .bind("profile")
        .execute(&pool)
        .await
        .unwrap();

        for status in ["observing", "uncertain", "failed"] {
            sqlx::query("UPDATE tag_semantic_edges SET status = ?")
                .bind(status)
                .execute(&pool)
                .await
                .unwrap();
            assert!(
                filter_uncached_pairs(&pool, vec![canonical.clone()], "profile", &config,)
                    .await
                    .unwrap()
                    .is_empty(),
                "status {status} should remain cached"
            );
        }

        let mut changed = canonical;
        changed.tag_b.support_count += 1;
        let changed = changed.canonicalize().unwrap();
        assert_eq!(
            filter_uncached_pairs(&pool, vec![changed], "profile", &config)
                .await
                .unwrap()
                .len(),
            0,
            "support count changes must not requeue an unchanged JEV input"
        );
    }
}
