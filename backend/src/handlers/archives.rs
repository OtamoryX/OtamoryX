use crate::middleware::path_permission;
use crate::models::{Archive, PaginatedResponse, TagModel};
use crate::services::{ArchiveCacheConfig, ArchiveCacheService};
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

// 全局缓存服务实例
use lazy_static::lazy_static;

lazy_static! {
    pub static ref ARCHIVE_CACHE: Arc<ArchiveCacheService> = {
        let config = ArchiveCacheConfig::default();
        Arc::new(ArchiveCacheService::new(config))
    };
}

#[derive(Deserialize)]
pub struct ArchiveQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

pub async fn get_archives(
    State(pool): State<Pool<Sqlite>>,
    Query(params): Query<ArchiveQuery>,
    axum::extract::Extension(user_id): axum::extract::Extension<String>,
) -> Result<Json<PaginatedResponse<Archive>>, StatusCode> {
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;

    // 获取总数
    let total_row = sqlx::query!("SELECT COUNT(*) as count FROM archives")
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error getting archive count: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let total = total_row.count as u32;

    // 获取分页数据
    let rows = sqlx::query!(
        "SELECT id, title, path, file_hash, file_size, page_count, created_at, updated_at 
         FROM archives 
         ORDER BY created_at DESC 
         LIMIT ? OFFSET ?",
        limit,
        offset
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting archives: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut archives = Vec::new();
    for row in rows {
        // 检查用户是否有访问此路径的权限
        if !path_permission::has_path_permission(&pool, &user_id, &row.path).await? {
            continue; // 跳过用户无权访问的档案
        }

        // 获取每个档案的标签
        let tag_rows = sqlx::query!(
            "SELECT t.id, t.name, t.namespace 
             FROM tags t 
             INNER JOIN archive_tags at ON t.id = at.tag_id 
             WHERE at.archive_id = ?",
            row.id
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

        archives.push(Archive {
            id: row.id.unwrap_or_default(),
            title: row.title,
            path: row.path,
            file_size: row.file_size,
            page_count: row.page_count as i32,
            hash: row.file_hash,
            created_at: chrono::DateTime::from_naive_utc_and_offset(row.created_at, chrono::Utc),
            updated_at: chrono::DateTime::from_naive_utc_and_offset(row.updated_at, chrono::Utc),
            tags,
        });
    }

    let has_next = (page * limit) < total;

    Ok(Json(PaginatedResponse {
        data: archives,
        page,
        limit,
        total,
        has_next,
    }))
}

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

    // 使用全局缓存服务获取页面
    match ARCHIVE_CACHE.get_page(&id, &archive_path, page).await {
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

//TODO remove or move to services
async fn create_placeholder_image(
    archive_id: &str,
    _page: u32,
) -> Result<Response<Body>, StatusCode> {
    use image::{ImageBuffer, ImageFormat, Rgba};
    use std::io::Cursor;

    let width = 400u32;
    let height = 600u32;

    // 创建图片缓冲区
    let mut img = ImageBuffer::new(width, height);

    // 根据漫画ID生成不同的背景色
    let color_seed = archive_id.chars().map(|c| c as u32).sum::<u32>();
    let r = ((color_seed * 37) % 200 + 55) as u8;
    let g = ((color_seed * 73) % 200 + 55) as u8;
    let b = ((color_seed * 131) % 200 + 55) as u8;

    // 填充背景色
    for pixel in img.pixels_mut() {
        *pixel = Rgba([r, g, b, 255]);
    }

    // 将图片编码为PNG
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);

    img.write_to(&mut cursor, ImageFormat::Png)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CACHE_CONTROL, "public, max-age=60") // 较短的缓存时间
        .body(Body::from(buffer))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response)
}

