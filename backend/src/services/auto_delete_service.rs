use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde_json::json;
use sqlx::{Pool, Sqlite};

use crate::{
    models::{AutoDeleteDecision, RecordBehaviorEventRequest},
    services::{CurationService, TrashService},
};

/// The rule engine must provide a high-confidence decision before this
/// internal boundary can perform an automatic deletion.
pub const AUTO_DELETE_CONFIDENCE_THRESHOLD: f64 = 0.85;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoDeleteResult {
    Applied,
    AlreadyCompleted,
}

pub struct AutoDeleteService {
    pool: Pool<Sqlite>,
}

impl AutoDeleteService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn execute(&self, decision: AutoDeleteDecision) -> Result<AutoDeleteResult> {
        validate_decision(&decision)?;

        if self
            .decision_already_completed(&decision.user_id, &decision.decision_key)
            .await?
        {
            return Ok(AutoDeleteResult::AlreadyCompleted);
        }

        let trash = TrashService::new(self.pool.clone());
        let entry = match trash
            .move_archive_to_trash_with_decision(
                &decision.user_id,
                &decision.archive_id,
                Some(&decision.reason),
                "auto_delete",
                Some(&decision.rule_version),
                Some(decision.model_confidence),
                &decision.evidence_pages,
                Some(&decision.decision_key),
            )
            .await
        {
            Ok(entry) => entry,
            Err(error) => {
                // A concurrent manual delete (or a first delivery that already
                // committed) is an idempotent completion, not a failed retry.
                if self
                    .archive_or_decision_already_in_trash(
                        &decision.user_id,
                        &decision.archive_id,
                        &decision.decision_key,
                    )
                    .await?
                {
                    return Ok(AutoDeleteResult::AlreadyCompleted);
                }
                return Err(error);
            }
        };

        // `TrashService` can return an already-active manual entry when it
        // wins the archive race. Do not mislabel that entry as an automatic
        // success or emit a false automatic disposition.
        if !self
            .entry_has_decision_key(&entry.id, &decision.decision_key)
            .await?
        {
            return Ok(AutoDeleteResult::AlreadyCompleted);
        }

        let metadata = json!({
            "source": "auto_delete",
            "reason": decision.reason,
            "rule_version": decision.rule_version,
            "model_confidence": decision.model_confidence,
            "evidence_pages": decision.evidence_pages,
            "decision_key": decision.decision_key,
            "trash_entry_id": entry.id,
        });
        let behavior = RecordBehaviorEventRequest {
            archive_id: Some(decision.archive_id.clone()),
            event_type: "auto_delete".to_string(),
            event_key: Some(format!("auto-delete:{}", decision.decision_key)),
            page: None,
            metadata: metadata.clone(),
            occurred_at: Some(Utc::now()),
        };
        if let Err(error) = CurationService::new(self.pool.clone())
            .record_event(&decision.user_id, &behavior)
            .await
        {
            tracing::warn!(
                archive_id = %decision.archive_id,
                decision_key = %decision.decision_key,
                error = %error,
                "Automatic deletion completed but behavior event recording failed"
            );
        }
        if let Err(error) = CurationService::new(self.pool.clone())
            .record_disposition_with_metadata(
                &decision.user_id,
                &decision.archive_id,
                "auto_delete",
                Some(&decision.reason),
                "auto_delete",
                &metadata,
                Some(&decision.decision_key),
            )
            .await
        {
            tracing::warn!(
                archive_id = %decision.archive_id,
                decision_key = %decision.decision_key,
                error = %error,
                "Automatic deletion completed but disposition recording failed"
            );
        }

