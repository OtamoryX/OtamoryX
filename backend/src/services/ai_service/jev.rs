//! Dedicated Alpha Decisions transports for observing tag relations.
//!
//! JEV is a typed decision endpoint, not an OpenAI-compatible Chat Completions model. The
//! request builder therefore owns its protocol and sends only the bounded tag pair metadata.

use super::*;
use crate::models::AISettings;
use crate::services::recommendations::semantic_edges::TagRelationPair;
use crate::services::recommendations::weighted_graph::{
    upsert_observing_relation_weight, TagRelationWeightWrite,
};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::Mutex;

const SCORE_TASK: &str = "signed_tag_affinity";
const SCORE_PAYLOAD_CONTRACT: &str = "jev-tag-affinity-score-v1";
const SCORE_SCORER_VERSION: &str = "jev-score-affinity-v1";
const SCORE_TOLERANCE: f64 = 0.025;
const ALIAS_CACHE_TTL_HOURS: i64 = 24;
const SCORE_CRITERIA: [&str; 5] = [
    "Strong opposition: the labels express clearly opposing states or ends of the same attribute for compatible referents. Mere difference, different subjects, or non-equivalence is insufficient.",
    "Partial opposition: a meaningful contrast on the same attribute or scope, weaker than a direct opposite. Require actual opposition, not simply different labels.",
    "Neutral: no dependable affinity or opposition, independent dimensions, incompatible referents, or insufficient tag-only context.",
    "Partial affinity: meaningful common concept or attribute with a scope, degree, form, or style difference; useful weak similarity without full interchangeability.",
    "Strong affinity: the labels express essentially the same concept and compatible scope, including equivalent spellings or names across namespaces.",
];

pub(super) const JEV_PROVIDER_IDENTITY: &str = "openrouterAlphaDecisions";
const JEV_EDGE_IDENTITY: &str = "tag_relation_jev";
// Candidate filtering and queue insertion use separate SQL calls. Serialize that short
// critical section so overlapping callers in this process cannot both observe the same
// unqueued pair. The queue transaction and unique active dedupe index still guard exact job
// duplicates; callers in another process are outside this mutex's scope.
static TAG_RELATION_ENQUEUE_LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();

fn tag_relation_enqueue_lock() -> &'static Arc<Mutex<()>> {
    TAG_RELATION_ENQUEUE_LOCK.get_or_init(|| Arc::new(Mutex::new(())))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TagRelationBatchPayload {
    #[serde(default)]
    contract_version: Option<String>,
    #[serde(default)]
    scorer_version: Option<String>,
    pairs: Vec<TagRelationPair>,
}

