use axum::{response::Json, http::StatusCode};
use chrono::Utc;
use crate::models::HealthStatus;

/// GET /health - 健康检查端点
pub async fn health_check() -> Result<Json<HealthStatus>, StatusCode> {
    let health = HealthStatus {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
    };
    
    Ok(Json(health))
}