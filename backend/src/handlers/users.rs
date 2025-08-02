use axum::{
    extract::{Path, State},
    response::Json,
    http::StatusCode,
};
use sqlx::{Pool, Sqlite, Row};
use uuid::Uuid;
use chrono::Utc;

use crate::models::{User, CreateUserRequest, UpdateUserRequest, UserPathsRequest, UserRole, BatchDeleteUsersRequest};
use crate::services::{
    auth_service::AuthService, 
    admin_service::{AdminService, SystemStats},
    access_control_service::{AccessControlService, UserPermissions}
};

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
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id, username, email, role, password_hash, api_key, created_at, updated_at
            "#
        )
        .bind(&user_id)
        .bind(&request.username)
        .bind(&request.email)
        .bind(request.role.as_deref().unwrap_or("user"))
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

    /// GET /api/v1/users/:id/paths - 获取用户路径权限
    pub async fn get_user_paths(
        State(pool): State<Pool<Sqlite>>,
        Path(user_id): Path<String>,
    ) -> Result<Json<Vec<String>>, StatusCode> {
        // 验证用户存在
        let _user = sqlx::query!("SELECT id FROM users WHERE id = ?", user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        let paths = sqlx::query!("SELECT path FROM user_paths WHERE user_id = ?", user_id)
            .fetch_all(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .into_iter()
            .map(|row| row.path)
            .collect();

        Ok(Json(paths))
    }

    /// DELETE /api/v1/users/batch-delete - 批量删除用户（管理员）
    pub async fn batch_delete_users(
        State(pool): State<Pool<Sqlite>>,
        Json(request): Json<BatchDeleteUsersRequest>,
    ) -> Result<StatusCode, StatusCode> {
        if request.user_ids.is_empty() {
            return Ok(StatusCode::OK);
        }

        // 防止删除管理员用户
        let admin_count = sqlx::query!("SELECT COUNT(*) as count FROM users WHERE role = 'admin'")
            .fetch_one(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .count;

        // 构建动态查询来检查要删除的管理员数量
        let placeholders = request.user_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query_str = format!(
            "SELECT COUNT(*) as count FROM users WHERE id IN ({}) AND role = 'admin'",
            placeholders
        );
        let mut admins_to_delete = sqlx::query(&query_str);
        
        for user_id in &request.user_ids {
            admins_to_delete = admins_to_delete.bind(user_id);
        }
        
        let admins_to_delete_result = admins_to_delete
            .fetch_one(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            
        let admins_to_delete_count = admins_to_delete_result.get::<i64, _>("count");

        // 确保至少保留一个管理员
        if (admin_count as i64) <= admins_to_delete_count {
            return Err(StatusCode::FORBIDDEN); // 不能删除所有管理员
        }

        // 批量删除用户
        let placeholders = request.user_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("DELETE FROM users WHERE id IN ({})", placeholders);
        
        let mut sqlx_query = sqlx::query(&query);
        for user_id in request.user_ids {
            sqlx_query = sqlx_query.bind(user_id);
        }

        let result = sqlx_query
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        tracing::info!("Batch deleted {} users", result.rows_affected());
        Ok(StatusCode::OK)
    }

    /// PUT /api/v1/users/:id/promote - 提升用户为管理员（管理员专用）
    pub async fn promote_to_admin(
        State(pool): State<Pool<Sqlite>>,
        Path(user_id): Path<String>,
    ) -> Result<StatusCode, StatusCode> {
        let admin_service = AdminService::new(pool);
        admin_service.promote_to_admin(&user_id).await?;
        Ok(StatusCode::OK)
    }

    /// PUT /api/v1/users/:id/demote - 降级管理员为普通用户（管理员专用）
    pub async fn demote_from_admin(
        State(pool): State<Pool<Sqlite>>,
        Path(user_id): Path<String>,
    ) -> Result<StatusCode, StatusCode> {
        let admin_service = AdminService::new(pool);
        admin_service.demote_from_admin(&user_id).await?;
        Ok(StatusCode::OK)
    }

    /// GET /api/v1/users/admins - 获取所有管理员用户（管理员专用）
    pub async fn get_admin_users(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<Vec<User>>, StatusCode> {
        let admin_service = AdminService::new(pool);
        let admins = admin_service.get_admin_users().await?;
        Ok(Json(admins))
    }

    /// GET /api/v1/system/stats - 获取系统统计信息（管理员专用）
    pub async fn get_system_stats(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<SystemStats>, StatusCode> {
        let admin_service = AdminService::new(pool);
        let stats = admin_service.get_system_stats().await?;
        Ok(Json(stats))
    }

    /// GET /api/v1/users/me/permissions - 获取当前用户权限信息
    pub async fn get_my_permissions(
        State(pool): State<Pool<Sqlite>>,
        axum::extract::Extension(user_id): axum::extract::Extension<String>,
    ) -> Result<Json<UserPermissions>, StatusCode> {
        let access_control = AccessControlService::new(pool);
        let permissions = access_control.get_user_permissions(&user_id).await?;
        Ok(Json(permissions))
    }
}