#[derive(Debug)]
struct JEVResponse {
    body: Value,
    provider: Option<String>,
    model: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct ScoreAnswer {
    score: f64,
    confidence: f64,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid JEV tag relation job payload: {message}")]
pub(crate) struct TagRelationJobValidationError {
    message: String,
}

impl TagRelationJobValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Enqueues only newly selected, bounded candidates. The caller is responsible for selecting
/// candidates from deterministic co-occurrence data; this function never scans the tag table.
pub async fn enqueue_tag_relation_jev_candidates(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    candidates: &[TagRelationPair],
) -> Result<BackfillResult> {
    let config = &settings.features.recommendations.tag_relation;
    if !config.enabled || !tag_relation_transport_supported(&config.transport) {
        return Ok(BackfillResult::default());
    }
    if !tag_relation_has_api_key(config) {
        // A missing key is a deliberate no-op. It must not create queue work that can never
        // issue a request, and it keeps the default local installation completely idle.
        return Ok(BackfillResult {
            queued: 0,
            skipped: candidates.len(),
        });
    }
    let _enqueue_guard = tag_relation_enqueue_lock().lock().await;

    let mut unique = BTreeMap::new();
    for candidate in candidates.iter().take(config.max_pairs_per_trigger) {
        let pair = candidate.clone().canonicalize()?;
        unique.entry(pair.pair_id.clone()).or_insert(pair);
    }
    let scorer_version = tag_relation_scorer_version(config, false);
    let unresolved_version = tag_relation_scorer_version(config, true);
    let pairs = filter_uncached_pairs(
        pool,
        unique.into_values().collect::<Vec<_>>(),
        &config.model,
        &scorer_version,
        &unresolved_version,
    )
    .await?;
    let pairs = filter_active_queued_pairs(pool, pairs, &scorer_version).await?;
    if pairs.is_empty() {
        return Ok(BackfillResult::default());
    }

    let batch_size = config.batch_size.clamp(1, 4);
    let mut queued = 0;
    for batch in pairs.chunks(batch_size) {
        let payload = TagRelationBatchPayload {
            contract_version: Some(SCORE_PAYLOAD_CONTRACT.to_string()),
            scorer_version: Some(scorer_version.clone()),
            pairs: batch.to_vec(),
        };
        let serialized = serde_json::to_string(&payload)?;
        let dedupe_key = tag_relation_dedupe_key(batch, &scorer_version, config);
        let source_hash = sha256_hex(serialized.as_bytes());
        if enqueue_pipeline_job(
            pool,
            None,
            &source_hash,
            TAG_RELATION_JEV_JOB,
            &serialized,
            "llm",
            None,
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

fn tag_relation_has_api_key(config: &crate::models::AITagRelationSettings) -> bool {
    config
        .api_key
        .as_deref()
        .is_some_and(|key| !key.trim().is_empty())
}

pub(super) fn tag_relation_transport_supported(transport: &str) -> bool {
    matches!(
        transport,
        "openrouterAlphaDecisions" | "gpuGateAlphaDecisions"
    )
}

fn tag_relation_endpoint(config: &crate::models::AITagRelationSettings) -> &str {
    if config.transport == "gpuGateAlphaDecisions" {
        &config.gpu_gate_endpoint
    } else {
        &config.endpoint
    }
}

pub(super) fn tag_relation_provider_identity(settings: &AISettings) -> &'static str {
    if settings.features.recommendations.tag_relation.transport == "gpuGateAlphaDecisions" {
        "gpuGateAlphaDecisions"
    } else {
        JEV_PROVIDER_IDENTITY
    }
}

pub(super) fn tag_relation_provider_state_model(settings: &AISettings) -> String {
    let config = &settings.features.recommendations.tag_relation;
    format!(
        "{}:{}",
        tag_relation_endpoint(config).trim().trim_end_matches('/'),
        config.model.trim()
    )
}

pub(super) fn tag_relation_is_available(settings: &AISettings) -> bool {
    let config = &settings.features.recommendations.tag_relation;
    config.enabled
        && tag_relation_transport_supported(&config.transport)
        && tag_relation_has_api_key(config)
}

async fn filter_uncached_pairs(
    pool: &Pool<Sqlite>,
    pairs: Vec<TagRelationPair>,
    requested_model: &str,
    scorer_version: &str,
    unresolved_scorer_version: &str,
) -> Result<Vec<TagRelationPair>> {
    if pairs.is_empty() {
        return Ok(pairs);
    }

    const LOOKUP_BATCH_SIZE: usize = 100;
    let mut cached_pairs = BTreeSet::new();
    for batch in pairs.chunks(LOOKUP_BATCH_SIZE) {
        let clauses = batch
            .iter()
            .map(|_| "(tag_a_id = ? AND tag_b_id = ? AND input_hash = ?)")
            .collect::<Vec<_>>()
            .join(" OR ");
        let alias_model = unresolved_model_alias(requested_model);
        let query = if alias_model {
            format!(
                "SELECT tag_a_id, tag_b_id FROM tag_relation_weight_edges \
                 WHERE status IN ('observing', 'active', 'rejected') \
                   AND scorer_version IN (?, ?) \
                   AND updated_at >= datetime('now', ?) AND ({clauses})"
            )
        } else {
            format!(
                "SELECT tag_a_id, tag_b_id FROM tag_relation_weight_edges \
                 WHERE status IN ('observing', 'active', 'rejected') AND model = ? \
                   AND scorer_version IN (?, ?) AND ({clauses})"
            )
        };
        let mut request = sqlx::query(&query);
        if alias_model {
            request = request
                .bind(scorer_version)
                .bind(unresolved_scorer_version)
                .bind(format!("-{ALIAS_CACHE_TTL_HOURS} hours"));
        } else {
            request = request
                .bind(requested_model.trim())
                .bind(scorer_version)
                .bind(unresolved_scorer_version);
        }
        for pair in batch {
            request = request
                .bind(&pair.tag_a.id)
                .bind(&pair.tag_b.id)
                .bind(&pair.pair_input_hash);
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
    scorer_version: &str,
    config: &crate::models::AITagRelationSettings,
) -> String {
    let mut values = pairs
        .iter()
        .map(|pair| pair.pair_input_hash.as_str())
        .collect::<Vec<_>>();
    values.sort_unstable();
    format!(
        "jev:{scorer_version}:{}:{}:{}",
        config.model.trim(),
        tag_relation_endpoint(config).trim().trim_end_matches('/'),
        values.join(","),
    )
}

fn tag_relation_scorer_version(
    config: &crate::models::AITagRelationSettings,
    model_resolution_unavailable: bool,
) -> String {
    let identity = format!(
        "{}\0{}\0{}\0{}\0{}\0{}\0{}",
        SCORE_SCORER_VERSION,
        config.model.trim(),
        config.transport.trim(),
        tag_relation_endpoint(config).trim().trim_end_matches('/'),
        config.protocol_version.trim(),
        config.prompt_version.trim(),
        config.schema_version.trim(),
    );
    let base = format!("{SCORE_SCORER_VERSION}:{}", sha256_hex(identity.as_bytes()));
    if model_resolution_unavailable {
        format!("{base}:model-resolution-unavailable")
    } else {
        base
    }
}

fn unresolved_model_alias(model: &str) -> bool {
    let model = model.trim().to_ascii_lowercase();
    model.starts_with('~')
        || model == "latest"
        || model.ends_with(":latest")
        || model.ends_with("-latest")
}

async fn filter_active_queued_pairs(
    pool: &Pool<Sqlite>,
    pairs: Vec<TagRelationPair>,
    scorer_version: &str,
) -> Result<Vec<TagRelationPair>> {
    if pairs.is_empty() {
        return Ok(pairs);
    }
    let payloads = sqlx::query_scalar::<_, Option<String>>(
        "SELECT payload FROM ai_processing_queue \
         WHERE job_type = ? AND status IN ('pending', 'processing', 'waiting_dependency') \
           AND payload IS NOT NULL",
    )
    .bind(TAG_RELATION_JEV_JOB)
    .fetch_all(pool)
    .await?;
    let mut active_pairs = BTreeSet::new();
    for payload in payloads.into_iter().flatten() {
        let Ok((batch, _)) = normalize_tag_relation_payload(&payload) else {
            continue;
        };
        if batch
            .scorer_version
            .as_deref()
            .is_some_and(|version| version != scorer_version)
        {
            continue;
        }
        for pair in batch.pairs {
            let Ok(pair) = pair.canonicalize() else {
                continue;
            };
            active_pairs.insert((pair.pair_id, pair.pair_input_hash));
        }
    }
    Ok(pairs
        .into_iter()
        .filter(|pair| {
            !active_pairs.contains(&(pair.pair_id.clone(), pair.pair_input_hash.clone()))
        })
        .collect())
}

async fn preserve_reviewed_relation_row(
    pool: &Pool<Sqlite>,
    pair: &TagRelationPair,
    scorer_version: &str,
    resolved_model: &str,
) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE tag_relation_weight_edges SET updated_at = CURRENT_TIMESTAMP \
         WHERE tag_a_id = ? AND tag_b_id = ? AND input_hash = ? \
           AND scorer_version = ? AND model = ? AND status IN ('active', 'rejected')",
    )
    .bind(&pair.tag_a.id)
    .bind(&pair.tag_b.id)
    .bind(&pair.pair_input_hash)
    .bind(scorer_version)
    .bind(resolved_model)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
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
        .ok_or_else(|| TagRelationJobValidationError::new("job has no payload"))?;
    let (mut batch, legacy_payload) = normalize_tag_relation_payload(payload)?;
    if batch.pairs.is_empty() || batch.pairs.len() > config.batch_size.max(1).min(4) {
        return Err(TagRelationJobValidationError::new("invalid batch size").into());
    }
    let mut unique_ids = BTreeSet::new();
    for pair in &mut batch.pairs {
        let canonical = pair.clone().canonicalize().map_err(|error| {
            TagRelationJobValidationError::new(format!("invalid pair: {error}"))
        })?;
        if canonical.tag_a.id != pair.tag_a.id
            || canonical.tag_b.id != pair.tag_b.id
            || canonical.pair_id != pair.pair_id
            || canonical.pair_input_hash != pair.pair_input_hash
        {
            return Err(TagRelationJobValidationError::new(
                "pair IDs or input hash are not canonical",
            )
            .into());
        }
        if !unique_ids.insert(canonical.pair_id.clone()) {
            return Err(
                TagRelationJobValidationError::new("batch contains duplicate pairs").into(),
            );
        }
        *pair = canonical;
    }
    validate_queued_pair_text(pool, &batch.pairs).await?;
    if legacy_payload {
        tracing::info!(job_id = %job.id, "normalized legacy JEV pair payload to Score contract");
    }

    let expected_ids = batch
        .pairs
        .iter()
        .map(|pair| pair.pair_id.clone())
        .collect::<Vec<_>>();
    let forward =
        request_score_batch(settings, config, &batch.pairs, false, request_context).await?;
    let forward_answers = parse_score_answers(&forward.body, &expected_ids).map_err(|error| {
        crate::services::content_analysis::service::InvalidWorkflowModelOutput::new(format!(
            "JEV forward Score output is invalid: {error}"
        ))
    })?;
    // JEV has its own transport and key. Its pacing must not inherit thermal protection
    // settings from whichever ordinary AI profile happens to be active.
    let reverse =
        request_score_batch(settings, config, &batch.pairs, true, request_context).await?;
    let reverse_answers = parse_score_answers(&reverse.body, &expected_ids).map_err(|error| {
        crate::services::content_analysis::service::InvalidWorkflowModelOutput::new(format!(
            "JEV reverse Score output is invalid: {error}"
        ))
    })?;

    let forward_model = resolved_response_model(&forward);
    let reverse_model = resolved_response_model(&reverse);
    if let (Some(forward_model), Some(reverse_model)) = (&forward_model, &reverse_model) {
        if forward_model != reverse_model {
            return Err(
                crate::services::content_analysis::service::InvalidWorkflowModelOutput::new(
                    "JEV forward and reverse requests resolved to different models",
                )
                .into(),
            );
        }
    }
    let model_resolution_available = forward_model.is_some() && reverse_model.is_some();
    let model = if model_resolution_available {
        forward_model.expect("resolved model was checked")
    } else {
        config.model.trim().to_string()
    };
    let scorer_version = tag_relation_scorer_version(config, !model_resolution_available);
    let provider = forward
        .provider
        .clone()
        .or_else(|| reverse.provider.clone())
        .or_else(|| Some(tag_relation_provider_identity(settings).to_string()));

    for pair in &batch.pairs {
        let forward_answer = forward_answers
            .get(&pair.pair_id)
            .expect("validated pair id must be present");
        let reverse_answer = reverse_answers
            .get(&pair.pair_id)
            .expect("validated pair id must be present");
        let (signed_weight, confidence) =
            combine_bidirectional_scores(forward_answer, reverse_answer);
        if model_resolution_available
            && preserve_reviewed_relation_row(pool, pair, &scorer_version, &model).await?
        {
            continue;
        }
        upsert_observing_relation_weight(
            pool,
            &TagRelationWeightWrite {
                tag_a_id: pair.tag_a.id.clone(),
                tag_b_id: pair.tag_b.id.clone(),
                signed_weight,
                confidence: Some(confidence),
                score_a_to_b: Some(normalize_score(forward_answer.score)),
                score_b_to_a: Some(normalize_score(reverse_answer.score)),
                input_hash: pair.pair_input_hash.clone(),
                scorer_version: scorer_version.clone(),
                profile_id: Some(JEV_EDGE_IDENTITY.to_string()),
                provider: provider.clone(),
                model: Some(model.clone()),
            },
        )
        .await?;
    }
    Ok(())
}

fn normalize_tag_relation_payload(
    payload: &str,
) -> Result<(TagRelationBatchPayload, bool), TagRelationJobValidationError> {
    let value: Value = serde_json::from_str(payload)
        .map_err(|error| TagRelationJobValidationError::new(format!("invalid JSON: {error}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| TagRelationJobValidationError::new("payload must be an object"))?;
    if object
        .keys()
        .any(|key| !matches!(key.as_str(), "contractVersion" | "scorerVersion" | "pairs"))
        || !object.contains_key("pairs")
    {
        return Err(TagRelationJobValidationError::new(
            "payload fields do not match the pair input contract",
        ));
    }
    let legacy_payload = match object.get("contractVersion") {
        None => true,
        Some(Value::String(version)) if version == SCORE_PAYLOAD_CONTRACT => false,
        Some(Value::String(_)) => {
            return Err(TagRelationJobValidationError::new(
                "unsupported contract version",
            ));
        }
        Some(_) => {
            return Err(TagRelationJobValidationError::new(
                "contractVersion must be a string",
            ));
        }
    };
    let mut batch: TagRelationBatchPayload = serde_json::from_value(value).map_err(|error| {
        TagRelationJobValidationError::new(format!("invalid pair data: {error}"))
    })?;
    batch.contract_version = Some(SCORE_PAYLOAD_CONTRACT.to_string());
    Ok((batch, legacy_payload))
}

async fn validate_queued_pair_text(pool: &Pool<Sqlite>, pairs: &[TagRelationPair]) -> Result<()> {
    let ids = pairs
        .iter()
        .flat_map(|pair| [pair.tag_a.id.clone(), pair.tag_b.id.clone()])
        .collect::<BTreeSet<_>>();
    let placeholders = std::iter::repeat("?")
        .take(ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let query = format!("SELECT id, namespace, name FROM tags WHERE id IN ({placeholders})");
    let mut request = sqlx::query(&query);
    for id in &ids {
        request = request.bind(id);
    }
    let mut current = HashMap::new();
    for row in request.fetch_all(pool).await? {
        current.insert(
            row.try_get::<String, _>("id")?,
            (
                row.try_get::<String, _>("namespace")?,
                row.try_get::<String, _>("name")?,
            ),
        );
    }
    for pair in pairs {
        for tag in [&pair.tag_a, &pair.tag_b] {
            let Some((namespace, name)) = current.get(&tag.id) else {
                return Err(TagRelationJobValidationError::new(format!(
                    "queued tag {} no longer exists",
                    tag.id
                ))
                .into());
            };
            if namespace.trim() != tag.namespace.trim() || name.trim() != tag.name.trim() {
                return Err(TagRelationJobValidationError::new(format!(
                    "queued text for tag {} is stale",
                    tag.id
                ))
                .into());
            }
        }
    }
    Ok(())
}

fn resolved_response_model(response: &JEVResponse) -> Option<String> {
    response
        .model
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty() && !unresolved_model_alias(model))
        .map(str::to_string)
}

fn combine_bidirectional_scores(forward: &ScoreAnswer, reverse: &ScoreAnswer) -> (f64, f64) {
    (
        (normalize_score(forward.score) + normalize_score(reverse.score)) / 2.0,
        forward.confidence.min(reverse.confidence),
    )
}

fn normalize_score(score: f64) -> f64 {
    (score - 2.0) / 2.0
}

fn parse_score_answers(
    response: &Value,
    expected_pair_ids: &[String],
) -> Result<BTreeMap<String, ScoreAnswer>> {
    let answers = response
        .get("answers")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("JEV response must contain an answers object"))?;
    if answers.len() != expected_pair_ids.len()
        || expected_pair_ids
            .iter()
            .any(|pair_id| !answers.contains_key(pair_id))
    {
        return Err(anyhow!("JEV Score answer IDs do not match requested pairs"));
    }

    let expected_keys = (0..SCORE_CRITERIA.len())
        .map(|index| index.to_string())
        .collect::<BTreeSet<_>>();
    let mut parsed = BTreeMap::new();
    for pair_id in expected_pair_ids {
        let answer = answers
            .get(pair_id)
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("JEV answer {pair_id} is not an object"))?;
        if answer.get("type").and_then(Value::as_str) != Some("score") {
            return Err(anyhow!("JEV answer {pair_id} is not a typed Score"));
        }
        let score = answer
            .get("score")
            .and_then(Value::as_f64)
            .filter(|score| score.is_finite() && (0.0..=4.0).contains(score))
            .ok_or_else(|| anyhow!("JEV answer {pair_id} has an invalid Score position"))?;
        let confidence = answer
            .get("confidence")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
            .ok_or_else(|| anyhow!("JEV answer {pair_id} has invalid confidence"))?;
        let probabilities = answer
            .get("probabilities")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("JEV answer {pair_id} has no Score probabilities"))?;
        if probabilities.keys().cloned().collect::<BTreeSet<_>>() != expected_keys {
            return Err(anyhow!(
                "JEV answer {pair_id} must include probabilities for all five levels"
            ));
        }
        let mut total = 0.0;
        let mut expected_score = 0.0;
        for level in 0..SCORE_CRITERIA.len() {
            let key = level.to_string();
            let probability = probabilities
                .get(&key)
                .and_then(Value::as_f64)
                .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
                .ok_or_else(|| {
                    anyhow!("JEV answer {pair_id} has invalid level {level} probability")
                })?;
            total += probability;
            expected_score += level as f64 * probability;
        }
        if (total - 1.0).abs() > SCORE_TOLERANCE {
            return Err(anyhow!(
                "JEV answer {pair_id} Score probabilities do not sum to one"
            ));
        }
        if (score - expected_score).abs() > SCORE_TOLERANCE {
            return Err(anyhow!(
                "JEV answer {pair_id} Score does not match its probability-weighted position"
            ));
        }
        parsed.insert(pair_id.clone(), ScoreAnswer { score, confidence });
    }
    Ok(parsed)
}

fn score_question(pair_id: &str) -> Value {
    json!({
        "type": "score",
        "instructions": format!(
            "Judge only state.pairs with id={pair_id}. Estimate continuous semantic affinity or opposition between the two tag labels, not certainty of synonymy and not observed user preference. Use only each tag's namespace and name. A lower level means stronger opposition on the same attribute for compatible referents; a higher level means stronger affinity. Mere difference, non-equivalence, or different subjects is not opposition. Use the neutral midpoint for independent dimensions or insufficient tag-only evidence, with uncertainty reflected in the probability distribution and confidence."
        ),
        "criteria": SCORE_CRITERIA,
    })
}

fn score_request_payload(model: &str, pairs: &[TagRelationPair], reverse: bool) -> Value {
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
        .map(|pair| (pair.pair_id.clone(), score_question(&pair.pair_id)))
        .collect::<serde_json::Map<_, _>>();
    json!({
        "model": model,
        "state": {"task": SCORE_TASK, "pairs": outbound_pairs},
        "questions": questions,
    })
}

async fn request_score_batch(
    settings: &AISettings,
    config: &crate::models::AITagRelationSettings,
    pairs: &[TagRelationPair],
    reverse: bool,
    request_context: &AIRequestContext,
) -> Result<JEVResponse> {
    let endpoint = tag_relation_endpoint(config).trim();
    if !(endpoint.starts_with("https://") || endpoint.starts_with("http://")) {
        return Err(anyhow!(
            "JEV Alpha Decisions endpoint must use http:// or https://"
        ));
    }
    let payload = score_request_payload(&config.model, pairs, reverse);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(
            settings.connection.timeout_seconds.clamp(5, 3_600),
        ))
        .build()?;
    let api_key = config
        .api_key
        .as_deref()
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| anyhow!("JEV Alpha Decisions API key is not configured"))?;
    let request = apply_request_context(
        jev_authenticated_post(&client, endpoint, api_key)?.json(&payload),
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
    let body: Value = serde_json::from_str(&body).map_err(|error| {
        crate::services::content_analysis::service::InvalidWorkflowModelOutput::new(format!(
            "JEV response was not valid JSON: {error}"
        ))
    })?;
    Ok(JEVResponse {
        provider: body
            .get("provider")
            .and_then(Value::as_str)
            .map(str::to_string),
        model: body
            .get("model")
            .and_then(Value::as_str)
            .map(|model| model.trim().to_string())
            .filter(|model| !model.is_empty()),
        body,
    })
}

fn jev_authenticated_post(
    client: &reqwest::Client,
    endpoint: &str,
    api_key: &str,
) -> Result<reqwest::RequestBuilder> {
    if api_key.trim().is_empty() {
        return Err(anyhow!("JEV Alpha Decisions API key is not configured"));
    }
    Ok(client.post(endpoint).bearer_auth(api_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::recommendations::semantic_edges::TagRelationTag;

    fn pair() -> TagRelationPair {
        pair_with_names("tag-a", "romance", "tag-b", "romantic")
    }

    fn pair_with_names(
        tag_a_id: &str,
        tag_a_name: &str,
        tag_b_id: &str,
        tag_b_name: &str,
    ) -> TagRelationPair {
        TagRelationPair {
            pair_id: String::new(),
            tag_a: TagRelationTag {
                id: tag_a_id.to_string(),
                namespace: "general".to_string(),
                name: tag_a_name.to_string(),
                support_count: 2,
            },
            tag_b: TagRelationTag {
                id: tag_b_id.to_string(),
                namespace: "general".to_string(),
                name: tag_b_name.to_string(),
                support_count: 3,
            },
            pair_input_hash: String::new(),
        }
        .canonicalize()
        .unwrap()
    }

    fn score_answer(pair_id: &str, score: f64, confidence: f64) -> Value {
        let lower = score.floor() as usize;
        let upper = score.ceil() as usize;
        let mut probabilities = serde_json::Map::new();
        if lower == upper {
            probabilities.insert(lower.to_string(), json!(1.0));
        } else {
            probabilities.insert(lower.to_string(), json!(upper as f64 - score));
            probabilities.insert(upper.to_string(), json!(score - lower as f64));
        }
        for level in 0..SCORE_CRITERIA.len() {
            probabilities
                .entry(level.to_string())
                .or_insert_with(|| json!(0.0));
        }
        json!({
            "answers": {
                pair_id: {
                    "type": "score",
                    "score": score,
                    "probabilities": probabilities,
                    "confidence": confidence,
                }
            }
        })
    }

    async fn test_pool() -> Pool<Sqlite> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::database::run_sqlite_migrations(&pool).await.unwrap();
        pool
    }

    async fn insert_pair_tags(pool: &Pool<Sqlite>, pair: &TagRelationPair) {
        sqlx::query("INSERT OR IGNORE INTO tags (id, name, namespace) VALUES (?, ?, ?), (?, ?, ?)")
            .bind(&pair.tag_a.id)
            .bind(&pair.tag_a.name)
            .bind(&pair.tag_a.namespace)
            .bind(&pair.tag_b.id)
            .bind(&pair.tag_b.name)
            .bind(&pair.tag_b.namespace)
            .execute(pool)
            .await
            .unwrap();
    }

    #[test]
    fn gpu_gate_transport_uses_its_own_endpoint_and_provider_identity() {
        let mut settings = AISettings::default();
        {
            let config = &mut settings.features.recommendations.tag_relation;
            config.enabled = true;
            config.transport = "gpuGateAlphaDecisions".to_string();
            config.gpu_gate_endpoint = "http://gpu-gate:8090/v1/jev/alpha/decisions".to_string();
            config.api_key = Some("test-provider-key".to_string());
        }

        assert!(tag_relation_is_available(&settings));
        let config = &settings.features.recommendations.tag_relation;
        assert_eq!(tag_relation_endpoint(config), config.gpu_gate_endpoint);
        assert_eq!(
            tag_relation_provider_identity(&settings),
            "gpuGateAlphaDecisions"
        );
        assert_eq!(
            tag_relation_provider_state_model(&settings),
            "http://gpu-gate:8090/v1/jev/alpha/decisions:~typesafe/jev-latest"
        );
    }

    #[test]
    fn dedupe_key_changes_with_model_prompt_and_pair_input() {
        let pair = pair().canonicalize().unwrap();
        let config = crate::models::AITagRelationSettings::default();
        let first = tag_relation_dedupe_key(
            std::slice::from_ref(&pair),
            &tag_relation_scorer_version(&config, false),
            &config,
        );
        let mut changed_config = config.clone();
        changed_config.prompt_version.push_str("-changed");
        assert_ne!(
            first,
            tag_relation_dedupe_key(
                std::slice::from_ref(&pair),
                &tag_relation_scorer_version(&changed_config, false),
                &changed_config,
            )
        );
        changed_config = config.clone();
        changed_config.model = "jev-1.13.0".to_string();
        assert_ne!(
            first,
            tag_relation_dedupe_key(
                std::slice::from_ref(&pair),
                &tag_relation_scorer_version(&changed_config, false),
                &changed_config,
            )
        );
    }

    #[test]
    fn score_payload_contains_only_pair_text_and_ordered_score_axis() {
        let pair = pair();
        let forward = score_request_payload("jev-1.13.0", std::slice::from_ref(&pair), false);
        let reverse = score_request_payload("jev-1.13.0", std::slice::from_ref(&pair), true);
        let forward_pair = &forward["state"]["pairs"][0];
        let reverse_pair = &reverse["state"]["pairs"][0];
        assert_eq!(forward["state"]["task"], SCORE_TASK);
        assert_eq!(forward_pair["left"]["name"], pair.tag_a.name);
        assert_eq!(forward_pair["right"]["name"], pair.tag_b.name);
        assert_eq!(reverse_pair["left"]["name"], pair.tag_b.name);
        assert_eq!(reverse_pair["right"]["name"], pair.tag_a.name);
        let question = &forward["questions"][&pair.pair_id];
        assert_eq!(question["type"], "score");
        assert_eq!(question["criteria"].as_array().unwrap().len(), 5);
        assert!(question["instructions"]
            .as_str()
            .unwrap()
            .contains("not observed user preference"));
        let encoded = forward.to_string();
        assert!(!encoded.contains("support_count"));
        assert!(!encoded.contains("archive"));
    }

    #[test]
    fn request_authentication_is_always_jev_bearer() {
        let request = jev_authenticated_post(
            &reqwest::Client::new(),
            "https://example.test/decisions",
            "test-provider-key",
        )
        .unwrap()
        .build()
        .unwrap();
        assert_eq!(
            request.headers()[reqwest::header::AUTHORIZATION],
            "Bearer test-provider-key"
        );
        assert!(jev_authenticated_post(
            &reqwest::Client::new(),
            "https://example.test/decisions",
            "  "
        )
        .is_err());
    }

    #[test]
    fn legacy_pair_payload_normalizes_but_categorical_payload_is_rejected() {
        let pair = pair();
        let legacy = json!({"pairs": [pair]});
        let (normalized, was_legacy) = normalize_tag_relation_payload(&legacy.to_string()).unwrap();
        assert!(was_legacy);
        assert_eq!(
            normalized.contract_version.as_deref(),
            Some(SCORE_PAYLOAD_CONTRACT)
        );
        assert!(normalize_tag_relation_payload(
            &json!({
                "contractVersion": "unknown-score-contract",
                "pairs": []
            })
            .to_string()
        )
        .is_err());
        assert!(normalize_tag_relation_payload(
            &json!({"pairs": [], "decision": "same_meaning"}).to_string()
        )
        .is_err());
        assert!(normalize_tag_relation_payload(
            &json!({"answers": {"tag-a:tag-b": {"choice": "same_meaning"}}}).to_string()
        )
        .is_err());
    }

    #[test]
    fn score_parser_checks_ids_type_probability_mass_and_expected_position() {
        let pair_id = "tag-a:tag-b".to_string();
        let parsed =
            parse_score_answers(&score_answer(&pair_id, 2.5, 0.37), &[pair_id.clone()]).unwrap();
        let answer = &parsed[&pair_id];
        assert_eq!(answer.score, 2.5);
        assert_eq!(normalize_score(answer.score), 0.25);
        assert_eq!(answer.confidence, 0.37);

        assert!(parse_score_answers(&score_answer("other", 2.0, 0.8), &[pair_id.clone()]).is_err());
        let mut wrong_type = score_answer(&pair_id, 2.0, 0.8);
        wrong_type["answers"][&pair_id]["type"] = json!("choice");
        assert!(parse_score_answers(&wrong_type, &[pair_id.clone()]).is_err());
        let mut wrong_mass = score_answer(&pair_id, 2.0, 0.8);
        wrong_mass["answers"][&pair_id]["probabilities"]["2"] = json!(0.8);
        assert!(parse_score_answers(&wrong_mass, &[pair_id.clone()]).is_err());
        let mut wrong_expected = score_answer(&pair_id, 2.0, 0.8);
        wrong_expected["answers"][&pair_id]["score"] = json!(3.0);
        assert!(parse_score_answers(&wrong_expected, &[pair_id]).is_err());
    }

    #[test]
    fn bidirectional_scores_remain_distinct_from_confidence() {
        let forward = ScoreAnswer {
            score: 3.8,
            confidence: 0.92,
        };
        let reverse = ScoreAnswer {
            score: 1.8,
            confidence: 0.41,
        };
        let (weight, confidence) = combine_bidirectional_scores(&forward, &reverse);
        assert!((weight - 0.4).abs() < f64::EPSILON);
        assert_eq!(confidence, 0.41);
    }

    #[tokio::test]
    async fn legacy_pair_is_revalidated_against_current_tag_text() {
        let pool = test_pool().await;
        let pair = pair();
        insert_pair_tags(&pool, &pair).await;
        validate_queued_pair_text(&pool, std::slice::from_ref(&pair))
            .await
            .unwrap();
        sqlx::query("UPDATE tags SET name = 'romance style' WHERE id = 'tag-b'")
            .execute(&pool)
            .await
            .unwrap();
        let error = validate_queued_pair_text(&pool, &[pair]).await.unwrap_err();
        assert!(error
            .downcast_ref::<TagRelationJobValidationError>()
            .is_some());
    }

    #[tokio::test]
    async fn numeric_cache_matches_pair_text_scorer_and_stable_model_only() {
        let pool = test_pool().await;
        let canonical = pair();
        insert_pair_tags(&pool, &canonical).await;
        let mut config = crate::models::AITagRelationSettings::default();
        let alias = config.model.clone();
        let stable_model = "jev-1.13.0";
        let scorer_version = tag_relation_scorer_version(&config, false);
        let unresolved_version = tag_relation_scorer_version(&config, true);
        upsert_observing_relation_weight(
            &pool,
            &TagRelationWeightWrite {
                tag_a_id: canonical.tag_a.id.clone(),
                tag_b_id: canonical.tag_b.id.clone(),
                signed_weight: 0.25,
                confidence: Some(0.8),
                score_a_to_b: Some(0.5),
                score_b_to_a: Some(0.0),
                input_hash: canonical.pair_input_hash.clone(),
                scorer_version: scorer_version.clone(),
                profile_id: Some(JEV_EDGE_IDENTITY.to_string()),
                provider: Some(JEV_PROVIDER_IDENTITY.to_string()),
                model: Some(stable_model.to_string()),
            },
        )
        .await
        .unwrap();

        assert_eq!(
            filter_uncached_pairs(
                &pool,
                vec![canonical.clone()],
                &alias,
                &scorer_version,
                &unresolved_version,
            )
            .await
            .unwrap()
            .len(),
            0,
            "an alias can reuse an exact input within its short TTL"
        );

        sqlx::query(
            "UPDATE tag_relation_weight_edges \
             SET updated_at = datetime('now', '-25 hours'), status = 'rejected'",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            filter_uncached_pairs(
                &pool,
                vec![canonical.clone()],
                &alias,
                &scorer_version,
                &unresolved_version,
            )
            .await
            .unwrap()
            .len(),
            1,
            "expired alias results are rejudged, including rejected rows"
        );
        assert!(
            preserve_reviewed_relation_row(&pool, &canonical, &scorer_version, stable_model,)
                .await
                .unwrap()
        );
        let reviewed = sqlx::query(
            "SELECT signed_weight, status FROM tag_relation_weight_edges \
             WHERE tag_a_id = ? AND tag_b_id = ?",
        )
        .bind(&canonical.tag_a.id)
        .bind(&canonical.tag_b.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(reviewed.get::<f64, _>("signed_weight"), 0.25);
        assert_eq!(reviewed.get::<String, _>("status"), "rejected");
        assert!(filter_uncached_pairs(
            &pool,
            vec![canonical.clone()],
            &alias,
            &scorer_version,
            &unresolved_version,
        )
        .await
        .unwrap()
        .is_empty());

        sqlx::query(
            "UPDATE tag_relation_weight_edges SET scorer_version = ?, model = ?, \
             status = 'observing', updated_at = CURRENT_TIMESTAMP",
        )
        .bind(&unresolved_version)
        .bind(&alias)
        .execute(&pool)
        .await
        .unwrap();
        assert!(filter_uncached_pairs(
            &pool,
            vec![canonical.clone()],
            &alias,
            &scorer_version,
            &unresolved_version,
        )
        .await
        .unwrap()
        .is_empty());
        sqlx::query(
            "UPDATE tag_relation_weight_edges SET updated_at = datetime('now', '-25 hours')",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            filter_uncached_pairs(
                &pool,
                vec![canonical.clone()],
                &alias,
                &scorer_version,
                &unresolved_version,
            )
            .await
            .unwrap()
            .len(),
            1,
            "an unresolved alias result is cacheable only within the TTL"
        );

        config.model = "~typesafe/jev-v2".to_string();
        let changed_alias_scorer = tag_relation_scorer_version(&config, false);
        let changed_alias_unresolved = tag_relation_scorer_version(&config, true);
        assert_eq!(
            filter_uncached_pairs(
                &pool,
                vec![canonical.clone()],
                &config.model,
                &changed_alias_scorer,
                &changed_alias_unresolved,
            )
            .await
            .unwrap()
            .len(),
            1,
            "a changed requested model alias misses the cache"
        );

        config.model = stable_model.to_string();
        let stable_scorer = tag_relation_scorer_version(&config, false);
        let stable_unresolved = tag_relation_scorer_version(&config, true);
        upsert_observing_relation_weight(
            &pool,
            &TagRelationWeightWrite {
                tag_a_id: canonical.tag_a.id.clone(),
                tag_b_id: canonical.tag_b.id.clone(),
                signed_weight: -0.25,
                confidence: Some(0.7),
                score_a_to_b: Some(-0.5),
                score_b_to_a: Some(0.0),
                input_hash: canonical.pair_input_hash.clone(),
                scorer_version: stable_scorer.clone(),
                profile_id: Some(JEV_EDGE_IDENTITY.to_string()),
                provider: Some(JEV_PROVIDER_IDENTITY.to_string()),
                model: Some(stable_model.to_string()),
            },
        )
        .await
        .unwrap();
        sqlx::query(
            "UPDATE tag_relation_weight_edges SET updated_at = datetime('now', '-25 hours'), \
             status = 'rejected'",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            filter_uncached_pairs(
                &pool,
                vec![canonical.clone()],
                stable_model,
                &stable_scorer,
                &stable_unresolved,
            )
            .await
            .unwrap()
            .len(),
            0,
            "an exact model and matching scorer version cache without a TTL"
        );
        sqlx::query("UPDATE tag_relation_weight_edges SET model = 'jev-1.14.0' WHERE tag_a_id = ?")
            .bind(&canonical.tag_a.id)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            filter_uncached_pairs(
                &pool,
                vec![canonical.clone()],
                stable_model,
                &stable_scorer,
                &stable_unresolved,
            )
            .await
            .unwrap()
            .len(),
            1,
            "a stable model version change misses the cache"
        );

        let mut changed = canonical;
        changed.tag_b.name = "romance style".to_string();
        let changed = changed.canonicalize().unwrap();
        assert_eq!(
            filter_uncached_pairs(
                &pool,
                vec![changed],
                stable_model,
                &stable_scorer,
                &stable_unresolved,
            )
            .await
            .unwrap()
            .len(),
            1,
            "changed pair text must miss the numeric cache"
        );
    }

    #[tokio::test]
    async fn disabled_lane_does_not_enqueue_jobs_and_enabled_lane_versions_payload() {
        let pool = test_pool().await;
        let pair = pair();
        insert_pair_tags(&pool, &pair).await;
        let mut settings = AISettings::default();
        settings.features.recommendations.tag_relation.api_key =
            Some("test-provider-key".to_string());
        let disabled =
            enqueue_tag_relation_jev_candidates(&pool, &settings, std::slice::from_ref(&pair))
                .await
                .unwrap();
        assert_eq!(disabled.queued, 0);
        let queued_jobs: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM ai_processing_queue WHERE job_type = ?")
                .bind(TAG_RELATION_JEV_JOB)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(queued_jobs, 0);

        settings.features.recommendations.tag_relation.enabled = true;
        let queued =
            enqueue_tag_relation_jev_candidates(&pool, &settings, std::slice::from_ref(&pair))
                .await
                .unwrap();
        assert_eq!(queued.queued, 1);
        let repeated =
            enqueue_tag_relation_jev_candidates(&pool, &settings, std::slice::from_ref(&pair))
                .await
                .unwrap();
        assert_eq!(repeated.queued, 0);
        let payload: String =
            sqlx::query_scalar("SELECT payload FROM ai_processing_queue WHERE job_type = ?")
                .bind(TAG_RELATION_JEV_JOB)
                .fetch_one(&pool)
                .await
                .unwrap();
        let value: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(value["contractVersion"], SCORE_PAYLOAD_CONTRACT);
        let active_jobs: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ai_processing_queue WHERE job_type = ? \
             AND status IN ('pending', 'processing', 'waiting_dependency')",
        )
        .bind(TAG_RELATION_JEV_JOB)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(active_jobs, 1);
    }

    #[tokio::test]
    async fn overlapping_batches_enqueue_each_current_pair_once() {
        let pool = test_pool().await;
        let first_pair = pair_with_names("tag-a", "alpha label", "tag-b", "bravo label");
        let overlap_pair = pair_with_names("tag-c", "cobalt label", "tag-d", "delta label");
        let later_pair = pair_with_names("tag-e", "echo label", "tag-f", "foxtrot label");
        for candidate in [&first_pair, &overlap_pair, &later_pair] {
            insert_pair_tags(&pool, candidate).await;
        }

        let mut settings = AISettings::default();
        settings.features.recommendations.tag_relation.enabled = true;
        settings.features.recommendations.tag_relation.api_key =
            Some("test-provider-key".to_string());
        let first_batch = [first_pair.clone(), overlap_pair.clone()];
        let second_batch = [overlap_pair.clone(), later_pair];
        let (first_result, second_result) = tokio::join!(
            enqueue_tag_relation_jev_candidates(&pool, &settings, &first_batch),
            enqueue_tag_relation_jev_candidates(&pool, &settings, &second_batch),
        );
        assert_eq!(first_result.unwrap().queued, 1);
        assert_eq!(second_result.unwrap().queued, 1);

        let payloads = sqlx::query_scalar::<_, String>(
            "SELECT payload FROM ai_processing_queue WHERE job_type = ? \
             AND status IN ('pending', 'processing', 'waiting_dependency')",
        )
        .bind(TAG_RELATION_JEV_JOB)
        .fetch_all(&pool)
        .await
        .unwrap();
        let overlap_occurrences = payloads
            .iter()
            .map(|payload| {
                let value: Value = serde_json::from_str(payload).unwrap();
                value["pairs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|pair| pair["pair_id"] == overlap_pair.pair_id)
                    .count()
            })
            .sum::<usize>();
        assert_eq!(overlap_occurrences, 1);
    }

    #[test]
    fn unresolved_model_aliases_are_not_cacheable() {
        for alias in ["~typesafe/jev-latest", "jev-latest", "model:latest"] {
            assert!(unresolved_model_alias(alias), "{alias}");
        }
        assert!(!unresolved_model_alias("jev-1.13.0"));
    }
}
