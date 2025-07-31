use axum::{
    extract::{Path, State},
    response::Json,
    http::StatusCode,
};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;
use chrono::Utc;

use crate::models::{User, CreateUserRequest, UpdateUserRequest, UserPathsRequest, UserRole};
use crate::services::auth_service::AuthService;

pub struct UserHandler;

impl UserHandler {
    /// GET /api/v1/users - 获取用户列表（管理员）
    pub async fn list_users(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<Vec<User>>, StatusCode> {
        let users = sqlx::query_as::<_, User>(
            "SELECT id, username, email, role, password_hash, api_key, created_at, updated_at FROM users ORDER BY created_at DESC"
        )
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(users))
    }

    /// POST /api/v1/users - 创建用户（管理员）
    pub async fn create_user(
        State(pool): State<Pool<Sqlite>>,
        Json(request): Json<CreateUserRequest>,
    ) -> Result<Json<User>, StatusCode> {
        let user_id = Uuid::new_v4().to_string();
        let api_key = Uuid::new_v4().to_string();
        let password_hash = AuthService::hash_password(&request.password)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(user))
    }

    /// GET /api/v1/users/:id - 获取用户详情
    pub async fn get_user(
        State(pool): State<Pool<Sqlite>>,
        Path(user_id): Path<String>,
    ) -> Result<Json<User>, StatusCode> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, username, email, role, password_hash, api_key, created_at, updated_at FROM users WHERE id = ?"
        )
        .bind(&user_id)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

        Ok(Json(user))
    }

    /// PUT /api/v1/users/:id - 更新用户信息
    pub async fn update_user(
        State(pool): State<Pool<Sqlite>>,
        Path(user_id): Path<String>,
        Json(request): Json<UpdateUserRequest>,
    ) -> Result<StatusCode, StatusCode> {
        let mut query = "UPDATE users SET updated_at = ?".to_string();
        let mut params: Vec<String> = vec![Utc::now().to_rfc3339()];

        if let Some(username) = &request.username {
            query.push_str(", username = ?");
            params.push(username.clone());
        }

        if let Some(email) = &request.email {
            query.push_str(", email = ?");
            params.push(email.clone());
        }

        if let Some(password) = &request.password {
            let password_hash = AuthService::hash_password(password)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            query.push_str(", password_hash = ?");
            params.push(password_hash);
        }

        if let Some(role) = &request.role {
            let role_str = match role {
                UserRole::Admin => "admin",
                UserRole::User => "user",
            };
            query.push_str(", role = ?");
            params.push(role_str.to_string());
        }

        query.push_str(" WHERE id = ?");
        params.push(user_id.clone());

        let mut sqlx_query = sqlx::query(&query);
        for param in params {
            sqlx_query = sqlx_query.bind(param);
        }

        let result = sqlx_query
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if result.rows_affected() == 0 {
            return Err(StatusCode::NOT_FOUND);
        }

        Ok(StatusCode::OK)
    }

    /// DELETE /api/v1/users/:id - 删除用户（管理员）
    pub async fn delete_user(
        State(pool): State<Pool<Sqlite>>,
        Path(user_id): Path<String>,
    ) -> Result<StatusCode, StatusCode> {
        let result = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(&user_id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if result.rows_affected() == 0 {
            return Err(StatusCode::NOT_FOUND);
        }

        Ok(StatusCode::OK)
    }

    /// PUT /api/v1/users/:id/paths - 管理用户路径权限（管理员）
    pub async fn update_user_paths(
        State(pool): State<Pool<Sqlite>>,
        Path(user_id): Path<String>,
        Json(request): Json<UserPathsRequest>,
    ) -> Result<StatusCode, StatusCode> {
        // 验证用户存在
        let _user = sqlx::query!("SELECT id FROM users WHERE id = ?", user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        // 删除现有路径
        sqlx::query("DELETE FROM user_paths WHERE user_id = ?")
            .bind(&user_id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // 添加新路径
        for path in request.paths {
            sqlx::query("INSERT INTO user_paths (user_id, path) VALUES (?, ?)")
                .bind(&user_id)
                .bind(&path)
                .execute(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }

        Ok(StatusCode::OK)
    }
}