use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite, Transaction};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

use crate::models::TrashEntry;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArchiveSnapshot {
    id: String,
    title: String,
    subtitle: Option<String>,
    subtitle_language: Option<String>,
    path: String,
    file_hash: String,
    file_size: i64,
    page_count: i32,
    created_at: String,
    updated_at: String,
    tags: Vec<TagSnapshot>,
    #[serde(default)]
    related_inserts: Vec<String>,
    #[serde(default)]
    related_updates: Vec<String>,
    source: Option<String>,
    #[serde(default)]
    evidence_pages: Vec<i32>,
    #[serde(default)]
    decision_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TagSnapshot {
    id: String,
    name: String,
    namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionRelationMigration {
    #[serde(default)]
    version: u8,
    keeper_archive_id: String,
    #[serde(default)]
    before_tag_ids: Vec<String>,
    #[serde(default)]
    after_tag_ids: Vec<String>,
    #[serde(default)]
    before_category_ids: Vec<String>,
    #[serde(default)]
    after_category_ids: Vec<String>,
    #[serde(default)]
    progress: Vec<VersionProgressMigration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionProgressMigration {
    user_id: String,
    before: Option<ReadingProgressSnapshot>,
    after: ReadingProgressSnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
struct ReadingProgressSnapshot {
    id: String,
    user_id: String,
    archive_id: String,
    current_page: i32,
    total_pages: i32,
    progress_percentage: f64,
    last_read_at: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
struct VersionOperationMember {
    id: String,
    user_id: String,
    archive_id: String,
    original_path: String,
    trash_path: Option<String>,
    reason: Option<String>,
    rule_version: Option<String>,
    rule_id: Option<String>,
    evaluation_id: Option<String>,
    model_confidence: Option<f64>,
    metadata_json: String,
    operation_id: Option<String>,
    operation_type: Option<String>,
    status: String,
    deleted_at: chrono::DateTime<Utc>,
    expires_at: Option<chrono::DateTime<Utc>>,
    restored_at: Option<chrono::DateTime<Utc>>,
    cleanup_attempts: i64,
    last_cleanup_attempt_at: Option<chrono::DateTime<Utc>>,
    last_cleanup_error: Option<String>,
    expired_at: Option<chrono::DateTime<Utc>>,
    migration_snapshot_json: String,
}

impl VersionOperationMember {
    fn entry(&self) -> TrashEntry {
        TrashEntry {
            id: self.id.clone(),
            user_id: self.user_id.clone(),
            archive_id: self.archive_id.clone(),
            original_path: self.original_path.clone(),
            trash_path: self.trash_path.clone(),
            reason: self.reason.clone(),
            rule_version: self.rule_version.clone(),
            rule_id: self.rule_id.clone(),
            evaluation_id: self.evaluation_id.clone(),
            model_confidence: self.model_confidence,
            metadata_json: self.metadata_json.clone(),
            operation_id: self.operation_id.clone(),
            operation_type: self.operation_type.clone(),
            status: self.status.clone(),
            deleted_at: self.deleted_at,
            expires_at: self.expires_at,
            restored_at: self.restored_at,
            cleanup_attempts: self.cleanup_attempts,
            last_cleanup_attempt_at: self.last_cleanup_attempt_at,
            last_cleanup_error: self.last_cleanup_error.clone(),
            expired_at: self.expired_at,
        }
    }
}

pub struct TrashService {
    pool: Pool<Sqlite>,
}

const TRASH_CLEANUP_BATCH_SIZE: u32 = 100;
const TRASH_CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TrashCleanupReport {
    pub claimed: u32,
    pub deleted_files: u32,
    pub missing_files: u32,
    pub failed: u32,
}

#[derive(sqlx::FromRow)]
struct TrashCleanupCandidate {
    id: String,
    trash_path: Option<String>,
}

enum TrashFileCleanupResult {
    Deleted,
    Missing,
}

pub fn spawn_trash_expiration_cleanup(pool: Pool<Sqlite>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(TRASH_CLEANUP_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            interval.tick().await;
            match TrashService::new(pool.clone())
                .cleanup_expired_entries(TRASH_CLEANUP_BATCH_SIZE)
                .await
            {
                Ok(report) if report.claimed > 0 || report.failed > 0 => {
                    tracing::info!(
                        claimed = report.claimed,
                        deleted_files = report.deleted_files,
                        missing_files = report.missing_files,
                        failed = report.failed,
                        "Finished trash expiration cleanup"
                    );
                }
                Ok(_) => tracing::debug!("Trash expiration cleanup found no pending entries"),
                Err(error) => tracing::warn!("Trash expiration cleanup failed: {error:#}"),
            }
        }
    });
}

impl TrashService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

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
        .context("failed to create archive trash directory")?;
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
                        "failed to move archive {} to trash",
                        original_path.display()
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
        .context("failed to create archive trash directory")?;

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
                        "failed to move version archive {} to trash",
                        original_path.display()
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

    async fn load_entry_by_operation_member(
        &self,
        user_id: &str,
        operation_id: &str,
        archive_id: &str,
    ) -> Result<Option<TrashEntry>> {
        sqlx::query_as::<_, TrashEntry>(
            "SELECT id, user_id, archive_id, original_path, trash_path, reason, rule_version, rule_id, evaluation_id,
                    model_confidence, metadata_json, operation_id, operation_type, status, deleted_at, expires_at, restored_at,
                    cleanup_attempts, last_cleanup_attempt_at, last_cleanup_error, expired_at
             FROM trash_entries
             WHERE user_id = ? AND operation_id = ? AND archive_id = ? LIMIT 1",
        )
        .bind(user_id)
        .bind(operation_id)
        .bind(archive_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load version cleanup trash entry")
    }

    async fn load_entry_by_decision_key(
        &self,
        user_id: &str,
        decision_key: &str,
    ) -> Result<Option<TrashEntry>> {
        sqlx::query_as::<_, TrashEntry>(
            "SELECT id, user_id, archive_id, original_path, trash_path, reason, rule_version, rule_id, evaluation_id,
                    model_confidence, metadata_json, operation_id, operation_type, status, deleted_at, expires_at, restored_at,
                    cleanup_attempts, last_cleanup_attempt_at, last_cleanup_error, expired_at
             FROM trash_entries WHERE user_id = ? AND decision_key = ? LIMIT 1",
        )
        .bind(user_id)
        .bind(decision_key)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load automatic deletion decision")
    }

    async fn load_active_entry_by_archive(
        &self,
        user_id: &str,
        archive_id: &str,
    ) -> Result<Option<TrashEntry>> {
        sqlx::query_as::<_, TrashEntry>(
            "SELECT id, user_id, archive_id, original_path, trash_path, reason, rule_version, rule_id, evaluation_id,
                    model_confidence, metadata_json, operation_id, operation_type, status, deleted_at, expires_at, restored_at,
                    cleanup_attempts, last_cleanup_attempt_at, last_cleanup_error, expired_at
             FROM trash_entries WHERE user_id = ? AND archive_id = ? AND status = 'active'
             ORDER BY deleted_at DESC LIMIT 1",
        )
        .bind(user_id)
        .bind(archive_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load active archive trash entry")
    }

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

    pub async fn list_entries(
        &self,
        user_id: &str,
        status: Option<&str>,
        limit: u32,
    ) -> Result<Vec<TrashEntry>> {
        let limit = limit.clamp(1, 200) as i64;
        let mut query = String::from(
            "SELECT id, user_id, archive_id, original_path, trash_path, reason, rule_version, rule_id, evaluation_id,
                    model_confidence, metadata_json, operation_id, operation_type, status, deleted_at, expires_at, restored_at,
                    cleanup_attempts, last_cleanup_attempt_at, last_cleanup_error, expired_at
             FROM trash_entries WHERE user_id = ?",
        );
        if status.is_some() {
            query.push_str(" AND status = ?");
        }
        query.push_str(" ORDER BY deleted_at DESC LIMIT ?");
        let mut request = sqlx::query_as::<_, TrashEntry>(&query).bind(user_id);
        if let Some(status) = status {
            request = request.bind(status);
        }
        Ok(request.bind(limit).fetch_all(&self.pool).await?)
    }

    pub async fn cleanup_expired_entries(&self, limit: u32) -> Result<TrashCleanupReport> {
        let limit = limit.clamp(1, TRASH_CLEANUP_BATCH_SIZE) as i64;
        let candidates = sqlx::query_as::<_, TrashCleanupCandidate>(
            "SELECT id, trash_path FROM trash_entries
            WHERE (status = 'active' AND expires_at IS NOT NULL
                    AND julianday(expires_at) <= julianday('now')
                    AND (restore_claimed_at IS NULL
                         OR julianday(restore_claimed_at) <= julianday('now', '-5 minutes')))
                OR (status = 'expired' AND expired_at IS NULL
                    AND (last_cleanup_attempt_at IS NULL
                         OR julianday(last_cleanup_attempt_at) <= julianday('now', '-5 minutes')))
             ORDER BY CASE status WHEN 'active' THEN 0 ELSE 1 END, expires_at ASC,
                      last_cleanup_attempt_at ASC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("failed to load expired trash entries")?;

        let mut report = TrashCleanupReport::default();
        for candidate in candidates {
            let claimed = sqlx::query(
                "UPDATE trash_entries
                 SET status = 'expired', cleanup_attempts = cleanup_attempts + 1,
                     last_cleanup_attempt_at = CURRENT_TIMESTAMP, last_cleanup_error = NULL
                 WHERE id = ? AND (
                     (status = 'active' AND expires_at IS NOT NULL
                      AND julianday(expires_at) <= julianday('now')
                      AND (restore_claimed_at IS NULL
                           OR julianday(restore_claimed_at) <= julianday('now', '-5 minutes')))
                     OR (status = 'expired' AND expired_at IS NULL
                         AND (last_cleanup_attempt_at IS NULL
                              OR julianday(last_cleanup_attempt_at) <= julianday('now', '-5 minutes')))
                 )",
            )
            .bind(&candidate.id)
            .execute(&self.pool)
            .await
            .with_context(|| format!("failed to claim expired trash entry {}", candidate.id))?;
            if claimed.rows_affected() == 0 {
                continue;
            }

            report.claimed += 1;
            match self
                .remove_trash_file(candidate.trash_path.as_deref())
                .await
            {
                Ok(TrashFileCleanupResult::Deleted) => {
                    report.deleted_files += 1;
                    self.finish_cleanup(&candidate.id, &mut report).await?;
                }
                Ok(TrashFileCleanupResult::Missing) => {
                    report.missing_files += 1;
                    self.finish_cleanup(&candidate.id, &mut report).await?;
                }
                Err(error) => {
                    report.failed += 1;
                    self.record_cleanup_failure(&candidate.id, &error.to_string())
                        .await
                        .with_context(|| {
                            format!(
                                "failed to record cleanup failure for trash entry {}",
                                candidate.id
                            )
                        })?;
                    tracing::warn!(
                        trash_entry_id = %candidate.id,
                        error = %error,
                        "Failed to remove expired trash file"
                    );
                }
            }
        }

        Ok(report)
    }

    async fn remove_trash_file(&self, trash_path: Option<&str>) -> Result<TrashFileCleanupResult> {
        let Some(trash_path) = trash_path else {
            return Ok(TrashFileCleanupResult::Missing);
        };
        if !tokio::fs::try_exists(trash_path)
            .await
            .context("failed to inspect expired trash file")?
        {
            return Ok(TrashFileCleanupResult::Missing);
        }
        tokio::fs::remove_file(trash_path)
            .await
            .with_context(|| format!("failed to permanently remove trash file {trash_path}"))?;
        Ok(TrashFileCleanupResult::Deleted)
    }

    async fn finish_cleanup(&self, entry_id: &str, report: &mut TrashCleanupReport) -> Result<()> {
        let completed = sqlx::query(
            "UPDATE trash_entries
             SET expired_at = CURRENT_TIMESTAMP, last_cleanup_error = NULL
             WHERE id = ? AND status = 'expired' AND expired_at IS NULL",
        )
        .bind(entry_id)
        .execute(&self.pool)
        .await;

        match completed {
            Ok(_) => Ok(()),
            Err(error) => {
                report.failed += 1;
                self.record_cleanup_failure(entry_id, &error.to_string())
                    .await
                    .context("failed to record cleanup finalization failure")?;
                tracing::warn!(
                    trash_entry_id = entry_id,
                    error = %error,
                    "Expired trash file was removed but cleanup finalization failed"
                );
                Ok(())
            }
        }
    }

    async fn record_cleanup_failure(&self, entry_id: &str, error: &str) -> Result<()> {
        sqlx::query(
            "UPDATE trash_entries
             SET last_cleanup_error = ?
             WHERE id = ? AND status = 'expired' AND expired_at IS NULL",
        )
        .bind(error)
        .bind(entry_id)
        .execute(&self.pool)
        .await
        .context("failed to save trash cleanup error")?;
        Ok(())
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

    async fn load_snapshot(&self, archive_id: &str) -> Result<ArchiveSnapshot> {
        let row = sqlx::query(
            "SELECT id, title, subtitle, subtitle_language, path, file_hash, file_size, page_count, created_at, updated_at
             FROM archives WHERE id = ?",
        )
        .bind(archive_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load archive for trash")?
        .ok_or_else(|| anyhow!("archive not found: {archive_id}"))?;

        let tag_rows = sqlx::query(
            "SELECT t.id, t.name, t.namespace FROM tags t
             INNER JOIN archive_tags at ON at.tag_id = t.id WHERE at.archive_id = ?",
        )
        .bind(archive_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to load archive tags for trash")?;

        let (related_inserts, related_updates) = self.load_related_snapshots(archive_id).await?;

        Ok(ArchiveSnapshot {
            id: row.try_get("id")?,
            title: row.try_get("title")?,
            subtitle: row.try_get("subtitle")?,
            subtitle_language: row.try_get("subtitle_language")?,
            path: row.try_get("path")?,
            file_hash: row.try_get("file_hash")?,
            file_size: row.try_get("file_size")?,
            page_count: row.try_get("page_count")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            tags: tag_rows
                .into_iter()
                .map(|tag| {
                    Ok(TagSnapshot {
                        id: tag.try_get("id")?,
                        name: tag.try_get("name")?,
                        namespace: tag.try_get("namespace")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            related_inserts,
            related_updates,
            source: None,
            evidence_pages: Vec::new(),
            decision_key: None,
        })
    }

    async fn load_related_snapshots(&self, archive_id: &str) -> Result<(Vec<String>, Vec<String>)> {
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list archive relation tables")?;

        let mut inserts = Vec::new();
        let mut updates = Vec::new();
        for table in tables {
            // Tags are captured separately in ArchiveSnapshot so restoring an
            // archive cannot attempt to insert the same archive_tags rows twice.
            if matches!(table.as_str(), "archives" | "archive_tags") {
                continue;
            }

            let table_sql = quote_identifier(&table);
            let foreign_keys = sqlx::query(&format!("PRAGMA foreign_key_list({table_sql})"))
                .fetch_all(&self.pool)
                .await
                .with_context(|| format!("failed to inspect foreign keys for {table}"))?;
            let archive_keys = foreign_keys
                .iter()
                .filter_map(|row| {
                    let referenced_table: String = row.try_get("table").ok()?;
                    if referenced_table != "archives" {
                        return None;
                    }
                    Some((
                        row.try_get::<String, _>("from").ok()?,
                        row.try_get::<String, _>("on_delete").ok()?,
                    ))
                })
                .collect::<Vec<_>>();
            if archive_keys.is_empty() {
                continue;
            }

            let column_rows = sqlx::query(&format!("PRAGMA table_info({table_sql})"))
                .fetch_all(&self.pool)
                .await
                .with_context(|| format!("failed to inspect columns for {table}"))?;
            let columns = column_rows
                .iter()
                .map(|row| row.try_get::<String, _>("name"))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let primary_key_columns = column_rows
                .iter()
                .filter_map(|row| {
                    let position: i64 = row.try_get("pk").ok()?;
                    if position == 0 {
                        return None;
                    }
                    Some((position, row.try_get::<String, _>("name").ok()?))
                })
                .collect::<Vec<_>>();
            if columns.is_empty() {
                continue;
            }

            let select_values = columns
                .iter()
                .map(|column| format!("quote({})", quote_identifier(column)))
                .collect::<Vec<_>>()
                .join(", ");
            let where_clause = archive_keys
                .iter()
                .map(|(column, _)| format!("{} = ?", quote_identifier(column)))
                .collect::<Vec<_>>()
                .join(" OR ");
            let query = format!("SELECT {select_values} FROM {table_sql} WHERE {where_clause}");
            let mut request = sqlx::query(&query);
            for _ in &archive_keys {
                request = request.bind(archive_id);
            }
            for row in request
                .fetch_all(&self.pool)
                .await
                .with_context(|| format!("failed to snapshot rows from {table}"))?
            {
                let values = (0..columns.len())
                    .map(|index| row.try_get::<String, _>(index))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                let cascade = archive_keys
                    .iter()
                    .any(|(_, action)| action.eq_ignore_ascii_case("CASCADE"));
                if cascade {
                    let columns_sql = columns
                        .iter()
                        .map(|column| quote_identifier(column))
                        .collect::<Vec<_>>()
                        .join(", ");
                    inserts.push(format!(
                        "INSERT INTO {table_sql} ({columns_sql}) VALUES ({})",
                        values.join(", ")
                    ));
                } else {
                    let where_columns = if primary_key_columns.is_empty() {
                        columns
                            .iter()
                            .enumerate()
                            .map(|(index, column)| (column.clone(), values[index].clone()))
                            .collect::<Vec<_>>()
                    } else {
                        let mut key_columns = primary_key_columns.clone();
                        key_columns.sort_by_key(|(position, _)| *position);
                        key_columns
                            .into_iter()
                            .filter_map(|(_, column)| {
                                let index = columns.iter().position(|value| value == &column)?;
                                Some((column, values[index].clone()))
                            })
                            .collect::<Vec<_>>()
                    };
                    let set_clause = archive_keys
                        .iter()
                        .map(|(column, _)| {
                            let index = columns.iter().position(|value| value == column).unwrap();
                            format!("{} = {}", quote_identifier(column), values[index])
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    let where_clause = where_columns
                        .iter()
                        .map(|(column, value)| {
                            if value == "NULL" {
                                format!("{} IS NULL", quote_identifier(column))
                            } else {
                                format!("{} = {value}", quote_identifier(column))
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" AND ");
                    updates.push(format!(
                        "UPDATE {table_sql} SET {set_clause} WHERE {where_clause}"
                    ));
                }
            }
        }

        Ok((inserts, updates))
    }
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

async fn migrate_version_relations(
    tx: &mut Transaction<'_, Sqlite>,
    archive_id: &str,
    keep_archive_id: &str,
    keeper_pages: i32,
) -> Result<VersionRelationMigration> {
    let before_tag_ids =
        keeper_relation_ids(tx, "archive_tags", "tag_id", "archive_id", keep_archive_id).await?;
    let source_tag_ids =
        keeper_relation_ids(tx, "archive_tags", "tag_id", "archive_id", archive_id).await?;
    for tag_id in source_tag_ids {
        sqlx::query("INSERT OR IGNORE INTO archive_tags (archive_id, tag_id) VALUES (?, ?)")
            .bind(keep_archive_id)
            .bind(tag_id)
            .execute(&mut **tx)
            .await?;
    }
    let after_tag_ids =
        keeper_relation_ids(tx, "archive_tags", "tag_id", "archive_id", keep_archive_id).await?;

    let before_category_ids = keeper_relation_ids(
        tx,
        "category_archives",
        "category_id",
        "archive_id",
        keep_archive_id,
    )
    .await?;
    let source_category_ids = keeper_relation_ids(
        tx,
        "category_archives",
        "category_id",
        "archive_id",
        archive_id,
    )
    .await?;
    for category_id in source_category_ids {
        sqlx::query(
            "INSERT OR IGNORE INTO category_archives (category_id, archive_id) VALUES (?, ?)",
        )
        .bind(category_id)
        .bind(keep_archive_id)
        .execute(&mut **tx)
        .await?;
    }
    let after_category_ids = keeper_relation_ids(
        tx,
        "category_archives",
        "category_id",
        "archive_id",
        keep_archive_id,
    )
    .await?;

    let progress_rows = sqlx::query_as::<_, ReadingProgressSnapshot>(
        "SELECT id, user_id, archive_id, current_page, total_pages, progress_percentage,
                last_read_at, created_at, updated_at
         FROM reading_progress WHERE archive_id = ?",
    )
    .bind(archive_id)
    .fetch_all(&mut **tx)
    .await?;
    let mut progress = Vec::with_capacity(progress_rows.len());
    for source_progress in progress_rows {
        let before = sqlx::query_as::<_, ReadingProgressSnapshot>(
            "SELECT id, user_id, archive_id, current_page, total_pages, progress_percentage,
                    last_read_at, created_at, updated_at
             FROM reading_progress WHERE user_id = ? AND archive_id = ?",
        )
        .bind(&source_progress.user_id)
        .bind(keep_archive_id)
        .fetch_optional(&mut **tx)
        .await?;
        let current_page =
            ((source_progress.progress_percentage * f64::from(keeper_pages)).ceil() as i32).max(1);
        sqlx::query(
            "INSERT INTO reading_progress
                (id, user_id, archive_id, current_page, total_pages, progress_percentage, last_read_at, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(user_id, archive_id) DO UPDATE SET
                current_page = CASE WHEN excluded.progress_percentage > reading_progress.progress_percentage THEN excluded.current_page ELSE reading_progress.current_page END,
                total_pages = CASE WHEN excluded.progress_percentage > reading_progress.progress_percentage THEN excluded.total_pages ELSE reading_progress.total_pages END,
                progress_percentage = MAX(reading_progress.progress_percentage, excluded.progress_percentage),
                last_read_at = CASE WHEN excluded.progress_percentage > reading_progress.progress_percentage THEN excluded.last_read_at ELSE reading_progress.last_read_at END,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&source_progress.user_id)
        .bind(keep_archive_id)
        .bind(current_page)
        .bind(keeper_pages)
        .bind(source_progress.progress_percentage)
        .execute(&mut **tx)
        .await?;
        let after = sqlx::query_as::<_, ReadingProgressSnapshot>(
            "SELECT id, user_id, archive_id, current_page, total_pages, progress_percentage,
                    last_read_at, created_at, updated_at
             FROM reading_progress WHERE user_id = ? AND archive_id = ?",
        )
        .bind(&source_progress.user_id)
        .bind(keep_archive_id)
        .fetch_one(&mut **tx)
        .await?;
        progress.push(VersionProgressMigration {
            user_id: source_progress.user_id,
            before,
            after,
        });
    }

    Ok(VersionRelationMigration {
        version: 1,
        keeper_archive_id: keep_archive_id.to_string(),
        before_tag_ids,
        after_tag_ids,
        before_category_ids,
        after_category_ids,
        progress,
    })
}

async fn keeper_relation_ids(
    tx: &mut Transaction<'_, Sqlite>,
    table: &str,
    relation_column: &str,
    archive_column: &str,
    archive_id: &str,
) -> Result<Vec<String>> {
    let table = quote_identifier(table);
    let relation_column = quote_identifier(relation_column);
    let archive_column = quote_identifier(archive_column);
    let mut ids = sqlx::query_scalar::<_, String>(&format!(
        "SELECT {relation_column} FROM {table} WHERE {archive_column} = ? ORDER BY {relation_column}"
    ))
    .bind(archive_id)
    .fetch_all(&mut **tx)
    .await?;
    ids.sort();
    Ok(ids)
}

async fn revert_version_relations(
    tx: &mut Transaction<'_, Sqlite>,
    migration: &VersionRelationMigration,
) -> Result<()> {
    let current_tag_ids = keeper_relation_ids(
        tx,
        "archive_tags",
        "tag_id",
        "archive_id",
        &migration.keeper_archive_id,
    )
    .await?;
    if current_tag_ids != migration.after_tag_ids {
        return Err(anyhow!(
            "version cleanup relation state changed since cleanup (tags)"
        ));
    }
    for tag_id in migration
        .after_tag_ids
        .iter()
        .filter(|id| !migration.before_tag_ids.contains(id))
    {
        sqlx::query("DELETE FROM archive_tags WHERE archive_id = ? AND tag_id = ?")
            .bind(&migration.keeper_archive_id)
            .bind(tag_id)
            .execute(&mut **tx)
            .await?;
    }

    let current_category_ids = keeper_relation_ids(
        tx,
        "category_archives",
        "category_id",
        "archive_id",
        &migration.keeper_archive_id,
    )
    .await?;
    if current_category_ids != migration.after_category_ids {
        return Err(anyhow!(
            "version cleanup relation state changed since cleanup (categories)"
        ));
    }
    for category_id in migration
        .after_category_ids
        .iter()
        .filter(|id| !migration.before_category_ids.contains(id))
    {
        sqlx::query("DELETE FROM category_archives WHERE category_id = ? AND archive_id = ?")
            .bind(category_id)
            .bind(&migration.keeper_archive_id)
            .execute(&mut **tx)
            .await?;
    }

    for progress in &migration.progress {
        let current = sqlx::query_as::<_, ReadingProgressSnapshot>(
            "SELECT id, user_id, archive_id, current_page, total_pages, progress_percentage,
                    last_read_at, created_at, updated_at
             FROM reading_progress WHERE user_id = ? AND archive_id = ?",
        )
        .bind(&progress.user_id)
        .bind(&migration.keeper_archive_id)
        .fetch_optional(&mut **tx)
        .await?;
        if current.as_ref() != Some(&progress.after) {
            return Err(anyhow!(
                "version cleanup relation state changed since cleanup (reading progress)"
            ));
        }
        if let Some(before) = &progress.before {
            sqlx::query(
                "UPDATE reading_progress SET id = ?, current_page = ?, total_pages = ?,
                        progress_percentage = ?, last_read_at = ?, created_at = ?, updated_at = ?
                 WHERE user_id = ? AND archive_id = ?",
            )
            .bind(&before.id)
            .bind(before.current_page)
            .bind(before.total_pages)
            .bind(before.progress_percentage)
            .bind(&before.last_read_at)
            .bind(&before.created_at)
            .bind(&before.updated_at)
            .bind(&progress.user_id)
            .bind(&migration.keeper_archive_id)
            .execute(&mut **tx)
            .await?;
        } else {
            sqlx::query("DELETE FROM reading_progress WHERE user_id = ? AND archive_id = ?")
                .bind(&progress.user_id)
                .bind(&migration.keeper_archive_id)
                .execute(&mut **tx)
                .await?;
        }
    }

    Ok(())
}

async fn restore_archive_snapshot(
    tx: &mut Transaction<'_, Sqlite>,
    snapshot: &ArchiveSnapshot,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO archives
         (id, title, subtitle, subtitle_language, path, file_hash, file_size, page_count, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&snapshot.id)
    .bind(&snapshot.title)
    .bind(&snapshot.subtitle)
    .bind(&snapshot.subtitle_language)
    .bind(&snapshot.path)
    .bind(&snapshot.file_hash)
    .bind(snapshot.file_size)
    .bind(snapshot.page_count)
    .bind(&snapshot.created_at)
    .bind(&snapshot.updated_at)
    .execute(&mut **tx)
    .await
    .context("failed to restore archive record")?;

    for tag in &snapshot.tags {
        sqlx::query("INSERT OR IGNORE INTO tags (id, name, namespace) VALUES (?, ?, ?)")
            .bind(&tag.id)
            .bind(&tag.name)
            .bind(&tag.namespace)
            .execute(&mut **tx)
            .await
            .context("failed to restore archive tag")?;
        sqlx::query("INSERT OR IGNORE INTO archive_tags (archive_id, tag_id) VALUES (?, ?)")
            .bind(&snapshot.id)
            .bind(&tag.id)
            .execute(&mut **tx)
            .await
            .context("failed to restore archive tag relation")?;
    }

    // Archive deletion cascades through several optional feature tables
    // (reading progress, categories, collections, AI data, ...). These
    // statements restore the rows and references captured before deletion.
    for statement in &snapshot.related_inserts {
        sqlx::query(statement)
            .execute(&mut **tx)
            .await
            .context("failed to restore archive relations")?;
    }
    for statement in &snapshot.related_updates {
        sqlx::query(statement)
            .execute(&mut **tx)
            .await
            .context("failed to restore archive references")?;
    }

    Ok(())
}

fn trash_path_for(original_path: &Path, entry_id: &str) -> Result<PathBuf> {
    let parent = original_path
        .parent()
        .ok_or_else(|| anyhow!("archive path has no parent directory"))?;
    let file_name = original_path
        .file_name()
        .ok_or_else(|| anyhow!("archive path has no file name"))?
        .to_string_lossy();
    Ok(parent
        .join(".otamoryx-trash")
        .join(format!("{entry_id}-{file_name}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup() -> (Pool<Sqlite>, std::path::PathBuf) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE archives (id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT, path TEXT NOT NULL, file_hash TEXT UNIQUE NOT NULL, file_size INTEGER NOT NULL, page_count INTEGER NOT NULL, created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL)").execute(&pool).await.unwrap();
        sqlx::query(
            "CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL, PRIMARY KEY (archive_id, tag_id), FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE, FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE)").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE categories (id TEXT PRIMARY KEY, name TEXT NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE category_archives (category_id TEXT NOT NULL, archive_id TEXT NOT NULL, PRIMARY KEY (category_id, archive_id), FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE, FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE reading_progress (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, archive_id TEXT NOT NULL, current_page INTEGER NOT NULL DEFAULT 1, total_pages INTEGER NOT NULL DEFAULT 0, progress_percentage REAL NOT NULL DEFAULT 0.0, last_read_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(user_id, archive_id), FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE collections (id TEXT PRIMARY KEY, cover_archive_id TEXT, FOREIGN KEY (cover_archive_id) REFERENCES archives(id) ON DELETE SET NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("CREATE TABLE trash_entries (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, archive_id TEXT NOT NULL, original_path TEXT NOT NULL, trash_path TEXT, reason TEXT, rule_version TEXT, rule_id TEXT, evaluation_id TEXT, model_confidence REAL, metadata_json TEXT NOT NULL, operation_id TEXT, operation_type TEXT, decision_key TEXT, status TEXT NOT NULL, deleted_at DATETIME NOT NULL, expires_at DATETIME, restored_at DATETIME, cleanup_attempts INTEGER NOT NULL DEFAULT 0, last_cleanup_attempt_at DATETIME, last_cleanup_error TEXT, expired_at DATETIME, restore_claimed_at DATETIME)").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE trash_operations (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, operation_type TEXT NOT NULL, group_key TEXT NOT NULL, keep_archive_id TEXT NOT NULL, idempotency_key TEXT NOT NULL, migration_snapshot_json TEXT NOT NULL DEFAULT '{}', status TEXT NOT NULL, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, expires_at DATETIME, restored_at DATETIME)").execute(&pool).await.unwrap();
        sqlx::query("CREATE TABLE trash_operation_members (id TEXT PRIMARY KEY, operation_id TEXT NOT NULL, archive_id TEXT NOT NULL, trash_entry_id TEXT NOT NULL, migration_snapshot_json TEXT NOT NULL DEFAULT '{}', sequence INTEGER NOT NULL DEFAULT 0, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(operation_id, archive_id), UNIQUE(operation_id, sequence), UNIQUE(trash_entry_id), FOREIGN KEY (operation_id) REFERENCES trash_operations(id) ON DELETE CASCADE, FOREIGN KEY (trash_entry_id) REFERENCES trash_entries(id) ON DELETE RESTRICT)").execute(&pool).await.unwrap();
        let temp_dir = std::env::temp_dir().join(format!("otamoryx-trash-test-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        (pool, temp_dir)
    }

    async fn insert_trash_entry(
        pool: &Pool<Sqlite>,
        id: &str,
        trash_path: Option<&Path>,
        status: &str,
        expires_at: &str,
    ) {
        sqlx::query(
            "INSERT INTO trash_entries
             (id, user_id, archive_id, original_path, trash_path, metadata_json, status,
              deleted_at, expires_at)
             VALUES (?, 'u1', ?, '/library/book.cbz', ?, '{}', ?, CURRENT_TIMESTAMP, ?)",
        )
        .bind(id)
        .bind(format!("archive-{id}"))
        .bind(trash_path.map(|path| path.to_string_lossy().to_string()))
        .bind(status)
        .bind(expires_at)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn seed_version_cleanup(pool: &Pool<Sqlite>, temp_dir: &Path) -> (PathBuf, PathBuf) {
        let keeper_path = temp_dir.join("keeper.cbz");
        let source_path = temp_dir.join("source.cbz");
        tokio::fs::write(&keeper_path, b"keeper").await.unwrap();
        tokio::fs::write(&source_path, b"source").await.unwrap();
        for (id, title, path, hash) in [
            ("keeper", "Keeper", &keeper_path, "hash-keeper"),
            ("source", "Source", &source_path, "hash-source"),
        ] {
            sqlx::query(
                "INSERT INTO archives
                 (id, title, path, file_hash, file_size, page_count, created_at, updated_at)
                 VALUES (?, ?, ?, ?, 4, 20, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            )
            .bind(id)
            .bind(title)
            .bind(path.to_string_lossy().as_ref())
            .bind(hash)
            .execute(pool)
            .await
            .unwrap();
        }
        for (id, name) in [("tag-keeper", "Keeper"), ("tag-source", "Source")] {
            sqlx::query("INSERT INTO tags (id, name, namespace) VALUES (?, ?, 'test')")
                .bind(id)
                .bind(name)
                .execute(pool)
                .await
                .unwrap();
        }
        for (id, name) in [("cat-keeper", "Keeper"), ("cat-source", "Source")] {
            sqlx::query("INSERT INTO categories (id, name) VALUES (?, ?)")
                .bind(id)
                .bind(name)
                .execute(pool)
                .await
                .unwrap();
        }
        sqlx::query("INSERT INTO archive_tags (archive_id, tag_id) VALUES ('keeper', 'tag-keeper'), ('source', 'tag-source')")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO category_archives (category_id, archive_id) VALUES ('cat-keeper', 'keeper'), ('cat-source', 'source')")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO reading_progress
             (id, user_id, archive_id, current_page, total_pages, progress_percentage, last_read_at, created_at, updated_at)
             VALUES
             ('keeper-progress', 'u1', 'keeper', 2, 20, 0.1, '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z'),
             ('source-progress', 'u1', 'source', 5, 10, 0.5, '2024-02-01T00:00:00Z', '2024-02-01T00:00:00Z', '2024-02-01T00:00:00Z')",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO trash_operations
             (id, user_id, operation_type, group_key, keep_archive_id, idempotency_key,
              migration_snapshot_json, status, expires_at)
             VALUES ('operation-1', 'u1', 'version_cleanup', 'group-1', 'keeper', 'key-1', '{}', 'pending', '2999-01-01T00:00:00Z')",
        )
        .execute(pool)
        .await
        .unwrap();
        (keeper_path, source_path)
    }

    async fn move_seeded_member(service: &TrashService) -> TrashEntry {
        service
            .move_version_group_member_to_trash("u1", "operation-1", "source", "keeper", 20)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn moves_archive_and_restores_snapshot() {
        let (pool, temp_dir) = setup().await;
        let path = temp_dir.join("book.cbz");
        tokio::fs::write(&path, b"book").await.unwrap();
        sqlx::query("INSERT INTO archives (id, title, path, file_hash, file_size, page_count, created_at, updated_at) VALUES ('a1', 'Book', ?, 'hash-a1', 4, 2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(path.to_string_lossy().as_ref()).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO categories (id, name) VALUES ('cat-1', 'Favorites')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO category_archives (category_id, archive_id) VALUES ('cat-1', 'a1')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO reading_progress (id, user_id, archive_id, current_page) VALUES ('progress-1', 'u1', 'a1', 7)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO collections (id, cover_archive_id) VALUES ('collection-1', 'a1')")
            .execute(&pool)
            .await
            .unwrap();

        let service = TrashService::new(pool.clone());
        let entry = service
            .move_archive_to_trash("u1", "a1", Some("manual"), "user")
            .await
            .unwrap();
        assert_eq!(entry.status, "active");
        assert!(!path.exists());
        assert!(entry
            .trash_path
            .as_deref()
            .is_some_and(|path| std::path::Path::new(path).exists()));
        assert!(sqlx::query("SELECT id FROM archives WHERE id = 'a1'")
            .fetch_optional(&pool)
            .await
            .unwrap()
            .is_none());

        service.restore_entry("u1", &entry.id).await.unwrap();
        assert!(path.exists());
        assert!(sqlx::query("SELECT id FROM archives WHERE id = 'a1'")
            .fetch_optional(&pool)
            .await
            .unwrap()
            .is_some());
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM category_archives WHERE category_id = 'cat-1' AND archive_id = 'a1'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT current_page FROM reading_progress WHERE id = 'progress-1'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            7
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT cover_archive_id FROM collections WHERE id = 'collection-1'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "a1"
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT status FROM trash_entries WHERE id = ?")
                .bind(&entry.id)
                .fetch_one(&pool)
                .await
                .unwrap(),
            "restored"
        );
        let restore_claim_released: i64 =
            sqlx::query_scalar("SELECT restore_claimed_at IS NULL FROM trash_entries WHERE id = ?")
                .bind(&entry.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(restore_claim_released, 1);
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn restores_version_cleanup_relations_as_one_operation() {
        let (pool, temp_dir) = setup().await;
        let (_keeper_path, source_path) = seed_version_cleanup(&pool, &temp_dir).await;
        let service = TrashService::new(pool.clone());
        let entry = move_seeded_member(&service).await;
        sqlx::query("UPDATE trash_operations SET status = 'active' WHERE id = 'operation-1'")
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT tag_id FROM archive_tags WHERE archive_id = 'keeper' ORDER BY tag_id",
            )
            .fetch_all(&pool)
            .await
            .unwrap(),
            vec!["tag-keeper".to_string(), "tag-source".to_string()]
        );
        assert_eq!(
            sqlx::query_as::<_, (i32, i32, f64)>(
                "SELECT current_page, total_pages, progress_percentage FROM reading_progress WHERE archive_id = 'keeper' AND user_id = 'u1'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            (10, 20, 0.5)
        );

        let restored = service
            .restore_operation("u1", "operation-1")
            .await
            .unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].id, entry.id);
        assert_eq!(restored[0].status, "restored");
        assert!(source_path.exists());
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT tag_id FROM archive_tags WHERE archive_id = 'keeper' ORDER BY tag_id",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "tag-keeper"
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT category_id FROM category_archives WHERE archive_id = 'keeper' ORDER BY category_id",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "cat-keeper"
        );
        assert_eq!(
            sqlx::query_as::<_, (String, i32, i32, f64)>(
                "SELECT id, current_page, total_pages, progress_percentage FROM reading_progress WHERE archive_id = 'keeper' AND user_id = 'u1'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            ("keeper-progress".to_string(), 2, 20, 0.1)
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM archives WHERE id = 'source'")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM reading_progress WHERE archive_id = 'source'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT status FROM trash_operations WHERE id = 'operation-1'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "restored"
        );
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn rolls_back_a_pending_version_cleanup_without_leaving_keeper_changes() {
        let (pool, temp_dir) = setup().await;
        let (_keeper_path, source_path) = seed_version_cleanup(&pool, &temp_dir).await;
        let service = TrashService::new(pool.clone());
        move_seeded_member(&service).await;

        service
            .rollback_version_cleanup("u1", "operation-1")
            .await
            .unwrap();

        assert!(source_path.exists());
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT tag_id FROM archive_tags WHERE archive_id = 'keeper' ORDER BY tag_id",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "tag-keeper"
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT status FROM trash_operations WHERE id = 'operation-1'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "failed"
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT status FROM trash_entries WHERE operation_id = 'operation-1'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "restored"
        );
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn rejects_single_entry_restore_for_version_cleanup() {
        let (pool, temp_dir) = setup().await;
        let (_keeper_path, _source_path) = seed_version_cleanup(&pool, &temp_dir).await;
        let service = TrashService::new(pool.clone());
        let entry = move_seeded_member(&service).await;

        let error = service.restore_entry("u1", &entry.id).await.unwrap_err();
        assert!(error
            .to_string()
            .contains("must be restored through their operation"));
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn rejects_version_cleanup_restore_after_keeper_relation_drift() {
        let (pool, temp_dir) = setup().await;
        let (_keeper_path, source_path) = seed_version_cleanup(&pool, &temp_dir).await;
        let service = TrashService::new(pool.clone());
        let entry = move_seeded_member(&service).await;
        sqlx::query("UPDATE trash_operations SET status = 'active' WHERE id = 'operation-1'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES ('tag-external', 'External', 'test')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO archive_tags (archive_id, tag_id) VALUES ('keeper', 'tag-external')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let error = service
            .restore_operation("u1", "operation-1")
            .await
            .unwrap_err();
        assert!(error.to_string().contains("changed since cleanup"));
        assert!(!source_path.exists());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM archives WHERE id = 'source'")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT status FROM trash_entries WHERE id = ?")
                .bind(&entry.id)
                .fetch_one(&pool)
                .await
                .unwrap(),
            "active"
        );
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT status FROM trash_operations WHERE id = 'operation-1'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "active"
        );
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn defers_expiration_while_a_restore_claim_is_fresh() {
        let (pool, temp_dir) = setup().await;
        let path = temp_dir.join("claimed.cbz");
        tokio::fs::write(&path, b"claimed").await.unwrap();
        insert_trash_entry(
            &pool,
            "claimed",
            Some(&path),
            "active",
            "2000-01-01T00:00:00Z",
        )
        .await;
        sqlx::query(
            "UPDATE trash_entries SET restore_claimed_at = CURRENT_TIMESTAMP WHERE id = 'claimed'",
        )
        .execute(&pool)
        .await
        .unwrap();

        let report = TrashService::new(pool.clone())
            .cleanup_expired_entries(100)
            .await
            .unwrap();
        assert_eq!(report, TrashCleanupReport::default());
        assert!(path.exists());
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn expires_due_entries_without_touching_future_or_restored_entries() {
        let (pool, temp_dir) = setup().await;
        let due_path = temp_dir.join("due.cbz");
        let future_path = temp_dir.join("future.cbz");
        let restored_path = temp_dir.join("restored.cbz");
        tokio::fs::write(&due_path, b"due").await.unwrap();
        tokio::fs::write(&future_path, b"future").await.unwrap();
        tokio::fs::write(&restored_path, b"restored").await.unwrap();
        insert_trash_entry(
            &pool,
            "due",
            Some(&due_path),
            "active",
            "2000-01-01T00:00:00Z",
        )
        .await;
        insert_trash_entry(
            &pool,
            "future",
            Some(&future_path),
            "active",
            "2999-01-01T00:00:00Z",
        )
        .await;
        insert_trash_entry(
            &pool,
            "restored",
            Some(&restored_path),
            "restored",
            "2000-01-01T00:00:00Z",
        )
        .await;

        let report = TrashService::new(pool.clone())
            .cleanup_expired_entries(100)
            .await
            .unwrap();
        assert_eq!(
            report,
            TrashCleanupReport {
                claimed: 1,
                deleted_files: 1,
                missing_files: 0,
                failed: 0,
            }
        );
        assert!(!due_path.exists());
        assert!(future_path.exists());
        assert!(restored_path.exists());

        let due = sqlx::query_as::<_, (String, i64, i64)>(
            "SELECT status, cleanup_attempts, expired_at IS NOT NULL
             FROM trash_entries WHERE id = 'due'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(due, ("expired".to_string(), 1, 1));
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn marks_missing_expired_files_complete_idempotently() {
        let (pool, temp_dir) = setup().await;
        let missing_path = temp_dir.join("missing.cbz");
        insert_trash_entry(
            &pool,
            "missing",
            Some(&missing_path),
            "active",
            "2000-01-01T00:00:00Z",
        )
        .await;

        let service = TrashService::new(pool.clone());
        let first = service.cleanup_expired_entries(100).await.unwrap();
        assert_eq!(first.missing_files, 1);
        assert_eq!(first.failed, 0);
        let second = service.cleanup_expired_entries(100).await.unwrap();
        assert_eq!(second, TrashCleanupReport::default());

        let complete: i64 = sqlx::query_scalar(
            "SELECT expired_at IS NOT NULL FROM trash_entries WHERE id = 'missing'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(complete, 1);
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn retries_failed_file_deletion_after_the_file_becomes_removable() {
        let (pool, temp_dir) = setup().await;
        let blocked_path = temp_dir.join("blocked.cbz");
        tokio::fs::create_dir(&blocked_path).await.unwrap();
        insert_trash_entry(
            &pool,
            "blocked",
            Some(&blocked_path),
            "active",
            "2000-01-01T00:00:00Z",
        )
        .await;

        let service = TrashService::new(pool.clone());
        let failed = service.cleanup_expired_entries(100).await.unwrap();
        assert_eq!(failed.claimed, 1);
        assert_eq!(failed.failed, 1);
        let state = sqlx::query_as::<_, (String, i64, Option<String>, i64)>(
            "SELECT status, cleanup_attempts, last_cleanup_error, expired_at IS NOT NULL
             FROM trash_entries WHERE id = 'blocked'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(state.0, "expired");
        assert_eq!(state.1, 1);
        assert!(state.2.is_some());
        assert_eq!(state.3, 0);

        tokio::fs::remove_dir(&blocked_path).await.unwrap();
        tokio::fs::write(&blocked_path, b"retry").await.unwrap();
        sqlx::query(
            "UPDATE trash_entries
             SET last_cleanup_attempt_at = datetime('now', '-10 minutes') WHERE id = 'blocked'",
        )
        .execute(&pool)
        .await
        .unwrap();

        let retried = service.cleanup_expired_entries(100).await.unwrap();
        assert_eq!(retried.claimed, 1);
        assert_eq!(retried.deleted_files, 1);
        assert_eq!(retried.failed, 0);
        assert!(!blocked_path.exists());
        let state = sqlx::query_as::<_, (i64, Option<String>, i64)>(
            "SELECT cleanup_attempts, last_cleanup_error, expired_at IS NOT NULL
             FROM trash_entries WHERE id = 'blocked'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(state.0, 2);
        assert_eq!(state.1, None);
        assert_eq!(state.2, 1);
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn retries_cleanup_after_final_database_update_fails() {
        let (pool, temp_dir) = setup().await;
        let path = temp_dir.join("finalization.cbz");
        tokio::fs::write(&path, b"finalization").await.unwrap();
        insert_trash_entry(
            &pool,
            "finalization",
            Some(&path),
            "active",
            "2000-01-01T00:00:00Z",
        )
        .await;
        sqlx::query(
            "CREATE TRIGGER fail_trash_finalization
             BEFORE UPDATE OF expired_at ON trash_entries
             WHEN NEW.expired_at IS NOT NULL
             BEGIN SELECT RAISE(ABORT, 'forced finalization failure'); END",
        )
        .execute(&pool)
        .await
        .unwrap();

        let service = TrashService::new(pool.clone());
        let failed = service.cleanup_expired_entries(100).await.unwrap();
        assert_eq!(failed.deleted_files, 1);
        assert_eq!(failed.failed, 1);
        assert!(!path.exists());
        let pending = sqlx::query_as::<_, (i64, Option<String>)>(
            "SELECT expired_at IS NOT NULL, last_cleanup_error
             FROM trash_entries WHERE id = 'finalization'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(pending.0, 0);
        assert!(pending.1.is_some());

        sqlx::query("DROP TRIGGER fail_trash_finalization")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE trash_entries
             SET last_cleanup_attempt_at = datetime('now', '-10 minutes')
             WHERE id = 'finalization'",
        )
        .execute(&pool)
        .await
        .unwrap();
        let retried = service.cleanup_expired_entries(100).await.unwrap();
        assert_eq!(retried.claimed, 1);
        assert_eq!(retried.missing_files, 1);
        assert_eq!(retried.failed, 0);

        let completed: i64 = sqlx::query_scalar(
            "SELECT expired_at IS NOT NULL FROM trash_entries WHERE id = 'finalization'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(completed, 1);
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }
}
