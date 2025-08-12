use crate::models::{
    BatchProgressRequest, BatchProgressResponse, ReadingProgress, UpdateProgressRequest,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::{Pool, Row, Sqlite};
use std::collections::HashMap;

pub async fn get_progress(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(user_id): axum::extract::Extension<String>,
    Path(archive_id): Path<String>,
) -> Result<Json<ReadingProgress>, StatusCode> {
    let row = sqlx::query!(
        "SELECT id, user_id, archive_id, current_page, total_pages, progress_percentage, last_read_at 
         FROM reading_progress 
         WHERE archive_id = ? AND user_id = ?",
        archive_id,
        user_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting reading progress: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some(progress_row) = row {
        let progress = ReadingProgress {
            id: progress_row.id.unwrap_or_default().parse().unwrap_or(0),
            archive_id: progress_row.archive_id,
            user_id: progress_row.user_id,
            current_page: progress_row.current_page as i32,
            total_pages: progress_row.total_pages as i32,
            progress_percentage: progress_row.progress_percentage,
            last_read_at: chrono::DateTime::from_naive_utc_and_offset(
                progress_row.last_read_at,
                chrono::Utc,
            ),
        };
        Ok(Json(progress))
    } else {
        // 如果没有进度记录，返回默认进度
        let progress = ReadingProgress {
            id: 0,
            archive_id: archive_id.clone(),
            user_id,
            current_page: 1,
            total_pages: 0,
            progress_percentage: 0.0,
            last_read_at: chrono::Utc::now(),
        };
        Ok(Json(progress))
    }
}

pub async fn update_progress(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(user_id): axum::extract::Extension<String>,
    Path(archive_id): Path<String>,
    Json(request): Json<UpdateProgressRequest>,
) -> Result<StatusCode, StatusCode> {
    // 获取档案的总页数
    let archive_info = sqlx::query!("SELECT page_count FROM archives WHERE id = ?", archive_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error getting archive info: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let total_pages = archive_info.map(|info| info.page_count as i32).unwrap_or(0);

    // 计算进度百分比
    let progress_percentage = if total_pages > 0 {
        (request.current_page as f64) / (total_pages as f64)
    } else {
        0.0
    };

    let progress_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    // 插入或更新阅读进度
    sqlx::query!(
        r#"
        INSERT INTO reading_progress (id, user_id, archive_id, current_page, total_pages, progress_percentage, last_read_at, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(user_id, archive_id) DO UPDATE SET
            current_page = excluded.current_page,
            total_pages = excluded.total_pages,
            progress_percentage = excluded.progress_percentage,
            last_read_at = excluded.last_read_at,
            updated_at = excluded.updated_at
        "#,
        progress_id,
        user_id,
        archive_id,
        request.current_page,
        total_pages,
        progress_percentage,
        now,
        now,
        now
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error updating reading progress: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // 如果阅读超过第1页，自动移除"new"标签
    if request.current_page > 1 {
        let _ = remove_new_tag(&pool, &archive_id).await;
    }

    tracing::info!(
        "Updated progress for archive {}: page {}/{} ({}%)",
        archive_id,
        request.current_page,
        total_pages,
        (progress_percentage * 100.0) as i32
    );

    Ok(StatusCode::OK)
}

async fn remove_new_tag(pool: &Pool<Sqlite>, archive_id: &str) -> Result<(), sqlx::Error> {
    let new_tag_id =
        sqlx::query!("SELECT id FROM tags WHERE name = 'new' AND namespace = 'system'")
            .fetch_optional(pool)
            .await?;

    if let Some(tag) = new_tag_id {
        sqlx::query!(
            "DELETE FROM archive_tags WHERE archive_id = ? AND tag_id = ?",
            archive_id,
            tag.id
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn get_batch_progress(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(user_id): axum::extract::Extension<String>,
    Json(request): Json<BatchProgressRequest>,
) -> Result<Json<BatchProgressResponse>, StatusCode> {
    if request.archive_ids.is_empty() {
        return Ok(Json(BatchProgressResponse {
            progress: HashMap::new(),
        }));
    }

    // 限制批量请求的数量以防止性能问题
    let archive_ids = if request.archive_ids.len() > 100 {
        &request.archive_ids[..100]
    } else {
        &request.archive_ids
    };

    // 构建IN查询的占位符
    let placeholders = archive_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let query = format!(
        "SELECT rp.id, rp.user_id, rp.archive_id, rp.current_page, rp.total_pages, 
                rp.progress_percentage, rp.last_read_at
         FROM reading_progress rp
         WHERE rp.archive_id IN ({}) AND rp.user_id = ?",
        placeholders
    );

    let mut query_builder = sqlx::query(&query);

    // 添加archive_ids参数
    for archive_id in archive_ids {
        query_builder = query_builder.bind(archive_id);
    }

    // 添加user_id参数
    query_builder = query_builder.bind(&user_id);

    let rows = query_builder.fetch_all(&pool).await.map_err(|e| {
        tracing::error!("Database error getting batch reading progress: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut progress_map = HashMap::new();

    // 处理已存在的进度记录
    for row in rows {
        let archive_id: String = row.get("archive_id");
        let progress = ReadingProgress {
            id: row
                .get::<Option<String>, _>("id")
                .unwrap_or_default()
                .parse()
                .unwrap_or(0),
            archive_id: archive_id.clone(),
            user_id: row.get("user_id"),
            current_page: row.get::<i64, _>("current_page") as i32,
            total_pages: row.get::<i64, _>("total_pages") as i32,
            progress_percentage: row.get("progress_percentage"),
            last_read_at: chrono::DateTime::from_naive_utc_and_offset(
                row.get("last_read_at"),
                chrono::Utc,
            ),
        };
        progress_map.insert(archive_id, progress);
    }

    // 为没有进度记录的档案创建默认进度，并获取档案的实际页数
    for archive_id in archive_ids {
        if !progress_map.contains_key(archive_id) {
            // 获取档案的总页数
            let archive_info =
                sqlx::query!("SELECT page_count FROM archives WHERE id = ?", archive_id)
                    .fetch_optional(&pool)
                    .await
                    .map_err(|e| {
                        tracing::error!(
                            "Database error getting archive info for {}: {}",
                            archive_id,
                            e
                        );
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

            let total_pages = archive_info.map(|info| info.page_count as i32).unwrap_or(0);

            let progress = ReadingProgress {
                id: 0,
                archive_id: archive_id.clone(),
                user_id: user_id.clone(),
                current_page: 1,
                total_pages,
                progress_percentage: 0.0,
                last_read_at: chrono::Utc::now(),
            };
            progress_map.insert(archive_id.clone(), progress);
        }
    }

    tracing::info!(
        "Retrieved batch progress for {} archives for user {}",
        progress_map.len(),
        user_id
    );

    Ok(Json(BatchProgressResponse {
        progress: progress_map,
    }))
}
