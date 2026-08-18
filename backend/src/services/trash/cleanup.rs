use super::{TrashCleanupCandidate, TrashCleanupReport, TrashFileCleanupResult, TrashService};
use anyhow::{anyhow, Context, Result};
use sqlx::Row;

impl TrashService {
    pub async fn purge_entry(&self, user_id: &str, entry_id: &str) -> Result<()> {
        let entry = sqlx::query(
            "SELECT status, operation_id, operation_type, trash_path
             FROM trash_entries WHERE id = ? AND user_id = ?",
        )
        .bind(entry_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load trash entry for permanent deletion")?
        .ok_or_else(|| anyhow!("trash entry not found"))?;
        let status: String = entry.get("status");
        if status != "active" {
            return Err(anyhow!("trash entry is not active"));
        }
        if entry.get::<Option<String>, _>("operation_type").is_some()
            || entry.get::<Option<String>, _>("operation_id").is_some()
        {
            return Err(anyhow!(
                "version cleanup entries must be permanently deleted through their operation"
            ));
        }

        let claimed = sqlx::query(
            "UPDATE trash_entries
             SET status = 'expired', cleanup_attempts = cleanup_attempts + 1,
                 last_cleanup_attempt_at = CURRENT_TIMESTAMP, last_cleanup_error = NULL
             WHERE id = ? AND user_id = ? AND status = 'active'
               AND (restore_claimed_at IS NULL
                    OR julianday(restore_claimed_at) <= julianday('now', '-5 minutes'))",
        )
        .bind(entry_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .context("failed to claim trash entry for permanent deletion")?;
        if claimed.rows_affected() == 0 {
            return Err(anyhow!("trash entry is no longer active"));
        }

        let trash_path = entry.get::<Option<String>, _>("trash_path");
        self.finish_permanent_delete(entry_id, trash_path.as_deref())
            .await
    }

    /// Permanently remove every active member of a version-cleanup operation.
    pub async fn purge_operation(&self, user_id: &str, operation_id: &str) -> Result<()> {
        let claimed = sqlx::query(
            "UPDATE trash_operations SET status = 'purging'
             WHERE id = ? AND user_id = ? AND operation_type = 'version_cleanup'
               AND status IN ('active', 'failed')",
        )
        .bind(operation_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .context("failed to claim trash operation for permanent deletion")?;
        if claimed.rows_affected() == 0 {
            return Err(anyhow!("trash operation is not active"));
        }

        let entries = sqlx::query_as::<_, (String, Option<String>)>(
            "SELECT id, trash_path FROM trash_entries
             WHERE user_id = ? AND operation_id = ? AND status = 'active'
             ORDER BY id",
        )
        .bind(user_id)
        .bind(operation_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to list trash operation members")?;

        for (entry_id, trash_path) in entries {
            let entry_claimed = sqlx::query(
                "UPDATE trash_entries
                 SET status = 'expired', cleanup_attempts = cleanup_attempts + 1,
                     last_cleanup_attempt_at = CURRENT_TIMESTAMP, last_cleanup_error = NULL
                 WHERE id = ? AND user_id = ? AND operation_id = ? AND status = 'active'",
            )
            .bind(&entry_id)
            .bind(user_id)
            .bind(operation_id)
            .execute(&self.pool)
            .await?;
            if entry_claimed.rows_affected() == 0 {
                continue;
            }
            if let Err(error) = self
                .finish_permanent_delete(&entry_id, trash_path.as_deref())
                .await
            {
                let _ = sqlx::query(
                    "UPDATE trash_operations SET status = 'active' WHERE id = ? AND user_id = ?",
                )
                .bind(operation_id)
                .bind(user_id)
                .execute(&self.pool)
                .await;
                return Err(error);
            }
        }

        sqlx::query(
            "UPDATE trash_operations SET status = 'expired' WHERE id = ? AND user_id = ? AND status = 'purging'",
        )
        .bind(operation_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .context("failed to finalize trash operation permanent deletion")?;
        Ok(())
    }

    async fn finish_permanent_delete(
        &self,
        entry_id: &str,
        trash_path: Option<&str>,
    ) -> Result<()> {
        if let Err(error) = self.remove_trash_file(trash_path).await {
            sqlx::query(
                "UPDATE trash_entries SET status = 'active', last_cleanup_error = ?
                 WHERE id = ? AND status = 'expired' AND expired_at IS NULL",
            )
            .bind(error.to_string())
            .bind(entry_id)
            .execute(&self.pool)
            .await
            .context("failed to restore trash entry after permanent deletion failure")?;
            return Err(error);
        }
        sqlx::query(
            "UPDATE trash_entries SET expired_at = CURRENT_TIMESTAMP, last_cleanup_error = NULL
             WHERE id = ? AND status = 'expired' AND expired_at IS NULL",
        )
        .bind(entry_id)
        .execute(&self.pool)
        .await
        .context("failed to finalize trash entry permanent deletion")?;
        Ok(())
    }

    pub async fn cleanup_expired_entries(&self, limit: u32) -> Result<TrashCleanupReport> {
        let limit = limit.clamp(1, super::TRASH_CLEANUP_BATCH_SIZE) as i64;
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
}
