use axum::{extract::State, http::StatusCode, Json};
use sqlx::{Pool, Sqlite};
use std::sync::Arc;

use crate::middleware::rate_limiter::LoginRateLimiter;
use crate::models::{
    AuthResponse, CreateUserRequest, InitializeSystemRequest, LoginRequest, SystemStatus,
};
use crate::services::{AuthError, AuthService};

pub async fn register(
    State(pool): State<Pool<Sqlite>>,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let auth_service = AuthService::new(pool);

    match auth_service.register(request).await {
        Ok(response) => Ok(Json(response)),
        Err(AuthError::ValidationError(msg)) => Err((StatusCode::BAD_REQUEST, msg)),
        Err(AuthError::Database(_)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )),
        Err(AuthError::PasswordHash(_)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

pub async fn login(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(rate_limiter): axum::extract::Extension<Arc<LoginRateLimiter>>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    // Check rate limit
    if let Err(remaining) = rate_limiter.check_rate_limit(&request.username) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            format!(
                "Too many login attempts. Try again in {} seconds.",
                remaining.as_secs()
            ),
        ));
    }

    let username = request.username.clone();
    let auth_service = AuthService::new(pool);

    match auth_service.login(request).await {
        Ok(response) => {
            rate_limiter.reset(&username);
            Ok(Json(response))
        }
        Err(AuthError::InvalidCredentials) => {
            rate_limiter.record_failure(&username);
            Err((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))
        }
        Err(AuthError::Database(_)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )),
        Err(_) => Err((StatusCode::BAD_REQUEST, "Bad request".to_string())),
    }
}

pub async fn logout(State(pool): State<Pool<Sqlite>>) -> Result<StatusCode, StatusCode> {
    let auth_service = AuthService::new(pool);

    match auth_service.logout().await {
        Ok(()) => Ok(StatusCode::OK),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_system_status(
    State(pool): State<Pool<Sqlite>>,
) -> Result<Json<SystemStatus>, StatusCode> {
    let auth_service = AuthService::new(pool);

    match auth_service.get_system_status().await {
        Ok(status) => Ok(Json(status)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn initialize_system(
    State(pool): State<Pool<Sqlite>>,
    Json(request): Json<InitializeSystemRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let auth_service = AuthService::new(pool);

    match auth_service.initialize_system(request).await {
        Ok(response) => Ok(Json(response)),
        Err(AuthError::AlreadyInitialized) => Err((
            StatusCode::CONFLICT,
            "System already initialized".to_string(),
        )),
        Err(AuthError::ValidationError(msg)) => Err((StatusCode::BAD_REQUEST, msg)),
        Err(AuthError::Database(_)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )),
        Err(AuthError::PasswordHash(_)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}
