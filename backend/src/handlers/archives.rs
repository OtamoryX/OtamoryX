use crate::middleware::auth::AuthInfo;
use crate::middleware::path_permission;
use crate::models::{Archive, TagModel};
use crate::services::{
    ArchiveCacheService, ArchiveDeleteTarget, ArchiveDeletionService, ArchiveService,
    CurationService, TrashService,
};
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use serde::Deserialize;
use sqlx::{Pool, Row, Sqlite};
use std::sync::Arc;

// 缓存服务现在通过扩展传递，不再需要全局静态变量

pub async fn get_archive(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
) -> Result<Json<Archive>, StatusCode> {
    // 从数据库获取档案信息
    let row = sqlx::query(
        "SELECT id, title, subtitle, subtitle_language, path, file_hash, file_size, page_count, created_at, updated_at
         FROM archives
         WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting archive: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let archive_data = row.ok_or(StatusCode::NOT_FOUND)?;

    // 检查用户是否有访问此路径的权限
    let archive_path: String = archive_data.get("path");
    if !path_permission::has_path_permission(&pool, &auth, &archive_path).await? {
        tracing::warn!(
            "User {} denied access to path {}",
            auth.user_id,
            archive_path
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // 获取档案的标签
    let tag_rows = sqlx::query(
        "SELECT t.id, t.name, t.namespace 
         FROM tags t 
         INNER JOIN archive_tags at ON t.id = at.tag_id 
         WHERE at.archive_id = ?",
    )
    .bind(&id)
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting archive tags: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let tags = tag_rows
        .into_iter()
        .map(|tag| TagModel {
            id: tag.get("id"),
            name: tag.get("name"),
            namespace: tag.get("namespace"),
        })
        .collect();

    let archive = Archive {
        id: archive_data.get("id"),
        title: archive_data.get("title"),
        subtitle: archive_data.get("subtitle"),
        subtitle_language: archive_data.get("subtitle_language"),
        path: archive_path,
        file_size: archive_data.get("file_size"),
        page_count: archive_data.get("page_count"),
        hash: archive_data.get("file_hash"),
        created_at: archive_data.get("created_at"),
        updated_at: archive_data.get("updated_at"),
        tags,
    };

    Ok(Json(archive))
}

pub async fn get_archive_page(
    State(pool): State<Pool<Sqlite>>,
    Path((id, page)): Path<(String, u32)>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
) -> Result<Response<Body>, StatusCode> {
    tracing::info!("Requesting page {} of archive {}", page, id);

    if page == 0 {
        tracing::warn!("Invalid page number 0 requested for archive {}", id);
        return Err(StatusCode::BAD_REQUEST);
    }

    let archive_path = path_permission::authorize_archive_access(&pool, &auth, &id).await?;

    // 使用缓存服务获取页面
    match archive_cache.get_page(&id, &archive_path, page).await {
        Ok(cached_page) => {
            let mut response_builder = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, cached_page.content_type)
                .header(header::ETAG, format!("\"{}\"", id));

            // 只有当数据不为空时才设置缓存
            if !cached_page.data.is_empty() {
                response_builder =
                    response_builder.header(header::CACHE_CONTROL, "public, max-age=3600");
            } else {
                // 数据为空时，不设置缓存或设置不缓存
                response_builder = response_builder
                    .header(header::CACHE_CONTROL, "no-cache, no-store, must-revalidate");
                tracing::warn!(
                    "Page {} of archive {} returned empty data, not caching",
                    page,
                    id
                );
            }

            let response = response_builder
                .body(Body::from(cached_page.data))
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            Ok(response)
        }
        Err(e) => {
            tracing::error!("Failed to get page {} from archive {}: {}", page, id, e);

            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 构建图片响应的辅助函数，消除重复的响应构建代码
fn build_image_response(
    data: Vec<u8>,
    content_type: &str,
    max_age: u32,
) -> Result<Response<Body>, StatusCode> {
    let cache_control = if !data.is_empty() {
        format!("public, max-age={}", max_age)
    } else {
        "no-cache, no-store, must-revalidate".to_string()
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, cache_control)
        .body(Body::from(data))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// 获取漫画封面缩略图
pub async fn get_archive_thumbnail(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
) -> Result<Response<Body>, StatusCode> {
    tracing::info!("Requesting thumbnail for archive {}", id);

    let archive_path = path_permission::authorize_archive_access(&pool, &auth, &id).await?;

    // 尝试读取封面文件（统一从 cache/covers 读取）
    let cache_path = ArchiveService::get_image_cache_path(&pool).await;
    let cover_path = ArchiveService::get_cover_file_path(&cache_path, &id);

    match tokio::fs::read(&cover_path).await {
        Ok(cover_data) => {
            if cover_data.is_empty() {
                tracing::warn!("Cover for archive {} is empty, not caching", id);
            }
            build_image_response(cover_data, "image/jpeg", 86400)
        }
        Err(_) => {
            // 如果没有封面文件，先尝试重新生成
            tracing::debug!(
                "Cover file not found: {}, attempting to generate",
                cover_path.display()
            );
            let cover_quality = ArchiveService::get_cover_quality(&pool).await;

            match ArchiveService::generate_cover_file_for_archive(
                &archive_path,
                cover_path.clone(),
                cover_quality,
            )
            .await
            {
                Ok(_) => {
                    // 重新尝试读取生成的封面文件
                    match tokio::fs::read(&cover_path).await {
                        Ok(cover_data) => {
                            if cover_data.is_empty() {
                                tracing::warn!(
                                    "Generated cover for archive {} is empty, not caching",
                                    id
                                );
                            }
                            build_image_response(cover_data, "image/jpeg", 86400)
                        }
                        Err(_) => {
                            // 生成后仍然无法读取，使用占位符
                            tracing::warn!(
                                "Failed to read generated cover file: {}",
                                cover_path.display()
                            );
                            let placeholder_data =
                                ArchiveService::create_placeholder_thumbnail(&id)
                                    .await
                                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                            if placeholder_data.is_empty() {
                                tracing::warn!(
                                    "Placeholder thumbnail for archive {} is empty, not caching",
                                    id
                                );
                            }
                            build_image_response(placeholder_data, "image/jpeg", 60)
                        }
                    }
                }
                Err(e) => {
                    // 生成失败，使用占位符
                    tracing::warn!("Failed to generate cover file for {}: {}", archive_path, e);
                    let placeholder_data = ArchiveService::create_placeholder_thumbnail(&id)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                    if placeholder_data.is_empty() {
                        tracing::warn!(
                            "Fallback placeholder thumbnail for archive {} is empty, not caching",
                            id
                        );
                    }
                    build_image_response(placeholder_data, "image/jpeg", 60)
                }
            }
        }
    }
}

/// GET /api/v1/archives/random - 获取随机漫画
pub async fn get_random_archives(
    State(pool): State<Pool<Sqlite>>,
    Query(params): Query<crate::services::RandomArchiveParams>,
) -> Result<Json<Vec<Archive>>, StatusCode> {
    let random_service = crate::services::RandomService::new(pool);

    match random_service.get_random_archives(params).await {
        Ok(archives) => Ok(Json(archives)),
        Err(e) => {
            tracing::error!("Failed to get random archives: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v1/archives/random/by-tag/:tag - 根据标签获取随机漫画
pub async fn get_random_archive_by_tag(
    State(pool): State<Pool<Sqlite>>,
    Path(tag): Path<String>,
) -> Result<Json<Option<Archive>>, StatusCode> {
    let random_service = crate::services::RandomService::new(pool);

    match random_service.get_random_archive_by_tag(&tag).await {
        Ok(archive) => Ok(Json(archive)),
        Err(e) => {
            tracing::error!("Failed to get random archive by tag: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v1/archives/random/unread - 获取随机未读漫画
#[derive(Deserialize)]
pub struct UnreadRandomQuery {
    pub count: Option<u32>,
}

pub async fn get_unread_random_archives(
    State(pool): State<Pool<Sqlite>>,
    Query(params): Query<UnreadRandomQuery>,
) -> Result<Json<Vec<Archive>>, StatusCode> {
    let random_service = crate::services::RandomService::new(pool);

    match random_service
        .get_unread_random_archives(params.count)
        .await
    {
        Ok(archives) => Ok(Json(archives)),
        Err(e) => {
            tracing::error!("Failed to get unread random archives: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v1/archives/random/by-date - 根据日期范围获取随机漫画
#[derive(Deserialize)]
pub struct DateRangeRandomQuery {
    pub start_date: String,
    pub end_date: String,
    pub count: Option<u32>,
}

pub async fn get_random_archives_by_date(
    State(pool): State<Pool<Sqlite>>,
    Query(params): Query<DateRangeRandomQuery>,
) -> Result<Json<Vec<Archive>>, StatusCode> {
    let random_service = crate::services::RandomService::new(pool);

    match random_service
        .get_random_archives_by_date_range(&params.start_date, &params.end_date, params.count)
        .await
    {
        Ok(archives) => Ok(Json(archives)),
        Err(e) => {
            tracing::error!("Failed to get random archives by date: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /api/v1/archives/random/min-pages/:pages - 获取至少指定页数的随机漫画
pub async fn get_random_archives_with_min_pages(
    State(pool): State<Pool<Sqlite>>,
    Path(min_pages): Path<i32>,
    Query(params): Query<UnreadRandomQuery>,
) -> Result<Json<Vec<Archive>>, StatusCode> {
    let random_service = crate::services::RandomService::new(pool);

    match random_service
        .get_random_archives_with_minimum_pages(min_pages, params.count)
        .await
    {
        Ok(archives) => Ok(Json(archives)),
        Err(e) => {
            tracing::error!("Failed to get random archives with min pages: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// DELETE /api/v1/archives/batch-delete - 批量删除漫画
#[derive(Deserialize)]
pub struct BatchDeleteRequest {
    #[serde(alias = "archiveIds")]
    pub archive_ids: Vec<String>,
}

pub async fn batch_delete_archives(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
    Json(request): Json<BatchDeleteRequest>,
) -> Result<StatusCode, StatusCode> {
    if request.archive_ids.is_empty() {
        return Ok(StatusCode::OK);
    }

    let mut targets = Vec::new();

    for archive_id in &request.archive_ids {
        let archive = sqlx::query!("SELECT path FROM archives WHERE id = ?", archive_id)
            .fetch_optional(&pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to query archive {}: {}", archive_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if let Some(archive) = archive {
            targets.push(ArchiveDeleteTarget {
                id: archive_id.clone(),
                path: archive.path,
            });
        }
    }

    let summary = ArchiveDeletionService::new(pool, archive_cache)
        .delete_targets(
            &auth.user_id,
            targets,
            "user initiated batch archive deletion",
            "archive_batch_delete",
        )
        .await
        .map_err(|e| {
            tracing::error!("Batch archive deletion failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!("Batch deleted {} archives", summary.deleted);

    if summary.failed > 0 {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct AddTagRequest {
    pub tag_id: String,
}

/// POST /api/v1/archives/:id/tags - 为存档添加标签
pub async fn add_tag_to_archive(
    State(pool): State<Pool<Sqlite>>,
    Path(archive_id): Path<String>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    Json(request): Json<AddTagRequest>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!("Adding tag {} to archive {}", request.tag_id, archive_id);

    let _archive_path =
        path_permission::authorize_archive_access(&pool, &auth, &archive_id).await?;

    // 验证标签存在
    let _tag = sqlx::query!("SELECT id FROM tags WHERE id = ?", request.tag_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // 添加标签关联（如果不存在）
    sqlx::query!(
        "INSERT OR IGNORE INTO archive_tags (archive_id, tag_id) VALUES (?, ?)",
        archive_id,
        request.tag_id
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error adding tag to archive: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(StatusCode::OK)
}

/// DELETE /api/v1/archives/:id/tags/:tag_id - 从存档移除标签
pub async fn remove_tag_from_archive(
    State(pool): State<Pool<Sqlite>>,
    Path((archive_id, tag_id)): Path<(String, String)>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!("Removing tag {} from archive {}", tag_id, archive_id);

    let _archive_path =
        path_permission::authorize_archive_access(&pool, &auth, &archive_id).await?;

    // 移除标签关联
    let result = sqlx::query!(
        "DELETE FROM archive_tags WHERE archive_id = ? AND tag_id = ?",
        archive_id,
        tag_id
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error removing tag from archive: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if result.rows_affected() == 0 {
        tracing::debug!("Tag {} already removed from archive {}", tag_id, archive_id);
    }

    Ok(StatusCode::OK)
}

/// DELETE /api/v1/archives/:id - 删除单个档案
pub async fn delete_archive(
    State(pool): State<Pool<Sqlite>>,
    Path(archive_id): Path<String>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
) -> Result<StatusCode, StatusCode> {
    path_permission::authorize_archive_access(&pool, &auth, &archive_id).await?;

    TrashService::new(pool.clone())
        .move_archive_to_trash(
            &auth.user_id,
            &archive_id,
            Some("user initiated archive deletion"),
            "user",
        )
        .await
        .map_err(|error| {
            tracing::error!("Failed to move archive {} to trash: {}", archive_id, error);
            let message = error.to_string();
            if message.contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    archive_cache.clear_archive_cache(&archive_id).await;

    let behavior_request = crate::models::RecordBehaviorEventRequest {
        archive_id: Some(archive_id.clone()),
        event_type: "manual_delete".to_string(),
        event_key: None,
        page: None,
        metadata: serde_json::json!({ "source": "archive_delete" }),
        occurred_at: Some(chrono::Utc::now()),
    };
    if let Err(error) = CurationService::new(pool.clone())
        .record_event(&auth.user_id, &behavior_request)
        .await
    {
        tracing::warn!(
            "Failed to record delete behavior for archive {}: {}",
            archive_id,
            error
        );
    }
    if let Err(error) = CurationService::new(pool.clone())
        .record_disposition(
            &auth.user_id,
            &archive_id,
            "manual_delete",
            Some("user initiated archive deletion"),
            "user",
        )
        .await
    {
        tracing::warn!(
            "Failed to record delete disposition for archive {}: {}",
            archive_id,
            error
        );
    }

    tracing::info!("Deleted archive: {}", archive_id);
    Ok(StatusCode::NO_CONTENT)
}
