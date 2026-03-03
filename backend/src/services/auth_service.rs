use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use uuid::Uuid;

use crate::models::{
    AuthResponse, CreateUserRequest, InitializeSystemRequest, LoginRequest, SystemStatus, User,
    UserResponse, UserRole,
};

/// JWT Claims 结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// 用户 ID (subject)
    pub sub: String,
    /// 用户角色
    pub role: String,
    /// 过期时间 (Unix timestamp)
    pub exp: usize,
    /// 签发时间 (Unix timestamp)
    pub iat: usize,
}

/// JWT token 有效期：24 小时（单位：秒）
const JWT_EXPIRATION_SECONDS: usize = 24 * 60 * 60;
const JWT_SECRET_ENV_KEY: &str = "JWT_SECRET";

static JWT_SECRET: OnceLock<String> = OnceLock::new();

fn resolve_env_file_path() -> PathBuf {
    if let Ok(path) = std::env::var("ENV_FILE") {
        return PathBuf::from(path);
    }

    let cwd_env = PathBuf::from(".env");
    if cwd_env.exists() {
        return cwd_env;
    }

    let backend_env = PathBuf::from("backend/.env");
    if backend_env.exists() {
        return backend_env;
    }

    cwd_env
}

fn strip_wrapping_quotes(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let first = bytes[0];
        let last = bytes[value.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &value[1..value.len() - 1];
        }
    }
    value
}

fn parse_jwt_secret_from_env_contents(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let candidate = if let Some(rest) = trimmed.strip_prefix("export ") {
            rest.trim()
        } else {
            trimmed
        };

        if let Some(value) = candidate.strip_prefix("JWT_SECRET=") {
            let value = strip_wrapping_quotes(value.trim());
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    None
}

fn load_jwt_secret_from_env_file(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(parse_jwt_secret_from_env_contents(&content)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

fn append_jwt_secret_to_env_file(path: &Path, secret: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let has_content = file.metadata()?.len() > 0;
    if has_content {
        writeln!(file)?;
    }
    writeln!(file, "JWT_SECRET={}", secret)?;
    Ok(())
}

/// 初始化 JWT 密钥。
/// 若环境变量未配置，则从 .env 读取；若仍不存在则生成并写入 .env，保证重启后可复用。
pub fn init_jwt_secret() -> &'static str {
    JWT_SECRET
        .get_or_init(|| {
            match std::env::var(JWT_SECRET_ENV_KEY) {
                Ok(secret) => {
                    if secret.len() < 32 {
                        tracing::warn!("JWT_SECRET is shorter than 32 bytes");
                    }
                    secret
                }
                Err(_) => {
                    let env_file = resolve_env_file_path();

                    match load_jwt_secret_from_env_file(&env_file) {
                        Ok(Some(secret)) => {
                            if secret.len() < 32 {
                                tracing::warn!(
                                    "JWT_SECRET loaded from {:?} is shorter than 32 bytes",
                                    env_file
                                );
                            }
                            std::env::set_var(JWT_SECRET_ENV_KEY, &secret);
                            return secret;
                        }
                        Ok(None) => {}
                        Err(e) => {
                            tracing::warn!("Failed to read {:?}: {}", env_file, e);
                        }
                    }

                    let generated = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
                    if let Err(e) = append_jwt_secret_to_env_file(&env_file, &generated) {
                        tracing::warn!(
                            "Failed to persist JWT_SECRET to {:?}: {}. Continuing with generated secret.",
                            env_file,
                            e
                        );
                    } else {
                        tracing::warn!(
                            "JWT_SECRET was missing; generated and persisted to {:?}.",
                            env_file
                        );
                    }

                    std::env::set_var(JWT_SECRET_ENV_KEY, &generated);
                    generated
                }
            }
        })
        .as_str()
}

/// 生成 JWT token
pub fn generate_jwt(user_id: &str, role: &str) -> Result<String, AuthError> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        role: role.to_string(),
        iat: now,
        exp: now + JWT_EXPIRATION_SECONDS,
    };

    let secret = init_jwt_secret();
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AuthError::TokenError(format!("Failed to generate JWT: {}", e)))
}

