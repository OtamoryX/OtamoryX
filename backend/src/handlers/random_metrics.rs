use crate::middleware::auth::AuthInfo;
use crate::services::RandomMetricsService;
use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use sqlx::{Pool, Sqlite};

#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub days: Option<i64>,
}

pub async fn get_random_metrics(
    State(pool): State<Pool<Sqlite>>,
    Extension(auth): Extension<AuthInfo>,
    Query(query): Query<MetricsQuery>,
) -> Result<Json<crate::services::RandomRecommendationMetrics>, StatusCode> {
    let days = query.days.unwrap_or(30);
    if ![7, 30, 90].contains(&days) {
        return Err(StatusCode::BAD_REQUEST);
    }
    RandomMetricsService::new(pool)
        .metrics(&auth.user_id, auth.role == "admin", days)
        .await
        .map(Json)
        .map_err(|error| {
            tracing::warn!(%error, "random metrics query failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
