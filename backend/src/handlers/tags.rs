use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;

use crate::middleware::auth::AuthInfo;
use crate::models::TagModel;
use crate::services::{
    is_system_managed_theme_namespace, ArchiveCacheService, ArchiveDeleteTarget,
    ArchiveDeletionService,
};

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub namespace: String,
}

pub struct TagHandler;

impl TagHandler {
    /// POST /api/v1/tags - 创建标签（如已存在则返回现有记录）
    pub async fn create_tag(
        State(pool): State<Pool<Sqlite>>,
        Json(req): Json<CreateTagRequest>,
    ) -> Result<Json<TagModel>, StatusCode> {
        if is_system_managed_theme_namespace(&req.namespace) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query!(
            "INSERT OR IGNORE INTO tags (id, name, namespace) VALUES (?, ?, ?)",
            id,
            req.name,
            req.namespace
        )
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let tag = sqlx::query_as::<_, TagModel>(
            "SELECT t.id, t.name, t.namespace, l.name AS localized_name \
             FROM tags t LEFT JOIN tag_localizations l \
             ON l.tag_id = t.id AND l.locale = 'zh-Hans' AND l.status = 'completed' \
             WHERE t.name = ? AND t.namespace = ?",
        )
        .bind(&req.name)
        .bind(&req.namespace)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Err(error) = crate::services::enqueue_tag_localization(&pool, &tag.id).await {
            tracing::warn!(tag_id = %tag.id, error = %error, "failed to queue tag localization");
        }

        Ok(Json(tag))
    }

    /// GET /api/v1/tags - 获取标签列表
    pub async fn list_tags(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<Vec<TagModel>>, StatusCode> {
        let tags = sqlx::query_as::<_, TagModel>(
            "SELECT t.id, t.name, t.namespace, l.name AS localized_name \
             FROM tags t LEFT JOIN tag_localizations l \
             ON l.tag_id = t.id AND l.locale = 'zh-Hans' AND l.status = 'completed' \
             ORDER BY t.namespace, t.name",
        )
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(tags))
    }

