use sqlx::{Pool, Sqlite};
use axum::http::StatusCode;
use crate::models::User;

pub struct AdminService {
    pool: Pool<Sqlite>,
}

impl AdminService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// 检查系统中是否至少有一个管理员
    pub async fn has_admin(&self) -> Result<bool, StatusCode> {
        let count = sqlx::query!(
            "SELECT COUNT(*) as count FROM users WHERE role = 'admin'"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error checking admin count: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .count;

        Ok(count > 0)
    }

    /// 获取所有管理员用户
    pub async fn get_admin_users(&self) -> Result<Vec<User>, StatusCode> {
        let users = sqlx::query_as::<_, User>(
            "SELECT id, username, email, role, password_hash, api_key, created_at, updated_at 
             FROM users 
             WHERE role = 'admin' 
             ORDER BY created_at ASC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error getting admin users: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        Ok(users)
    }

    /// 提升用户为管理员
    pub async fn promote_to_admin(&self, user_id: &str) -> Result<(), StatusCode> {
        let result = sqlx::query!(
            "UPDATE users SET role = 'admin', updated_at = datetime('now') WHERE id = ?",
            user_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error promoting user to admin: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        if result.rows_affected() == 0 {
            return Err(StatusCode::NOT_FOUND);
        }

        tracing::info!("User {} promoted to admin", user_id);
        Ok(())
    }

    /// 降级管理员为普通用户
    /// 确保不会删除最后一个管理员
    pub async fn demote_from_admin(&self, user_id: &str) -> Result<(), StatusCode> {
        // 检查管理员数量
        let admin_count = sqlx::query!(
            "SELECT COUNT(*) as count FROM users WHERE role = 'admin'"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error checking admin count: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .count;

        // 如果只有一个管理员，不允许降级
        if admin_count <= 1 {
            tracing::warn!("Attempt to demote the last admin user: {}", user_id);
            return Err(StatusCode::FORBIDDEN);
        }

        let result = sqlx::query!(
            "UPDATE users SET role = 'user', updated_at = datetime('now') WHERE id = ? AND role = 'admin'",
            user_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error demoting admin user: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        if result.rows_affected() == 0 {
            return Err(StatusCode::NOT_FOUND);
        }

        tracing::info!("Admin user {} demoted to regular user", user_id);
        Ok(())
    }

    /// 验证用户是否为管理员
    pub async fn verify_admin(&self, user_id: &str) -> Result<bool, StatusCode> {
        let user = sqlx::query!(
            "SELECT role FROM users WHERE id = ?",
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error verifying admin: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

        Ok(user.role == "admin")
    }

    /// 获取系统统计信息（管理员专用）
    pub async fn get_system_stats(&self) -> Result<SystemStats, StatusCode> {
        let total_users = sqlx::query!(
            "SELECT COUNT(*) as count FROM users"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .count as u32;

        let admin_users = sqlx::query!(
            "SELECT COUNT(*) as count FROM users WHERE role = 'admin'"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .count as u32;

        let total_archives = sqlx::query!(
            "SELECT COUNT(*) as count FROM archives"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .count as u32;

        let total_categories = sqlx::query!(
            "SELECT COUNT(*) as count FROM categories"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .count as u32;

        let total_tags = sqlx::query!(
            "SELECT COUNT(*) as count FROM tags"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .count as u32;

        Ok(SystemStats {
            total_users,
            admin_users,
            regular_users: total_users - admin_users,
            total_archives,
            total_categories,
            total_tags,
        })
    }
}

#[derive(serde::Serialize)]
pub struct SystemStats {
    pub total_users: u32,
    pub admin_users: u32,
    pub regular_users: u32,
    pub total_archives: u32,
    pub total_categories: u32,
    pub total_tags: u32,
}