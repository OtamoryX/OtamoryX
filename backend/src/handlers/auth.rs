use axum::{extract::State, http::StatusCode, Json};
use sqlx::{Pool, Sqlite};

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
    Json(request): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    let auth_service = AuthService::new(pool);

    match auth_service.login(request).await {
        Ok(response) => Ok(Json(response)),
        Err(AuthError::InvalidCredentials) => Err(StatusCode::UNAUTHORIZED),
        Err(AuthError::Database(_)) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        Err(_) => Err(StatusCode::BAD_REQUEST),
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
