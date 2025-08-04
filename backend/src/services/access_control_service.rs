use crate::middleware::{admin, path_permission};
use axum::http::StatusCode;
use sqlx::{Pool, Sqlite};

/// 访问控制服务
/// 提供综合的用户权限验证和访问控制功能
pub struct AccessControlService {
    pool: Pool<Sqlite>,
}

impl AccessControlService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// 检查用户是否可以访问特定资源
    /// 综合考虑角色权限和路径权限
    pub async fn can_access_resource(
        &self,
        user_id: &str,
        resource_type: ResourceType,
        resource_path: Option<&str>,
    ) -> Result<bool, StatusCode> {
        match resource_type {
            ResourceType::AdminOnly => {
                // 管理员专用资源
                admin::is_admin(&self.pool, user_id).await
            }
            ResourceType::Archive => {
                // 档案资源需要路径权限验证
                if let Some(path) = resource_path {
                    path_permission::has_path_permission(&self.pool, user_id, path).await
                } else {
                    Ok(true) // 没有路径信息则允许访问
                }
            }
            ResourceType::UserData => {
                // 用户数据：用户只能访问自己的数据，管理员可以访问所有
                Ok(true) // 具体的用户ID检查在handler中进行
            }
            ResourceType::SystemSettings => {
                // 系统设置：只有管理员可以修改
                admin::is_admin(&self.pool, user_id).await
            }
            ResourceType::Public => {
                // 公共资源：所有已认证用户可访问
                Ok(true)
            }
        }
    }

    /// 检查用户对特定用户数据的访问权限
    /// 用户只能访问自己的数据，管理员可以访问所有用户数据
    pub async fn can_access_user_data(
        &self,
        requesting_user_id: &str,
        target_user_id: &str,
    ) -> Result<bool, StatusCode> {
        // 用户可以访问自己的数据
        if requesting_user_id == target_user_id {
            return Ok(true);
        }

        // 检查是否为管理员
        admin::is_admin(&self.pool, requesting_user_id).await
    }

    /// 验证用户操作权限
    /// 检查用户是否有权限执行特定操作
    pub async fn can_perform_action(
        &self,
        user_id: &str,
        action: UserAction,
    ) -> Result<bool, StatusCode> {
        match action {
            UserAction::ReadArchives => Ok(true), // 所有用户可以读取档案
            UserAction::ManageUsers => admin::is_admin(&self.pool, user_id).await,
            UserAction::ManageSystem => admin::is_admin(&self.pool, user_id).await,
            UserAction::ManagePlugins => admin::is_admin(&self.pool, user_id).await,
            UserAction::ConfigureAI => admin::is_admin(&self.pool, user_id).await,
            UserAction::ViewSystemStats => admin::is_admin(&self.pool, user_id).await,
            UserAction::ManageCache => admin::is_admin(&self.pool, user_id).await,
            UserAction::BatchOperations => admin::is_admin(&self.pool, user_id).await,
        }
    }

    /// 获取用户的权限摘要
    pub async fn get_user_permissions(&self, user_id: &str) -> Result<UserPermissions, StatusCode> {
        let user = sqlx::query!("SELECT role FROM users WHERE id = ?", user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Database error getting user permissions: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;

        let is_admin = user.role == "admin";

        // 获取用户路径权限
        let paths = if is_admin {
            vec!["*".to_string()] // 管理员有所有路径权限
        } else {
            sqlx::query!("SELECT path FROM user_paths WHERE user_id = ?", user_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| {
                    tracing::error!("Database error getting user paths: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
                .into_iter()
                .map(|row| row.path)
                .collect()
        };

        Ok(UserPermissions {
            is_admin,
            can_manage_users: is_admin,
            can_manage_system: is_admin,
            can_manage_plugins: is_admin,
            can_configure_ai: is_admin,
            can_view_system_stats: is_admin,
            can_manage_cache: is_admin,
            can_batch_operations: is_admin,
            accessible_paths: paths,
        })
    }
}

#[derive(Debug, Clone)]
pub enum ResourceType {
    AdminOnly,
    Archive,
    UserData,
    SystemSettings,
    Public,
}

#[derive(Debug, Clone)]
pub enum UserAction {
    ReadArchives,
    ManageUsers,
    ManageSystem,
    ManagePlugins,
    ConfigureAI,
    ViewSystemStats,
    ManageCache,
    BatchOperations,
}

#[derive(serde::Serialize)]
pub struct UserPermissions {
    pub is_admin: bool,
    pub can_manage_users: bool,
    pub can_manage_system: bool,
    pub can_manage_plugins: bool,
    pub can_configure_ai: bool,
    pub can_view_system_stats: bool,
    pub can_manage_cache: bool,
    pub can_batch_operations: bool,
    pub accessible_paths: Vec<String>,
}
