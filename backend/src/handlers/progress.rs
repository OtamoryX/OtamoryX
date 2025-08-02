use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::{Pool, Sqlite};
use crate::models::{ReadingProgress, UpdateProgressRequest};

pub async fn get_progress(
    State(pool): State<Pool<Sqlite>>,
    Path(archive_id): Path<String>,
    axum::extract::Extension(user_id): axum::extract::Extension<String>,
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
            last_read_at: chrono::DateTime::from_naive_utc_and_offset(progress_row.last_read_at, chrono::Utc),
        };
        Ok(Json(progress))
    } else {
        // 如果没有进度记录，返回默认进度
        let progress = ReadingProgress {
            id: 0,
            archive_id: archive_id.clone(),
            user_id,
            current_page: 1,
            total_pages: 0, // 从档案信息中获取
            progress_percentage: 0.0,
            last_read_at: chrono::Utc::now(),
        };
        Ok(Json(progress))
    }
}

pub async fn update_progress(
    State(pool): State<Pool<Sqlite>>,
    Path(archive_id): Path<String>,
    Json(request): Json<UpdateProgressRequest>,
    axum::extract::Extension(user_id): axum::extract::Extension<String>,
) -> Result<StatusCode, StatusCode> {

    // 获取档案的总页数
    let archive_info = sqlx::query!(
        "SELECT page_count FROM archives WHERE id = ?",
        archive_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting archive info: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total_pages = archive_info
        .map(|info| info.page_count as i32)
        .unwrap_or(0);

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
    let new_tag_id = sqlx::query!(
        "SELECT id FROM tags WHERE name = 'new' AND namespace = 'system'"
    )
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