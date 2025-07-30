use axum::{
    extract::Path,
    http::StatusCode,
    Json,
};
use crate::models::{ReadingProgress, UpdateProgressRequest};

pub async fn get_progress(
    Path(archive_id): Path<String>,
) -> Result<Json<ReadingProgress>, StatusCode> {
    // TODO: 从数据库获取阅读进度
    let progress = ReadingProgress {
        id: 1,
        archive_id: archive_id.clone(),
        user_id: "user1".to_string(),
        current_page: 5,
        total_pages: 20,
        progress_percentage: 0.25,
        last_read_at: chrono::Utc::now(),
    };

    Ok(Json(progress))
}

pub async fn update_progress(
    Path(archive_id): Path<String>,
    Json(request): Json<UpdateProgressRequest>,
) -> Result<StatusCode, StatusCode> {
    // TODO: 更新数据库中的阅读进度
    tracing::info!(
        "Updating progress for archive {}: page {}",
        archive_id,
        request.current_page
    );
    
    Ok(StatusCode::OK)
}