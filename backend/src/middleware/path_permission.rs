use axum::{
    extract::State,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use axum::extract::Request as AxumRequest;
use sqlx::{Pool, Sqlite};
use std::path::Path;

/// 基于路径的权限验证中间件
/// 验证用户是否有访问特定路径的权限
pub async fn path_permission_middleware(
    State(pool): State<Pool<Sqlite>>,
    mut request: AxumRequest,
    next: Next,
) -> Result<Response, StatusCode> {
    // 从request扩展中获取用户ID（由auth_middleware设置）
    let user_id = request
        .extensions()
        .get::<String>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 检查用户是否为管理员（管理员有所有权限）
    let user = sqlx::query!(
        "SELECT role FROM users WHERE id = ?",
        user_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error checking user role: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::UNAUTHORIZED)?;

    // 管理员跳过路径检查
    if user.role == "admin" {
        return Ok(next.run(request).await);
    }

    // 从request中获取要访问的路径（这里需要根据具体API设计）
    // 暂时跳过路径检查，在具体需要的API端点中单独实现
    
    Ok(next.run(request).await)
}

/// 检查用户是否有访问指定路径的权限
pub async fn has_path_permission(
    pool: &Pool<Sqlite>, 
    user_id: &str, 
    path: &str
) -> Result<bool, StatusCode> {
    // 检查用户是否为管理员
    let user = sqlx::query!(
        "SELECT role FROM users WHERE id = ?",
        user_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error checking user role: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    // 管理员有所有权限
    if user.role == "admin" {
        return Ok(true);
    }

    // 获取用户的路径权限
    let user_paths = sqlx::query!(
        "SELECT path FROM user_paths WHERE user_id = ?",
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting user paths: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // 如果没有配置路径权限，则允许访问（向后兼容）
    if user_paths.is_empty() {
        return Ok(true);
    }

    // 检查路径是否匹配
    for user_path in user_paths {
        if path_matches(&user_path.path, path) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// 检查路径是否匹配权限规则
/// 支持通配符匹配
fn path_matches(permission_path: &str, actual_path: &str) -> bool {
    // 确保路径以'/'开头
    let perm_path = if permission_path.starts_with('/') {
        permission_path
    } else {
        &format!("/{}", permission_path)
    };
    
    let act_path = if actual_path.starts_with('/') {
        actual_path
    } else {
        &format!("/{}", actual_path)
    };

    // 精确匹配
    if perm_path == act_path {
        return true;
    }

    // 通配符匹配（以*结尾表示匹配子路径）
    if perm_path.ends_with('*') {
        let prefix = &perm_path[..perm_path.len() - 1];
        return act_path.starts_with(prefix);
    }

    // 目录匹配（权限路径是实际路径的父目录）
    if act_path.starts_with(perm_path) && act_path.chars().nth(perm_path.len()) == Some('/') {
        return true;
    }

    false
}

/// 获取用户的所有路径权限
pub async fn get_user_paths(pool: &Pool<Sqlite>, user_id: &str) -> Result<Vec<String>, StatusCode> {
    let paths = sqlx::query!(
        "SELECT path FROM user_paths WHERE user_id = ?",
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting user paths: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .into_iter()
    .map(|row| row.path)
    .collect();

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_matches() {
        // 精确匹配
        assert!(path_matches("/comics", "/comics"));
        assert!(path_matches("comics", "/comics"));
        assert!(path_matches("/comics", "comics"));
        
        // 通配符匹配
        assert!(path_matches("/comics/*", "/comics/manga"));
        assert!(path_matches("/comics/*", "/comics/manga/volume1"));
        assert!(!path_matches("/comics/*", "/books"));
        
        // 目录匹配
        assert!(path_matches("/comics", "/comics/manga"));
        assert!(!path_matches("/comics", "/comicsxyz"));
        
        // 不匹配情况
        assert!(!path_matches("/comics", "/books"));
        assert!(!path_matches("/comics/manga", "/comics"));
    }
}