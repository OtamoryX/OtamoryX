//! Provider-neutral storage and bounded one-hop scoring for signed tag relations.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use uuid::Uuid;

pub const DEFAULT_WEIGHTED_GRAPH_GAIN: f64 = 0.3;
const MAX_SEED_TAGS: usize = 100;
pub const MAX_SOURCE_EVIDENCE: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeightedGraphPolicy {
    pub enabled: bool,
    pub global_gain: f64,
    pub version: i64,
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

/// Writes a scorer result to the numeric cache. New scores always start in observing state.
pub async fn upsert_observing_relation_weight(
    pool: &Pool<Sqlite>,
    edge: &TagRelationWeightWrite,
) -> Result<()> {
    let edge = edge.clone().canonicalize()?;
    sqlx::query(
        "INSERT INTO tag_relation_weight_edges
         (tag_a_id, tag_b_id, signed_weight, confidence, score_a_to_b, score_b_to_a,
          input_hash, scorer_version, profile_id, provider, model, status, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'observing', CURRENT_TIMESTAMP)
         ON CONFLICT(tag_a_id, tag_b_id) DO UPDATE SET
          signed_weight = excluded.signed_weight, confidence = excluded.confidence,
          score_a_to_b = excluded.score_a_to_b, score_b_to_a = excluded.score_b_to_a,
          input_hash = excluded.input_hash, scorer_version = excluded.scorer_version,
          profile_id = excluded.profile_id, provider = excluded.provider,
          model = excluded.model, status = 'observing',
          revision = tag_relation_weight_edges.revision + 1,
          updated_at = CURRENT_TIMESTAMP",
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
    .execute(pool)
    .await?;
    Ok(())
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

pub async fn review_observing_relation_weight(
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
    if edge.get::<String, _>("status") != "observing"
        || edge.get::<i64, _>("revision") != expected_revision
        || edge.get::<String, _>("input_hash") != expected_input_hash
        || edge.get::<String, _>("scorer_version") != expected_scorer_version
    {
        transaction.rollback().await?;
        return Ok(false);
    }
    let updated = sqlx::query(
        "UPDATE tag_relation_weight_edges SET status = ?, updated_at = CURRENT_TIMESTAMP
         WHERE tag_a_id = ? AND tag_b_id = ? AND status = 'observing'
           AND revision = ? AND input_hash = ? AND scorer_version = ?",
    )
    .bind(next_status)
    .bind(&left)
    .bind(&right)
    .bind(expected_revision)
    .bind(expected_input_hash)
    .bind(expected_scorer_version)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(updated.rows_affected() == 1)
}

pub async fn load_weighted_graph_policy(pool: &Pool<Sqlite>) -> Result<WeightedGraphPolicy> {
    let row = sqlx::query(
        "SELECT enabled, global_gain, version FROM tag_weighted_graph_policy WHERE id = 1",
    )
    .fetch_one(pool)
    .await?;
    let policy = WeightedGraphPolicy {
        enabled: row.get::<i64, _>("enabled") != 0,
        global_gain: row.get("global_gain"),
        version: row.get("version"),
    };
    validate_gain(policy.global_gain)?;
    Ok(policy)
}

/// Updates the centralized graph policy with optimistic concurrency. Callers must make graph
/// activation an explicit post-evaluation decision; this module does not infer readiness.
pub async fn update_weighted_graph_policy(
    pool: &Pool<Sqlite>,
    expected_version: i64,
    enabled: bool,
    global_gain: f64,
) -> Result<bool> {
    validate_gain(global_gain)?;
    let result = sqlx::query(
        "UPDATE tag_weighted_graph_policy
         SET enabled = ?, global_gain = ?, version = version + 1,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = 1 AND version = ?",
    )
    .bind(enabled as i64)
    .bind(global_gain)
    .bind(expected_version)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Loads explicitly activated numeric edges around the user's strongest positive tag seeds.
/// Observing scores and the graph-wide disabled default never reach recommendation scoring.
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
                    source.source_strength * edge.signed_weight * edge.user_influence;
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

/// Moves the personal factor toward agreement or away from contradiction by the magnitude of
/// the relation contribution that was actually recorded. Unobserved items never update it.
pub fn updated_user_influence(
    current: f64,
    signed_contribution: f64,
    outcome: GraphFeedbackOutcome,
) -> Result<f64> {
    validate_influence(current)?;
    validate_signed_weight(signed_contribution)?;
    if outcome == GraphFeedbackOutcome::Unobserved || signed_contribution == 0.0 {
        return Ok(current);
    }
    let outcome_sign = match outcome {
        GraphFeedbackOutcome::Positive => 1.0,
        GraphFeedbackOutcome::Negative => -1.0,
        GraphFeedbackOutcome::Unobserved => return Ok(current),
    };
    let amount = signed_contribution.abs();
    let next = if signed_contribution.signum() == outcome_sign {
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
        let next = updated_user_influence(current, row.get("signed_contribution"), outcome)?;
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
            "UPDATE random_recommendation_graph_trials
             SET {feedback_column} = CURRENT_TIMESTAMP WHERE id = ?"
        ))
        .bind(row.get::<String, _>("id"))
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
        .bind(archive_id)
        .bind(&tag_a_id)
        .bind(&tag_b_id)
        .execute(&mut *transaction)
        .await?;
        updated += 1;
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
    fn positive_and_negative_weights_share_one_scoring_equation() {
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
        assert_eq!(positive.contribution, 0.06);
        assert_eq!(negative.contribution, -0.06);
        for score in [&positive, &negative] {
            let attributed_total = score
                .edge_attributions
                .iter()
                .map(|attribution| attribution.signed_contribution)
                .sum::<f64>();
            assert!((attributed_total - score.contribution).abs() < f64::EPSILON);
        }
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
    fn direct_target_preference_allows_same_sign_and_clamps_opposite_sign() {
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
            Some(0.2),
        )
        .unwrap();
        assert_eq!(negative.contribution, -0.2);
        assert!((0.2 + negative.contribution).abs() < f64::EPSILON);
        assert_eq!(
            negative
                .edge_attributions
                .iter()
                .map(|attribution| attribution.signed_contribution)
                .sum::<f64>(),
            negative.contribution
        );

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
        assert!((-0.2 + positive_over_negative.contribution).abs() < f64::EPSILON);
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
    fn opposing_attributed_feedback_reduces_personal_influence_but_unobserved_does_not() {
        let after_unobserved =
            updated_user_influence(0.8, 0.3, GraphFeedbackOutcome::Unobserved).unwrap();
        let after_contradiction =
            updated_user_influence(0.8, -0.3, GraphFeedbackOutcome::Positive).unwrap();
        assert_eq!(after_unobserved, 0.8);
        assert!((after_contradiction - 0.56).abs() < f64::EPSILON);
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
        sqlx::query("UPDATE tag_weighted_graph_policy SET enabled = 1 WHERE id = 1")
            .execute(&pool)
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
        let write = TagRelationWeightWrite {
            tag_a_id: "tag-a".to_string(),
            tag_b_id: "tag-b".to_string(),
            signed_weight: 0.4,
            confidence: Some(0.8),
            score_a_to_b: Some(0.5),
            score_b_to_a: Some(0.3),
            input_hash: "pair-hash-v1".to_string(),
            scorer_version: "jev-score-v1".to_string(),
            profile_id: Some("profile-a".to_string()),
            provider: Some("jev".to_string()),
            model: Some("score-model".to_string()),
        };

        upsert_observing_relation_weight(&pool, &write)
            .await
            .unwrap();
        let first = list_scored_relation_weights(&pool, "observing", 10)
            .await
            .unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].revision, 1);
        assert_eq!(first[0].name_a, "woman");

        assert!(review_observing_relation_weight(
            &pool,
            "tag-a",
            "tag-b",
            1,
            "pair-hash-v1",
            "jev-score-v1",
            "active",
        )
        .await
        .unwrap());

        let rescored = TagRelationWeightWrite {
            model: Some("score-model-v2".to_string()),
            ..write.clone()
        };
        upsert_observing_relation_weight(&pool, &rescored)
            .await
            .unwrap();
        let second = list_scored_relation_weights(&pool, "observing", 10)
            .await
            .unwrap();
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].revision, 2);
        assert_eq!(second[0].model.as_deref(), Some("score-model-v2"));
        assert!(!review_observing_relation_weight(
            &pool,
            "tag-a",
            "tag-b",
            1,
            "pair-hash-v1",
            "jev-score-v1",
            "active",
        )
        .await
        .unwrap());
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT status FROM tag_relation_weight_edges
                 WHERE tag_a_id='tag-a' AND tag_b_id='tag-b'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "observing"
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
        upsert_observing_relation_weight(&pool, &write)
            .await
            .unwrap();

        let error = review_observing_relation_weight(
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

        assert!(update_weighted_graph_policy(&pool, 0, true, 0.4)
            .await
            .unwrap());
        assert!(!update_weighted_graph_policy(&pool, 0, false, 0.1)
            .await
            .unwrap());
        let updated = load_weighted_graph_policy(&pool).await.unwrap();
        assert!(updated.enabled);
        assert_eq!(updated.global_gain, 0.4);
        assert_eq!(updated.version, 1);
    }

    #[tokio::test]
    async fn archive_feedback_ledger_dedupes_across_sessions_and_preserves_outcome_order() {
        let pool = feedback_test_pool().await;
        insert_feedback_trial(
            &pool,
            "session-first",
            "item-first",
            "trial-first",
            "archive-one",
            -0.4,
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
        assert!((after_positive - 0.6).abs() < f64::EPSILON);

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
            -0.4,
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
        assert!((after_negative - 0.76).abs() < f64::EPSILON);

        insert_feedback_trial(
            &pool,
            "session-third",
            "item-third",
            "trial-third",
            "archive-one",
            -0.4,
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
}