        Ok(AutoDeleteResult::Applied)
    }

    pub async fn apply(&self, decision: AutoDeleteDecision) -> Result<AutoDeleteResult> {
        self.execute(decision).await
    }

    async fn decision_already_completed(&self, user_id: &str, decision_key: &str) -> Result<bool> {
        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM trash_entries WHERE user_id = ? AND decision_key = ? LIMIT 1",
        )
        .bind(user_id)
        .bind(decision_key)
        .fetch_optional(&self.pool)
        .await
        .context("failed to check automatic deletion decision")?;
        Ok(exists.is_some())
    }

    async fn archive_or_decision_already_in_trash(
        &self,
        user_id: &str,
        archive_id: &str,
        decision_key: &str,
    ) -> Result<bool> {
        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM trash_entries
             WHERE user_id = ? AND (decision_key = ? OR (archive_id = ? AND status = 'active'))
             LIMIT 1",
        )
        .bind(user_id)
        .bind(decision_key)
        .bind(archive_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to inspect concurrent trash deletion")?;
        Ok(exists.is_some())
    }

    async fn entry_has_decision_key(&self, entry_id: &str, decision_key: &str) -> Result<bool> {
        let stored: Option<String> =
            sqlx::query_scalar("SELECT decision_key FROM trash_entries WHERE id = ? LIMIT 1")
                .bind(entry_id)
                .fetch_optional(&self.pool)
                .await
                .context("failed to verify automatic deletion entry")?;
        Ok(stored.as_deref() == Some(decision_key))
    }
}

