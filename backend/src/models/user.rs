use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub role: UserRole,
    #[serde(skip_serializing)]
    pub password_hash: String,
    #[serde(skip_serializing)]
    pub api_key: String,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum UserRole {
    #[sqlx(rename = "admin")]
    Admin,
    #[sqlx(rename = "user")]
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserPath {
    pub id: i64,
    pub user_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: Option<String>,
    pub password: String,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemStatus {
    pub initialized: bool,
    #[serde(rename = "hasAdmin")]
    pub has_admin: bool,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InitializeSystemRequest {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub role: Option<UserRole>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserPathsRequest {
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchDeleteUsersRequest {
    pub user_ids: Vec<String>,
}