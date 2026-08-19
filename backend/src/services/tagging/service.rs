//! Persistent review and application workflow for AI-generated tag suggestions.
//!
//! This module deliberately does not call an LLM or claim jobs. A job handler builds a
//! [`CreateTaggingRun`] from its immutable input manifest, then stores the parsed candidates
//! here. Keeping those responsibilities separate makes suggestions reproducible and safe to
//! review after an archive has been re-analysed.

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Pool, Row, Sqlite};
use std::collections::BTreeMap;
use uuid::Uuid;

const DEFAULT_NAMESPACE: &str = "general";
const MAX_TAG_NAME_CHARS: usize = 255;
const MAX_NAMESPACE_CHARS: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaggingRun {
    pub archive_id: String,
    pub analysis_id: Option<String>,
    pub job_id: Option<String>,
    pub content_fingerprint: String,
    pub provider: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaggingRun {
    pub id: String,
    pub archive_id: String,
    pub analysis_id: Option<String>,
    pub job_id: Option<String>,
    pub content_fingerprint: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagSuggestionCandidate {
    pub name: String,
    #[serde(default = "default_namespace")]
    pub namespace: String,
    pub confidence: f64,
    /// Evidence must remain tied to the exact input pages/text used by the model.
    #[serde(default = "empty_array")]
    pub evidence: Value,
    /// Optional, source-specific detail beyond the run's analysis/job/fingerprint provenance.
    #[serde(default = "empty_object")]
    pub provenance: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AITagSuggestion {
    pub id: String,
    pub run_id: String,
    pub archive_id: String,
    pub name: String,
    pub namespace: String,
    pub confidence: f64,
    pub evidence: Value,
    pub provenance: Value,
    pub status: String,
    pub reviewed_at: Option<String>,
    pub reviewed_by: Option<String>,
    pub edited_tag_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingTagSuggestion {
    #[serde(flatten)]
    pub suggestion: AITagSuggestion,
    pub archive_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagSuggestionReviewAction {
    Approve,
    Reject,
    Edit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewTagSuggestion {
    pub action: TagSuggestionReviewAction,
    pub reviewed_by: Option<String>,
    /// Required for `edit`; ignored for `approve` and `reject`.
    pub edited_name: Option<String>,
    /// Falls back to the suggested namespace for `edit`.
    pub edited_namespace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedTag {
    pub application_id: String,
    pub tag_id: String,
    pub name: String,
    pub namespace: String,
    pub created_archive_tag: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewTagSuggestionResult {
    pub suggestion: AITagSuggestion,
    pub application: Option<AppliedTag>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoTaggingRunResult {
    pub run_id: String,
    pub applications_undone: usize,
    pub archive_tags_removed: usize,
    pub archive_tags_preserved: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoApplyTaggingRunResult {
    pub run_id: String,
    pub suggestions_applied: usize,
    pub archive_tags_created: usize,
    pub archive_tags_already_present: usize,
}

#[derive(Clone)]
pub struct TaggingService {
    pool: Pool<Sqlite>,
}

impl TaggingService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Starts an immutable provenance boundary for one tag-suggestion attempt.
    pub async fn create_run(&self, input: CreateTaggingRun) -> Result<TaggingRun> {
        validate_run(&input)?;
        let run = TaggingRun {
            id: Uuid::new_v4().to_string(),
            archive_id: input.archive_id,
            analysis_id: input.analysis_id,
            job_id: input.job_id,
            content_fingerprint: input.content_fingerprint,
            provider: clean_optional(input.provider),
            model: clean_optional(input.model),
            status: "running".to_string(),
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        };
        sqlx::query(
            "INSERT INTO ai_tagging_runs \
             (id, archive_id, analysis_id, job_id, content_fingerprint, provider, model, status, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&run.id)
        .bind(&run.archive_id)
        .bind(&run.analysis_id)
        .bind(&run.job_id)
        .bind(&run.content_fingerprint)
        .bind(&run.provider)
        .bind(&run.model)
        .bind(&run.status)
        .bind(&run.created_at)
        .execute(&self.pool)
        .await
        .context("failed to create AI tagging run")?;
        Ok(run)
    }

    /// Persists candidates idempotently. Duplicate model output is reduced to its highest
    /// confidence candidate, preserving the evidence/provenance attached to that candidate.
    pub async fn persist_suggestions(
        &self,
        run_id: &str,
        candidates: Vec<TagSuggestionCandidate>,
    ) -> Result<Vec<AITagSuggestion>> {
        let mut transaction = self.pool.begin().await?;
        let run = get_run_in_transaction(&mut transaction, run_id)
            .await?
            .ok_or_else(|| anyhow!("AI tagging run not found"))?;
        if run.status != "running" {
            return Err(anyhow!(
                "cannot persist suggestions for AI tagging run in {} state",
                run.status
            ));
        }

        let candidates = normalize_and_dedupe_candidates(candidates)?;
        let now = Utc::now().to_rfc3339();
        let mut saved = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let id = Uuid::new_v4().to_string();
            let provenance = json!({
                "analysisId": &run.analysis_id,
                "jobId": &run.job_id,
                "contentFingerprint": &run.content_fingerprint,
                "provider": &run.provider,
                "model": &run.model,
                "candidate": candidate.provenance,
            });
            sqlx::query(
                "INSERT INTO ai_tag_suggestions \
                 (id, run_id, archive_id, normalized_name, display_name, namespace, confidence, evidence_json, provenance_json, status, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?) \
                 ON CONFLICT(run_id, normalized_name, namespace) DO UPDATE SET \
                   display_name = excluded.display_name, \
                   confidence = excluded.confidence, \
                   evidence_json = excluded.evidence_json, \
                   provenance_json = excluded.provenance_json, \
                   updated_at = excluded.updated_at \
                 WHERE ai_tag_suggestions.status = 'pending'",
            )
            .bind(&id)
            .bind(&run.id)
            .bind(&run.archive_id)
            .bind(&candidate.normalized_name)
            .bind(&candidate.display_name)
            .bind(&candidate.namespace)
            .bind(candidate.confidence)
            .bind(serde_json::to_string(&candidate.evidence)?)
            .bind(serde_json::to_string(&provenance)?)
            .bind(&now)
            .bind(&now)
            .execute(&mut *transaction)
            .await
            .context("failed to store AI tag suggestion")?;

            let suggestion = get_suggestion_for_run_in_transaction(
                &mut transaction,
                &run.id,
                &candidate.normalized_name,
                &candidate.namespace,
            )
            .await?
            .ok_or_else(|| anyhow!("stored AI tag suggestion disappeared"))?;
            saved.push(suggestion);
        }
        sqlx::query(
            "UPDATE ai_tagging_runs SET status = 'completed', completed_at = ? WHERE id = ? AND status = 'running'",
        )
        .bind(&now)
        .bind(&run.id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(saved)
    }

    pub async fn fail_run(&self, run_id: &str) -> Result<()> {
        let updated = sqlx::query(
            "UPDATE ai_tagging_runs SET status = 'failed', completed_at = ? WHERE id = ? AND status = 'running'",
        )
        .bind(Utc::now().to_rfc3339())
        .bind(run_id)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() == 0 {
            return Err(anyhow!("AI tagging run is not running or does not exist"));
        }
        Ok(())
    }

    /// Applies still-pending suggestions at or above `threshold` with traceable evidence for a
    /// completed run. Namespaces are deliberately not special-cased: adult and sensitive tags
    /// follow the same evidence and reliability policy as every other tag.
    ///
    /// The status transition is the concurrency boundary: a concurrent reviewer that accepts or
    /// rejects a suggestion wins if it updates the pending row first. Every accepted suggestion
    /// receives a dedicated application audit row, including when the archive already had that
    /// tag, so [`Self::undo_run`] can distinguish pre-existing tags from tags created by AI.
    pub async fn auto_apply_reliable(
        &self,
        run_id: &str,
        threshold: f32,
    ) -> Result<AutoApplyTaggingRunResult> {
        let threshold = f64::from(threshold);
        if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
            return Err(anyhow!(
                "AI tag auto-apply threshold must be between 0 and 1"
            ));
        }

        let mut transaction = self.pool.begin().await?;
        let run = get_run_in_transaction(&mut transaction, run_id)
            .await?
            .ok_or_else(|| anyhow!("AI tagging run not found"))?;
        if run.status != "completed" {
            return Err(anyhow!(
                "cannot auto-apply suggestions for AI tagging run in {} state",
                run.status
            ));
        }

        let suggestions = sqlx::query(
            "SELECT id, archive_id, display_name, normalized_name, namespace, evidence_json \
             FROM ai_tag_suggestions \
             WHERE run_id = ? AND status = 'pending' AND confidence >= ? \
             ORDER BY created_at ASC, id ASC",
        )
        .bind(&run.id)
        .bind(threshold)
        .fetch_all(&mut *transaction)
        .await?;
        let now = Utc::now().to_rfc3339();
        let mut outcome = AutoApplyTaggingRunResult {
            run_id: run.id.clone(),
            ..Default::default()
        };

        for suggestion in suggestions {
            let suggestion_id: String = suggestion.get("id");
            let archive_id: String = suggestion.get("archive_id");
            let display_name: String = suggestion.get("display_name");
            let normalized_name: String = suggestion.get("normalized_name");
            let namespace: String = suggestion.get("namespace");
            let evidence: String = suggestion.get("evidence_json");
            let has_traceable_evidence = serde_json::from_str::<Value>(&evidence)
                .map(|value| evidence_supports_automatic_application(&value))
                .unwrap_or(false);
            if !has_traceable_evidence {
                continue;
            }

            // Claim the suggestion before creating a tag. This prevents an automatic
            // application from racing a human review and leaves every mutation rollback-safe.
            let claimed = sqlx::query(
                "UPDATE ai_tag_suggestions \
                 SET status = 'auto_applied', updated_at = ? \
                 WHERE id = ? AND status = 'pending' AND confidence >= ?",
            )
            .bind(&now)
            .bind(&suggestion_id)
            .bind(threshold)
            .execute(&mut *transaction)
            .await?;
            if claimed.rows_affected() != 1 {
                continue;
            }

            let tag = resolve_tag_in_transaction(
                &mut transaction,
                &display_name,
                &namespace,
                &normalized_name,
            )
            .await?;
            let mapping_exists: Option<i64> = sqlx::query_scalar(
                "SELECT 1 FROM archive_tags WHERE archive_id = ? AND tag_id = ? LIMIT 1",
            )
            .bind(&archive_id)
            .bind(&tag.id)
            .fetch_optional(&mut *transaction)
            .await?;
            let created_archive_tag = mapping_exists.is_none();
            if created_archive_tag {
                sqlx::query("INSERT INTO archive_tags (archive_id, tag_id) VALUES (?, ?)")
                    .bind(&archive_id)
                    .bind(&tag.id)
                    .execute(&mut *transaction)
                    .await?;
                outcome.archive_tags_created += 1;
            } else {
                outcome.archive_tags_already_present += 1;
            }

            sqlx::query(
                "INSERT INTO ai_tag_applications \
                 (id, run_id, suggestion_id, archive_id, tag_id, application_source, applied_at, created_archive_tag) \
                 VALUES (?, ?, ?, ?, ?, 'automatic', ?, ?)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&run.id)
            .bind(&suggestion_id)
            .bind(&archive_id)
            .bind(&tag.id)
            .bind(&now)
            .bind(created_archive_tag)
            .execute(&mut *transaction)
            .await?;
            sqlx::query(
                "UPDATE ai_tag_suggestions SET edited_tag_id = ?, updated_at = ? WHERE id = ?",
            )
            .bind(&tag.id)
            .bind(&now)
            .bind(&suggestion_id)
            .execute(&mut *transaction)
            .await?;
            outcome.suggestions_applied += 1;
        }
        transaction.commit().await?;
        Ok(outcome)
    }

    pub async fn list_pending(
        &self,
        archive_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<PendingTagSuggestion>> {
        let limit = i64::from(limit.clamp(1, 200));
        let rows = if let Some(archive_id) = archive_id {
            sqlx::query(
                "SELECT s.id, s.run_id, s.archive_id, s.display_name, s.namespace, s.confidence, \
                        s.evidence_json, s.provenance_json, s.status, s.reviewed_at, s.reviewed_by, \
                        s.edited_tag_id, s.created_at, s.updated_at, a.title AS archive_title \
                 FROM ai_tag_suggestions s JOIN archives a ON a.id = s.archive_id \
                 WHERE s.status = 'pending' AND s.archive_id = ? \
                 ORDER BY s.created_at ASC LIMIT ?",
            )
            .bind(archive_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query(
                "SELECT s.id, s.run_id, s.archive_id, s.display_name, s.namespace, s.confidence, \
                        s.evidence_json, s.provenance_json, s.status, s.reviewed_at, s.reviewed_by, \
                        s.edited_tag_id, s.created_at, s.updated_at, a.title AS archive_title \
                 FROM ai_tag_suggestions s JOIN archives a ON a.id = s.archive_id \
                 WHERE s.status = 'pending' \
                 ORDER BY s.created_at ASC LIMIT ?",
            )
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };
        rows.into_iter()
            .map(|row| {
                Ok(PendingTagSuggestion {
                    suggestion: suggestion_from_row(&row)?,
                    archive_title: row.get("archive_title"),
                })
            })
            .collect()
    }

    /// Records one review and, for approvals, creates an auditable archive-tag application.
    /// Calling this twice for the same suggestion fails rather than silently overwriting review
    /// history.
    pub async fn review_suggestion(
        &self,
        suggestion_id: &str,
        input: ReviewTagSuggestion,
    ) -> Result<ReviewTagSuggestionResult> {
        let mut transaction = self.pool.begin().await?;
        let suggestion = get_suggestion_in_transaction(&mut transaction, suggestion_id)
            .await?
            .ok_or_else(|| anyhow!("AI tag suggestion not found"))?;
        if suggestion.status != "pending" {
            return Err(anyhow!("AI tag suggestion has already been reviewed"));
        }
        let now = Utc::now().to_rfc3339();
        let reviewed_by = clean_optional(input.reviewed_by);

        if matches!(&input.action, TagSuggestionReviewAction::Reject) {
            sqlx::query(
                "UPDATE ai_tag_suggestions \
                 SET status = 'rejected', reviewed_at = ?, reviewed_by = ?, updated_at = ? \
                 WHERE id = ? AND status = 'pending'",
            )
            .bind(&now)
            .bind(&reviewed_by)
            .bind(&now)
            .bind(suggestion_id)
            .execute(&mut *transaction)
            .await?;
            let suggestion = get_suggestion_in_transaction(&mut transaction, suggestion_id)
                .await?
                .ok_or_else(|| anyhow!("reviewed AI tag suggestion disappeared"))?;
            transaction.commit().await?;
            return Ok(ReviewTagSuggestionResult {
                suggestion,
                application: None,
            });
        }

        let (name, namespace) = match input.action {
            TagSuggestionReviewAction::Approve => {
                (suggestion.name.clone(), suggestion.namespace.clone())
            }
            TagSuggestionReviewAction::Edit => (
                input
                    .edited_name
                    .ok_or_else(|| anyhow!("an edited tag name is required"))?,
                input
                    .edited_namespace
                    .unwrap_or(suggestion.namespace.clone()),
            ),
            TagSuggestionReviewAction::Reject => unreachable!(),
        };
        let normalized = normalize_tag_identity(&name, &namespace)?;
        let tag = resolve_tag_in_transaction(
            &mut transaction,
            &normalized.display_name,
            &normalized.namespace,
            &normalized.normalized_name,
        )
        .await?;
        let mapping_exists: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM archive_tags WHERE archive_id = ? AND tag_id = ? LIMIT 1",
        )
        .bind(&suggestion.archive_id)
        .bind(&tag.id)
        .fetch_optional(&mut *transaction)
        .await?;
        let created_archive_tag = mapping_exists.is_none();
        if created_archive_tag {
            sqlx::query("INSERT INTO archive_tags (archive_id, tag_id) VALUES (?, ?)")
                .bind(&suggestion.archive_id)
                .bind(&tag.id)
                .execute(&mut *transaction)
                .await?;
        }

        let application_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO ai_tag_applications \
             (id, run_id, suggestion_id, archive_id, tag_id, application_source, applied_by, applied_at, created_archive_tag) \
             VALUES (?, ?, ?, ?, ?, 'review', ?, ?, ?)",
        )
        .bind(&application_id)
        .bind(&suggestion.run_id)
        .bind(&suggestion.id)
        .bind(&suggestion.archive_id)
        .bind(&tag.id)
        .bind(&reviewed_by)
        .bind(&now)
        .bind(created_archive_tag)
        .execute(&mut *transaction)
        .await?;
        let updated = sqlx::query(
            "UPDATE ai_tag_suggestions \
             SET status = 'approved', reviewed_at = ?, reviewed_by = ?, edited_tag_id = ?, updated_at = ? \
             WHERE id = ? AND status = 'pending'",
        )
        .bind(&now)
        .bind(&reviewed_by)
        .bind(&tag.id)
        .bind(&now)
        .bind(suggestion_id)
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(anyhow!("AI tag suggestion changed while being reviewed"));
        }
        let suggestion = get_suggestion_in_transaction(&mut transaction, suggestion_id)
            .await?
            .ok_or_else(|| anyhow!("reviewed AI tag suggestion disappeared"))?;
        transaction.commit().await?;
        Ok(ReviewTagSuggestionResult {
            suggestion,
            application: Some(AppliedTag {
                application_id,
                tag_id: tag.id,
                name: tag.name,
                namespace: tag.namespace,
                created_archive_tag,
            }),
        })
    }

    /// Undoes the applications belonging to a run. Pre-existing archive tags are never removed.
    /// Tags inserted by this run are removed only when no other active AI application retains
    /// them. A manual provenance layer can make this policy stricter without changing this API.
    pub async fn undo_run(
        &self,
        run_id: &str,
        undone_by: Option<String>,
    ) -> Result<UndoTaggingRunResult> {
        let mut transaction = self.pool.begin().await?;
        let run = get_run_in_transaction(&mut transaction, run_id)
            .await?
            .ok_or_else(|| anyhow!("AI tagging run not found"))?;
        let applications = sqlx::query(
            "SELECT id, suggestion_id, archive_id, tag_id, created_archive_tag \
             FROM ai_tag_applications WHERE run_id = ? AND undone_at IS NULL",
        )
        .bind(run_id)
        .fetch_all(&mut *transaction)
        .await?;
        let now = Utc::now().to_rfc3339();
        let undone_by = clean_optional(undone_by);
        let mut outcome = UndoTaggingRunResult {
            run_id: run.id,
            ..Default::default()
        };
        for application in applications {
            let application_id: String = application.get("id");
            let suggestion_id: String = application.get("suggestion_id");
            let archive_id: String = application.get("archive_id");
            let tag_id: String = application.get("tag_id");
            let created_archive_tag = application.get::<bool, _>("created_archive_tag");
            let marked = sqlx::query(
                "UPDATE ai_tag_applications SET undone_at = ?, undone_by = ? WHERE id = ? AND undone_at IS NULL",
            )
            .bind(&now)
            .bind(&undone_by)
            .bind(&application_id)
            .execute(&mut *transaction)
            .await?;
            if marked.rows_affected() != 1 {
                continue;
            }
            outcome.applications_undone += 1;
            sqlx::query(
                "UPDATE ai_tag_suggestions \
                 SET status = 'undone', updated_at = ? \
                 WHERE id = ? AND status IN ('approved', 'auto_applied')",
            )
            .bind(&now)
            .bind(&suggestion_id)
            .execute(&mut *transaction)
            .await?;
            if !created_archive_tag {
                outcome.archive_tags_preserved += 1;
                continue;
            }
            let retained_elsewhere: Option<i64> = sqlx::query_scalar(
                "SELECT 1 FROM ai_tag_applications \
                 WHERE archive_id = ? AND tag_id = ? AND undone_at IS NULL LIMIT 1",
            )
            .bind(&archive_id)
            .bind(&tag_id)
            .fetch_optional(&mut *transaction)
            .await?;
            if retained_elsewhere.is_some() {
                // Preserve exactly one owner for an AI-created mapping. Without this handoff,
                // undoing the original run before a later run would leave the tag association
                // behind permanently after the later run is undone.
                if created_archive_tag {
                    sqlx::query(
                        "UPDATE ai_tag_applications SET created_archive_tag = 1 \
                         WHERE id = ( \
                           SELECT id FROM ai_tag_applications \
                           WHERE archive_id = ? AND tag_id = ? AND undone_at IS NULL \
                           ORDER BY applied_at ASC, id ASC LIMIT 1 \
                         )",
                    )
                    .bind(&archive_id)
                    .bind(&tag_id)
                    .execute(&mut *transaction)
                    .await?;
                }
                outcome.archive_tags_preserved += 1;
                continue;
            }
            let deleted =
                sqlx::query("DELETE FROM archive_tags WHERE archive_id = ? AND tag_id = ?")
                    .bind(&archive_id)
                    .bind(&tag_id)
                    .execute(&mut *transaction)
                    .await?;
            if deleted.rows_affected() == 1 {
                outcome.archive_tags_removed += 1;
            } else {
                outcome.archive_tags_preserved += 1;
            }
        }
        sqlx::query(
            "UPDATE ai_tagging_runs SET status = 'undone' WHERE id = ? AND status IN ('completed', 'partial')",
        )
        .bind(run_id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(outcome)
    }
}

#[derive(Debug)]
struct NormalizedTagIdentity {
    display_name: String,
    normalized_name: String,
    namespace: String,
}

#[derive(Debug)]
struct NormalizedCandidate {
    display_name: String,
    normalized_name: String,
    namespace: String,
    confidence: f64,
    evidence: Value,
    provenance: Value,
}

#[derive(Debug)]
struct ResolvedTag {
    id: String,
    name: String,
    namespace: String,
}

fn default_namespace() -> String {
    DEFAULT_NAMESPACE.to_string()
}

fn empty_array() -> Value {
    Value::Array(Vec::new())
}

fn empty_object() -> Value {
    Value::Object(Default::default())
}

fn validate_run(input: &CreateTaggingRun) -> Result<()> {
    if input.archive_id.trim().is_empty() {
        return Err(anyhow!("AI tagging run requires an archive id"));
    }
    if input.content_fingerprint.trim().is_empty() {
        return Err(anyhow!("AI tagging run requires a content fingerprint"));
    }
    Ok(())
}

fn normalize_and_dedupe_candidates(
    candidates: Vec<TagSuggestionCandidate>,
) -> Result<Vec<NormalizedCandidate>> {
    let mut deduped = BTreeMap::<(String, String), NormalizedCandidate>::new();
    for candidate in candidates {
        let identity = normalize_tag_identity(&candidate.name, &candidate.namespace)?;
        if !candidate.confidence.is_finite() || !(0.0..=1.0).contains(&candidate.confidence) {
            return Err(anyhow!(
                "AI tag suggestion confidence must be between 0 and 1"
            ));
        }
        let candidate = NormalizedCandidate {
            display_name: identity.display_name,
            normalized_name: identity.normalized_name,
            namespace: identity.namespace,
            confidence: candidate.confidence,
            evidence: candidate.evidence,
            provenance: candidate.provenance,
        };
        let key = (
            candidate.namespace.to_lowercase(),
            candidate.normalized_name.clone(),
        );
        match deduped.get(&key) {
            Some(existing) if existing.confidence >= candidate.confidence => {}
            _ => {
                deduped.insert(key, candidate);
            }
        }
    }
    Ok(deduped.into_values().collect())
}

fn normalize_tag_identity(name: &str, namespace: &str) -> Result<NormalizedTagIdentity> {
    let display_name = collapse_whitespace(name);
    let namespace = collapse_whitespace(namespace);
    if display_name.is_empty() || display_name.chars().count() > MAX_TAG_NAME_CHARS {
        return Err(anyhow!("AI tag suggestion has an invalid name"));
    }
    if namespace.is_empty() || namespace.chars().count() > MAX_NAMESPACE_CHARS {
        return Err(anyhow!("AI tag suggestion has an invalid namespace"));
    }
    Ok(NormalizedTagIdentity {
        normalized_name: display_name.to_lowercase(),
        display_name,
        namespace: namespace.to_lowercase(),
    })
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn evidence_supports_automatic_application(evidence: &Value) -> bool {
    match evidence {
        Value::String(value) => !value.trim().is_empty(),
        Value::Array(items) => items.iter().any(evidence_item_is_traceable),
        Value::Object(_) => evidence_item_is_traceable(evidence),
        _ => false,
    }
}

fn evidence_item_is_traceable(item: &Value) -> bool {
    match item {
        Value::String(value) => !value.trim().is_empty(),
        Value::Object(fields) => {
            let detail = ["reason", "excerpt", "text", "summary"]
                .iter()
                .filter_map(|key| fields.get(*key).and_then(Value::as_str))
                .any(|value| !value.trim().is_empty());
            let page = fields
                .get("page")
                .and_then(Value::as_i64)
                .is_some_and(|page| page > 0);
            let source = fields
                .get("source")
                .and_then(Value::as_str)
                .is_some_and(|source| !source.trim().is_empty());
            detail && (page || source)
        }
        _ => false,
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = collapse_whitespace(&value);
        (!value.is_empty()).then_some(value)
    })
}

async fn get_run_in_transaction(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    run_id: &str,
) -> Result<Option<TaggingRun>> {
    let row = sqlx::query(
        "SELECT id, archive_id, analysis_id, job_id, content_fingerprint, provider, model, status, created_at, completed_at \
         FROM ai_tagging_runs WHERE id = ?",
    )
    .bind(run_id)
    .fetch_optional(&mut **transaction)
    .await?;
    row.map(tagging_run_from_row).transpose()
}

async fn get_suggestion_in_transaction(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    suggestion_id: &str,
) -> Result<Option<AITagSuggestion>> {
    let row = sqlx::query(
        "SELECT id, run_id, archive_id, display_name, namespace, confidence, evidence_json, provenance_json, \
                status, reviewed_at, reviewed_by, edited_tag_id, created_at, updated_at \
         FROM ai_tag_suggestions WHERE id = ?",
    )
    .bind(suggestion_id)
    .fetch_optional(&mut **transaction)
    .await?;
    row.map(|row| suggestion_from_row(&row)).transpose()
}

async fn get_suggestion_for_run_in_transaction(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    run_id: &str,
    normalized_name: &str,
    namespace: &str,
) -> Result<Option<AITagSuggestion>> {
    let row = sqlx::query(
        "SELECT id, run_id, archive_id, display_name, namespace, confidence, evidence_json, provenance_json, \
                status, reviewed_at, reviewed_by, edited_tag_id, created_at, updated_at \
         FROM ai_tag_suggestions WHERE run_id = ? AND normalized_name = ? AND namespace = ?",
    )
    .bind(run_id)
    .bind(normalized_name)
    .bind(namespace)
    .fetch_optional(&mut **transaction)
    .await?;
    row.map(|row| suggestion_from_row(&row)).transpose()
}

async fn resolve_tag_in_transaction(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    display_name: &str,
    namespace: &str,
    normalized_name: &str,
) -> Result<ResolvedTag> {
    let existing = sqlx::query(
        "SELECT id, name, namespace FROM tags \
         WHERE lower(name) = ? AND lower(namespace) = ? LIMIT 1",
    )
    .bind(normalized_name)
    .bind(namespace.to_lowercase())
    .fetch_optional(&mut **transaction)
    .await?;
    if let Some(row) = existing {
        return Ok(ResolvedTag {
            id: row.get("id"),
            name: row.get("name"),
            namespace: row.get("namespace"),
        });
    }
    let tag_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO tags (id, name, namespace) VALUES (?, ?, ?)")
        .bind(&tag_id)
        .bind(display_name)
        .bind(namespace)
        .execute(&mut **transaction)
        .await?;
    Ok(ResolvedTag {
        id: tag_id,
        name: display_name.to_string(),
        namespace: namespace.to_string(),
    })
}

fn tagging_run_from_row(row: sqlx::sqlite::SqliteRow) -> Result<TaggingRun> {
    Ok(TaggingRun {
        id: row.get("id"),
        archive_id: row.get("archive_id"),
        analysis_id: row.get("analysis_id"),
        job_id: row.get("job_id"),
        content_fingerprint: row.get("content_fingerprint"),
        provider: row.get("provider"),
        model: row.get("model"),
        status: row.get("status"),
        created_at: row.get("created_at"),
        completed_at: row.get("completed_at"),
    })
}

fn suggestion_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<AITagSuggestion> {
    let evidence: String = row.get("evidence_json");
    let provenance: String = row.get("provenance_json");
    Ok(AITagSuggestion {
        id: row.get("id"),
        run_id: row.get("run_id"),
        archive_id: row.get("archive_id"),
        name: row.get("display_name"),
        namespace: row.get("namespace"),
        confidence: row.get("confidence"),
        evidence: serde_json::from_str(&evidence).context("invalid AI tag evidence JSON")?,
        provenance: serde_json::from_str(&provenance).context("invalid AI tag provenance JSON")?,
        status: row.get("status"),
        reviewed_at: row.get("reviewed_at"),
        reviewed_by: row.get("reviewed_by"),
        edited_tag_id: row.get("edited_tag_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for statement in [
            "CREATE TABLE archives (id TEXT PRIMARY KEY)",
            "CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL)",
            "CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL, PRIMARY KEY (archive_id, tag_id))",
            "CREATE TABLE ai_tagging_runs (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, analysis_id TEXT, job_id TEXT, content_fingerprint TEXT NOT NULL, provider TEXT, model TEXT, status TEXT NOT NULL, created_at TEXT NOT NULL, completed_at TEXT)",
            "CREATE TABLE ai_tag_suggestions (id TEXT PRIMARY KEY, run_id TEXT NOT NULL, archive_id TEXT NOT NULL, normalized_name TEXT NOT NULL, display_name TEXT NOT NULL, namespace TEXT NOT NULL, confidence REAL NOT NULL, evidence_json TEXT NOT NULL, provenance_json TEXT NOT NULL, status TEXT NOT NULL, reviewed_at TEXT, reviewed_by TEXT, edited_tag_id TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, UNIQUE (run_id, normalized_name, namespace))",
            "CREATE TABLE ai_tag_applications (id TEXT PRIMARY KEY, run_id TEXT NOT NULL, suggestion_id TEXT NOT NULL UNIQUE, archive_id TEXT NOT NULL, tag_id TEXT NOT NULL, application_source TEXT NOT NULL, applied_by TEXT, applied_at TEXT NOT NULL, created_archive_tag INTEGER NOT NULL, undone_at TEXT, undone_by TEXT)",
        ] {
            sqlx::query(statement).execute(&pool).await.unwrap();
        }
        sqlx::query("INSERT INTO archives (id) VALUES ('archive-1')")
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    fn candidate(name: &str, confidence: f64) -> TagSuggestionCandidate {
        TagSuggestionCandidate {
            name: name.to_string(),
            namespace: "general".to_string(),
            confidence,
            evidence: json!([{"source": "metadata", "excerpt": "verified metadata"}]),
            provenance: json!({}),
        }
    }

    async fn suggestion_status(pool: &Pool<Sqlite>, run_id: &str, name: &str) -> String {
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM ai_tag_suggestions WHERE run_id = ? AND display_name = ?",
        )
        .bind(run_id)
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn auto_apply_only_applies_reliable_suggestions_and_undo_preserves_existing_tags() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES ('existing', 'existing', 'general')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO archive_tags (archive_id, tag_id) VALUES ('archive-1', 'existing')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let service = TaggingService::new(pool.clone());
        let run = service
            .create_run(CreateTaggingRun {
                archive_id: "archive-1".to_string(),
                analysis_id: None,
                job_id: None,
                content_fingerprint: "fingerprint".to_string(),
                provider: None,
                model: None,
            })
            .await
            .unwrap();
        service
            .persist_suggestions(
                &run.id,
                vec![
                    candidate("reliable", 0.95),
                    candidate("needs review", 0.79),
                    candidate("existing", 0.99),
                ],
            )
            .await
            .unwrap();

        let applied = service.auto_apply_reliable(&run.id, 0.8).await.unwrap();
        assert_eq!(applied.suggestions_applied, 2);
        assert_eq!(applied.archive_tags_created, 1);
        assert_eq!(applied.archive_tags_already_present, 1);

        assert_eq!(
            suggestion_status(&pool, &run.id, "reliable").await,
            "auto_applied"
        );
        assert_eq!(
            suggestion_status(&pool, &run.id, "existing").await,
            "auto_applied"
        );
        assert_eq!(
            suggestion_status(&pool, &run.id, "needs review").await,
            "pending"
        );
        let automatic_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ai_tag_applications WHERE run_id = ? AND application_source = 'automatic'",
        )
        .bind(&run.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(automatic_count, 2);

        // A later AI run may independently endorse the same tag. The original run's undo must
        // keep the tag while that application remains, then hand over removal responsibility.
        let second_run = service
            .create_run(CreateTaggingRun {
                archive_id: "archive-1".to_string(),
                analysis_id: None,
                job_id: None,
                content_fingerprint: "fingerprint".to_string(),
                provider: None,
                model: None,
            })
            .await
            .unwrap();
        service
            .persist_suggestions(&second_run.id, vec![candidate("reliable", 0.98)])
            .await
            .unwrap();
        let second_applied = service
            .auto_apply_reliable(&second_run.id, 0.8)
            .await
            .unwrap();
        assert_eq!(second_applied.archive_tags_created, 0);
        assert_eq!(second_applied.archive_tags_already_present, 1);

        let undone = service.undo_run(&run.id, None).await.unwrap();
        assert_eq!(undone.applications_undone, 2);
        assert_eq!(undone.archive_tags_removed, 0);
        assert_eq!(undone.archive_tags_preserved, 2);
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT status FROM ai_tagging_runs WHERE id = ?",)
                .bind(&run.id)
                .fetch_one(&pool)
                .await
                .unwrap(),
            "undone"
        );
        assert_eq!(
            suggestion_status(&pool, &run.id, "reliable").await,
            "undone"
        );
        assert_eq!(
            suggestion_status(&pool, &run.id, "existing").await,
            "undone"
        );
        let existing_mapping: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM archive_tags WHERE archive_id = 'archive-1' AND tag_id = 'existing'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert_eq!(existing_mapping, Some(1));
        let reliable_mapping_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM archive_tags at JOIN tags t ON t.id = at.tag_id \
             WHERE at.archive_id = 'archive-1' AND t.name = 'reliable'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(reliable_mapping_count, 1);

        let second_undone = service.undo_run(&second_run.id, None).await.unwrap();
        assert_eq!(second_undone.applications_undone, 1);
        assert_eq!(second_undone.archive_tags_removed, 1);
        assert_eq!(
            suggestion_status(&pool, &second_run.id, "reliable").await,
            "undone"
        );
        let reliable_mapping_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM archive_tags at JOIN tags t ON t.id = at.tag_id \
             WHERE at.archive_id = 'archive-1' AND t.name = 'reliable'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(reliable_mapping_count, 0);
    }

    #[tokio::test]
    async fn auto_apply_requires_traceable_evidence() {
        let pool = test_pool().await;
        let service = TaggingService::new(pool.clone());
        let run = service
            .create_run(CreateTaggingRun {
                archive_id: "archive-1".to_string(),
                analysis_id: None,
                job_id: None,
                content_fingerprint: "fingerprint".to_string(),
                provider: None,
                model: None,
            })
            .await
            .unwrap();
        let mut unsupported = candidate("unsupported", 0.99);
        unsupported.evidence = json!([]);
        service
            .persist_suggestions(&run.id, vec![candidate("supported", 0.85), unsupported])
            .await
            .unwrap();

        let applied = service.auto_apply_reliable(&run.id, 0.8).await.unwrap();
        assert_eq!(applied.suggestions_applied, 1);
        assert_eq!(
            suggestion_status(&pool, &run.id, "supported").await,
            "auto_applied"
        );
        assert_eq!(
            suggestion_status(&pool, &run.id, "unsupported").await,
            "pending"
        );
    }
}
