use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
use std::path::{Path, PathBuf};
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
        })
    }

    pub async fn restore_entry(&self, user_id: &str, entry_id: &str) -> Result<TrashEntry> {
        let entry = sqlx::query_as::<_, TrashEntry>(
            "SELECT id, user_id, archive_id, original_path, trash_path, reason, rule_version,
                    model_confidence, metadata_json, status, deleted_at, expires_at, restored_at
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
        let trash_path = entry
            .trash_path
            .as_deref()
            .ok_or_else(|| anyhow!("trash entry has no file path"))?;
        let original_path = Path::new(&entry.original_path);
        if tokio::fs::try_exists(original_path).await? {
            return Err(anyhow!("original archive path already exists"));
        }
        if !tokio::fs::try_exists(trash_path).await? {
            return Err(anyhow!("trash file is missing"));
        }
        if let Some(parent) = original_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("failed to create archive directory")?;
        }

        let snapshot: ArchiveSnapshot = serde_json::from_str(&entry.metadata_json)
            .context("failed to decode archive snapshot")?;
        tokio::fs::rename(trash_path, original_path)
            .await
            .context("failed to restore archive file")?;

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

            sqlx::query(
                "UPDATE trash_entries SET status = 'restored', restored_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?",
            )
            .bind(entry_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .context("failed to mark trash entry restored")?;
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
                    model_confidence, metadata_json, status, deleted_at, expires_at, restored_at
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
        sqlx::query("CREATE TABLE trash_entries (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, archive_id TEXT NOT NULL, original_path TEXT NOT NULL, trash_path TEXT, reason TEXT, rule_version TEXT, model_confidence REAL, metadata_json TEXT NOT NULL, status TEXT NOT NULL, deleted_at DATETIME NOT NULL, expires_at DATETIME, restored_at DATETIME)").execute(&pool).await.unwrap();
        let temp_dir = std::env::temp_dir().join(format!("otamoryx-trash-test-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&temp_dir).await.unwrap();
        (pool, temp_dir)
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
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }
}