fn validate_decision(decision: &AutoDeleteDecision) -> Result<()> {
    if decision.archive_id.trim().is_empty()
        || decision.user_id.trim().is_empty()
        || decision.reason.trim().is_empty()
        || decision.rule_version.trim().is_empty()
        || decision.decision_key.trim().is_empty()
    {
        return Err(anyhow!(
            "automatic deletion decision contains an empty required field"
        ));
    }
    if !decision.model_confidence.is_finite() || !(0.0..=1.0).contains(&decision.model_confidence) {
        return Err(anyhow!(
            "automatic deletion model confidence must be between 0 and 1"
        ));
    }
    if decision.model_confidence < AUTO_DELETE_CONFIDENCE_THRESHOLD {
        return Err(anyhow!(
            "automatic deletion confidence is below the configured threshold"
        ));
    }
    if decision.evidence_pages.iter().any(|page| *page < 1) {
        return Err(anyhow!(
            "automatic deletion evidence pages must be positive"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use uuid::Uuid;

    async fn setup() -> (Pool<Sqlite>, std::path::PathBuf) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE archives (id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT, path TEXT NOT NULL, file_hash TEXT UNIQUE NOT NULL, file_size INTEGER NOT NULL, page_count INTEGER NOT NULL, created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL)")
            .execute(&pool).await.unwrap();
        sqlx::query(
            "CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL, PRIMARY KEY (archive_id, tag_id))")
            .execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE trash_entries (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, archive_id TEXT NOT NULL, original_path TEXT NOT NULL, trash_path TEXT, reason TEXT, rule_version TEXT, model_confidence REAL, metadata_json TEXT NOT NULL, decision_key TEXT, status TEXT NOT NULL, deleted_at DATETIME NOT NULL, expires_at DATETIME, restored_at DATETIME, cleanup_attempts INTEGER NOT NULL DEFAULT 0, last_cleanup_attempt_at DATETIME, last_cleanup_error TEXT, expired_at DATETIME, restore_claimed_at DATETIME)")
            .execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE user_behavior_events (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, archive_id TEXT, event_type TEXT NOT NULL, event_key TEXT, page INTEGER, metadata_json TEXT NOT NULL, occurred_at DATETIME NOT NULL, created_at DATETIME NOT NULL, UNIQUE(user_id, event_key))")
            .execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE archive_dispositions (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, archive_id TEXT NOT NULL, disposition TEXT NOT NULL, reason TEXT, source TEXT NOT NULL, metadata_json TEXT NOT NULL, decision_key TEXT, created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL)")
            .execute(&pool).await.unwrap();
        let dir = std::env::temp_dir().join(format!("otamoryx-auto-delete-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        (pool, dir)
    }

    async fn insert_archive(pool: &Pool<Sqlite>, dir: &std::path::Path, id: &str) {
        let path = dir.join(format!("{id}.cbz"));
        tokio::fs::write(&path, b"archive").await.unwrap();
        sqlx::query("INSERT INTO archives (id, title, path, file_hash, file_size, page_count, created_at, updated_at) VALUES (?, ?, ?, ?, 7, 3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(id).bind("Test").bind(path.to_string_lossy().as_ref()).bind(format!("hash-{id}"))
            .execute(pool).await.unwrap();
    }

    fn decision(archive_id: &str, key: &str) -> AutoDeleteDecision {
        AutoDeleteDecision {
            archive_id: archive_id.to_string(),
            user_id: "user-1".to_string(),
            reason: "negative preference combination".to_string(),
            rule_version: "rules-v3".to_string(),
            model_confidence: 0.97,
            evidence_pages: vec![1, 2, 8],
            decision_key: key.to_string(),
        }
    }

    #[tokio::test]
    async fn applies_auditable_auto_delete_decision() {
        let (pool, dir) = setup().await;
        insert_archive(&pool, &dir, "a1").await;
        let result = AutoDeleteService::new(pool.clone())
            .execute(decision("a1", "decision-1"))
            .await
            .unwrap();
        assert_eq!(result, AutoDeleteResult::Applied);
        let entry = sqlx::query_as::<_, (String, String, String, f64, String)>(
            "SELECT reason, rule_version, decision_key, model_confidence, metadata_json FROM trash_entries",
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(entry.0, "negative preference combination");
        assert_eq!(entry.1, "rules-v3");
        assert_eq!(entry.2, "decision-1");
        assert_eq!(entry.3, 0.97);
        assert!(entry.4.contains("evidence_pages"));
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM archive_dispositions WHERE disposition = 'auto_delete'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM user_behavior_events WHERE event_type = 'auto_delete'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
        let _ = tokio::fs::remove_dir_all(dir).await;
    }

    #[tokio::test]
    async fn decision_key_makes_retries_idempotent() {
        let (pool, dir) = setup().await;
        insert_archive(&pool, &dir, "a1").await;
        let service = AutoDeleteService::new(pool.clone());
        assert_eq!(
            service.execute(decision("a1", "decision-1")).await.unwrap(),
            AutoDeleteResult::Applied
        );
        assert_eq!(
            service.execute(decision("a1", "decision-1")).await.unwrap(),
            AutoDeleteResult::AlreadyCompleted
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM trash_entries")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM archive_dispositions")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        let _ = tokio::fs::remove_dir_all(dir).await;
    }

    #[tokio::test]
    async fn rejects_decisions_below_the_automatic_threshold() {
        let (pool, dir) = setup().await;
        insert_archive(&pool, &dir, "a1").await;
        let mut low = decision("a1", "decision-low");
        low.model_confidence = 0.84;
        let error = AutoDeleteService::new(pool.clone())
            .execute(low)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("below the configured threshold"));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM trash_entries")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        let _ = tokio::fs::remove_dir_all(dir).await;
    }

    #[tokio::test]
    async fn manual_trash_entry_wins_without_a_second_auto_disposition() {
        let (pool, dir) = setup().await;
        insert_archive(&pool, &dir, "a1").await;
        TrashService::new(pool.clone())
            .move_archive_to_trash("user-1", "a1", Some("manual"), "user")
            .await
            .unwrap();

        let result = AutoDeleteService::new(pool.clone())
            .execute(decision("a1", "decision-after-manual"))
            .await
            .unwrap();
        assert_eq!(result, AutoDeleteResult::AlreadyCompleted);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM trash_entries")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM archive_dispositions")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        let _ = tokio::fs::remove_dir_all(dir).await;
    }

    #[tokio::test]
    async fn failed_file_move_does_not_record_a_successful_disposition() {
        let (pool, dir) = setup().await;
        let missing_path = dir.join("missing.cbz");
        sqlx::query("INSERT INTO archives (id, title, path, file_hash, file_size, page_count, created_at, updated_at) VALUES ('a1', 'Test', ?, 'hash-a1', 7, 3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(missing_path.to_string_lossy().as_ref())
            .execute(&pool).await.unwrap();

        let result = AutoDeleteService::new(pool.clone())
            .execute(decision("a1", "decision-failed-move"))
            .await;
        assert!(result.is_err());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM trash_entries")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM archive_dispositions")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        let _ = tokio::fs::remove_dir_all(dir).await;
    }
}
