use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;

use crate::models::{AITagDecision, ReviewAction, TagModel};
use crate::services::{delete_archive_file, ArchiveCacheService};

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
            "SELECT id, name, namespace FROM tags WHERE name = ? AND namespace = ?",
        )
        .bind(&req.name)
        .bind(&req.namespace)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(tag))
    }

    /// GET /api/v1/tags - 获取标签列表
    pub async fn list_tags(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<Vec<TagModel>>, StatusCode> {
        let tags = sqlx::query_as::<_, TagModel>(
            "SELECT id, name, namespace FROM tags ORDER BY namespace, name",
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
        axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
    ) -> Result<StatusCode, StatusCode> {
        // 验证标签存在
        let _tag = sqlx::query!("SELECT id FROM tags WHERE id = ?", tag_id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

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

        if archive_rows.is_empty() {
            return Ok(StatusCode::OK);
        }

        for row in archive_rows {
            let archive_id = row.id.unwrap_or_default();
            if archive_id.is_empty() {
                continue;
            }

            let mut tx = pool
                .begin()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let archive_id_for_query = archive_id.clone();
            let result = sqlx::query!("DELETE FROM archives WHERE id = ?", archive_id_for_query)
                .execute(&mut *tx)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            if result.rows_affected() > 0 {
                if let Err(e) = delete_archive_file(&row.path).await {
                    tracing::error!(
                        "Failed to delete archive file {} for {}, rolling back transaction: {}",
                        row.path,
                        archive_id,
                        e
                    );
                    if let Err(rollback_err) = tx.rollback().await {
                        tracing::error!(
                            "Failed to rollback transaction after file delete error: {}",
                            rollback_err
                        );
                    }
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
                tx.commit()
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                archive_cache.clear_archive_cache(&archive_id).await;
            } else {
                let _ = tx.rollback().await;
            }
        }

        Ok(StatusCode::OK)
    }

    /// DELETE /api/v1/tags/prune - 清理未使用的标签
    pub async fn prune_unused_tags(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<StatusCode, StatusCode> {
        // 删除没有关联任何存档的标签
        sqlx::query!(
            r#"
            DELETE FROM tags 
            WHERE id NOT IN (
                SELECT DISTINCT tag_id FROM archive_tags
            )
            AND name != 'new'  -- 保护"new"系统标签
            "#
        )
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(StatusCode::OK)
    }

    /// POST /api/v1/ai/tags/review - 批量审核AI标签
    pub async fn review_ai_tags(
        State(pool): State<Pool<Sqlite>>,
        Json(reviews): Json<Vec<AITagDecision>>,
    ) -> Result<StatusCode, StatusCode> {
        let mut tx = pool
            .begin()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        for decision in reviews {
            match decision.action {
                ReviewAction::Approve => {
                    // 批准AI标签，添加到正式标签关联
                    let ai_tag = sqlx::query!(
                        "SELECT archive_id, tag_id FROM ai_generated_tags WHERE id = ?",
                        decision.tag_id
                    )
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                    .ok_or(StatusCode::NOT_FOUND)?;

                    // 添加到正式标签关联
                    sqlx::query!(
                        "INSERT OR IGNORE INTO archive_tags (archive_id, tag_id) VALUES (?, ?)",
                        ai_tag.archive_id,
                        ai_tag.tag_id
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    // 更新AI标签状态
                    sqlx::query!(
                        "UPDATE ai_generated_tags SET approved = TRUE, reviewed_at = CURRENT_TIMESTAMP WHERE id = ?",
                        decision.tag_id
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                }
                ReviewAction::Reject => {
                    // 拒绝AI标签
                    sqlx::query!(
                        "UPDATE ai_generated_tags SET approved = FALSE, reviewed_at = CURRENT_TIMESTAMP WHERE id = ?",
                        decision.tag_id
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                }
                ReviewAction::Edit => {
                    if let Some(edited_name) = decision.edited_name {
                        // 编辑并批准AI标签
                        let ai_tag = sqlx::query!(
                            "SELECT archive_id, tag_id FROM ai_generated_tags WHERE id = ?",
                            decision.tag_id
                        )
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                        .ok_or(StatusCode::NOT_FOUND)?;

                        // 创建或获取编辑后的标签
                        let tag_id = uuid::Uuid::new_v4().to_string();
                        sqlx::query!(
                            "INSERT OR IGNORE INTO tags (id, name, namespace) VALUES (?, ?, 'general')",
                            tag_id,
                            edited_name
                        )
                        .execute(&mut *tx)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                        // 获取实际的标签ID（如果已存在）
                        let actual_tag = sqlx::query!(
                            "SELECT id FROM tags WHERE name = ? AND namespace = 'general'",
                            edited_name
                        )
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                        // 添加到正式标签关联
                        sqlx::query!(
                            "INSERT OR IGNORE INTO archive_tags (archive_id, tag_id) VALUES (?, ?)",
                            ai_tag.archive_id,
                            actual_tag.id
                        )
                        .execute(&mut *tx)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                        // 更新AI标签状态
                        sqlx::query!(
                            "UPDATE ai_generated_tags SET approved = TRUE, reviewed_at = CURRENT_TIMESTAMP WHERE id = ?",
                            decision.tag_id
                        )
                        .execute(&mut *tx)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    }
                }
            }
        }

        tx.commit()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(StatusCode::OK)
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
    axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
) -> Result<StatusCode, StatusCode> {
    TagHandler::batch_delete_tag_archives(State(pool), Path(tag_id), axum::extract::Extension(archive_cache)).await
}

pub async fn prune_unused_tags(State(pool): State<Pool<Sqlite>>) -> Result<StatusCode, StatusCode> {
    TagHandler::prune_unused_tags(State(pool)).await
}

pub async fn review_ai_tags(
    State(pool): State<Pool<Sqlite>>,
    Json(decisions): Json<Vec<AITagDecision>>,
) -> Result<StatusCode, StatusCode> {
    TagHandler::review_ai_tags(State(pool), Json(decisions)).await
}
