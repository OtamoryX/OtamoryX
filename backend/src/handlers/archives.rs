use crate::middleware::path_permission;
use crate::models::{Archive, TagModel};
use crate::services::{ArchiveCacheService, ArchiveService};
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use serde::Deserialize;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;

// 缓存服务现在通过扩展传递，不再需要全局静态变量

pub async fn get_archive(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
    axum::extract::Extension(user_id): axum::extract::Extension<String>,
) -> Result<Json<Archive>, StatusCode> {
    // 从数据库获取档案信息
    let row = sqlx::query!(
        "SELECT id, title, path, file_hash, file_size, page_count, created_at, updated_at 
         FROM archives 
         WHERE id = ?",
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting archive: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let archive_data = row.ok_or(StatusCode::NOT_FOUND)?;

    // 检查用户是否有访问此路径的权限
    if !path_permission::has_path_permission(&pool, &user_id, &archive_data.path).await? {
        tracing::warn!(
            "User {} denied access to path {}",
            user_id,
            archive_data.path
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // 获取档案的标签
    let tag_rows = sqlx::query!(
        "SELECT t.id, t.name, t.namespace 
         FROM tags t 
         INNER JOIN archive_tags at ON t.id = at.tag_id 
         WHERE at.archive_id = ?",
        id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting archive tags: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let tags = tag_rows
        .into_iter()
        .map(|tag| TagModel {
            id: tag.id.unwrap_or_default(),
            name: tag.name,
            namespace: tag.namespace,
        })
        .collect();

    let archive = Archive {
        id: archive_data.id.unwrap_or_default(),
        title: archive_data.title,
        path: archive_data.path,
        file_size: archive_data.file_size,
        page_count: archive_data.page_count as i32,
        hash: archive_data.file_hash,
        created_at: chrono::DateTime::from_naive_utc_and_offset(
            archive_data.created_at,
            chrono::Utc,
        ),
        updated_at: chrono::DateTime::from_naive_utc_and_offset(
            archive_data.updated_at,
            chrono::Utc,
        ),
        tags,
    };

    Ok(Json(archive))
}

pub async fn get_archive_page(
    State(pool): State<Pool<Sqlite>>,
    Path((id, page)): Path<(String, u32)>,
    axum::extract::Extension(user_id): axum::extract::Extension<String>,
    axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
) -> Result<Response<Body>, StatusCode> {
    tracing::info!("Requesting page {} of archive {}", page, id);

    // 首先从数据库获取存档信息
    let archive_info = sqlx::query!("SELECT path FROM archives WHERE id = ?", id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let archive_path = match archive_info {
        Some(info) => info.path,
        None => {
            tracing::warn!("Archive {} not found", id);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // 检查用户是否有访问此路径的权限
    if !path_permission::has_path_permission(&pool, &user_id, &archive_path).await? {
        tracing::warn!("User {} denied access to path {}", user_id, archive_path);
        return Err(StatusCode::FORBIDDEN);
    }

    // 使用缓存服务获取页面
    match archive_cache.get_page(&id, &archive_path, page).await {
        Ok(cached_page) => {
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, cached_page.content_type)
                .header(header::CACHE_CONTROL, "public, max-age=3600")
                .header(header::ETAG, format!("\"{}\"", id))
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

// 获取漫画封面缩略图
pub async fn get_archive_thumbnail(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
) -> Result<Response<Body>, StatusCode> {
    tracing::info!("Requesting thumbnail for archive {}", id);

    // 从数据库获取存档路径
    let result = sqlx::query_as::<_, (String,)>("SELECT path FROM archives WHERE id = ?")
        .bind(&id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let archive_path = match result {
        Some((path,)) => path,
        None => {
            tracing::warn!("Archive {} not found", id);
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // 尝试读取封面文件
    let cover_path = ArchiveService::get_cover_file_path(&archive_path);

    match tokio::fs::read(&cover_path).await {
        Ok(cover_data) => {
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "image/jpeg")
                .header(header::CACHE_CONTROL, "public, max-age=86400") // 24小时缓存
                .body(Body::from(cover_data))
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            Ok(response)
        }
        Err(_) => {
            // 如果没有封面文件，先尝试重新生成
            tracing::debug!(
                "Cover file not found: {}, attempting to generate",
                cover_path.display()
            );

            match ArchiveService::generate_cover_file_for_archive(&archive_path).await {
                Ok(_) => {
                    // 重新尝试读取生成的封面文件
                    match tokio::fs::read(&cover_path).await {
                        Ok(cover_data) => {
                            let response = Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "image/jpeg")
                                .header(header::CACHE_CONTROL, "public, max-age=86400") // 24小时缓存
                                .body(Body::from(cover_data))
                                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                            Ok(response)
                        }
                        Err(_) => {
                            // 生成后仍然无法读取，使用占位符
                            tracing::warn!(
                                "Failed to read generated cover file: {}",
                                cover_path.display()
                            );
                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "image/jpeg")
                                .header(header::CACHE_CONTROL, "public, max-age=60") // 1分钟缓存
                                .body(Body::from(
                                    ArchiveService::create_placeholder_thumbnail(&id)
                                        .await
                                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                                ))
                                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
                        }
                    }
                }
                Err(e) => {
                    // 生成失败，使用占位符
                    tracing::warn!("Failed to generate cover file for {}: {}", archive_path, e);
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "image/jpeg")
                        .header(header::CACHE_CONTROL, "public, max-age=60") // 1分钟缓存
                        .body(Body::from(
                            ArchiveService::create_placeholder_thumbnail(&id)
                                .await
                                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                        ))
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
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
    pub archive_ids: Vec<String>,
}

pub async fn batch_delete_archives(
    State(pool): State<Pool<Sqlite>>,
    Json(request): Json<BatchDeleteRequest>,
) -> Result<StatusCode, StatusCode> {
    if request.archive_ids.is_empty() {
        return Ok(StatusCode::OK);
    }

    // 实现批量删除逻辑:
    // 1. 验证所有存档存在
    // 2. 删除存档记录（级联删除会处理关联表）
    // 3. 文件清理在此版本中不实现，保留原始文件

    // 使用事务批量删除
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut deleted_count = 0;

    for archive_id in &request.archive_ids {
        match sqlx::query!("DELETE FROM archives WHERE id = ?", archive_id)
            .execute(&mut *tx)
            .await
        {
            Ok(result) => deleted_count += result.rows_affected(),
            Err(e) => {
                tracing::error!("Failed to delete archive {}: {}", archive_id, e);
            }
        }
    }

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!("Batch deleted {} archives", deleted_count);

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
    axum::extract::Extension(user_id): axum::extract::Extension<String>,
    Json(request): Json<AddTagRequest>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!("Adding tag {} to archive {}", request.tag_id, archive_id);

    // 验证存档存在并检查权限
    let archive_info = sqlx::query!("SELECT path FROM archives WHERE id = ?", archive_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // 检查用户权限
    if !path_permission::has_path_permission(&pool, &user_id, &archive_info.path)
        .await
        .unwrap_or(false)
    {
        tracing::warn!("User {} denied access to archive {}", user_id, archive_id);
        return Err(StatusCode::FORBIDDEN);
    }

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
    axum::extract::Extension(user_id): axum::extract::Extension<String>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!("Removing tag {} from archive {}", tag_id, archive_id);

    // 验证存档存在并检查权限
    let archive_info = sqlx::query!("SELECT path FROM archives WHERE id = ?", archive_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // 检查用户权限
    if !path_permission::has_path_permission(&pool, &user_id, &archive_info.path)
        .await
        .unwrap_or(false)
    {
        tracing::warn!("User {} denied access to archive {}", user_id, archive_id);
        return Err(StatusCode::FORBIDDEN);
    }

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
        tracing::warn!("Tag {} not found on archive {}", tag_id, archive_id);
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::OK)
}

/// DELETE /api/v1/archives/:id - 删除单个档案
pub async fn delete_archive(
    State(pool): State<Pool<Sqlite>>,
    Path(archive_id): Path<String>,
    axum::extract::Extension(user_id): axum::extract::Extension<String>,
) -> Result<StatusCode, StatusCode> {
    // 检查档案是否存在
    let archive = sqlx::query!("SELECT path FROM archives WHERE id = ?", archive_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error fetching archive {}: {}", archive_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("Archive not found: {}", archive_id);
            StatusCode::NOT_FOUND
        })?;

    // 检查用户权限
    if !path_permission::has_path_permission(&pool, &user_id, &archive.path)
        .await
        .unwrap_or(false)
    {
        tracing::warn!(
            "User {} denied access to delete archive {}",
            user_id,
            archive_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // 删除档案（级联删除会处理所有关联表）
    let result = sqlx::query!("DELETE FROM archives WHERE id = ?", archive_id)
        .execute(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error deleting archive {}: {}", archive_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    tracing::info!("Deleted archive: {}", archive_id);
    Ok(StatusCode::NO_CONTENT)
}
