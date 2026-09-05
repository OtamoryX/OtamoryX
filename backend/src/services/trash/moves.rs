use super::relations::migrate_version_relations;
use super::snapshot::trash_path_for;
use crate::models::TrashEntry;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use std::path::PathBuf;
use uuid::Uuid;

impl super::TrashService {
    pub async fn move_archive_to_trash(
        &self,
        user_id: &str,
        archive_id: &str,
        reason: Option<&str>,
        source: &str,
    ) -> Result<TrashEntry> {
        self.move_archive_to_trash_with_decision(
            user_id,
            archive_id,
            reason,
            source,
            None,
            None,
            None,
            None,
            &[],
            None,
        )
        .await
    }

    pub async fn move_archive_to_trash_with_decision(
        &self,
        user_id: &str,
        archive_id: &str,
        reason: Option<&str>,
        source: &str,
        rule_version: Option<&str>,
        model_confidence: Option<f64>,
        rule_id: Option<&str>,
        evaluation_id: Option<&str>,
        evidence_pages: &[i32],
        decision_key: Option<&str>,
    ) -> Result<TrashEntry> {
        if let Some(decision_key) = decision_key {
            if let Some(entry) = self
                .load_entry_by_decision_key(user_id, decision_key)
                .await?
            {
                return Ok(entry);
            }
            // A manual deletion may have won the archive-level race. Treat it
            // as an idempotent completion for automatic delivery.
            if let Some(entry) = self
                .load_active_entry_by_archive(user_id, archive_id)
                .await?
            {
                return Ok(entry);
            }
        }
        let mut snapshot = self.load_snapshot(archive_id).await?;
        snapshot.source = Some(source.to_string());
        snapshot.evidence_pages = evidence_pages.to_vec();
        snapshot.decision_key = decision_key.map(str::to_string);
        let original_path = PathBuf::from(&snapshot.path);
        let entry_id = Uuid::new_v4().to_string();
        let trash_path = trash_path_for(&original_path, &entry_id)?;

        tokio::fs::create_dir_all(
            trash_path
                .parent()
                .ok_or_else(|| anyhow!("archive path has no parent directory"))?,
        )
        .await
        .with_context(|| {
            format!(
                "failed to create archive trash directory for {}",
                trash_path.display()
            )
        })?;
        let metadata_json =
            serde_json::to_string(&snapshot).context("failed to encode archive snapshot")?;
        let result = async {
            let mut tx = self
                .pool
                .begin()
                .await
                .context("failed to start trash transaction")?;
            let deleted = sqlx::query("DELETE FROM archives WHERE id = ?")
                .bind(archive_id)
                .execute(&mut *tx)
                .await
                .context("failed to remove archive record")?;
            if deleted.rows_affected() == 0 {
                return Err(anyhow!("archive not found: {archive_id}"));
            }

            sqlx::query(
                "INSERT INTO trash_entries
                 (id, user_id, archive_id, original_path, trash_path, reason, rule_version, rule_id, evaluation_id,
                  model_confidence, metadata_json, decision_key, status, deleted_at, expires_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'active', CURRENT_TIMESTAMP, datetime('now', '+14 days'))",
            )
            .bind(&entry_id)
            .bind(user_id)
            .bind(archive_id)
            .bind(&snapshot.path)
            .bind(trash_path.to_string_lossy().as_ref())
            .bind(reason)
            .bind(rule_version)
            .bind(rule_id)
            .bind(evaluation_id)
            .bind(model_confidence)
            .bind(&metadata_json)
            .bind(decision_key)
            .execute(&mut *tx)
            .await
            .context("failed to create trash entry")?;

            // Keep the active trash row visible while the filesystem move is in
            // flight. The file monitor uses it to distinguish this internal
            // rename from an external file deletion.
            tokio::fs::rename(&original_path, &trash_path)
                .await
                .with_context(|| {
                    format!(
                        "failed to move archive {} to trash {}",
                        original_path.display(),
                        trash_path.display()
                    )
                })?;

            tx.commit().await.context("failed to commit trash transaction")?;
            Ok::<(), anyhow::Error>(())
        }
        .await;

        if let Err(error) = result {
            if tokio::fs::try_exists(&trash_path).await.unwrap_or(false) {
                if let Err(rollback_error) = tokio::fs::rename(&trash_path, &original_path).await {
                    tracing::error!(
                        "failed to restore archive {} after trash transaction error: {}",
                        archive_id,
                        rollback_error
                    );
                }
            }
            return Err(error);
        }

        Ok(TrashEntry {
            id: entry_id,
            user_id: user_id.to_string(),
            archive_id: archive_id.to_string(),
            original_path: snapshot.path,
            trash_path: Some(trash_path.to_string_lossy().to_string()),
            reason: reason.map(str::to_string),
            rule_version: rule_version.map(str::to_string),
            rule_id: rule_id.map(str::to_string),
            evaluation_id: evaluation_id.map(str::to_string),
            model_confidence,
            metadata_json,
            operation_id: None,
            operation_type: None,
            status: "active".to_string(),
            deleted_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::days(14)),
            restored_at: None,
            cleanup_attempts: 0,
            last_cleanup_attempt_at: None,
            last_cleanup_error: None,
            expired_at: None,
        })
    }

    /// Move one member of a version cleanup into trash while transferring the
    /// additive relations to the kept version. The relation transfer, archive
    /// deletion, audit entry, and file rename are coordinated as one unit: a
    /// failed rename rolls the database transaction back before it is committed.
    pub async fn move_version_group_member_to_trash(
        &self,
        user_id: &str,
        operation_id: &str,
        archive_id: &str,
        keep_archive_id: &str,
        keeper_pages: i32,
    ) -> Result<TrashEntry> {
        if let Some(entry) = self
            .load_entry_by_operation_member(user_id, operation_id, archive_id)
            .await?
        {
            return Ok(entry);
        }

        let mut snapshot = self.load_snapshot(archive_id).await?;
        snapshot.source = Some("version_cleanup".to_string());
        snapshot.decision_key = Some(format!("version-cleanup:{operation_id}:{archive_id}"));
        let original_path = PathBuf::from(&snapshot.path);
        let entry_id = Uuid::new_v4().to_string();
        let trash_path = trash_path_for(&original_path, &entry_id)?;
        tokio::fs::create_dir_all(
            trash_path
                .parent()
                .ok_or_else(|| anyhow!("archive path has no parent directory"))?,
        )
        .await
        .with_context(|| {
            format!(
                "failed to create archive trash directory for {}",
                trash_path.display()
            )
        })?;

        let metadata_json =
            serde_json::to_string(&snapshot).context("failed to encode archive snapshot")?;
        let result = async {
            let mut tx = self
                .pool
                .begin()
                .await
                .context("failed to start version cleanup transaction")?;
            let operation_status = sqlx::query_scalar::<_, String>(
                "SELECT status FROM trash_operations
                 WHERE id = ? AND user_id = ? AND operation_type = 'version_cleanup'",
            )
            .bind(operation_id)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;
            if operation_status.as_deref() != Some("pending") {
                return Err(anyhow!("version cleanup operation is not pending"));
            }
            let migration_snapshot =
                migrate_version_relations(&mut tx, archive_id, keep_archive_id, keeper_pages)
                    .await?;
            let deleted = sqlx::query("DELETE FROM archives WHERE id = ?")
                .bind(archive_id)
                .execute(&mut *tx)
                .await
                .context("failed to remove version archive record")?;
            if deleted.rows_affected() != 1 {
                return Err(anyhow!("archive no longer exists: {archive_id}"));
            }
            sqlx::query(
                "INSERT INTO trash_entries
                 (id, user_id, archive_id, original_path, trash_path, reason, metadata_json,
                  operation_id, operation_type, decision_key, status, deleted_at, expires_at)
                 VALUES (?, ?, ?, ?, ?, 'version_cleanup', ?, ?, 'version_cleanup', ?, 'active',
                         CURRENT_TIMESTAMP, datetime('now', '+14 days'))",
            )
            .bind(&entry_id)
            .bind(user_id)
            .bind(archive_id)
            .bind(&snapshot.path)
            .bind(trash_path.to_string_lossy().as_ref())
            .bind(&metadata_json)
            .bind(operation_id)
            .bind(format!("version-cleanup:{operation_id}:{archive_id}"))
            .execute(&mut *tx)
            .await
            .context("failed to create version cleanup trash entry")?;
            sqlx::query(
                "INSERT INTO trash_operation_members
                 (id, operation_id, archive_id, trash_entry_id, migration_snapshot_json, sequence)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(operation_id)
            .bind(archive_id)
            .bind(&entry_id)
            .bind(serde_json::to_string(&migration_snapshot)?)
            .bind(sqlx::query_scalar::<_, i64>(
                "SELECT COALESCE(MAX(sequence), -1) + 1 FROM trash_operation_members WHERE operation_id = ?",
            )
            .bind(operation_id)
            .fetch_one(&mut *tx)
            .await?)
            .execute(&mut *tx)
            .await
            .context("failed to record version cleanup member")?;

            tokio::fs::rename(&original_path, &trash_path)
                .await
                .with_context(|| {
                    format!(
                        "failed to move version archive {} to trash {}",
                        original_path.display(),
                        trash_path.display()
                    )
                })?;
            tx.commit()
                .await
                .context("failed to commit version cleanup transaction")?;
            Ok::<_, anyhow::Error>(())
        }
        .await;
        if let Err(error) = result {
            if tokio::fs::try_exists(&trash_path).await.unwrap_or(false) {
                if let Err(rollback_error) = tokio::fs::rename(&trash_path, &original_path).await {
                    tracing::error!(%archive_id, %rollback_error, "failed to roll back version trash file");
                }
            }
            return Err(error);
        }

        Ok(TrashEntry {
            id: entry_id,
            user_id: user_id.to_string(),
            archive_id: archive_id.to_string(),
            original_path: snapshot.path,
            trash_path: Some(trash_path.to_string_lossy().to_string()),
            reason: Some("version_cleanup".to_string()),
            rule_version: None,
            rule_id: None,
            evaluation_id: None,
            model_confidence: None,
            metadata_json,
            operation_id: Some(operation_id.to_string()),
            operation_type: Some("version_cleanup".to_string()),
            status: "active".to_string(),
            deleted_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::days(14)),
            restored_at: None,
            cleanup_attempts: 0,
            last_cleanup_attempt_at: None,
            last_cleanup_error: None,
            expired_at: None,
        })
    }
}
