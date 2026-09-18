//! Observing-only semantic relations between ordinary tags.
//!
//! These rows are deliberately separate from the deterministic co-occurrence graph. They are
//! suitable for diagnostics and later quality evaluation; this module does not alter archive tag
//! membership, aliases, preference feedback, or recommendation expansion.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};
use std::collections::BTreeMap;

pub const TAG_SEMANTIC_EDGE_ALGORITHM_VERSION: &str = "tag-semantic-edge-v1";

pub const CHOICE_LABELS: [&str; 5] = [
    "same_meaning",
    "related_nonreplaceable",
    "broader_or_narrower",
    "unrelated",
    "uncertain",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct TagRelationTag {
    pub id: String,
    pub namespace: String,
    pub name: String,
    #[serde(default)]
    pub support_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TagRelationPair {
    pub pair_id: String,
    pub tag_a: TagRelationTag,
    pub tag_b: TagRelationTag,
    pub pair_input_hash: String,
}

impl TagRelationPair {
    pub fn canonicalize(mut self) -> Result<Self> {
        self.tag_a.id = self.tag_a.id.trim().to_string();
        self.tag_b.id = self.tag_b.id.trim().to_string();
        self.tag_a.namespace = self.tag_a.namespace.trim().to_string();
        self.tag_b.namespace = self.tag_b.namespace.trim().to_string();
        self.tag_a.name = self.tag_a.name.trim().to_string();
        self.tag_b.name = self.tag_b.name.trim().to_string();
        if self.tag_a.id.is_empty()
            || self.tag_b.id.is_empty()
            || self.tag_a.namespace.is_empty()
            || self.tag_b.namespace.is_empty()
            || self.tag_a.name.is_empty()
            || self.tag_b.name.is_empty()
            || self.tag_a.id == self.tag_b.id
        {
            return Err(anyhow!("tag relation pair has invalid or identical tags"));
        }
        if self.tag_a.id > self.tag_b.id {
            std::mem::swap(&mut self.tag_a, &mut self.tag_b);
        }
        self.pair_id = canonical_pair_id(&self.tag_a.id, &self.tag_b.id);
        self.pair_input_hash = pair_input_hash(&self.tag_a, &self.tag_b);
        Ok(self)
    }
}

pub fn canonical_pair_id(tag_a_id: &str, tag_b_id: &str) -> String {
    let (left, right) = if tag_a_id <= tag_b_id {
        (tag_a_id, tag_b_id)
    } else {
        (tag_b_id, tag_a_id)
    };
    format!("{left}:{right}")
}

pub fn pair_input_hash(tag_a: &TagRelationTag, tag_b: &TagRelationTag) -> String {
    let mut items = vec![
        (
            tag_a.id.as_str(),
            tag_a.namespace.trim().to_string(),
            tag_a.name.trim().to_string(),
        ),
        (
            tag_b.id.as_str(),
            tag_b.namespace.trim().to_string(),
            tag_b.name.trim().to_string(),
        ),
    ];
    items.sort_by(|left, right| left.0.cmp(right.0));
    let mut hasher = Sha256::new();
    for (id, namespace, name) in items {
        hasher.update(id.as_bytes());
        hasher.update([0]);
        hasher.update(namespace.as_bytes());
        hasher.update([0]);
        hasher.update(name.as_bytes());
        hasher.update([0xff]);
    }
    format_hash(hasher.finalize())
}

fn format_hash(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChoiceAnswer {
    pub choice: String,
    pub probabilities: BTreeMap<String, f64>,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SemanticRelationDecision {
    pub relation_kind: String,
    pub status: String,
    pub selected_confidence: Option<f64>,
    pub reason: Option<String>,
}

pub fn parse_choice_answers(
    response: &Value,
    expected_pair_ids: &[String],
) -> Result<BTreeMap<String, ChoiceAnswer>> {
    let answers = response
        .get("answers")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("JEV response must contain an answers object"))?;
    if answers.len() != expected_pair_ids.len()
        || expected_pair_ids
            .iter()
            .any(|pair_id| !answers.contains_key(pair_id))
    {
        return Err(anyhow!(
            "JEV answer ids do not exactly match requested pairs"
        ));
    }

    let mut parsed = BTreeMap::new();
    for pair_id in expected_pair_ids {
        let answer = answers
            .get(pair_id)
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("JEV answer {pair_id} is not an object"))?;
        if answer.get("type").and_then(Value::as_str) != Some("choice") {
            return Err(anyhow!("JEV answer {pair_id} is not a typed Choice"));
        }
        let choice = answer
            .get("choice")
            .and_then(Value::as_str)
            .filter(|choice| CHOICE_LABELS.contains(choice))
            .ok_or_else(|| anyhow!("JEV answer {pair_id} has an invalid choice"))?
            .to_string();
        let probabilities = answer
            .get("probabilities")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("JEV answer {pair_id} has no probabilities"))?;
        if probabilities.is_empty()
            || probabilities
                .keys()
                .any(|label| !CHOICE_LABELS.contains(&label.as_str()))
            || !probabilities.contains_key(&choice)
        {
            return Err(anyhow!("JEV answer {pair_id} has incomplete probabilities"));
        }
        let mut normalized = BTreeMap::new();
        let mut sum = 0.0;
        for (label, value) in probabilities {
            let value = value
                .as_f64()
                .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
                .ok_or_else(|| anyhow!("JEV answer {pair_id} has an invalid probability"))?;
            sum += value;
            normalized.insert(label.clone(), value);
        }
        if (sum - 1.0).abs() > 0.02 {
            return Err(anyhow!(
                "JEV answer {pair_id} probabilities do not sum to one"
            ));
        }
        let confidence = answer
            .get("confidence")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
            .ok_or_else(|| anyhow!("JEV answer {pair_id} has invalid confidence"))?;
        parsed.insert(
            pair_id.clone(),
            ChoiceAnswer {
                choice,
                probabilities: normalized,
                confidence,
            },
        );
    }
    Ok(parsed)
}

pub fn combine_bidirectional_choices(
    forward: &ChoiceAnswer,
    reverse: &ChoiceAnswer,
    min_confidence: f64,
) -> SemanticRelationDecision {
    let confidence = forward.confidence.min(reverse.confidence);
    if forward.choice != reverse.choice {
        return SemanticRelationDecision {
            relation_kind: "uncertain".to_string(),
            status: "uncertain".to_string(),
            selected_confidence: Some(confidence),
            reason: Some("forward_reverse_disagreement".to_string()),
        };
    }
    if forward.choice == "unrelated" || forward.choice == "uncertain" || confidence < min_confidence
    {
        return SemanticRelationDecision {
            relation_kind: "uncertain".to_string(),
            status: "uncertain".to_string(),
            selected_confidence: Some(confidence),
            reason: Some(
                if confidence < min_confidence {
                    "low_confidence"
                } else {
                    "non_publishable_choice"
                }
                .to_string(),
            ),
        };
    }
    SemanticRelationDecision {
        relation_kind: forward.choice.clone(),
        status: "observing".to_string(),
        selected_confidence: Some(confidence),
        reason: None,
    }
}

#[derive(Debug, Clone)]
pub struct TagSemanticEdgeWrite {
    pub pair: TagRelationPair,
    pub decision: SemanticRelationDecision,
    pub forward: ChoiceAnswer,
    pub reverse: ChoiceAnswer,
    pub candidate_algorithm_version: String,
    pub protocol_version: String,
    pub prompt_version: String,
    pub schema_version: String,
    pub profile_id: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
    pub cost_usd: Option<f64>,
    pub latency_ms: Option<i64>,
    pub attempts: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagSemanticEdge {
    pub tag_a_id: String,
    pub tag_b_id: String,
    pub relation_kind: String,
    pub status: String,
    pub selected_confidence: Option<f64>,
    pub forward_choice: Option<String>,
    pub reverse_choice: Option<String>,
    pub forward_confidence: Option<f64>,
    pub reverse_confidence: Option<f64>,
    pub pair_input_hash: String,
    pub candidate_algorithm_version: String,
    pub protocol_version: String,
    pub prompt_version: String,
    pub schema_version: String,
    pub profile_id: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub updated_at: String,
}

pub async fn upsert_tag_semantic_edge(
    pool: &Pool<Sqlite>,
    edge: &TagSemanticEdgeWrite,
) -> Result<()> {
    let pair = edge.pair.clone().canonicalize()?;
    if pair.tag_a.id != edge.pair.tag_a.id || pair.tag_b.id != edge.pair.tag_b.id {
        return Err(anyhow!("semantic edge pair must already be canonical"));
    }
    let decision = &edge.decision;
    sqlx::query(
        "INSERT INTO tag_semantic_edges
         (tag_a_id, tag_b_id, relation_kind, status, forward_choice, reverse_choice,
          selected_confidence, forward_confidence, reverse_confidence,
          forward_probabilities_json, reverse_probabilities_json, pair_input_hash,
          candidate_algorithm_version, protocol_version, prompt_version, schema_version,
          profile_id, provider, model, input_tokens, output_tokens, total_tokens, cost_usd,
          latency_ms, attempts, last_error, next_attempt_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, CURRENT_TIMESTAMP)
         ON CONFLICT(tag_a_id, tag_b_id) DO UPDATE SET
          relation_kind = excluded.relation_kind, status = excluded.status,
          forward_choice = excluded.forward_choice, reverse_choice = excluded.reverse_choice,
          selected_confidence = excluded.selected_confidence,
          forward_confidence = excluded.forward_confidence,
          reverse_confidence = excluded.reverse_confidence,
          forward_probabilities_json = excluded.forward_probabilities_json,
          reverse_probabilities_json = excluded.reverse_probabilities_json,
          pair_input_hash = excluded.pair_input_hash,
          candidate_algorithm_version = excluded.candidate_algorithm_version,
          protocol_version = excluded.protocol_version, prompt_version = excluded.prompt_version,
          schema_version = excluded.schema_version, profile_id = excluded.profile_id,
          provider = excluded.provider, model = excluded.model,
          input_tokens = excluded.input_tokens, output_tokens = excluded.output_tokens,
          total_tokens = excluded.total_tokens, cost_usd = excluded.cost_usd,
          latency_ms = excluded.latency_ms, attempts = excluded.attempts,
          last_error = excluded.last_error, next_attempt_at = excluded.next_attempt_at,
          updated_at = CURRENT_TIMESTAMP",
    )
    .bind(&pair.tag_a.id)
    .bind(&pair.tag_b.id)
    .bind(&decision.relation_kind)
    .bind(&decision.status)
    .bind(&edge.forward.choice)
    .bind(&edge.reverse.choice)
    .bind(decision.selected_confidence)
    .bind(edge.forward.confidence)
    .bind(edge.reverse.confidence)
    .bind(serde_json::to_string(&edge.forward.probabilities)?)
    .bind(serde_json::to_string(&edge.reverse.probabilities)?)
    .bind(&pair.pair_input_hash)
    .bind(&edge.candidate_algorithm_version)
    .bind(&edge.protocol_version)
    .bind(&edge.prompt_version)
    .bind(&edge.schema_version)
    .bind(&edge.profile_id)
    .bind(&edge.provider)
    .bind(&edge.model)
    .bind(edge.input_tokens)
    .bind(edge.output_tokens)
    .bind(edge.total_tokens)
    .bind(edge.cost_usd)
    .bind(edge.latency_ms)
    .bind(edge.attempts)
    .bind(&edge.last_error)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_tag_semantic_edge_failed(
    pool: &Pool<Sqlite>,
    pair: &TagRelationPair,
    config: &crate::models::AITagRelationSettings,
    profile_id: Option<&str>,
    error: &str,
    _job_id: &str,
) -> Result<()> {
    let pair = pair.clone().canonicalize()?;
    sqlx::query(
        "INSERT INTO tag_semantic_edges
         (tag_a_id, tag_b_id, relation_kind, status, pair_input_hash,
          candidate_algorithm_version, protocol_version, prompt_version, schema_version,
          profile_id, attempts, last_error, updated_at)
         VALUES (?, ?, 'uncertain', 'failed', ?, ?, ?, ?, ?, ?, 1, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(tag_a_id, tag_b_id) DO UPDATE SET
          relation_kind = 'uncertain', status = 'failed', pair_input_hash = excluded.pair_input_hash,
          candidate_algorithm_version = excluded.candidate_algorithm_version,
          protocol_version = excluded.protocol_version, prompt_version = excluded.prompt_version,
          schema_version = excluded.schema_version, profile_id = excluded.profile_id,
          attempts = tag_semantic_edges.attempts + 1, last_error = excluded.last_error,
          updated_at = CURRENT_TIMESTAMP",
    )
    .bind(&pair.tag_a.id)
    .bind(&pair.tag_b.id)
    .bind(&pair.pair_input_hash)
    .bind(&config.candidate_algorithm_version)
    .bind(&config.protocol_version)
    .bind(&config.prompt_version)
    .bind(&config.schema_version)
    .bind(profile_id)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn load_tag_semantic_edges(
    pool: &Pool<Sqlite>,
    seed_tag_ids: &[String],
    limit: usize,
) -> Result<Vec<TagSemanticEdge>> {
    if seed_tag_ids.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let seeds = seed_tag_ids
        .iter()
        .filter(|id| !id.trim().is_empty())
        .collect::<Vec<_>>();
    if seeds.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = std::iter::repeat("?")
        .take(seeds.len())
        .collect::<Vec<_>>()
        .join(",");
    let query = format!(
        "SELECT tag_a_id, tag_b_id, relation_kind, status, selected_confidence,
                forward_choice, reverse_choice, forward_confidence, reverse_confidence,
                pair_input_hash, candidate_algorithm_version, protocol_version, prompt_version,
                schema_version, profile_id, provider, model, updated_at
         FROM tag_semantic_edges
         WHERE status = 'observing' AND (tag_a_id IN ({placeholders}) OR tag_b_id IN ({placeholders}))
         ORDER BY selected_confidence DESC, updated_at DESC, tag_a_id, tag_b_id LIMIT ?"
    );
    let mut request = sqlx::query(&query);
    for seed in &seeds {
        request = request.bind(*seed);
    }
    for seed in &seeds {
        request = request.bind(*seed);
    }
    request = request.bind(limit as i64);
    let mut edges = Vec::new();
    for row in request.fetch_all(pool).await? {
        edges.push(TagSemanticEdge {
            tag_a_id: row.try_get("tag_a_id")?,
            tag_b_id: row.try_get("tag_b_id")?,
            relation_kind: row.try_get("relation_kind")?,
            status: row.try_get("status")?,
            selected_confidence: row.try_get("selected_confidence")?,
            forward_choice: row.try_get("forward_choice")?,
            reverse_choice: row.try_get("reverse_choice")?,
            forward_confidence: row.try_get("forward_confidence")?,
            reverse_confidence: row.try_get("reverse_confidence")?,
            pair_input_hash: row.try_get("pair_input_hash")?,
            candidate_algorithm_version: row.try_get("candidate_algorithm_version")?,
            protocol_version: row.try_get("protocol_version")?,
            prompt_version: row.try_get("prompt_version")?,
            schema_version: row.try_get("schema_version")?,
            profile_id: row.try_get("profile_id")?,
            provider: row.try_get("provider")?,
            model: row.try_get("model")?,
            updated_at: row.try_get("updated_at")?,
        });
    }
    Ok(edges)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagGraphObservingSnapshot {
    /// Deterministic edges remain the only recommendation recall input.
    pub cooccurrence: Vec<super::tag_cooccurrence::TagCooccurrenceEdge>,
    /// JEV rows are returned for explanations/quality observation only.
    pub semantic: Vec<TagSemanticEdge>,
}

/// Loads both graph layers for diagnostics or an explanation view. Callers must continue to use
/// `cooccurrence` for archive recall; `semantic` is intentionally a side channel until a later
/// quality gate publishes it.
pub async fn load_tag_graph_observing_snapshot(
    pool: &Pool<Sqlite>,
    seed_tag_ids: &[String],
    cooccurrence_limit_per_seed: usize,
    semantic_limit: usize,
) -> Result<TagGraphObservingSnapshot> {
    let cooccurrence = super::tag_cooccurrence::load_tag_cooccurrence_neighbors(
        pool,
        seed_tag_ids,
        cooccurrence_limit_per_seed,
    )
    .await?;
    let semantic = load_tag_semantic_edges(pool, seed_tag_ids, semantic_limit).await?;
    Ok(TagGraphObservingSnapshot {
        cooccurrence,
        semantic,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair() -> TagRelationPair {
        TagRelationPair {
            pair_id: "wrong".to_string(),
            tag_a: TagRelationTag {
                id: "tag-b".to_string(),
                namespace: "general".to_string(),
                name: "Romantic".to_string(),
                support_count: 2,
            },
            tag_b: TagRelationTag {
                id: "tag-a".to_string(),
                namespace: "general".to_string(),
                name: "romance".to_string(),
                support_count: 3,
            },
            pair_input_hash: String::new(),
        }
    }

    fn answer(choice: &str, confidence: f64) -> ChoiceAnswer {
        ChoiceAnswer {
            choice: choice.to_string(),
            probabilities: [(choice.to_string(), 1.0)].into_iter().collect(),
            confidence,
        }
    }

    #[test]
    fn canonical_pair_hash_is_order_independent_and_excludes_archive_context() {
        let first = pair().canonicalize().unwrap();
        let second = TagRelationPair {
            tag_a: first.tag_b.clone(),
            tag_b: first.tag_a.clone(),
            ..first.clone()
        }
        .canonicalize()
        .unwrap();
        assert_eq!(first.pair_id, second.pair_id);
        assert_eq!(first.pair_input_hash, second.pair_input_hash);
        assert!(!first.pair_input_hash.contains("archive"));
    }

    #[test]
    fn pair_input_hash_ignores_support_count_changes() {
        let first = pair().canonicalize().unwrap();
        let mut changed = first.clone();
        changed.tag_a.support_count += 1;
        changed.tag_b.support_count += 7;
        let changed = changed.canonicalize().unwrap();

        assert_eq!(first.pair_input_hash, changed.pair_input_hash);
    }

    #[test]
    fn parses_typed_choice_and_rejects_extra_pair_ids() {
        let response = serde_json::json!({
            "answers": {
                "tag-a:tag-b": {
                    "type": "choice",
                    "choice": "same_meaning",
                    "probabilities": {"same_meaning": 0.9, "uncertain": 0.1},
                    "confidence": 0.9
                }
            }
        });
        let ids = vec!["tag-a:tag-b".to_string()];
        assert!(parse_choice_answers(&response, &ids).is_ok());
        let mut extra = response.clone();
        extra["answers"]["extra"] = serde_json::json!({
            "type": "choice", "choice": "uncertain",
            "probabilities": {"uncertain": 1.0}, "confidence": 1.0
        });
        assert!(parse_choice_answers(&extra, &ids).is_err());
    }

    #[test]
    fn disagreement_and_low_confidence_are_uncertain() {
        let disagreement = combine_bidirectional_choices(
            &answer("same_meaning", 0.95),
            &answer("related_nonreplaceable", 0.95),
            0.7,
        );
        assert_eq!(disagreement.status, "uncertain");
        assert_eq!(
            disagreement.reason.as_deref(),
            Some("forward_reverse_disagreement")
        );
        let low = combine_bidirectional_choices(
            &answer("same_meaning", 0.69),
            &answer("same_meaning", 0.95),
            0.7,
        );
        assert_eq!(low.status, "uncertain");
        assert_eq!(low.reason.as_deref(), Some("low_confidence"));
    }

    #[tokio::test]
    async fn upsert_keeps_semantic_edge_separate_and_updates_observing_state() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::database::run_sqlite_migrations(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES ('tag-a', 'romance', 'general'), ('tag-b', 'romantic', 'general')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let pair = pair().canonicalize().unwrap();
        let forward = answer("same_meaning", 0.95);
        let reverse = answer("same_meaning", 0.91);
        upsert_tag_semantic_edge(
            &pool,
            &TagSemanticEdgeWrite {
                pair,
                decision: combine_bidirectional_choices(&forward, &reverse, 0.7),
                forward,
                reverse,
                candidate_algorithm_version: "candidate-v1".to_string(),
                protocol_version: "protocol-v1".to_string(),
                prompt_version: "prompt-v1".to_string(),
                schema_version: "schema-v1".to_string(),
                profile_id: Some("profile".to_string()),
                provider: Some("TypeSafe".to_string()),
                model: Some("jev".to_string()),
                input_tokens: Some(1),
                output_tokens: Some(1),
                total_tokens: Some(2),
                cost_usd: Some(0.01),
                latency_ms: Some(10),
                attempts: 1,
                last_error: None,
            },
        )
        .await
        .unwrap();
        let row: (String, String, String) = sqlx::query_as(
            "SELECT relation_kind, status, forward_probabilities_json FROM tag_semantic_edges",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, "same_meaning");
        assert_eq!(row.1, "observing");
        assert!(row.2.contains("same_meaning"));
        let cooccurrence_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tag_cooccurrence_edges")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(cooccurrence_count, 0);
    }

    #[tokio::test]
    async fn graph_snapshot_exposes_semantic_edges_as_a_separate_side_channel() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::database::run_sqlite_migrations(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO archives (id, title, path, file_hash, file_size, page_count) VALUES
             ('archive-1', 'one', '/one.cbz', 'hash-1', 1, 1),
             ('archive-2', 'two', '/two.cbz', 'hash-2', 1, 1)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES
             ('tag-a', 'romance', 'general'), ('tag-b', 'romantic', 'general')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO archive_tags (archive_id, tag_id) VALUES
             ('archive-1', 'tag-a'), ('archive-1', 'tag-b'),
             ('archive-2', 'tag-a'), ('archive-2', 'tag-b')",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            super::super::tag_cooccurrence::rebuild_tag_cooccurrence_edges(&pool)
                .await
                .unwrap(),
            1
        );

        let pair = pair().canonicalize().unwrap();
        let forward = answer("related_nonreplaceable", 0.95);
        let reverse = answer("related_nonreplaceable", 0.92);
        upsert_tag_semantic_edge(
            &pool,
            &TagSemanticEdgeWrite {
                pair,
                decision: combine_bidirectional_choices(&forward, &reverse, 0.7),
                forward,
                reverse,
                candidate_algorithm_version: "candidate-v1".to_string(),
                protocol_version: "protocol-v1".to_string(),
                prompt_version: "prompt-v1".to_string(),
                schema_version: "schema-v1".to_string(),
                profile_id: Some("profile".to_string()),
                provider: Some("OpenRouter".to_string()),
                model: Some("jev".to_string()),
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                cost_usd: None,
                latency_ms: None,
                attempts: 1,
                last_error: None,
            },
        )
        .await
        .unwrap();

        let snapshot = load_tag_graph_observing_snapshot(&pool, &["tag-a".to_string()], 20, 20)
            .await
            .unwrap();
        assert_eq!(snapshot.cooccurrence.len(), 1);
        assert_eq!(snapshot.semantic.len(), 1);
        assert_eq!(snapshot.semantic[0].relation_kind, "related_nonreplaceable");
        assert_eq!(snapshot.semantic[0].status, "observing");
    }
}
