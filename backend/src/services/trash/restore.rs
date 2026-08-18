use super::relations::revert_version_relations;
use super::snapshot::restore_archive_snapshot;
use super::{ArchiveSnapshot, TrashService, VersionOperationMember, VersionRelationMigration};
use crate::models::TrashEntry;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use sqlx::Row;
use std::path::Path;

impl TrashService {
    pub async fn restore_entry(&self, user_id: &str, entry_id: &str) -> Result<TrashEntry> {
        let entry = sqlx::query_as::<_, TrashEntry>(
            "SELECT id, user_id, archive_id, original_path, trash_path, reason, rule_version, rule_id, evaluation_id,
                    model_confidence, metadata_json, operation_id, operation_type, status, deleted_at, expires_at, restored_at,
                    cleanup_attempts, last_cleanup_attempt_at, last_cleanup_error, expired_at
             FROM trash_entries WHERE id = ? AND user_id = ?",
        )
        .bind(entry_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load trash entry")?
        .ok_or_else(|| anyhow!("trash entry not found"))?;

        if entry.status != "active" {
            return Err(anyhow!("trash entry is not active"));
        }
        if entry.operation_type.as_deref() == Some("version_cleanup") {
            return Err(anyhow!(
                "version cleanup entries must be restored through their operation"
            ));
        }
        if !self.claim_restore_entry(user_id, entry_id).await? {
            return Err(anyhow!("trash entry is not active"));
        }

        let trash_path = match entry.trash_path.as_deref() {
            Some(path) => path,
            None => {
                self.release_restore_claim(user_id, entry_id).await?;
                return Err(anyhow!("trash entry has no file path"));
            }
        };
        let original_path = Path::new(&entry.original_path);
        let original_exists = match tokio::fs::try_exists(original_path).await {
            Ok(exists) => exists,
            Err(error) => {
                self.release_restore_claim(user_id, entry_id).await?;
                return Err(error.into());
            }
        };
        if original_exists {
            self.release_restore_claim(user_id, entry_id).await?;
            return Err(anyhow!("original archive path already exists"));
        }
        let trash_exists = match tokio::fs::try_exists(trash_path).await {
            Ok(exists) => exists,
            Err(error) => {
                self.release_restore_claim(user_id, entry_id).await?;
                return Err(error.into());
            }
        };
        if !trash_exists {
            self.release_restore_claim(user_id, entry_id).await?;
            return Err(anyhow!("trash file is missing"));
        }
        if let Some(parent) = original_path.parent() {
            if let Err(error) = tokio::fs::create_dir_all(parent)
                .await
                .context("failed to create archive directory")
            {
                self.release_restore_claim(user_id, entry_id).await?;
                return Err(error);
            }
        }

        let snapshot: ArchiveSnapshot = match serde_json::from_str(&entry.metadata_json)
            .context("failed to decode archive snapshot")
        {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.release_restore_claim(user_id, entry_id).await?;
                return Err(error);
            }
        };
        if let Err(error) = tokio::fs::rename(trash_path, original_path)
            .await
            .context("failed to restore archive file")
        {
            self.release_restore_claim(user_id, entry_id).await?;
            return Err(error);
        }

        let result = async {
            let mut tx = self
                .pool
                .begin()
                .await
                .context("failed to start restore transaction")?;
            restore_archive_snapshot(&mut tx, &snapshot).await?;

            let restored = sqlx::query(
                "UPDATE trash_entries
                 SET status = 'restored', restored_at = CURRENT_TIMESTAMP, restore_claimed_at = NULL
                 WHERE id = ? AND user_id = ? AND status = 'active'",
            )
            .bind(entry_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .context("failed to mark trash entry restored")?;
            if restored.rows_affected() == 0 {
                return Err(anyhow!("trash entry is not active"));
            }
            tx.commit()
                .await
                .context("failed to commit restore transaction")?;
            Ok::<(), anyhow::Error>(())
        }
        .await;

        if let Err(error) = result {
            if let Err(rollback_error) = tokio::fs::rename(original_path, trash_path).await {
                tracing::error!(
                    "failed to move archive back to trash after restore error: {}",
                    rollback_error
                );
            }
            self.release_restore_claim(user_id, entry_id).await?;
            return Err(error);
        }

        let mut restored = entry;
        restored.status = "restored".to_string();
        restored.restored_at = Some(Utc::now());
        Ok(restored)
    }

    /// Restore all members of a version cleanup as one compensating operation.
    /// Filesystem moves are performed only after every member has been
    /// preflighted; all database changes then commit in one transaction.
    pub async fn restore_operation(
        &self,
        user_id: &str,
        operation_id: &str,
    ) -> Result<Vec<TrashEntry>> {
        self.restore_version_operation(user_id, operation_id, "restored")
            .await
    }

    /// Compensate a partially applied cleanup. The operation remains visible
    /// as failed, but all members that were moved before the failure are
    /// restored through the same group-level path as an explicit undo.
    pub async fn rollback_version_cleanup(&self, user_id: &str, operation_id: &str) -> Result<()> {
        self.restore_version_operation(user_id, operation_id, "failed")
            .await
            .map(|_| ())
    }

    async fn restore_version_operation(
        &self,
        user_id: &str,
        operation_id: &str,
        final_status: &str,
    ) -> Result<Vec<TrashEntry>> {
        let operation = sqlx::query(
            "SELECT status, expires_at,
                    CASE WHEN expires_at IS NOT NULL AND julianday(expires_at) <= julianday('now')
                         THEN 1 ELSE 0 END AS is_expired
             FROM trash_operations
             WHERE id = ? AND user_id = ? AND operation_type = 'version_cleanup'",
        )
        .bind(operation_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow!("trash operation not found"))?;
        let status: String = operation.get("status");
        let is_expired: i64 = operation.get("is_expired");
        let allowed = if final_status == "failed" {
            matches!(
                status.as_str(),
                "pending" | "active" | "failed" | "restoring"
            )
        } else {
            matches!(status.as_str(), "active" | "failed")
        };
        if !allowed || is_expired != 0 {
            return Err(anyhow!("trash operation is not restorable"));
        }

        let members = self
            .load_version_operation_members(user_id, operation_id)
            .await?;
        let member_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM trash_operation_members WHERE operation_id = ?",
        )
        .bind(operation_id)
        .fetch_one(&self.pool)
        .await? as usize;
        if members.is_empty() {
            if final_status == "failed" {
                sqlx::query("UPDATE trash_operations SET status = 'failed' WHERE id = ?")
                    .bind(operation_id)
                    .execute(&self.pool)
                    .await?;
                return Ok(Vec::new());
            }
            return Err(anyhow!("version cleanup operation has no members"));
        }
        if final_status == "restored" && members.len() != member_count {
            return Err(anyhow!(
                "version cleanup operation no longer has all active members"
            ));
        }

        let previous_status = status.clone();
        let claimed = sqlx::query(
            "UPDATE trash_operations SET status = 'restoring'
             WHERE id = ? AND user_id = ? AND status = ?",
        )
        .bind(operation_id)
        .bind(user_id)
        .bind(&previous_status)
        .execute(&self.pool)
        .await?;
        if claimed.rows_affected() != 1 {
            return Err(anyhow!("trash operation is already being restored"));
        }

        let active_members: Vec<&VersionOperationMember> = members
            .iter()
            .filter(|member| member.status == "active")
            .collect();
        if active_members.len() != members.len() {
            self.reset_version_operation_claim(operation_id, &previous_status)
                .await?;
            return Err(anyhow!(
                "version cleanup operation no longer has all active members"
            ));
        }
        for member in &active_members {
            let claimed = sqlx::query(
                "UPDATE trash_entries SET restore_claimed_at = CURRENT_TIMESTAMP
                 WHERE id = ? AND user_id = ? AND status = 'active'
                   AND (restore_claimed_at IS NULL
                        OR julianday(restore_claimed_at) <= julianday('now', '-5 minutes'))",
            )
            .bind(&member.id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
            if claimed.rows_affected() != 1 {
                self.reset_version_operation_claim(operation_id, &previous_status)
                    .await?;
                return Err(anyhow!(
                    "version cleanup operation is already being restored"
                ));
            }
        }

        let mut renames = Vec::with_capacity(active_members.len());
        let filesystem_result = async {
            for member in active_members.iter().rev() {
                let trash_path = member
                    .trash_path
                    .as_deref()
                    .ok_or_else(|| anyhow!("version cleanup member has no trash file"))?;
                if tokio::fs::try_exists(&member.original_path).await? {
                    return Err(anyhow!(
                        "original archive path already exists: {}",
                        member.original_path
                    ));
                }
                if !tokio::fs::try_exists(trash_path).await? {
                    return Err(anyhow!(
                        "version cleanup trash file is missing: {trash_path}"
                    ));
                }
            }
            for member in active_members.iter().rev() {
                let trash_path = member
                    .trash_path
                    .as_deref()
                    .expect("preflight checked path");
                if let Some(parent) = Path::new(&member.original_path).parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::rename(trash_path, &member.original_path).await?;
                renames.push((member.original_path.clone(), trash_path.to_string()));
            }
            Ok::<(), anyhow::Error>(())
        }
        .await;
        if let Err(error) = filesystem_result {
            if let Err(rollback_error) = self.rollback_file_renames(&renames).await {
                tracing::error!(%operation_id, %rollback_error, "failed to roll back version cleanup file restore");
            }
            self.reset_version_operation_claim(operation_id, &previous_status)
                .await?;
            return Err(error);
        }

        let result = async {
            let mut tx = self.pool.begin().await?;
            for member in active_members.iter().rev() {
                let migration: VersionRelationMigration =
                    serde_json::from_str(&member.migration_snapshot_json)
                        .context("failed to decode version relation migration")?;
                if migration.version != 1 {
                    return Err(anyhow!(
                        "version cleanup operation does not contain a recoverable relation snapshot"
                    ));
                }
                revert_version_relations(&mut tx, &migration).await?;
            }
            let mut restored = Vec::with_capacity(active_members.len());
            for member in &active_members {
                let snapshot: ArchiveSnapshot = serde_json::from_str(&member.metadata_json)
                    .context("failed to decode archive snapshot")?;
                restore_archive_snapshot(&mut tx, &snapshot).await?;
                let updated = sqlx::query(
                    "UPDATE trash_entries
                     SET status = 'restored', restored_at = CURRENT_TIMESTAMP, restore_claimed_at = NULL
                     WHERE id = ? AND user_id = ? AND status = 'active'",
                )
                .bind(&member.id)
                .bind(user_id)
                .execute(&mut *tx)
                .await?;
                if updated.rows_affected() != 1 {
                    return Err(anyhow!("version cleanup member is no longer active"));
                }
                let mut entry = member.entry();
                entry.status = "restored".to_string();
                entry.restored_at = Some(Utc::now());
                restored.push(entry);
            }
            sqlx::query(
                "UPDATE trash_operations
                 SET status = ?, restored_at = CASE WHEN ? = 'restored' THEN CURRENT_TIMESTAMP ELSE restored_at END
                 WHERE id = ? AND user_id = ? AND status = 'restoring'",
            )
            .bind(final_status)
            .bind(final_status)
            .bind(operation_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok::<Vec<TrashEntry>, anyhow::Error>(restored)
        }
        .await;

        match result {
            Ok(restored) => Ok(restored),
            Err(error) => {
                let rollback_error = self.rollback_file_renames(&renames).await.err();
                let status = if rollback_error.is_some() {
                    "failed"
                } else {
                    previous_status.as_str()
                };
                sqlx::query(
                    "UPDATE trash_operations SET status = ? WHERE id = ? AND status = 'restoring'",
                )
                .bind(status)
                .bind(operation_id)
                .execute(&self.pool)
                .await?;
                sqlx::query(
                    "UPDATE trash_entries SET restore_claimed_at = NULL
                     WHERE operation_id = ? AND user_id = ? AND status = 'active'",
                )
                .bind(operation_id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
                if let Some(rollback_error) = rollback_error {
                    return Err(error.context(format!(
                        "failed to compensate filesystem rename: {rollback_error}"
                    )));
                }
                Err(error)
            }
        }
    }

    async fn load_version_operation_members(
        &self,
        user_id: &str,
        operation_id: &str,
    ) -> Result<Vec<VersionOperationMember>> {
        Ok(sqlx::query_as::<_, VersionOperationMember>(
            "SELECT t.id, t.user_id, t.archive_id, t.original_path, t.trash_path, t.reason,
                    t.rule_version, t.rule_id, t.evaluation_id, t.model_confidence, t.metadata_json,
                    t.operation_id, t.operation_type, t.status, t.deleted_at, t.expires_at,
                    t.restored_at, t.cleanup_attempts, t.last_cleanup_attempt_at, t.last_cleanup_error,
                    t.expired_at, m.migration_snapshot_json
             FROM trash_operation_members m
             JOIN trash_entries t ON t.id = m.trash_entry_id
             WHERE m.operation_id = ? AND t.user_id = ?
             ORDER BY m.sequence, m.created_at, m.id",
        )
        .bind(operation_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn reset_version_operation_claim(&self, operation_id: &str, status: &str) -> Result<()> {
        sqlx::query("UPDATE trash_operations SET status = ? WHERE id = ? AND status = 'restoring'")
            .bind(status)
            .bind(operation_id)
            .execute(&self.pool)
            .await?;
        sqlx::query(
            "UPDATE trash_entries SET restore_claimed_at = NULL
             WHERE operation_id = ? AND status = 'active'",
        )
        .bind(operation_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn rollback_file_renames(&self, renames: &[(String, String)]) -> Result<()> {
        let mut first_error = None;
        for (original_path, trash_path) in renames.iter().rev() {
            if let Err(error) = tokio::fs::rename(original_path, trash_path).await {
                tracing::error!(%original_path, %trash_path, %error, "failed to roll back version cleanup rename");
                first_error.get_or_insert(error);
            }
        }
        if let Some(error) = first_error {
            Err(error.into())
        } else {
            Ok(())
        }
    }

    async fn claim_restore_entry(&self, user_id: &str, entry_id: &str) -> Result<bool> {
        let claimed = sqlx::query(
            "UPDATE trash_entries SET restore_claimed_at = CURRENT_TIMESTAMP
             WHERE id = ? AND user_id = ? AND status = 'active'
               AND (restore_claimed_at IS NULL
                    OR julianday(restore_claimed_at) <= julianday('now', '-5 minutes'))",
        )
        .bind(entry_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .context("failed to claim trash entry restore")?;
        Ok(claimed.rows_affected() == 1)
    }

    async fn release_restore_claim(&self, user_id: &str, entry_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE trash_entries SET restore_claimed_at = NULL
             WHERE id = ? AND user_id = ? AND status = 'active'",
        )
        .bind(entry_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .context("failed to release trash entry restore claim")?;
        Ok(())
    }
}