    /// DELETE /api/v1/tags/:id/archives/batch-delete - 批量删除标签下的漫画
    pub async fn batch_delete_tag_archives(
        State(pool): State<Pool<Sqlite>>,
        Path(tag_id): Path<String>,
        axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
        axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
    ) -> Result<StatusCode, StatusCode> {
        // Canonical themes have their own content-analysis relation and cannot be deleted through
        // the ordinary archive-tag batch workflow.
        let tag_namespace =
            sqlx::query_scalar::<_, String>("SELECT namespace FROM tags WHERE id = ?")
                .bind(&tag_id)
                .fetch_optional(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let Some(tag_namespace) = tag_namespace else {
            tracing::debug!(
                "Tag {} not found during batch delete, treating as no-op",
                tag_id
            );
            return Ok(StatusCode::OK);
        };
        if is_system_managed_theme_namespace(&tag_namespace) {
            return Err(StatusCode::BAD_REQUEST);
        }

        // 获取该标签关联的所有存档ID和文件路径
        let archive_rows = sqlx::query!(
            "SELECT a.id, a.path
             FROM archives a
             INNER JOIN archive_tags at ON a.id = at.archive_id
             WHERE at.tag_id = ?",
            tag_id
        )
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let targets = archive_rows
            .into_iter()
            .filter_map(|row| row.id.map(|id| ArchiveDeleteTarget { id, path: row.path }))
            .collect();
        let summary = ArchiveDeletionService::new(pool, archive_cache)
            .delete_targets(
                &auth.user_id,
                targets,
                "user initiated tag batch deletion",
                "tag_batch_delete",
            )
            .await
            .map_err(|e| {
                tracing::error!("Tag {} batch deletion failed: {}", tag_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if summary.failed > 0 {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        Ok(StatusCode::OK)
    }

    /// DELETE /api/v1/tags/prune - 清理未使用的标签
    pub async fn prune_unused_tags(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<StatusCode, StatusCode> {
        // 删除没有关联任何存档的标签
        sqlx::query(
            r#"
            DELETE FROM tags 
            WHERE id NOT IN (
                SELECT DISTINCT tag_id FROM archive_tags
            )
            AND name != 'new'  -- 保护"new"系统标签
            AND lower(trim(namespace)) != 'theme'  -- canonical themes are referenced outside archive_tags
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(StatusCode::OK)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::archive::ArchiveCacheConfig;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_tags_schema(pool: &Pool<Sqlite>) {
        sqlx::query(
            r#"
            CREATE TABLE tags (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                namespace TEXT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("create tags");

        sqlx::query(
            r#"
            CREATE TABLE archives (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("create archives");

        sqlx::query(
            r#"
            CREATE TABLE archive_tags (
                archive_id TEXT NOT NULL,
                tag_id TEXT NOT NULL,
                PRIMARY KEY (archive_id, tag_id)
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("create archive_tags");
    }

    #[tokio::test]
    async fn pruning_unused_tags_preserves_canonical_themes() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite pool");
        setup_tags_schema(&pool).await;
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES
             ('new-tag', 'new', 'system'),
             ('unused-tag', 'unused', 'general'),
             ('theme-tag', 'sample theme', 'theme'),
             ('used-tag', 'used', 'general')",
        )
        .execute(&pool)
        .await
        .expect("insert test tags");
        sqlx::query("INSERT INTO archives (id, path) VALUES ('archive-1', '/tmp/archive.cbz')")
            .execute(&pool)
            .await
            .expect("insert test archive");
        sqlx::query(
            "INSERT INTO archive_tags (archive_id, tag_id) VALUES ('archive-1', 'used-tag')",
        )
        .execute(&pool)
        .await
        .expect("associate used tag");

        TagHandler::prune_unused_tags(State(pool.clone()))
            .await
            .expect("prune should succeed");

        let remaining: Vec<(String, String)> =
            sqlx::query_as("SELECT name, namespace FROM tags ORDER BY id")
                .fetch_all(&pool)
                .await
                .expect("read remaining tags");
        assert_eq!(
            remaining,
            vec![
                ("new".to_string(), "system".to_string()),
                ("sample theme".to_string(), "theme".to_string()),
                ("used".to_string(), "general".to_string()),
            ]
        );
    }

    #[tokio::test]
    async fn create_tag_rejects_system_managed_theme_namespace() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite pool");
        setup_tags_schema(&pool).await;

        let result = TagHandler::create_tag(
            State(pool.clone()),
            Json(CreateTagRequest {
                name: "Space Opera".to_string(),
                namespace: " THEME ".to_string(),
            }),
        )
        .await;

        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags")
            .fetch_one(&pool)
            .await
            .expect("count tags");
        assert_eq!(count, 0);
    }

    fn test_cache_service() -> Arc<ArchiveCacheService> {
        Arc::new(ArchiveCacheService::new(ArchiveCacheConfig::default()))
    }

    fn test_auth_info() -> AuthInfo {
        AuthInfo {
            user_id: "user-1".to_string(),
            role: "admin".to_string(),
        }
    }

    #[tokio::test]
    async fn batch_delete_tag_archives_is_noop_for_missing_tag() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite memory pool");
        setup_tags_schema(&pool).await;

        let status = TagHandler::batch_delete_tag_archives(
            State(pool),
            Path("missing-tag".to_string()),
            axum::extract::Extension(test_auth_info()),
            axum::extract::Extension(test_cache_service()),
        )
        .await
        .expect("missing tag should be a no-op");

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn batch_delete_rejects_system_managed_theme_tags() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite memory pool");
        setup_tags_schema(&pool).await;
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES ('theme-tag', 'Sample theme', 'theme')",
        )
        .execute(&pool)
        .await
        .expect("insert theme tag");
        sqlx::query("INSERT INTO archives (id, path) VALUES ('archive-1', '/tmp/archive.cbz')")
            .execute(&pool)
            .await
            .expect("insert archive");
        sqlx::query(
            "INSERT INTO archive_tags (archive_id, tag_id) VALUES ('archive-1', 'theme-tag')",
        )
        .execute(&pool)
        .await
        .expect("insert legacy theme relation");

        let result = TagHandler::batch_delete_tag_archives(
            State(pool.clone()),
            Path("theme-tag".to_string()),
            axum::extract::Extension(test_auth_info()),
            axum::extract::Extension(test_cache_service()),
        )
        .await;

        assert_eq!(result, Err(StatusCode::BAD_REQUEST));
        let archive_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM archives")
            .fetch_one(&pool)
            .await
            .expect("count archives");
        assert_eq!(archive_count, 1);
    }
}

// 独立函数用于路由注册
pub async fn create_tag(
    State(pool): State<Pool<Sqlite>>,
    Json(req): Json<CreateTagRequest>,
) -> Result<Json<TagModel>, StatusCode> {
    TagHandler::create_tag(State(pool), Json(req)).await
}

pub async fn list_tags(
    State(pool): State<Pool<Sqlite>>,
) -> Result<Json<Vec<TagModel>>, StatusCode> {
    TagHandler::list_tags(State(pool)).await
}

pub async fn batch_delete_tag_archives(
    State(pool): State<Pool<Sqlite>>,
    Path(tag_id): Path<String>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
) -> Result<StatusCode, StatusCode> {
    TagHandler::batch_delete_tag_archives(
        State(pool),
        Path(tag_id),
        axum::extract::Extension(auth),
        axum::extract::Extension(archive_cache),
    )
    .await
}

pub async fn prune_unused_tags(State(pool): State<Pool<Sqlite>>) -> Result<StatusCode, StatusCode> {
    TagHandler::prune_unused_tags(State(pool)).await
}