// 获取漫画封面缩略图（公开接口）
pub async fn get_archive_thumbnail_public(
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
    let cover_path = get_cover_file_path(&archive_path);

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

            match generate_cover_file_for_archive(&archive_path).await {
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
                            create_placeholder_thumbnail(&id).await
                        }
                    }
                }
                Err(e) => {
                    // 生成失败，使用占位符
                    tracing::warn!("Failed to generate cover file for {}: {}", archive_path, e);
                    create_placeholder_thumbnail(&id).await
                }
            }
        }
    }
}

// 获取封面文件路径
fn get_cover_file_path(archive_path: &str) -> std::path::PathBuf {
    use std::path::Path;

    let path = Path::new(archive_path);
    if let Some(parent) = path.parent() {
        parent.join("cover.jpg")
    } else {
        // 如果无法获取父目录，在同级目录创建
        path.with_file_name("cover.jpg")
    }
}

// 为存档生成封面文件
async fn generate_cover_file_for_archive(
    archive_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::utils::ArchiveExtractor;
    use image::{load_from_memory, GenericImageView, ImageFormat};
    use std::{fs, path::Path};

    let archive_path_buf = Path::new(archive_path);
    let cover_path = get_cover_file_path(archive_path);

    // 如果封面文件已存在，跳过生成
    if cover_path.exists() {
        return Ok(());
    }

    // 提取存档的第一页
    let extractor = ArchiveExtractor::new();
    let files = extractor.extract_files(archive_path_buf)?;

    let image_files = extractor.get_image_files(files);

    if image_files.is_empty() {
        return Err("No image files found in archive".into());
    }

    // 按文件名排序并获取第一个
    let mut sorted_files = image_files;
    sorted_files.sort_by(|a, b| natord::compare(&a.name, &b.name));

    let first_image = &sorted_files[0];

    // 解码图片
    let img = load_from_memory(&first_image.data)?;

    // 计算缩略图尺寸（保持宽高比）
    let (original_width, original_height) = img.dimensions();
    let target_width = 150u32;
    let target_height = 200u32;

    // 计算缩放比例
    let width_ratio = target_width as f32 / original_width as f32;
    let height_ratio = target_height as f32 / original_height as f32;
    let scale = width_ratio.min(height_ratio);

    let new_width = (original_width as f32 * scale) as u32;
    let new_height = (original_height as f32 * scale) as u32;

    // 调整图片大小
    let resized = img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3);

    // 确保目录存在
    if let Some(parent) = cover_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // 保存为JPEG文件
    resized.save_with_format(&cover_path, ImageFormat::Jpeg)?;

    tracing::info!("Generated cover file: {}", cover_path.display());
    Ok(())
}

// 创建占位符缩略图
async fn create_placeholder_thumbnail(archive_id: &str) -> Result<Response<Body>, StatusCode> {
    use image::{ImageBuffer, ImageFormat, Rgba};
    use std::io::Cursor;

    let width = 150u32;
    let height = 200u32;

    // 创建图片缓冲区
    let mut img = ImageBuffer::new(width, height);

    // 根据漫画ID生成不同的背景色
    let color_seed = archive_id.chars().map(|c| c as u32).sum::<u32>();
    let r = ((color_seed * 37) % 200 + 55) as u8;
    let g = ((color_seed * 73) % 200 + 55) as u8;
    let b = ((color_seed * 131) % 200 + 55) as u8;

    // 填充背景色
    for pixel in img.pixels_mut() {
        *pixel = Rgba([r, g, b, 255]);
    }

    // 将图片编码为JPEG
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);

    img.write_to(&mut cursor, ImageFormat::Jpeg)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/jpeg")
        .header(header::CACHE_CONTROL, "public, max-age=60") // 较短的缓存时间
        .body(Body::from(buffer))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response)
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

    let placeholders = request
        .archive_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let query = format!("DELETE FROM archives WHERE id IN ({})", placeholders);

    let mut sqlx_query = sqlx::query(&query);
    for archive_id in request.archive_ids {
        sqlx_query = sqlx_query.bind(archive_id);
    }

    let result = sqlx_query
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!("Batch deleted {} archives", result.rows_affected());

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
