//! Provider-neutral storage and bounded one-hop scoring for signed tag relations.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite, Transaction};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use uuid::Uuid;

pub const DEFAULT_WEIGHTED_GRAPH_GAIN: f64 = 0.3;
const MAX_SEED_TAGS: usize = 100;
pub const MAX_SOURCE_EVIDENCE: usize = 500;
const JEV_QUEUE_JOB_TYPE: &str = "tag_relation_jev";

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeightedGraphPolicy {
    pub enabled: bool,
    pub global_gain: f64,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeightedTagGraphStatus {
    pub enabled: bool,
    pub configured: bool,
    pub state: String,
    pub active_relation_count: usize,
    pub queued_task_count: usize,
    pub processing_task_count: usize,
    pub retry_waiting_task_count: usize,
    pub paused: bool,
    pub next_retry_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TagRelationWeightWrite {
    pub tag_a_id: String,
    pub tag_b_id: String,
    pub signed_weight: f64,
    pub confidence: Option<f64>,
    pub score_a_to_b: Option<f64>,
    pub score_b_to_a: Option<f64>,
    pub input_hash: String,
    pub scorer_version: String,
    pub profile_id: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
}

impl TagRelationWeightWrite {
    pub fn canonicalize(mut self) -> Result<Self> {
        self.tag_a_id = self.tag_a_id.trim().to_string();
        self.tag_b_id = self.tag_b_id.trim().to_string();
        self.input_hash = self.input_hash.trim().to_string();
        self.scorer_version = self.scorer_version.trim().to_string();
        if self.tag_a_id.is_empty()
            || self.tag_b_id.is_empty()
            || self.tag_a_id == self.tag_b_id
            || self.input_hash.is_empty()
            || self.scorer_version.is_empty()
        {
            return Err(anyhow!(
                "tag relation weight has invalid identity or provenance"
            ));
        }
        validate_signed_weight(self.signed_weight)?;
        validate_confidence(self.confidence)?;
        validate_optional_weight(self.score_a_to_b)?;
        validate_optional_weight(self.score_b_to_a)?;
        if self.tag_a_id > self.tag_b_id {
            std::mem::swap(&mut self.tag_a_id, &mut self.tag_b_id);
            std::mem::swap(&mut self.score_a_to_b, &mut self.score_b_to_a);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeightedTagRelationEdge {
    pub tag_a_id: String,
    pub tag_b_id: String,
    pub signed_weight: f64,
    pub confidence: Option<f64>,
    pub status: String,
    pub user_influence: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoredWeightedTagRelationEdge {
    pub tag_a_id: String,
    pub namespace_a: String,
    pub name_a: String,
    pub tag_b_id: String,
    pub namespace_b: String,
    pub name_b: String,
    pub signed_weight: f64,
    pub confidence: Option<f64>,
    pub score_a_to_b: Option<f64>,
    pub score_b_to_a: Option<f64>,
    pub input_hash: String,
    pub scorer_version: String,
    pub profile_id: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub revision: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceArchiveEvidence {
    pub archive_id: String,
    pub source_tag_id: String,
    pub source_strength: f64,
    pub archive_tag_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GraphEdgeAttribution {
    pub tag_a_id: String,
    pub tag_b_id: String,
    pub signed_contribution: f64,
    pub source_archive_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GraphScore {
    pub contribution: f64,
    pub edge_attributions: Vec<GraphEdgeAttribution>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphFeedbackOutcome {
    Positive,
    Negative,
    Unobserved,
}

#[derive(Debug, Clone)]
struct EvidencePath {
    source_archive_id: String,
    source_tag_id: String,
    edge: WeightedTagRelationEdge,
    contribution: f64,
}

/// Stores a scorer result and activates it only if its current input, version, namespaces, and
/// feature configuration still pass validation in the same transaction.
pub async fn upsert_scored_relation_weight(
    pool: &Pool<Sqlite>,
    edge: &TagRelationWeightWrite,
) -> Result<()> {
    let edge = edge.clone().canonicalize()?;
    let mut transaction = pool.begin().await?;
    let settings = load_graph_settings_for_transaction(&mut transaction).await?;
    let metadata_namespaces = metadata_namespaces_for_transaction(&mut transaction).await?;
    let current_tags = sqlx::query(
        "SELECT tag_a.namespace AS namespace_a, tag_a.name AS name_a,
                tag_b.namespace AS namespace_b, tag_b.name AS name_b
         FROM tags tag_a JOIN tags tag_b ON tag_b.id = ?
         WHERE tag_a.id = ?",
    )
    .bind(&edge.tag_b_id)
    .bind(&edge.tag_a_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let (input_is_current, namespaces_are_valid) = if let Some(tags) = current_tags {
        let namespace_a = tags.get::<String, _>("namespace_a");
        let namespace_b = tags.get::<String, _>("namespace_b");
        let pair = crate::services::recommendations::semantic_edges::TagRelationPair {
            pair_id: String::new(),
            tag_a: crate::services::recommendations::semantic_edges::TagRelationTag {
                id: edge.tag_a_id.clone(),
                namespace: namespace_a.clone(),
                name: tags.get("name_a"),
                support_count: 0,
            },
            tag_b: crate::services::recommendations::semantic_edges::TagRelationTag {
                id: edge.tag_b_id.clone(),
                namespace: namespace_b.clone(),
                name: tags.get("name_b"),
                support_count: 0,
            },
            pair_input_hash: String::new(),
        }
        .canonicalize()?;
        let normalize_namespace = |value: &str| value.trim().to_ascii_lowercase();
        (
            pair.pair_input_hash == edge.input_hash,
            [namespace_a, namespace_b].iter().all(|namespace| {
                let namespace = normalize_namespace(namespace);
                namespace != "theme" && !metadata_namespaces.contains(&namespace)
            }),
        )
    } else {
        (false, false)
    };

    let existing_status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM tag_relation_weight_edges WHERE tag_a_id = ? AND tag_b_id = ?",
    )
    .bind(&edge.tag_a_id)
    .bind(&edge.tag_b_id)
    .fetch_optional(&mut *transaction)
    .await?;
    if existing_status.as_deref() == Some("rejected") {
        transaction.rollback().await?;
        return Ok(());
    }

    let current_scorer_version = crate::services::ai_service::tag_relation_scorer_version(
        &settings.features.recommendations.tag_relation,
        false,
    );
    let version_is_current = edge.scorer_version == current_scorer_version;
    let valid_score = input_is_current && namespaces_are_valid && version_is_current;
    let status = if valid_score && crate::services::ai_service::tag_relation_is_available(&settings)
    {
        "active"
    } else {
        "observing"
    };
    sqlx::query(
        "INSERT INTO tag_relation_weight_edges
         (tag_a_id, tag_b_id, signed_weight, confidence, score_a_to_b, score_b_to_a,
          input_hash, scorer_version, profile_id, provider, model, status, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(tag_a_id, tag_b_id) DO UPDATE SET
          signed_weight = excluded.signed_weight, confidence = excluded.confidence,
          score_a_to_b = excluded.score_a_to_b, score_b_to_a = excluded.score_b_to_a,
          input_hash = excluded.input_hash, scorer_version = excluded.scorer_version,
          profile_id = excluded.profile_id, provider = excluded.provider,
          model = excluded.model, status = excluded.status,
          revision = tag_relation_weight_edges.revision + 1,
          updated_at = CURRENT_TIMESTAMP
         WHERE tag_relation_weight_edges.status <> 'rejected'",
    )
    .bind(edge.tag_a_id)
    .bind(edge.tag_b_id)
    .bind(edge.signed_weight)
    .bind(edge.confidence)
    .bind(edge.score_a_to_b)
    .bind(edge.score_b_to_a)
    .bind(edge.input_hash)
    .bind(edge.scorer_version)
    .bind(edge.profile_id)
    .bind(edge.provider)
    .bind(edge.model)
    .bind(status)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn load_graph_settings_for_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
) -> Result<crate::models::AISettings> {
    let stored_raw =
        sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = 'ai_settings'")
            .fetch_optional(&mut **transaction)
            .await?;
    let mut settings = stored_raw
        .as_deref()
        .and_then(|value| serde_json::from_str::<crate::models::AISettings>(value).ok())
        .unwrap_or_default();
    settings.features.recommendations.tag_relation.api_key = sqlx::query_scalar::<_, String>(
        "SELECT value FROM settings WHERE key = 'ai_tag_relation_jev_api_key'",
    )
    .fetch_optional(&mut **transaction)
    .await?;
    Ok(settings)
}

async fn metadata_namespaces_for_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
) -> Result<HashSet<String>> {
    let namespaces = sqlx::query_scalar::<_, String>(
        "SELECT namespace FROM recommendation_metadata_namespaces WHERE policy_version = 'metadata-v1'",
    )
    .fetch_all(&mut **transaction)
    .await?;
    Ok(namespaces
        .into_iter()
        .map(|namespace| namespace.to_ascii_lowercase())
        .collect())
}

pub async fn list_scored_relation_weights(
    pool: &Pool<Sqlite>,
    status: &str,
    limit: usize,
) -> Result<Vec<ScoredWeightedTagRelationEdge>> {
    if !matches!(status, "observing" | "active" | "rejected") {
        return Err(anyhow!("unsupported weighted tag relation status"));
    }
    let rows = sqlx::query(
        "SELECT edge.tag_a_id, tag_a.namespace AS namespace_a, tag_a.name AS name_a,
                edge.tag_b_id, tag_b.namespace AS namespace_b, tag_b.name AS name_b,
                edge.signed_weight, edge.confidence, edge.score_a_to_b, edge.score_b_to_a,
                edge.input_hash, edge.scorer_version, edge.profile_id, edge.provider,
                edge.model, edge.status, edge.revision, edge.updated_at
         FROM tag_relation_weight_edges edge
         JOIN tags tag_a ON tag_a.id = edge.tag_a_id
         JOIN tags tag_b ON tag_b.id = edge.tag_b_id
         WHERE edge.status = ?
         ORDER BY edge.updated_at DESC, edge.tag_a_id, edge.tag_b_id
         LIMIT ?",
    )
    .bind(status)
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| ScoredWeightedTagRelationEdge {
            tag_a_id: row.get("tag_a_id"),
            namespace_a: row.get("namespace_a"),
            name_a: row.get("name_a"),
            tag_b_id: row.get("tag_b_id"),
            namespace_b: row.get("namespace_b"),
            name_b: row.get("name_b"),
            signed_weight: row.get("signed_weight"),
            confidence: row.get("confidence"),
            score_a_to_b: row.get("score_a_to_b"),
            score_b_to_a: row.get("score_b_to_a"),
            input_hash: row.get("input_hash"),
            scorer_version: row.get("scorer_version"),
            profile_id: row.get("profile_id"),
            provider: row.get("provider"),
            model: row.get("model"),
            status: row.get("status"),
            revision: row.get("revision"),
            updated_at: row.get("updated_at"),
        })
        .collect())
}

pub async fn review_weighted_relation_edge(
    pool: &Pool<Sqlite>,
    tag_a_id: &str,
    tag_b_id: &str,
    expected_revision: i64,
    expected_input_hash: &str,
    expected_scorer_version: &str,
    next_status: &str,
) -> Result<bool> {
    if !matches!(next_status, "active" | "rejected") {
        return Err(anyhow!("review status must be active or rejected"));
    }
    if expected_revision < 1
        || expected_input_hash.trim().is_empty()
        || expected_scorer_version.trim().is_empty()
    {
        return Err(anyhow!("weighted tag relation review identity is invalid"));
    }
    let mut left = tag_a_id.trim().to_string();
    let mut right = tag_b_id.trim().to_string();
    if left.is_empty() || right.is_empty() || left == right {
        return Err(anyhow!("weighted tag relation review IDs are invalid"));
    }
    if left > right {
        std::mem::swap(&mut left, &mut right);
    }

    let metadata_namespaces = super::namespace_policy::load_metadata_namespace_set(pool).await?;
    let mut transaction = pool.begin().await?;
    let edge = sqlx::query(
        "SELECT edge.status, edge.revision, edge.input_hash, edge.scorer_version,
                tag_a.namespace AS namespace_a, tag_b.namespace AS namespace_b
         FROM tag_relation_weight_edges edge
         JOIN tags tag_a ON tag_a.id = edge.tag_a_id
         JOIN tags tag_b ON tag_b.id = edge.tag_b_id
         WHERE edge.tag_a_id = ? AND edge.tag_b_id = ?",
    )
    .bind(&left)
    .bind(&right)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(edge) = edge else {
        transaction.rollback().await?;
        return Ok(false);
    };
    let namespace_a: String = edge.get("namespace_a");
    let namespace_b: String = edge.get("namespace_b");
    if next_status == "active"
        && [namespace_a, namespace_b].iter().any(|namespace| {
            let normalized = namespace.trim().to_ascii_lowercase();
            normalized == "theme" || metadata_namespaces.contains(&normalized)
        })
    {
        transaction.rollback().await?;
        return Err(anyhow!("metadata tag relations cannot be activated"));
    }
    let current_status: String = edge.get("status");
    if edge.get::<i64, _>("revision") != expected_revision
        || edge.get::<String, _>("input_hash") != expected_input_hash
        || edge.get::<String, _>("scorer_version") != expected_scorer_version
        || current_status == next_status
    {
        transaction.rollback().await?;
        return Ok(false);
    }
    let updated = sqlx::query(
        "UPDATE tag_relation_weight_edges SET status = ?, revision = revision + 1,
             updated_at = CURRENT_TIMESTAMP
         WHERE tag_a_id = ? AND tag_b_id = ? AND status = ?
           AND revision = ? AND input_hash = ? AND scorer_version = ?",
    )
    .bind(next_status)
    .bind(&left)
    .bind(&right)
    .bind(current_status)
    .bind(expected_revision)
    .bind(expected_input_hash)
    .bind(expected_scorer_version)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(updated.rows_affected() == 1)
}

pub async fn load_weighted_graph_policy(pool: &Pool<Sqlite>) -> Result<WeightedGraphPolicy> {
    let row =
        sqlx::query("SELECT global_gain, version FROM tag_weighted_graph_policy WHERE id = 1")
            .fetch_one(pool)
            .await?;
    let settings = crate::services::ai_service::load_ai_settings(pool).await?;
    let policy = WeightedGraphPolicy {
        enabled: crate::services::ai_service::tag_relation_is_available(&settings),
        global_gain: row.get("global_gain"),
        version: row.get("version"),
    };
    validate_gain(policy.global_gain)?;
    Ok(policy)
}

/// Updates graph scoring gain with optimistic concurrency. Feature intent lives in AI settings.
pub async fn update_weighted_graph_policy(
    pool: &Pool<Sqlite>,
    expected_version: i64,
    global_gain: f64,
) -> Result<bool> {
    validate_gain(global_gain)?;
    let result = sqlx::query(
        "UPDATE tag_weighted_graph_policy
         SET global_gain = ?, version = version + 1,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = 1 AND version = ?",
    )
    .bind(global_gain)
    .bind(expected_version)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn load_weighted_tag_graph_status(pool: &Pool<Sqlite>) -> Result<WeightedTagGraphStatus> {
    let settings = crate::services::ai_service::load_ai_settings(pool).await?;
    let enabled = settings.features.recommendations.tag_graph_enabled;
    let configured = crate::services::ai_service::tag_relation_configuration_ready(&settings);
    let counts = sqlx::query(
        "WITH jobs AS (
            SELECT * FROM ai_processing_queue WHERE job_type = ?
         )
         SELECT
            COUNT(CASE WHEN status = 'pending' AND attempts = 0 AND last_error IS NULL THEN 1 END) AS queued_count,
            COUNT(CASE WHEN status = 'processing' THEN 1 END) AS processing_count,
            COUNT(CASE WHEN status = 'pending' AND (attempts > 0 OR last_error IS NOT NULL) THEN 1 END) AS retry_waiting_count,
            MIN(CASE WHEN status = 'pending' AND (attempts > 0 OR last_error IS NOT NULL) THEN next_run_at END) AS next_retry_at,
            (SELECT last_error FROM jobs
             WHERE status IN ('pending', 'failed') AND last_error IS NOT NULL
             ORDER BY created_at DESC LIMIT 1) AS last_error,
            EXISTS (SELECT 1 FROM jobs WHERE status = 'failed') AS has_failed_job
         FROM jobs",
    )
    .bind(JEV_QUEUE_JOB_TYPE)
    .fetch_one(pool)
    .await?;
    let paused = sqlx::query_scalar::<_, i64>(
        "SELECT manually_paused FROM ai_queue_controls WHERE job_type = ?",
    )
    .bind(JEV_QUEUE_JOB_TYPE)
    .fetch_optional(pool)
    .await?
    .unwrap_or_default()
        != 0;
    let active_relation_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tag_relation_weight_edges WHERE status = 'active'",
    )
    .fetch_one(pool)
    .await? as usize;
    let queued_task_count = counts.get::<i64, _>("queued_count") as usize;
    let processing_task_count = counts.get::<i64, _>("processing_count") as usize;
    let retry_waiting_task_count = counts.get::<i64, _>("retry_waiting_count") as usize;
    let next_retry_at = counts.get("next_retry_at");
    let last_error = counts.get("last_error");
    let state = weighted_tag_graph_state(
        enabled,
        configured,
        paused,
        counts.get::<i64, _>("has_failed_job") != 0,
        retry_waiting_task_count > 0,
        queued_task_count > 0 || processing_task_count > 0,
        active_relation_count > 0,
    );
    Ok(WeightedTagGraphStatus {
        enabled,
        configured,
        state: state.to_string(),
        active_relation_count,
        queued_task_count,
        processing_task_count,
        retry_waiting_task_count,
        paused,
        next_retry_at,
        last_error,
    })
}

fn weighted_tag_graph_state(
    enabled: bool,
    configured: bool,
    paused: bool,
    needs_attention: bool,
    retry_waiting: bool,
    updating: bool,
    ready: bool,
) -> &'static str {
    if !enabled {
        "disabled"
    } else if !configured {
        "unconfigured"
    } else if paused {
        "paused"
    } else if needs_attention {
        "needs_attention"
    } else if retry_waiting {
        "retry_waiting"
    } else if updating {
        "updating"
    } else if ready {
        "ready"
    } else {
        "waiting_tags"
    }
}

/// Loads explicitly activated numeric edges around the user's strongest positive tag seeds.
/// Observing scores and disabled or unconfigured graph settings never reach recommendation scoring.
pub async fn load_active_weighted_neighbors(
    pool: &Pool<Sqlite>,
    user_id: &str,
    seed_strengths: &HashMap<String, f64>,
    limit_per_seed: usize,
) -> Result<(WeightedGraphPolicy, Vec<WeightedTagRelationEdge>)> {
    let policy = load_weighted_graph_policy(pool).await?;
    if !policy.enabled || seed_strengths.is_empty() || limit_per_seed == 0 {
        return Ok((policy, Vec::new()));
    }
    let mut seeds = seed_strengths.iter().collect::<Vec<_>>();
    seeds.sort_by(|(left_id, left_strength), (right_id, right_strength)| {
        right_strength
            .total_cmp(left_strength)
            .then_with(|| left_id.cmp(right_id))
    });
    seeds.truncate(MAX_SEED_TAGS);
    let seed_ids = seeds
        .into_iter()
        .map(|(tag_id, strength)| {
            validate_strength(*strength)?;
            Ok(tag_id.clone())
        })
        .collect::<Result<Vec<_>>>()?;
    let placeholders = std::iter::repeat("?")
        .take(seed_ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let query = format!(
        "SELECT edge.tag_a_id, edge.tag_b_id, edge.signed_weight, edge.confidence,
                edge.status, COALESCE(factor.influence, 1.0) AS user_influence,
                tag_a.namespace AS namespace_a, tag_b.namespace AS namespace_b
         FROM tag_relation_weight_edges edge
         JOIN tags tag_a ON tag_a.id = edge.tag_a_id
         JOIN tags tag_b ON tag_b.id = edge.tag_b_id
         LEFT JOIN tag_relation_user_factors factor
           ON factor.user_id = ? AND factor.tag_a_id = edge.tag_a_id
          AND factor.tag_b_id = edge.tag_b_id
         WHERE edge.status = 'active'
           AND edge.signed_weight > 0.0
           AND (edge.tag_a_id IN ({placeholders}) OR edge.tag_b_id IN ({placeholders}))
         ORDER BY ABS(edge.signed_weight) DESC, edge.tag_a_id, edge.tag_b_id"
    );
    let mut request = sqlx::query(&query).bind(user_id);
    for id in &seed_ids {
        request = request.bind(id);
    }
    for id in &seed_ids {
        request = request.bind(id);
    }
    let metadata_namespaces = super::namespace_policy::load_metadata_namespace_set(pool).await?;
    let mut per_seed_counts: HashMap<String, usize> = HashMap::new();
    let mut edges = Vec::new();
    for row in request.fetch_all(pool).await? {
        let tag_a_id: String = row.get("tag_a_id");
        let tag_b_id: String = row.get("tag_b_id");
        let namespace_a: String = row.get("namespace_a");
        let namespace_b: String = row.get("namespace_b");
        let normalize_namespace = |value: &str| value.trim().to_ascii_lowercase();
        let namespace_a = normalize_namespace(&namespace_a);
        let namespace_b = normalize_namespace(&namespace_b);
        if namespace_a == "theme"
            || namespace_b == "theme"
            || metadata_namespaces.contains(&namespace_a)
            || metadata_namespaces.contains(&namespace_b)
        {
            continue;
        }
        let touching_seeds = seed_ids
            .iter()
            .filter(|seed| **seed == tag_a_id || **seed == tag_b_id)
            .cloned()
            .collect::<Vec<_>>();
        if touching_seeds
            .iter()
            .any(|seed| per_seed_counts.get(seed).copied().unwrap_or_default() < limit_per_seed)
        {
            for seed in touching_seeds {
                let count = per_seed_counts.entry(seed).or_default();
                if *count < limit_per_seed {
                    *count += 1;
                }
            }
            let edge = WeightedTagRelationEdge {
                tag_a_id,
                tag_b_id,
                signed_weight: row.get("signed_weight"),
                confidence: row.get("confidence"),
                status: row.get("status"),
                user_influence: row.get("user_influence"),
            };
            validate_edge(&edge)?;
            edges.push(edge);
        }
    }
    Ok((policy, edges))
}

/// Uses only archives with positive direct reading evidence and no recorded deletion. The
/// evidence list is read on demand from existing feedback; it does not write learner state.
pub async fn load_positive_source_archive_evidence(
    pool: &Pool<Sqlite>,
    user_id: &str,
    seed_strengths: &HashMap<String, f64>,
) -> Result<Vec<SourceArchiveEvidence>> {
    let mut seeds = seed_strengths.iter().collect::<Vec<_>>();
    seeds.sort_by(|(left_id, left_strength), (right_id, right_strength)| {
        right_strength
            .total_cmp(left_strength)
            .then_with(|| left_id.cmp(right_id))
    });
    seeds.truncate(MAX_SEED_TAGS);
    if seeds.is_empty() {
        return Ok(Vec::new());
    }
    let mut strengths = HashMap::new();
    for (tag_id, strength) in seeds {
        validate_strength(*strength)?;
        strengths.insert(tag_id.clone(), *strength);
    }
    let placeholders = std::iter::repeat("?")
        .take(strengths.len())
        .collect::<Vec<_>>()
        .join(",");
    let query = format!(
        "SELECT evidence.tag_id, evidence.archive_id, evidence.evidence_count
         FROM (
             SELECT association.tag_id, association.archive_id,
                    feedback.last_event_at,
                    COUNT(*) OVER (PARTITION BY association.tag_id) AS evidence_count,
                    ROW_NUMBER() OVER (
                        PARTITION BY association.tag_id
                        ORDER BY feedback.last_event_at DESC, association.archive_id
                    ) AS evidence_rank
             FROM archive_tags association
             JOIN preference_feedback_aggregates feedback
               ON feedback.archive_id = association.archive_id
             JOIN preference_learning_state state ON state.id = 'default'
             WHERE feedback.user_id = ?
               AND feedback.last_event_at >= state.cold_start_started_at
               AND (feedback.effective_read = 1 OR feedback.deep_read = 1
                    OR feedback.completed_read = 1 OR feedback.continue_count > 0
                    OR feedback.repeat_open_count > 0
                    OR (feedback.open_count > 0 AND feedback.max_duration_ms >= 30000
                        AND feedback.quick_exit = 0))
               AND feedback.manual_delete = 0
               AND association.tag_id IN ({placeholders})
         ) evidence
         WHERE evidence.evidence_rank <= 100
         ORDER BY evidence.last_event_at DESC, evidence.archive_id, evidence.tag_id
         LIMIT {MAX_SOURCE_EVIDENCE}"
    );
    let mut request = sqlx::query(&query).bind(user_id);
    let mut seed_ids = strengths.keys().cloned().collect::<Vec<_>>();
    seed_ids.sort();
    for tag_id in &seed_ids {
        request = request.bind(tag_id);
    }
    let rows = request.fetch_all(pool).await?;
    let source_archive_ids = rows
        .iter()
        .map(|row| row.get::<String, _>("archive_id"))
        .collect::<BTreeSet<_>>();
    let source_archive_tag_ids = if source_archive_ids.is_empty() {
        HashMap::new()
    } else {
        let archive_placeholders = std::iter::repeat("?")
            .take(source_archive_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let tag_query = format!(
            "SELECT archive_id, tag_id FROM archive_tags \
             WHERE archive_id IN ({archive_placeholders})"
        );
        let mut tag_request = sqlx::query(&tag_query);
        for archive_id in &source_archive_ids {
            tag_request = tag_request.bind(archive_id);
        }
        let mut tags_by_archive: HashMap<String, Vec<String>> = HashMap::new();
        for row in tag_request.fetch_all(pool).await? {
            tags_by_archive
                .entry(row.get("archive_id"))
                .or_default()
                .push(row.get("tag_id"));
        }
        tags_by_archive
    };
    let mut evidence = Vec::new();
    for row in rows {
        let tag_id: String = row.get("tag_id");
        let count: i64 = row.get("evidence_count");
        let Some(seed_strength) = strengths.get(&tag_id) else {
            continue;
        };
        if count <= 0 || *seed_strength <= 0.0 {
            continue;
        }
        let archive_id: String = row.get("archive_id");
        evidence.push(SourceArchiveEvidence {
            archive_tag_ids: source_archive_tag_ids
                .get(&archive_id)
                .cloned()
                .unwrap_or_default(),
            archive_id,
            source_tag_id: tag_id,
            source_strength: *seed_strength / count as f64,
        });
    }
    Ok(evidence)
}

/// Applies the same one-hop equation to every numeric edge and returns attribution for the
/// bounded contribution. One source archive contributes at most its strongest path, and a path
/// is skipped when that source archive already carries the target tag.
pub fn score_archive(
    archive_id: &str,
    target_tag_ids: &[String],
    source_evidence: &[SourceArchiveEvidence],
    edges: &[WeightedTagRelationEdge],
    global_gain: f64,
    direct_target_score: Option<f64>,
) -> Result<GraphScore> {
    validate_gain(global_gain)?;
    if let Some(score) = direct_target_score {
        if !score.is_finite() {
            return Err(anyhow!("direct target score must be finite"));
        }
    }
    let target_tags = target_tag_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut evidence_by_tag: HashMap<&str, Vec<&SourceArchiveEvidence>> = HashMap::new();
    for source in source_evidence {
        validate_strength(source.source_strength)?;
        if !source.archive_id.trim().is_empty() && source.archive_id != archive_id {
            evidence_by_tag
                .entry(source.source_tag_id.as_str())
                .or_default()
                .push(source);
        }
    }
    let mut paths = Vec::new();
    for edge in edges {
        validate_edge(edge)?;
        if edge.status != "active" {
            continue;
        }
        for (source_tag_id, target_tag_id) in [
            (edge.tag_a_id.as_str(), edge.tag_b_id.as_str()),
            (edge.tag_b_id.as_str(), edge.tag_a_id.as_str()),
        ] {
            if !target_tags.contains(target_tag_id) {
                continue;
            }
            let Some(sources) = evidence_by_tag.get(source_tag_id) else {
                continue;
            };
            for source in sources {
                if source
                    .archive_tag_ids
                    .iter()
                    .any(|source_tag_id| source_tag_id == target_tag_id)
                {
                    continue;
                }
                let contribution =
                    source.source_strength * edge.signed_weight.max(0.0) * edge.user_influence;
                if contribution != 0.0 {
                    paths.push(EvidencePath {
                        source_archive_id: source.archive_id.clone(),
                        source_tag_id: source_tag_id.to_string(),
                        edge: edge.clone(),
                        contribution,
                    });
                }
            }
        }
    }
    if paths.is_empty() {
        return Ok(GraphScore::default());
    }

    let mut best_by_archive: BTreeMap<String, EvidencePath> = BTreeMap::new();
    for path in paths {
        let replace = best_by_archive
            .get(&path.source_archive_id)
            .is_none_or(|current| {
                path.contribution
                    .abs()
                    .total_cmp(&current.contribution.abs())
                    .is_gt()
                    || (path.contribution.abs() == current.contribution.abs()
                        && (
                            &path.edge.tag_a_id,
                            &path.edge.tag_b_id,
                            &path.source_tag_id,
                        ) < (
                            &current.edge.tag_a_id,
                            &current.edge.tag_b_id,
                            &current.source_tag_id,
                        ))
            });
        if replace {
            best_by_archive.insert(path.source_archive_id.clone(), path);
        }
    }
    let raw_total = best_by_archive
        .values()
        .map(|path| path.contribution)
        .sum::<f64>();
    if raw_total == 0.0 {
        return Ok(GraphScore::default());
    }
    let mut contribution = raw_total.clamp(-1.0, 1.0) * global_gain;
    if let Some(direct_score) = direct_target_score {
        let combined_score = direct_score + contribution;
        if (direct_score > 0.0 && combined_score < 0.0)
            || (direct_score < 0.0 && combined_score > 0.0)
        {
            contribution = -direct_score;
        }
    }
    let normalization = contribution / raw_total;
    let mut by_edge: BTreeMap<(String, String), (f64, BTreeSet<String>)> = BTreeMap::new();
    for path in best_by_archive.values() {
        let entry = by_edge
            .entry((path.edge.tag_a_id.clone(), path.edge.tag_b_id.clone()))
            .or_default();
        entry.0 += path.contribution * normalization;
        entry.1.insert(path.source_archive_id.clone());
    }
    let edge_attributions = by_edge
        .into_iter()
        .filter_map(
            |((tag_a_id, tag_b_id), (signed_contribution, source_archive_ids))| {
                (signed_contribution != 0.0).then_some(GraphEdgeAttribution {
                    tag_a_id,
                    tag_b_id,
                    signed_contribution,
                    source_archive_ids: source_archive_ids.into_iter().collect(),
                })
            },
        )
        .collect();
    Ok(GraphScore {
        contribution,
        edge_attributions,
    })
}

/// Positive actual recommendation contributions train personal influence. Nonpositive historical
/// contributions and unobserved items never update it.
pub fn updated_user_influence(
    current: f64,
    signed_contribution: f64,
    outcome: GraphFeedbackOutcome,
) -> Result<f64> {
    validate_influence(current)?;
    validate_signed_weight(signed_contribution)?;
    if outcome == GraphFeedbackOutcome::Unobserved || signed_contribution <= 0.0 {
        return Ok(current);
    }
    let outcome_sign = match outcome {
        GraphFeedbackOutcome::Positive => 1.0,
        GraphFeedbackOutcome::Negative => -1.0,
        GraphFeedbackOutcome::Unobserved => return Ok(current),
    };
    let amount = signed_contribution;
    let next = if outcome_sign > 0.0 {
        current + (1.0 - current) * amount
    } else {
        current * (1.0 - amount)
    };
    Ok(next.clamp(0.0, 1.0))
}

/// Records edge attribution for a returned recommendation item.
pub async fn record_graph_trials(
    pool: &Pool<Sqlite>,
    item_id: &str,
    user_id: &str,
    attributions: &[GraphEdgeAttribution],
) -> Result<()> {
    if attributions.is_empty() {
        return Ok(());
    }
    let mut transaction = pool.begin().await?;
    for attribution in attributions {
        validate_signed_weight(attribution.signed_contribution)?;
        if attribution.signed_contribution <= 0.0 {
            return Err(anyhow!("graph trial contributions must be positive"));
        }
        if attribution.tag_a_id >= attribution.tag_b_id {
            return Err(anyhow!("graph trial edge IDs must be canonical"));
        }
        sqlx::query(
            "INSERT OR IGNORE INTO random_recommendation_graph_trials
             (id, item_id, user_id, tag_a_id, tag_b_id, signed_contribution,
              source_archive_ids_json)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(item_id)
        .bind(user_id)
        .bind(&attribution.tag_a_id)
        .bind(&attribution.tag_b_id)
        .bind(attribution.signed_contribution)
        .bind(serde_json::to_string(&attribution.source_archive_ids)?)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(())
}

/// Applies each positive outcome once per user/archive/edge and permits one later negative outcome.
pub async fn update_user_factors_for_item(
    pool: &Pool<Sqlite>,
    item_id: &str,
    user_id: &str,
    outcome: GraphFeedbackOutcome,
) -> Result<usize> {
    if outcome == GraphFeedbackOutcome::Unobserved {
        return Ok(0);
    }
    let mut transaction = pool.begin().await?;
    let feedback_column = match outcome {
        GraphFeedbackOutcome::Positive => "positive_feedback_at",
        GraphFeedbackOutcome::Negative => "negative_feedback_at",
        GraphFeedbackOutcome::Unobserved => return Ok(0),
    };
    let previous_outcome_guard = match outcome {
        GraphFeedbackOutcome::Positive => "AND ledger.positive_feedback_at IS NOT NULL",
        GraphFeedbackOutcome::Negative => "AND ledger.negative_feedback_at IS NOT NULL",
        GraphFeedbackOutcome::Unobserved => return Ok(0),
    };
    let positive_guard = if outcome == GraphFeedbackOutcome::Positive {
        "AND trial.negative_feedback_at IS NULL AND item.manual_delete_at IS NULL
           AND item.quick_exit_at IS NULL
           AND NOT EXISTS (
             SELECT 1 FROM tag_relation_user_archive_feedback prior
             WHERE prior.user_id = trial.user_id AND prior.archive_id = item.archive_id
               AND prior.tag_a_id = trial.tag_a_id AND prior.tag_b_id = trial.tag_b_id
               AND prior.negative_feedback_at IS NOT NULL
           )"
    } else {
        ""
    };
    let query = format!(
        "SELECT trial.id, trial.tag_a_id, trial.tag_b_id, trial.signed_contribution,
                item.archive_id
         FROM random_recommendation_graph_trials trial
         JOIN random_recommendation_items item ON item.id = trial.item_id
         WHERE trial.item_id = ? AND trial.user_id = ?
           AND trial.{feedback_column} IS NULL {positive_guard}
           AND NOT EXISTS (
             SELECT 1 FROM tag_relation_user_archive_feedback ledger
             WHERE ledger.user_id = trial.user_id AND ledger.archive_id = item.archive_id
               AND ledger.tag_a_id = trial.tag_a_id AND ledger.tag_b_id = trial.tag_b_id
               {previous_outcome_guard}
           )"
    );
    let rows = sqlx::query(&query)
        .bind(item_id)
        .bind(user_id)
        .fetch_all(&mut *transaction)
        .await?;
    let mut updated = 0;
    for row in rows {
        let archive_id: String = row.get("archive_id");
        let tag_a_id: String = row.get("tag_a_id");
        let tag_b_id: String = row.get("tag_b_id");
        let signed_contribution: f64 = row.get("signed_contribution");
        if signed_contribution > 0.0 {
            let current = sqlx::query_scalar::<_, f64>(
                "SELECT influence FROM tag_relation_user_factors
                 WHERE user_id = ? AND tag_a_id = ? AND tag_b_id = ?",
            )
            .bind(user_id)
            .bind(&tag_a_id)
            .bind(&tag_b_id)
            .fetch_optional(&mut *transaction)
            .await?
            .unwrap_or(1.0);
            let next = updated_user_influence(current, signed_contribution, outcome)?;
            sqlx::query(
                "INSERT INTO tag_relation_user_factors
                 (user_id, tag_a_id, tag_b_id, influence, updated_at)
                 VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)
                 ON CONFLICT(user_id, tag_a_id, tag_b_id) DO UPDATE SET
                  influence = excluded.influence, updated_at = CURRENT_TIMESTAMP",
            )
            .bind(user_id)
            .bind(&tag_a_id)
            .bind(&tag_b_id)
            .bind(next)
            .execute(&mut *transaction)
            .await?;
            sqlx::query(&format!(
                "INSERT INTO tag_relation_user_archive_feedback
                 (user_id, archive_id, tag_a_id, tag_b_id, {feedback_column}, updated_at)
                 VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                 ON CONFLICT(user_id, archive_id, tag_a_id, tag_b_id) DO UPDATE SET
                  {feedback_column} = COALESCE(
                      tag_relation_user_archive_feedback.{feedback_column}, excluded.{feedback_column}),
                  updated_at = CURRENT_TIMESTAMP"
            ))
            .bind(user_id)
            .bind(&archive_id)
            .bind(&tag_a_id)
            .bind(&tag_b_id)
            .execute(&mut *transaction)
            .await?;
            updated += 1;
        }
        sqlx::query(&format!(
            "UPDATE random_recommendation_graph_trials
             SET {feedback_column} = CURRENT_TIMESTAMP WHERE id = ?"
        ))
        .bind(row.get::<String, _>("id"))
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(updated)
}

fn validate_edge(edge: &WeightedTagRelationEdge) -> Result<()> {
    if edge.tag_a_id >= edge.tag_b_id {
        return Err(anyhow!("tag relation edge IDs must be canonical"));
    }
    validate_signed_weight(edge.signed_weight)?;
    validate_confidence(edge.confidence)?;
    validate_influence(edge.user_influence)?;
    Ok(())
}

fn validate_signed_weight(value: f64) -> Result<()> {
    if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
        return Err(anyhow!("signed tag relation weight must be within [-1, 1]"));
    }
    Ok(())
}

fn validate_optional_weight(value: Option<f64>) -> Result<()> {
    if let Some(value) = value {
        validate_signed_weight(value)?;
    }
    Ok(())
}

fn validate_confidence(value: Option<f64>) -> Result<()> {
    if value.is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value)) {
        return Err(anyhow!("tag relation confidence must be within [0, 1]"));
    }
    Ok(())
}

fn validate_strength(value: f64) -> Result<()> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(anyhow!("positive source strength must be within [0, 1]"));
    }
    Ok(())
}

fn validate_influence(value: f64) -> Result<()> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(anyhow!("user edge influence must be within [0, 1]"));
    }
    Ok(())
}

fn validate_gain(value: f64) -> Result<()> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(anyhow!("weighted graph gain must be within [0, 1]"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn feedback_test_pool() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .unwrap();
        for statement in [
            "CREATE TABLE random_recommendation_sessions (
                id TEXT PRIMARY KEY, user_id TEXT NOT NULL, expires_at DATETIME NOT NULL
            )",
            "CREATE TABLE random_recommendation_items (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL REFERENCES random_recommendation_sessions(id) ON DELETE CASCADE,
                user_id TEXT NOT NULL, archive_id TEXT NOT NULL,
                quick_exit_at DATETIME, manual_delete_at DATETIME
            )",
            "CREATE TABLE random_recommendation_graph_trials (
                id TEXT PRIMARY KEY,
                item_id TEXT NOT NULL REFERENCES random_recommendation_items(id) ON DELETE CASCADE,
                user_id TEXT NOT NULL, tag_a_id TEXT NOT NULL, tag_b_id TEXT NOT NULL,
                signed_contribution REAL NOT NULL, source_archive_ids_json TEXT NOT NULL,
                positive_feedback_at DATETIME, negative_feedback_at DATETIME,
                UNIQUE(item_id, tag_a_id, tag_b_id)
            )",
            "CREATE TABLE tag_relation_user_factors (
                user_id TEXT NOT NULL, tag_a_id TEXT NOT NULL, tag_b_id TEXT NOT NULL,
                influence REAL NOT NULL, updated_at DATETIME NOT NULL,
                PRIMARY KEY(user_id, tag_a_id, tag_b_id)
            )",
            "CREATE TABLE tag_relation_user_archive_feedback (
                user_id TEXT NOT NULL, archive_id TEXT NOT NULL,
                tag_a_id TEXT NOT NULL, tag_b_id TEXT NOT NULL,
                positive_feedback_at DATETIME, negative_feedback_at DATETIME,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY(user_id, archive_id, tag_a_id, tag_b_id)
            )",
        ] {
            sqlx::query(statement).execute(&pool).await.unwrap();
        }
        pool
    }

    async fn insert_feedback_trial(
        pool: &Pool<Sqlite>,
        session_id: &str,
        item_id: &str,
        trial_id: &str,
        archive_id: &str,
        signed_contribution: f64,
    ) {
        sqlx::query(
            "INSERT INTO random_recommendation_sessions (id, user_id, expires_at)
             VALUES (?, 'user-1', datetime('now', '+1 day'))",
        )
        .bind(session_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO random_recommendation_items (id, session_id, user_id, archive_id)
             VALUES (?, ?, 'user-1', ?)",
        )
        .bind(item_id)
        .bind(session_id)
        .bind(archive_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO random_recommendation_graph_trials
             (id, item_id, user_id, tag_a_id, tag_b_id, signed_contribution,
              source_archive_ids_json)
             VALUES (?, ?, 'user-1', 'tag-a', 'tag-b', ?, '[]')",
        )
        .bind(trial_id)
        .bind(item_id)
        .bind(signed_contribution)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn insert_migrated_feedback_trial(
        pool: &Pool<Sqlite>,
        session_id: &str,
        item_id: &str,
        trial_id: &str,
        archive_id: &str,
        signed_contribution: f64,
    ) {
        sqlx::query(
            "INSERT INTO random_recommendation_sessions (id, user_id, exploration_ratio)
             VALUES (?, 'user-1', 0.0)",
        )
        .bind(session_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO random_recommendation_items
             (id, session_id, user_id, archive_id, position, preference_tier, sampling_weight)
             VALUES (?, ?, 'user-1', ?, 1, 'unknown', 1.0)",
        )
        .bind(item_id)
        .bind(session_id)
        .bind(archive_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO random_recommendation_graph_trials
             (id, item_id, user_id, tag_a_id, tag_b_id, signed_contribution,
              source_archive_ids_json)
             VALUES (?, ?, 'user-1', 'tag-a', 'tag-b', ?, '[]')",
        )
        .bind(trial_id)
        .bind(item_id)
        .bind(signed_contribution)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn migrate_sqlite_through_0039(pool: &Pool<Sqlite>) {
        let mut migrator = sqlx::migrate!("./migrations/sqlite");
        migrator.set_ignore_missing(true);
        migrator.migrations = std::borrow::Cow::Owned(
            migrator
                .migrations
                .iter()
                .filter(|migration| migration.version < 40)
                .cloned()
                .collect(),
        );
        let mut connection = pool.acquire().await.unwrap();
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(&mut *connection)
            .await
            .unwrap();
        migrator.run(&mut *connection).await.unwrap();
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&mut *connection)
            .await
            .unwrap();
    }

    fn edge(weight: f64, influence: f64) -> WeightedTagRelationEdge {
        WeightedTagRelationEdge {
            tag_a_id: "tag-a".to_string(),
            tag_b_id: "tag-b".to_string(),
            signed_weight: weight,
            confidence: Some(0.8),
            status: "active".to_string(),
            user_influence: influence,
        }
    }

    fn source(archive_id: &str, tag_id: &str, strength: f64) -> SourceArchiveEvidence {
        SourceArchiveEvidence {
            archive_id: archive_id.to_string(),
            source_tag_id: tag_id.to_string(),
            source_strength: strength,
            archive_tag_ids: Vec::new(),
        }
    }

    #[test]
    fn only_positive_relations_contribute_and_are_attributed() {
        let positive = score_archive(
            "target",
            &["tag-b".to_string()],
            &[source("source", "tag-a", 0.5)],
            &[edge(0.8, 0.5)],
            DEFAULT_WEIGHTED_GRAPH_GAIN,
            None,
        )
        .unwrap();
        let negative = score_archive(
            "target",
            &["tag-b".to_string()],
            &[source("source", "tag-a", 0.5)],
            &[edge(-0.8, 0.5)],
            DEFAULT_WEIGHTED_GRAPH_GAIN,
            None,
        )
        .unwrap();
        let neutral = score_archive(
            "target",
            &["tag-b".to_string()],
            &[source("source", "tag-a", 0.5)],
            &[edge(0.0, 0.5)],
            DEFAULT_WEIGHTED_GRAPH_GAIN,
            None,
        )
        .unwrap();
        assert_eq!(positive.contribution, 0.06);
        assert!(positive.edge_attributions[0].signed_contribution > 0.0);
        assert_eq!(
            positive.edge_attributions[0].signed_contribution,
            positive.contribution
        );
        assert_eq!(negative, GraphScore::default());
        assert_eq!(neutral, GraphScore::default());
    }

    #[test]
    fn rejects_out_of_range_and_non_finite_scores() {
        let write = |weight| TagRelationWeightWrite {
            tag_a_id: "tag-a".to_string(),
            tag_b_id: "tag-b".to_string(),
            signed_weight: weight,
            confidence: None,
            score_a_to_b: None,
            score_b_to_a: None,
            input_hash: "hash".to_string(),
            scorer_version: "scorer-v1".to_string(),
            profile_id: None,
            provider: None,
            model: None,
        };
        assert!(write(1.01).canonicalize().is_err());
        assert!(write(f64::NAN).canonicalize().is_err());
        assert!(write(f64::INFINITY).canonicalize().is_err());
    }

    #[test]
    fn source_archive_dedup_and_total_bound_prevent_multiplication() {
        let score = score_archive(
            "target",
            &["tag-b".to_string(), "tag-c".to_string()],
            &[
                source("same-book", "tag-a", 0.7),
                source("same-book", "tag-d", 0.7),
                source("book-two", "tag-a", 0.7),
                source("book-three", "tag-a", 0.7),
                source("book-four", "tag-a", 0.7),
            ],
            &[
                edge(0.9, 1.0),
                WeightedTagRelationEdge {
                    tag_a_id: "tag-c".to_string(),
                    tag_b_id: "tag-d".to_string(),
                    ..edge(0.8, 1.0)
                },
            ],
            DEFAULT_WEIGHTED_GRAPH_GAIN,
            None,
        )
        .unwrap();
        assert_eq!(score.contribution, DEFAULT_WEIGHTED_GRAPH_GAIN);
        assert_eq!(
            score
                .edge_attributions
                .iter()
                .map(|attribution| attribution.source_archive_ids.len())
                .sum::<usize>(),
            4
        );
        let source_attribution_counts = score
            .edge_attributions
            .iter()
            .flat_map(|attribution| &attribution.source_archive_ids)
            .fold(BTreeMap::new(), |mut counts, archive_id| {
                *counts.entry(archive_id.as_str()).or_insert(0) += 1;
                counts
            });
        assert_eq!(source_attribution_counts.len(), 4);
        assert!(source_attribution_counts.values().all(|count| *count == 1));
    }

    #[test]
    fn direct_dislike_cannot_be_overridden_by_positive_graph_transfer() {
        let positive = score_archive(
            "target",
            &["tag-b".to_string()],
            &[source("source", "tag-a", 0.5)],
            &[edge(1.0, 1.0)],
            DEFAULT_WEIGHTED_GRAPH_GAIN,
            Some(0.2),
        )
        .unwrap();
        assert!(positive.contribution > 0.0);

        let negative = score_archive(
            "target",
            &["tag-b".to_string()],
            &[source("source", "tag-a", 1.0)],
            &[edge(-1.0, 1.0)],
            DEFAULT_WEIGHTED_GRAPH_GAIN,
            Some(-0.2),
        )
        .unwrap();
        assert_eq!(negative, GraphScore::default());

        let positive_over_negative = score_archive(
            "target",
            &["tag-b".to_string()],
            &[source("source", "tag-a", 1.0)],
            &[edge(1.0, 1.0)],
            DEFAULT_WEIGHTED_GRAPH_GAIN,
            Some(-0.2),
        )
        .unwrap();
        assert_eq!(positive_over_negative.contribution, 0.2);
        assert!(-0.2 + positive_over_negative.contribution <= 0.0);
        assert_eq!(
            positive_over_negative
                .edge_attributions
                .iter()
                .map(|attribution| attribution.signed_contribution)
                .sum::<f64>(),
            positive_over_negative.contribution
        );
    }

    #[test]
    fn source_archive_is_not_counted_again_for_a_tag_it_already_has() {
        let mut evidence = source("same-book", "tag-a", 0.7);
        evidence.archive_tag_ids.push("tag-b".to_string());
        let score = score_archive(
            "target",
            &["tag-b".to_string()],
            &[evidence],
            &[edge(0.9, 1.0)],
            DEFAULT_WEIGHTED_GRAPH_GAIN,
            None,
        )
        .unwrap();
        assert_eq!(score, GraphScore::default());
    }

    #[test]
    fn positive_attributions_restore_or_reduce_influence_but_negative_legacy_ones_do_not() {
        let after_unobserved =
            updated_user_influence(0.8, 0.3, GraphFeedbackOutcome::Unobserved).unwrap();
        let after_positive =
            updated_user_influence(0.8, 0.3, GraphFeedbackOutcome::Positive).unwrap();
        let after_negative =
            updated_user_influence(0.8, 0.3, GraphFeedbackOutcome::Negative).unwrap();
        let after_legacy_negative =
            updated_user_influence(0.8, -0.3, GraphFeedbackOutcome::Positive).unwrap();
        assert_eq!(after_unobserved, 0.8);
        assert!((after_positive - 0.86).abs() < f64::EPSILON);
        assert!((after_negative - 0.56).abs() < f64::EPSILON);
        assert_eq!(after_legacy_negative, 0.8);
        assert_eq!(
            updated_user_influence(0.8, 0.0, GraphFeedbackOutcome::Negative).unwrap(),
            0.8
        );
    }

    #[test]
    fn observing_edges_do_not_score() {
        let mut observed = edge(0.9, 1.0);
        observed.status = "observing".to_string();
        let score = score_archive(
            "target",
            &["tag-b".to_string()],
            &[source("source", "tag-a", 1.0)],
            &[observed],
            DEFAULT_WEIGHTED_GRAPH_GAIN,
            None,
        )
        .unwrap();
        assert_eq!(score, GraphScore::default());
    }

    #[tokio::test]
    async fn observer_rows_are_not_loaded_even_when_policy_is_enabled() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::database::run_sqlite_migrations(&pool).await.unwrap();
        sqlx::query("INSERT INTO tags (id, name, namespace) VALUES ('tag-a', 'clothing', 'general'), ('tag-b', 'clothing', 'metadata')")
            .execute(&pool)
            .await
            .unwrap();
        let mut settings = crate::services::ai_service::load_ai_settings(&pool)
            .await
            .unwrap();
        settings.features.recommendations.tag_relation.api_key =
            Some("test-provider-key".to_string());
        crate::services::ai_service::save_ai_settings(&pool, settings)
            .await
            .unwrap();
        sqlx::query("INSERT INTO tag_relation_weight_edges (tag_a_id, tag_b_id, signed_weight, input_hash, scorer_version, status) VALUES ('tag-a', 'tag-b', 0.9, 'hash', 'test-v1', 'observing')")
            .execute(&pool)
            .await
            .unwrap();
        let seeds = HashMap::from([("tag-a".to_string(), 0.5)]);
        let (policy, edges) = load_active_weighted_neighbors(&pool, "user-1", &seeds, 20)
            .await
            .unwrap();
        assert!(policy.enabled);
        assert!(edges.is_empty());
    }

    #[tokio::test]
    async fn negative_edges_do_not_use_the_per_seed_neighbor_limit() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::database::run_sqlite_migrations(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO tags (id, name, namespace)
             VALUES ('tag-a', 'seed', 'general'),
                    ('tag-b', 'weak-positive', 'general'),
                    ('tag-c', 'strong-negative', 'general')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let mut settings = crate::services::ai_service::load_ai_settings(&pool)
            .await
            .unwrap();
        settings.features.recommendations.tag_relation.api_key =
            Some("test-provider-key".to_string());
        crate::services::ai_service::save_ai_settings(&pool, settings)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO tag_relation_weight_edges
             (tag_a_id, tag_b_id, signed_weight, input_hash, scorer_version, status)
             VALUES ('tag-a', 'tag-b', 0.1, 'positive-hash', 'test-v1', 'active'),
                    ('tag-a', 'tag-c', -0.99, 'negative-hash', 'test-v1', 'active')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let seeds = HashMap::from([("tag-a".to_string(), 0.5)]);
        let (policy, edges) = load_active_weighted_neighbors(&pool, "user-1", &seeds, 1)
            .await
            .unwrap();

        assert!(policy.enabled);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].tag_a_id, "tag-a");
        assert_eq!(edges[0].tag_b_id, "tag-b");
        assert_eq!(edges[0].signed_weight, 0.1);
        let diagnostic_edges = list_scored_relation_weights(&pool, "active", 10)
            .await
            .unwrap();
        assert_eq!(diagnostic_edges.len(), 2);
        assert!(diagnostic_edges
            .iter()
            .any(|edge| edge.signed_weight == -0.99));
    }

    #[tokio::test]
    async fn score_upserts_increment_revision_and_review_requires_the_current_revision() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::database::run_sqlite_migrations(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO tags (id, name, namespace)
             VALUES ('tag-a', 'woman', 'general'), ('tag-b', 'clothing', 'general')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let mut settings = crate::services::ai_service::load_ai_settings(&pool)
            .await
            .unwrap();
        settings.features.recommendations.tag_relation.api_key =
            Some("test-provider-key".to_string());
        crate::services::ai_service::save_ai_settings(&pool, settings.clone())
            .await
            .unwrap();
        let pair = crate::services::recommendations::semantic_edges::TagRelationPair {
            pair_id: String::new(),
            tag_a: crate::services::recommendations::semantic_edges::TagRelationTag {
                id: "tag-a".to_string(),
                namespace: "general".to_string(),
                name: "woman".to_string(),
                support_count: 0,
            },
            tag_b: crate::services::recommendations::semantic_edges::TagRelationTag {
                id: "tag-b".to_string(),
                namespace: "general".to_string(),
                name: "clothing".to_string(),
                support_count: 0,
            },
            pair_input_hash: String::new(),
        }
        .canonicalize()
        .unwrap();
        let write = TagRelationWeightWrite {
            tag_a_id: "tag-a".to_string(),
            tag_b_id: "tag-b".to_string(),
            signed_weight: 0.4,
            confidence: Some(0.8),
            score_a_to_b: Some(0.5),
            score_b_to_a: Some(0.3),
            input_hash: pair.pair_input_hash.clone(),
            scorer_version: crate::services::ai_service::tag_relation_scorer_version(
                &settings.features.recommendations.tag_relation,
                false,
            ),
            profile_id: Some("profile-a".to_string()),
            provider: Some("jev".to_string()),
            model: Some("score-model".to_string()),
        };

        upsert_scored_relation_weight(&pool, &write).await.unwrap();
        let first = list_scored_relation_weights(&pool, "active", 10)
            .await
            .unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].revision, 1);
        assert_eq!(first[0].name_a, "woman");

        settings.features.recommendations.tag_graph_enabled = false;
        crate::services::ai_service::save_ai_settings(&pool, settings.clone())
            .await
            .unwrap();
        let rescored_while_disabled = TagRelationWeightWrite {
            model: Some("score-model-v2".to_string()),
            ..write.clone()
        };
        upsert_scored_relation_weight(&pool, &rescored_while_disabled)
            .await
            .unwrap();
        let observing = list_scored_relation_weights(&pool, "observing", 10)
            .await
            .unwrap();
        assert_eq!(observing.len(), 1);
        assert_eq!(observing[0].revision, 2);
        assert_eq!(observing[0].model.as_deref(), Some("score-model-v2"));

        settings.features.recommendations.tag_graph_enabled = true;
        crate::services::ai_service::save_ai_settings(&pool, settings.clone())
            .await
            .unwrap();
        let seeds = HashMap::from([("tag-a".to_string(), 0.5)]);
        let (policy, edges) = load_active_weighted_neighbors(&pool, "user-1", &seeds, 20)
            .await
            .unwrap();
        assert!(policy.enabled);
        assert!(edges.is_empty());

        assert!(review_weighted_relation_edge(
            &pool,
            "tag-a",
            "tag-b",
            2,
            &write.input_hash,
            &write.scorer_version,
            "active",
        )
        .await
        .unwrap());

        let active = list_scored_relation_weights(&pool, "active", 10)
            .await
            .unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].revision, 3);
        assert_eq!(active[0].model.as_deref(), Some("score-model-v2"));
        assert!(review_weighted_relation_edge(
            &pool,
            "tag-a",
            "tag-b",
            3,
            &write.input_hash,
            &write.scorer_version,
            "rejected",
        )
        .await
        .unwrap());
        let rescored_after_rejection = TagRelationWeightWrite {
            model: Some("score-model-v3".to_string()),
            ..write.clone()
        };
        upsert_scored_relation_weight(&pool, &rescored_after_rejection)
            .await
            .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT status FROM tag_relation_weight_edges
                 WHERE tag_a_id='tag-a' AND tag_b_id='tag-b'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "rejected"
        );
        let rejected = list_scored_relation_weights(&pool, "rejected", 10)
            .await
            .unwrap();
        assert_eq!(rejected[0].revision, 4);
        assert_eq!(rejected[0].model.as_deref(), Some("score-model-v2"));
    }

    #[tokio::test]
    async fn weighted_tag_graph_status_counts_only_jev_jobs_and_reports_pause_and_failed_jobs() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::database::run_sqlite_migrations(&pool).await.unwrap();

        let unconfigured = load_weighted_tag_graph_status(&pool).await.unwrap();
        assert!(unconfigured.enabled);
        assert!(!unconfigured.configured);
        assert_eq!(unconfigured.state, "unconfigured");

        let mut settings = crate::services::ai_service::load_ai_settings(&pool)
            .await
            .unwrap();
        settings.features.recommendations.tag_relation.api_key =
            Some("test-provider-key".to_string());
        crate::services::ai_service::save_ai_settings(&pool, settings)
            .await
            .unwrap();
        assert_eq!(
            load_weighted_tag_graph_status(&pool).await.unwrap().state,
            "waiting_tags"
        );

        for (id, job_type, status, attempts, error, created_offset) in [
            (
                "jev-queued",
                JEV_QUEUE_JOB_TYPE,
                "pending",
                0,
                None,
                "-1 minute",
            ),
            (
                "jev-processing",
                JEV_QUEUE_JOB_TYPE,
                "processing",
                1,
                None,
                "-1 minute",
            ),
            (
                "jev-retry",
                JEV_QUEUE_JOB_TYPE,
                "pending",
                2,
                Some("timeout"),
                "-1 minute",
            ),
            (
                "jev-auth-failed",
                JEV_QUEUE_JOB_TYPE,
                "failed",
                4,
                Some("HTTP 401 unauthorized"),
                "+1 second",
            ),
            (
                "other-retry",
                "auto_tagging",
                "pending",
                2,
                Some("timeout"),
                "+1 minute",
            ),
        ] {
            sqlx::query(
                "INSERT INTO ai_processing_queue
                 (id, archive_id, status, attempts, job_type, last_error, next_run_at, created_at)
                 VALUES (?, NULL, ?, ?, ?, ?, datetime('now', '+1 hour'), datetime('now', ?))",
            )
            .bind(id)
            .bind(status)
            .bind(attempts)
            .bind(job_type)
            .bind(error)
            .bind(created_offset)
            .execute(&pool)
            .await
            .unwrap();
        }
        sqlx::query("UPDATE ai_queue_controls SET manually_paused = 1 WHERE job_type = ?")
            .bind(JEV_QUEUE_JOB_TYPE)
            .execute(&pool)
            .await
            .unwrap();

        let paused = load_weighted_tag_graph_status(&pool).await.unwrap();
        assert!(paused.paused);
        assert_eq!(paused.state, "paused");
        assert_eq!(paused.queued_task_count, 1);
        assert_eq!(paused.processing_task_count, 1);
        assert_eq!(paused.retry_waiting_task_count, 1);
        assert!(paused.next_retry_at.is_some());
        assert_eq!(paused.last_error.as_deref(), Some("HTTP 401 unauthorized"));

        sqlx::query("UPDATE ai_queue_controls SET manually_paused = 0 WHERE job_type = ?")
            .bind(JEV_QUEUE_JOB_TYPE)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            load_weighted_tag_graph_status(&pool).await.unwrap().state,
            "needs_attention"
        );

        sqlx::query(
            "UPDATE ai_processing_queue SET status = 'completed'
             WHERE job_type = ? AND status IN ('pending', 'processing', 'failed')",
        )
        .bind(JEV_QUEUE_JOB_TYPE)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO ai_processing_queue
             (id, archive_id, status, attempts, job_type, last_error, created_at)
             VALUES ('jev-format-failed', NULL, 'failed', 4, ?, 'invalid response shape', datetime('now', '+2 seconds'))",
        )
        .bind(JEV_QUEUE_JOB_TYPE)
        .execute(&pool)
        .await
        .unwrap();
        let terminal_failure = load_weighted_tag_graph_status(&pool).await.unwrap();
        assert_eq!(terminal_failure.state, "needs_attention");
        assert_eq!(terminal_failure.retry_waiting_task_count, 0);
        assert_eq!(terminal_failure.queued_task_count, 0);
        assert_eq!(terminal_failure.processing_task_count, 0);
        assert_eq!(
            terminal_failure.last_error.as_deref(),
            Some("invalid response shape")
        );

        let mut settings = crate::services::ai_service::load_ai_settings(&pool)
            .await
            .unwrap();
        settings.features.recommendations.tag_graph_enabled = false;
        crate::services::ai_service::save_ai_settings(&pool, settings.clone())
            .await
            .unwrap();
        assert_eq!(
            load_weighted_tag_graph_status(&pool).await.unwrap().state,
            "disabled"
        );

        sqlx::query("DELETE FROM settings WHERE key = 'ai_tag_relation_jev_api_key'")
            .execute(&pool)
            .await
            .unwrap();
        settings.features.recommendations.tag_graph_enabled = true;
        settings.features.recommendations.tag_relation.api_key = None;
        crate::services::ai_service::save_ai_settings(&pool, settings)
            .await
            .unwrap();
        assert_eq!(
            load_weighted_tag_graph_status(&pool).await.unwrap().state,
            "unconfigured"
        );
    }

    #[tokio::test]
    async fn metadata_tag_relations_cannot_be_activated_by_review() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::database::run_sqlite_migrations(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO tags (id, name, namespace)
             VALUES ('tag-a', 'woman', 'general'), ('tag-z', 'Space', 'theme')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let write = TagRelationWeightWrite {
            tag_a_id: "tag-a".to_string(),
            tag_b_id: "tag-z".to_string(),
            signed_weight: 0.4,
            confidence: None,
            score_a_to_b: None,
            score_b_to_a: None,
            input_hash: "theme-pair-hash".to_string(),
            scorer_version: "jev-score-v1".to_string(),
            profile_id: None,
            provider: None,
            model: None,
        };
        upsert_scored_relation_weight(&pool, &write).await.unwrap();

        let error = review_weighted_relation_edge(
            &pool,
            "tag-a",
            "tag-z",
            1,
            "theme-pair-hash",
            "jev-score-v1",
            "active",
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("metadata tag relations"));
    }

    #[tokio::test]
    async fn weighted_graph_policy_starts_disabled_and_uses_optimistic_versions() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::database::run_sqlite_migrations(&pool).await.unwrap();

        let initial = load_weighted_graph_policy(&pool).await.unwrap();
        assert!(!initial.enabled);
        assert_eq!(initial.global_gain, DEFAULT_WEIGHTED_GRAPH_GAIN);
        assert_eq!(initial.version, 0);

        assert!(update_weighted_graph_policy(&pool, 0, 0.4).await.unwrap());
        assert!(!update_weighted_graph_policy(&pool, 0, 0.1).await.unwrap());
        let updated = load_weighted_graph_policy(&pool).await.unwrap();
        assert!(!updated.enabled);
        assert_eq!(updated.global_gain, 0.4);
        assert_eq!(updated.version, 1);
    }

    #[tokio::test]
    async fn archive_feedback_ledger_dedupes_across_sessions_and_preserves_outcome_order() {
        let pool = feedback_test_pool().await;
        sqlx::query(
            "INSERT INTO tag_relation_user_factors
             (user_id, tag_a_id, tag_b_id, influence, updated_at)
             VALUES ('user-1', 'tag-a', 'tag-b', 0.2, CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .unwrap();
        insert_feedback_trial(
            &pool,
            "session-first",
            "item-first",
            "trial-first",
            "archive-one",
            0.4,
        )
        .await;

        assert_eq!(
            update_user_factors_for_item(
                &pool,
                "item-first",
                "user-1",
                GraphFeedbackOutcome::Positive,
            )
            .await
            .unwrap(),
            1
        );
        let after_positive: f64 = sqlx::query_scalar(
            "SELECT influence FROM tag_relation_user_factors
             WHERE user_id='user-1' AND tag_a_id='tag-a' AND tag_b_id='tag-b'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!((after_positive - 0.52).abs() < f64::EPSILON);

        sqlx::query("DELETE FROM random_recommendation_sessions WHERE id='session-first'")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM tag_relation_user_archive_feedback
                 WHERE user_id='user-1' AND archive_id='archive-one'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );

        insert_feedback_trial(
            &pool,
            "session-second",
            "item-second",
            "trial-second",
            "archive-one",
            0.4,
        )
        .await;
        assert_eq!(
            update_user_factors_for_item(
                &pool,
                "item-second",
                "user-1",
                GraphFeedbackOutcome::Positive,
            )
            .await
            .unwrap(),
            0
        );
        assert_eq!(
            update_user_factors_for_item(
                &pool,
                "item-second",
                "user-1",
                GraphFeedbackOutcome::Negative,
            )
            .await
            .unwrap(),
            1
        );
        let after_negative: f64 = sqlx::query_scalar(
            "SELECT influence FROM tag_relation_user_factors
             WHERE user_id='user-1' AND tag_a_id='tag-a' AND tag_b_id='tag-b'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!((after_negative - 0.312).abs() < f64::EPSILON);

        insert_feedback_trial(
            &pool,
            "session-third",
            "item-third",
            "trial-third",
            "archive-one",
            0.4,
        )
        .await;
        assert_eq!(
            update_user_factors_for_item(
                &pool,
                "item-third",
                "user-1",
                GraphFeedbackOutcome::Negative,
            )
            .await
            .unwrap(),
            0
        );
        assert_eq!(
            update_user_factors_for_item(
                &pool,
                "item-third",
                "user-1",
                GraphFeedbackOutcome::Positive,
            )
            .await
            .unwrap(),
            0
        );
        let final_influence: f64 = sqlx::query_scalar(
            "SELECT influence FROM tag_relation_user_factors
             WHERE user_id='user-1' AND tag_a_id='tag-a' AND tag_b_id='tag-b'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(final_influence, after_negative);
    }

    #[tokio::test]
    async fn legacy_negative_and_zero_trials_record_feedback_without_training_or_blocking_new_positive_trials(
    ) {
        let pool = feedback_test_pool().await;
        sqlx::query(
            "INSERT INTO tag_relation_user_factors
             (user_id, tag_a_id, tag_b_id, influence, updated_at)
             VALUES ('user-1', 'tag-a', 'tag-b', 0.4, CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .unwrap();
        insert_feedback_trial(
            &pool,
            "session-old-negative",
            "item-old-negative",
            "trial-old-negative",
            "archive-negative",
            -0.3,
        )
        .await;
        insert_feedback_trial(
            &pool,
            "session-old-zero",
            "item-old-zero",
            "trial-old-zero",
            "archive-zero",
            0.0,
        )
        .await;

        assert_eq!(
            update_user_factors_for_item(
                &pool,
                "item-old-negative",
                "user-1",
                GraphFeedbackOutcome::Positive,
            )
            .await
            .unwrap(),
            0
        );
        assert_eq!(
            update_user_factors_for_item(
                &pool,
                "item-old-zero",
                "user-1",
                GraphFeedbackOutcome::Negative,
            )
            .await
            .unwrap(),
            0
        );
        let unchanged: f64 = sqlx::query_scalar(
            "SELECT influence FROM tag_relation_user_factors
             WHERE user_id='user-1' AND tag_a_id='tag-a' AND tag_b_id='tag-b'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(unchanged, 0.4);
        let historical_outcomes: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT id, positive_feedback_at, negative_feedback_at
             FROM random_recommendation_graph_trials ORDER BY id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(historical_outcomes[0].0, "trial-old-negative");
        assert!(historical_outcomes[0].1.is_some());
        assert!(historical_outcomes[0].2.is_none());
        assert_eq!(historical_outcomes[1].0, "trial-old-zero");
        assert!(historical_outcomes[1].1.is_none());
        assert!(historical_outcomes[1].2.is_some());
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM tag_relation_user_archive_feedback",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            0
        );

        insert_feedback_trial(
            &pool,
            "session-current-positive",
            "item-current-positive",
            "trial-current-positive",
            "archive-negative",
            0.25,
        )
        .await;
        assert_eq!(
            update_user_factors_for_item(
                &pool,
                "item-current-positive",
                "user-1",
                GraphFeedbackOutcome::Positive,
            )
            .await
            .unwrap(),
            1
        );
        let learned: f64 = sqlx::query_scalar(
            "SELECT influence FROM tag_relation_user_factors
             WHERE user_id='user-1' AND tag_a_id='tag-a' AND tag_b_id='tag-b'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!((learned - 0.55).abs() < f64::EPSILON);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM tag_relation_user_archive_feedback
                 WHERE user_id='user-1' AND archive_id='archive-negative'
                   AND positive_feedback_at IS NOT NULL",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn nonnegative_transfer_migration_resets_only_graph_factors_and_preserves_feedback_history(
    ) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        migrate_sqlite_through_0039(&pool).await;
        sqlx::query(
            "INSERT INTO users (id, username, email, role, password_hash, api_key)
             VALUES ('user-1', 'graph-user', NULL, 'user', 'test-hash', 'test-key')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tags (id, name, namespace)
             VALUES ('tag-a', 'source', 'general'), ('tag-b', 'target', 'general')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO random_recommendation_sessions (id, user_id, exploration_ratio)
             VALUES ('session-old', 'user-1', 0.0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO random_recommendation_items
             (id, session_id, user_id, archive_id, position, preference_tier,
              sampling_weight)
             VALUES ('item-old', 'session-old', 'user-1', 'archive-old', 1, 'unknown', 1.0)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO random_recommendation_graph_trials
             (id, item_id, user_id, tag_a_id, tag_b_id, signed_contribution,
              source_archive_ids_json, negative_feedback_at)
             VALUES ('trial-old', 'item-old', 'user-1', 'tag-a', 'tag-b', -0.3, '[]',
                     '2026-10-02 10:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tag_relation_user_factors
             (user_id, tag_a_id, tag_b_id, influence)
             VALUES ('user-1', 'tag-a', 'tag-b', 0.23)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tag_relation_user_archive_feedback
             (user_id, archive_id, tag_a_id, tag_b_id,
              negative_feedback_at, updated_at)
             VALUES ('user-1', 'archive-old', 'tag-a', 'tag-b',
                     '2026-10-02 10:00:00', '2026-10-02 10:05:00')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO preference_feedback_aggregates
             (user_id, archive_id, manual_delete, first_event_at, last_event_at)
             VALUES ('user-1', 'archive-direct', 1,
                     '2026-10-02 10:00:00', '2026-10-02 10:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();

        crate::database::run_sqlite_migrations(&pool).await.unwrap();

        let influence: f64 = sqlx::query_scalar(
            "SELECT influence FROM tag_relation_user_factors
             WHERE user_id='user-1' AND tag_a_id='tag-a' AND tag_b_id='tag-b'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(influence, 1.0);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM tag_relation_user_archive_feedback",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            0
        );
        let old_trial: (f64, Option<String>) = sqlx::query_as(
            "SELECT signed_contribution, negative_feedback_at
             FROM random_recommendation_graph_trials WHERE id='trial-old'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(old_trial.0, -0.3);
        assert_eq!(old_trial.1.as_deref(), Some("2026-10-02 10:00:00"));
        let old_ledger: (Option<String>, Option<String>, String) = sqlx::query_as(
            "SELECT positive_feedback_at, negative_feedback_at, updated_at
             FROM tag_relation_user_archive_feedback_history
             WHERE user_id='user-1' AND archive_id='archive-old'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(old_ledger.0.is_none());
        assert_eq!(old_ledger.1.as_deref(), Some("2026-10-02 10:00:00"));
        assert_eq!(old_ledger.2, "2026-10-02 10:05:00");
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT manual_delete FROM preference_feedback_aggregates
                 WHERE user_id='user-1' AND archive_id='archive-direct'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );

        insert_migrated_feedback_trial(
            &pool,
            "session-new-negative",
            "item-new-negative",
            "trial-new-negative",
            "archive-training",
            0.4,
        )
        .await;
        assert_eq!(
            update_user_factors_for_item(
                &pool,
                "item-new-negative",
                "user-1",
                GraphFeedbackOutcome::Negative,
            )
            .await
            .unwrap(),
            1
        );
        let after_current_negative: f64 = sqlx::query_scalar(
            "SELECT influence FROM tag_relation_user_factors
             WHERE user_id='user-1' AND tag_a_id='tag-a' AND tag_b_id='tag-b'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!((after_current_negative - 0.6).abs() < f64::EPSILON);

        insert_migrated_feedback_trial(
            &pool,
            "session-new-positive",
            "item-new-positive",
            "trial-new-positive",
            "archive-old",
            0.25,
        )
        .await;
        assert_eq!(
            update_user_factors_for_item(
                &pool,
                "item-new-positive",
                "user-1",
                GraphFeedbackOutcome::Positive,
            )
            .await
            .unwrap(),
            1
        );
        let after_current_positive: f64 = sqlx::query_scalar(
            "SELECT influence FROM tag_relation_user_factors
             WHERE user_id='user-1' AND tag_a_id='tag-a' AND tag_b_id='tag-b'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!((after_current_positive - 0.7).abs() < f64::EPSILON);

        insert_migrated_feedback_trial(
            &pool,
            "session-new-negative-same-book",
            "item-new-negative-same-book",
            "trial-new-negative-same-book",
            "archive-old",
            0.25,
        )
        .await;
        assert_eq!(
            update_user_factors_for_item(
                &pool,
                "item-new-negative-same-book",
                "user-1",
                GraphFeedbackOutcome::Positive,
            )
            .await
            .unwrap(),
            0
        );
        assert_eq!(
            update_user_factors_for_item(
                &pool,
                "item-new-negative-same-book",
                "user-1",
                GraphFeedbackOutcome::Negative,
            )
            .await
            .unwrap(),
            1
        );
        insert_migrated_feedback_trial(
            &pool,
            "session-new-positive-after-negative",
            "item-new-positive-after-negative",
            "trial-new-positive-after-negative",
            "archive-old",
            0.25,
        )
        .await;
        assert_eq!(
            update_user_factors_for_item(
                &pool,
                "item-new-positive-after-negative",
                "user-1",
                GraphFeedbackOutcome::Positive,
            )
            .await
            .unwrap(),
            0
        );
        let current_ledger: (Option<String>, Option<String>) = sqlx::query_as(
            "SELECT positive_feedback_at, negative_feedback_at
             FROM tag_relation_user_archive_feedback
             WHERE user_id='user-1' AND archive_id='archive-old'
               AND tag_a_id='tag-a' AND tag_b_id='tag-b'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(current_ledger.0.is_some());
        assert!(current_ledger.1.is_some());

        crate::database::run_sqlite_migrations(&pool).await.unwrap();
        let after_migration_rerun: f64 = sqlx::query_scalar(
            "SELECT influence FROM tag_relation_user_factors
             WHERE user_id='user-1' AND tag_a_id='tag-a' AND tag_b_id='tag-b'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!((after_migration_rerun - 0.525).abs() < f64::EPSILON);
    }
}
