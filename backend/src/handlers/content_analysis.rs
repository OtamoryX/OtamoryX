use crate::middleware::{auth::AuthInfo, path_permission};
use crate::services::ContentAnalysisService;
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::Json,
};
use sqlx::{Pool, Sqlite};

pub async fn get_content_analysis(
    State(pool): State<Pool<Sqlite>>,
    Path(archive_id): Path<String>,
    Extension(auth): Extension<AuthInfo>,
) -> Result<Json<crate::models::ContentAnalysisResponse>, StatusCode> {
    let archive_path = sqlx::query_scalar::<_, String>("SELECT path FROM archives WHERE id = ?")
        .bind(&archive_id)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if !path_permission::has_path_permission(&pool, &auth, &archive_path).await? {
        return Err(StatusCode::FORBIDDEN);
    }
    ContentAnalysisService::new(pool)
        .get(&archive_id)
        .await
        .map_err(|err| {
            tracing::warn!(archive_id, "content analysis lookup failed: {err:#}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}
