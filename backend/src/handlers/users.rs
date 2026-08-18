use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use sqlx::{Pool, Row, Sqlite};
use uuid::Uuid;

use crate::middleware::auth::AuthInfo;
use crate::models::{
    BatchDeleteUsersRequest, CreateUserRequest, UpdateUserRequest, UserPathsRequest, UserResponse,
    UserRole, UserSummary,
};
use crate::services::{
    identity::access_control::{AccessControlService, UserPermissions},
    identity::admin::{AdminService, SystemStats},
    identity::auth::AuthService,
};

pub struct UserHandler;

const USER_RESPONSE_COLUMNS: &str = "id, username, email, role, created_at, updated_at";

fn normalize_optional_email(email: Option<&str>) -> Option<String> {
    email
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn map_create_user_db_error(err: sqlx::Error) -> StatusCode {
    let err_text = err.to_string();
    tracing::error!("Create user failed: {}", err_text);

    if err_text.contains("UNIQUE constraint failed: users.username")
        || err_text.contains("duplicate key value violates unique constraint")
            && err_text.contains("users_username")
    {
        return StatusCode::CONFLICT;
    }

    if err_text.contains("UNIQUE constraint failed: users.email")
        || err_text.contains("duplicate key value violates unique constraint")
            && err_text.contains("users_email")
    {
        return StatusCode::CONFLICT;
    }

    StatusCode::INTERNAL_SERVER_ERROR
}

impl UserHandler {
    /// GET /api/v1/users - 获取用户列表（管理员）
    pub async fn list_users(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<Vec<UserResponse>>, StatusCode> {
        let query = format!(
            "SELECT {} FROM users ORDER BY created_at DESC",
            USER_RESPONSE_COLUMNS
        );
        let users = sqlx::query_as::<_, UserSummary>(&query)
            .fetch_all(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(users.into_iter().map(UserResponse::from).collect()))
    }

    /// POST /api/v1/users - 创建用户（管理员）
    pub async fn create_user(
        State(pool): State<Pool<Sqlite>>,
        Json(request): Json<CreateUserRequest>,
    ) -> Result<Json<UserResponse>, StatusCode> {
        let email = normalize_optional_email(request.email.as_deref());
        let user_id = Uuid::new_v4().to_string();
        let api_key = Uuid::new_v4().to_string();
        let password_hash = AuthService::hash_password(&request.password)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let query = format!(
            r#"
            INSERT INTO users (id, username, email, role, password_hash, api_key, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING {}
            "#,
            USER_RESPONSE_COLUMNS
        );
        let user = sqlx::query_as::<_, UserSummary>(&query)
            .bind(&user_id)
            .bind(&request.username)
            .bind(email.as_deref())
            .bind(request.role.as_deref().unwrap_or("user"))
            .bind(&password_hash)
            .bind(&api_key)
            .bind(Utc::now())
            .bind(Utc::now())
            .fetch_one(&pool)
            .await
            .map_err(map_create_user_db_error)?;

        Ok(Json(UserResponse::from(user)))
    }

    /// GET /api/v1/users/:id - 获取用户详情
    pub async fn get_user(
        State(pool): State<Pool<Sqlite>>,
        Path(user_id): Path<String>,
    ) -> Result<Json<UserResponse>, StatusCode> {
        let query = format!("SELECT {} FROM users WHERE id = ?", USER_RESPONSE_COLUMNS);
        let user = sqlx::query_as::<_, UserSummary>(&query)
            .bind(&user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        Ok(Json(UserResponse::from(user)))
    }

    /// PUT /api/v1/users/:id - 更新用户信息
    pub async fn update_user(
        State(pool): State<Pool<Sqlite>>,
        Path(user_id): Path<String>,
        Json(request): Json<UpdateUserRequest>,
    ) -> Result<StatusCode, StatusCode> {
        let mut builder = sqlx::QueryBuilder::<Sqlite>::new("UPDATE users SET updated_at = ");
        builder.push_bind(Utc::now().to_rfc3339());

        if let Some(username) = &request.username {
            builder.push(", username = ");
            builder.push_bind(username.clone());
        }

        if let Some(email) = &request.email {
            builder.push(", email = ");
            builder.push_bind(normalize_optional_email(Some(email.as_str())));
        }

        if let Some(password) = &request.password {
            let password_hash = AuthService::hash_password(password)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            builder.push(", password_hash = ");
            builder.push_bind(password_hash);
        }

        if let Some(role) = &request.role {
            let role_str = match role {
                UserRole::Admin => "admin",
                UserRole::User => "user",
            };
            builder.push(", role = ");
            builder.push_bind(role_str.to_string());
        }

        builder.push(" WHERE id = ");
        builder.push_bind(user_id.clone());

        let result = builder
            .build()
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

        // 使用事务保护 DELETE + INSERT 操作
        let mut tx = pool
            .begin()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // 删除现有路径
        sqlx::query("DELETE FROM user_paths WHERE user_id = ?")
            .bind(&user_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // 添加新路径
        for path in request.paths {
            sqlx::query("INSERT INTO user_paths (user_id, path) VALUES (?, ?)")
                .bind(&user_id)
                .bind(&path)
                .execute(&mut *tx)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }

        tx.commit()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
        let placeholders = request
            .user_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
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
        let placeholders = request
            .user_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
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
    ) -> Result<Json<Vec<UserResponse>>, StatusCode> {
        let admin_service = AdminService::new(pool);
        let admins = admin_service.get_admin_users().await?;
        Ok(Json(admins.into_iter().map(UserResponse::from).collect()))
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
        axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    ) -> Result<Json<UserPermissions>, StatusCode> {
        let access_control = AccessControlService::new(pool);
        let permissions = access_control.get_user_permissions(&auth.user_id).await?;
        Ok(Json(permissions))
    }
}

#[cfg(test)]
mod tests {
    use super::USER_RESPONSE_COLUMNS;

    #[test]
    fn user_response_columns_exclude_sensitive_fields() {
        assert!(!USER_RESPONSE_COLUMNS.contains("password_hash"));
        assert!(!USER_RESPONSE_COLUMNS.contains("api_key"));
    }
}
