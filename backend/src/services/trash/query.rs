use super::TrashService;
use crate::models::TrashEntry;
use anyhow::{Context, Result};

impl TrashService {
    pub(super) async fn load_entry_by_operation_member(
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

    pub(super) async fn load_entry_by_decision_key(
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

    pub(super) async fn load_active_entry_by_archive(
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
}
