use crate::middleware::auth::AuthInfo;
use crate::models::{
    BehaviorEventQuery, RecordBehaviorEventRequest, RecordBehaviorEventResponse, UserBehaviorEvent,
};
use crate::services::CurationService;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use sqlx::{Pool, Sqlite};

pub async fn record_behavior_event(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    Json(request): Json<RecordBehaviorEventRequest>,
) -> Result<Json<RecordBehaviorEventResponse>, StatusCode> {
    let service = CurationService::new(pool);
    let (event, duplicate) = service
        .record_event(&auth.user_id, &request)
        .await
        .map_err(|error| {
            tracing::warn!("Invalid behavior event: {error}");
            if error.to_string().contains("unsupported") || error.to_string().contains("page") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(RecordBehaviorEventResponse { event, duplicate }))
}

pub async fn list_behavior_events(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    Query(query): Query<BehaviorEventQuery>,
) -> Result<Json<Vec<UserBehaviorEvent>>, StatusCode> {
    let events = CurationService::new(pool)
        .list_events(
            &auth.user_id,
            query.archive_id.as_deref(),
            query.event_type.as_deref(),
            query.limit.unwrap_or(100),
        )
        .await
        .map_err(|error| {
            tracing::warn!("Failed to list behavior events: {error}");
            if error.to_string().contains("unsupported") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(events))
}
