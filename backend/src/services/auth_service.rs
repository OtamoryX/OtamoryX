use sqlx::{Pool, Sqlite};
use uuid::Uuid;
use chrono::Utc;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};

use crate::models::{User, CreateUserRequest, LoginRequest, InitializeSystemRequest, AuthResponse, SystemStatus};

pub struct AuthService {
    pool: Pool<Sqlite>,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Password hashing error: {0}")]
    PasswordHash(String),
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("User not found")]
    UserNotFound,
    #[error("System already initialized")]
    AlreadyInitialized,
}

impl AuthService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn register(&self, request: CreateUserRequest) -> Result<AuthResponse, AuthError> {
        let user_id = Uuid::new_v4().to_string();
        let api_key = Uuid::new_v4().to_string();
        let password_hash = Self::hash_password(&request.password)?;

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, username, email, role, password_hash, api_key, created_at, updated_at)
            VALUES (?, ?, ?, 'user', ?, ?, ?, ?)
            RETURNING id, username, email, role, password_hash, api_key, created_at, updated_at
            "#
        )
        .bind(&user_id)
        .bind(&request.username)
        .bind(&request.email)
        .bind(&password_hash)
        .bind(&api_key)
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;

        Ok(AuthResponse {
            token: api_key,
            user,
        })
    }

    pub async fn login(&self, request: LoginRequest) -> Result<AuthResponse, AuthError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, email, role, password_hash, api_key, created_at, updated_at FROM users WHERE username = ?"
        )
        .bind(&request.username)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AuthError::InvalidCredentials)?;

        if Self::verify_password(&request.password, &user.password_hash)? {
            Ok(AuthResponse {
                token: user.api_key.clone(),
                user,
            })
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }

    pub async fn initialize_system(&self, request: InitializeSystemRequest) -> Result<AuthResponse, AuthError> {
        // 检查是否已经初始化
        let admin_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE role = 'admin'"
        )
        .fetch_one(&self.pool)
        .await?;

        if admin_count > 0 {
            return Err(AuthError::AlreadyInitialized);
        }

        let user_id = Uuid::new_v4().to_string();
        let api_key = Uuid::new_v4().to_string();
        let password_hash = Self::hash_password(&request.password)?;

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, username, email, role, password_hash, api_key, created_at, updated_at)
            VALUES (?, ?, ?, 'admin', ?, ?, ?, ?)
            RETURNING id, username, email, role, password_hash, api_key, created_at, updated_at
            "#
        )
        .bind(&user_id)
        .bind(&request.username)
        .bind(&request.email)
        .bind(&password_hash)
        .bind(&api_key)
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;

        Ok(AuthResponse {
            token: api_key,
            user,
        })
    }

    pub async fn get_system_status(&self) -> Result<SystemStatus, AuthError> {
        let admin_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE role = 'admin'"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(SystemStatus {
            initialized: admin_count > 0,
            has_admin: admin_count > 0,
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    pub fn hash_password(password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::PasswordHash(e.to_string()))?;
        
        Ok(hash.to_string())
    }

    pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| AuthError::PasswordHash(e.to_string()))?;
        
        let argon2 = Argon2::default();
        Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }

    pub async fn logout(&self) -> Result<(), AuthError> {
        // TODO: 实现token失效逻辑（如果需要）
        // 对于简单的API Key系统，登出可能不需要做什么
        Ok(())
    }

    pub async fn validate_token(&self, token: &str) -> Result<String, AuthError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, email, role, password_hash, api_key, created_at, updated_at FROM users WHERE api_key = ?"
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AuthError::InvalidCredentials)?;

        Ok(user.id)
    }
}