/// 验证 JWT token，返回解析后的 Claims
pub fn validate_jwt(token: &str) -> Result<Claims, AuthError> {
    let secret = init_jwt_secret();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| match *e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
        jsonwebtoken::errors::ErrorKind::InvalidToken
        | jsonwebtoken::errors::ErrorKind::InvalidSignature => AuthError::InvalidCredentials,
        _ => AuthError::TokenError(format!("JWT validation failed: {}", e)),
    })?;

    Ok(token_data.claims)
}

/// 将 UserRole 枚举转换为 JWT claims 中使用的字符串
fn role_to_string(role: &UserRole) -> &'static str {
    match role {
        UserRole::Admin => "admin",
        UserRole::User => "user",
    }
}

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
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Token expired")]
    TokenExpired,
    #[error("Token error: {0}")]
    TokenError(String),
}

impl AuthService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Validate password strength: must be non-empty and at least 6 characters.
    pub fn validate_password(password: &str) -> Result<(), AuthError> {
        if password.is_empty() {
            return Err(AuthError::ValidationError(
                "Password cannot be empty".to_string(),
            ));
        }
        if password.len() < 6 {
            return Err(AuthError::ValidationError(
                "Password must be at least 6 characters".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate username: 1-32 characters, only alphanumeric, underscore, and hyphen.
    pub fn validate_username(username: &str) -> Result<(), AuthError> {
        if username.is_empty() {
            return Err(AuthError::ValidationError(
                "Username cannot be empty".to_string(),
            ));
        }
        if username.len() > 32 {
            return Err(AuthError::ValidationError(
                "Username must be at most 32 characters".to_string(),
            ));
        }
        if !username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(AuthError::ValidationError(
                "Username can only contain letters, digits, underscores, and hyphens".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn register(&self, request: CreateUserRequest) -> Result<AuthResponse, AuthError> {
        Self::validate_username(&request.username)?;
        Self::validate_password(&request.password)?;

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

        let token = generate_jwt(&user.id, role_to_string(&user.role))?;

        Ok(AuthResponse { token, user: UserResponse::from(user) })
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
            let token = generate_jwt(&user.id, role_to_string(&user.role))?;
            Ok(AuthResponse { token, user: UserResponse::from(user) })
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }

    pub async fn initialize_system(
        &self,
        request: InitializeSystemRequest,
    ) -> Result<AuthResponse, AuthError> {
        Self::validate_username(&request.username)?;
        Self::validate_password(&request.password)?;

        let user_id = Uuid::new_v4().to_string();
        let api_key = Uuid::new_v4().to_string();
        let password_hash = Self::hash_password(&request.password)?;
        let now = Utc::now();

        // 原子操作：仅在不存在管理员时插入，避免 TOCTOU 竞态条件 (P1-004)
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, username, email, role, password_hash, api_key, created_at, updated_at)
            SELECT ?, ?, ?, 'admin', ?, ?, ?, ?
            WHERE NOT EXISTS (SELECT 1 FROM users WHERE role = 'admin')
            RETURNING id, username, email, role, password_hash, api_key, created_at, updated_at
            "#
        )
        .bind(&user_id)
        .bind(&request.username)
        .bind(&request.email)
        .bind(&password_hash)
        .bind(&api_key)
        .bind(now)
        .bind(now)
        .fetch_optional(&self.pool)
        .await?;

        match user {
            Some(user) => {
                let token = generate_jwt(&user.id, role_to_string(&user.role))?;
                Ok(AuthResponse { token, user: UserResponse::from(user) })
            }
            None => Err(AuthError::AlreadyInitialized),
        }
    }

    pub async fn get_system_status(&self) -> Result<SystemStatus, AuthError> {
        let admin_count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE role = 'admin'")
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

        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::PasswordHash(e.to_string()))?;

        Ok(hash.to_string())
    }

    pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| AuthError::PasswordHash(e.to_string()))?;

        let argon2 = Argon2::default();
        Ok(argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    pub async fn logout(&self) -> Result<(), AuthError> {
        // TODO: JWT 是无状态的，logout 需要 token 黑名单机制才能真正使旧 token 失效。
        // 当前实现为空操作，客户端应自行删除本地存储的 token。
        // 未来可通过 Redis 或数据库维护已撤销 token 列表来实现。
        Ok(())
    }

    /// 验证 token 并返回 user_id。
    /// 内部调用 validate_jwt 从 Claims 中提取用户信息，不再查询数据库。
    pub fn validate_token(token: &str) -> Result<String, AuthError> {
        let claims = validate_jwt(token)?;
        Ok(claims.sub)
    }
}
