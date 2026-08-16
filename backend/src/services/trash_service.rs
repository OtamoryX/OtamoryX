use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
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
    source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TagSnapshot {
    id: String,
    name: String,
    namespace: String,
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
        let mut snapshot = self.load_snapshot(archive_id).await?;
        snapshot.source = Some(source.to_string());
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
        tokio::fs::rename(&original_path, &trash_path)
            .await
            .with_context(|| {
                format!(
                    "failed to move archive {} to trash",
                    original_path.display()
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
                 (id, user_id, archive_id, original_path, trash_path, reason, metadata_json, status, deleted_at, expires_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, 'active', CURRENT_TIMESTAMP, datetime('now', '+14 days'))",
            )
            .bind(&entry_id)
            .bind(user_id)
            .bind(archive_id)
            .bind(&snapshot.path)
            .bind(trash_path.to_string_lossy().as_ref())
            .bind(reason)
            .bind(&metadata_json)
            .execute(&mut *tx)
            .await
            .context("failed to create trash entry")?;
            tx.commit().await.context("failed to commit trash transaction")?;
            Ok::<(), anyhow::Error>(())
        }
        .await;

        if let Err(error) = result {
            if let Err(rollback_error) = tokio::fs::rename(&trash_path, &original_path).await {
                tracing::error!(
                    "failed to restore archive {} after trash transaction error: {}",
                    archive_id,
                    rollback_error
                );
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
            rule_version: None,
            model_confidence: None,
            metadata_json,
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

    pub async fn restore_entry(&self, user_id: &str, entry_id: &str) -> Result<TrashEntry> {
        let entry = sqlx::query_as::<_, TrashEntry>(
            "SELECT id, user_id, archive_id, original_path, trash_path, reason, rule_version,
                    model_confidence, metadata_json, status, deleted_at, expires_at, restored_at,
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
            let mut tx = self.pool.begin().await.context("failed to start restore transaction")?;
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
            .execute(&mut *tx)
            .await
            .context("failed to restore archive record")?;

            for tag in &snapshot.tags {
                sqlx::query("INSERT OR IGNORE INTO tags (id, name, namespace) VALUES (?, ?, ?)")
                    .bind(&tag.id)
                    .bind(&tag.name)
                    .bind(&tag.namespace)
                    .execute(&mut *tx)
                    .await
                    .context("failed to restore archive tag")?;
                sqlx::query("INSERT OR IGNORE INTO archive_tags (archive_id, tag_id) VALUES (?, ?)")
                    .bind(&snapshot.id)
                    .bind(&tag.id)
                    .execute(&mut *tx)
                    .await
                    .context("failed to restore archive tag relation")?;
            }

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
            tx.commit().await.context("failed to commit restore transaction")?;
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

    pub async fn list_entries(
        &self,
        user_id: &str,
        status: Option<&str>,
        limit: u32,
    ) -> Result<Vec<TrashEntry>> {
        let limit = limit.clamp(1, 200) as i64;
        let mut query = String::from(
            "SELECT id, user_id, archive_id, original_path, trash_path, reason, rule_version,
                    model_confidence, metadata_json, status, deleted_at, expires_at, restored_at,
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
            source: None,
        })
    }
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
        sqlx::query("CREATE TABLE trash_entries (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, archive_id TEXT NOT NULL, original_path TEXT NOT NULL, trash_path TEXT, reason TEXT, rule_version TEXT, model_confidence REAL, metadata_json TEXT NOT NULL, status TEXT NOT NULL, deleted_at DATETIME NOT NULL, expires_at DATETIME, restored_at DATETIME, cleanup_attempts INTEGER NOT NULL DEFAULT 0, last_cleanup_attempt_at DATETIME, last_cleanup_error TEXT, expired_at DATETIME, restore_claimed_at DATETIME)").execute(&pool).await.unwrap();
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

    #[tokio::test]
    async fn moves_archive_and_restores_snapshot() {
        let (pool, temp_dir) = setup().await;
        let path = temp_dir.join("book.cbz");
        tokio::fs::write(&path, b"book").await.unwrap();
        sqlx::query("INSERT INTO archives (id, title, path, file_hash, file_size, page_count, created_at, updated_at) VALUES ('a1', 'Book', ?, 'hash-a1', 4, 2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(path.to_string_lossy().as_ref()).execute(&pool).await.unwrap();

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
