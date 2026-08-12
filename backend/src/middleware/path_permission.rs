use crate::middleware::auth::AuthInfo;
use axum::extract::Request as AxumRequest;
use axum::{http::StatusCode, middleware::Next, response::Response};
use sqlx::{Pool, Sqlite};

/// 基于路径的权限验证中间件
/// 验证用户是否有访问特定路径的权限
pub async fn path_permission_middleware(
    request: AxumRequest,
    next: Next,
) -> Result<Response, StatusCode> {
    // 从request扩展中获取AuthInfo（由auth_middleware设置）
    let auth_info = request
        .extensions()
        .get::<AuthInfo>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 管理员跳过路径检查
    if auth_info.role == "admin" {
        return Ok(next.run(request).await);
    }

    // 从request中获取要访问的路径（这里需要根据具体API设计）
    // 暂时跳过路径检查，在具体需要的API端点中单独实现

    Ok(next.run(request).await)
}

/// 检查用户是否有访问指定路径的权限
/// 使用预先从JWT提取的AuthInfo，避免额外DB查询获取角色
pub async fn has_path_permission(
    pool: &Pool<Sqlite>,
    auth_info: &AuthInfo,
    path: &str,
) -> Result<bool, StatusCode> {
    // 管理员有所有权限
    if auth_info.role == "admin" {
        return Ok(true);
    }

    // 获取用户的路径权限
    let user_paths = sqlx::query!(
        "SELECT path FROM user_paths WHERE user_id = ?",
        auth_info.user_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting user paths: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(has_path_permission_with_paths(
        &auth_info.role,
        &user_paths
            .into_iter()
            .map(|row| row.path)
            .collect::<Vec<_>>(),
        path,
    ))
}

/// Check a path against permissions that were already loaded for this request.
/// List endpoints use this to avoid issuing the same user_paths query per item.
pub fn has_path_permission_with_paths(role: &str, user_paths: &[String], path: &str) -> bool {
    if role == "admin" || user_paths.is_empty() {
        return true;
    }
    user_paths
        .iter()
        .any(|permission_path| path_matches(permission_path, path))
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
    let paths = sqlx::query!("SELECT path FROM user_paths WHERE user_id = ?", user_id)
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

/// 根据 archive_id 统一完成路径查询与权限校验。
/// 返回已授权的 archive 路径，便于 handler 复用。
pub async fn authorize_archive_access(
    pool: &Pool<Sqlite>,
    auth_info: &AuthInfo,
    archive_id: &str,
) -> Result<String, StatusCode> {
    let archive = sqlx::query!("SELECT path FROM archives WHERE id = ?", archive_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error getting archive {} path: {}", archive_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    if !has_path_permission(pool, auth_info, &archive.path).await? {
        tracing::warn!(
            "User {} denied access to archive {} path {}",
            auth_info.user_id,
            archive_id,
            archive.path
        );
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(archive.path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    fn test_auth_info(user_id: &str, role: &str) -> AuthInfo {
        AuthInfo {
            user_id: user_id.to_string(),
            role: role.to_string(),
        }
    }

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

    #[tokio::test]
    async fn authorize_archive_access_enforces_user_paths() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite memory pool");

        sqlx::query("CREATE TABLE archives (id TEXT PRIMARY KEY, path TEXT NOT NULL)")
            .execute(&pool)
            .await
            .expect("create archives");
        sqlx::query("CREATE TABLE user_paths (user_id TEXT NOT NULL, path TEXT NOT NULL)")
            .execute(&pool)
            .await
            .expect("create user_paths");

        sqlx::query("INSERT INTO archives (id, path) VALUES ('archive-1', '/comics/a/file.cbz')")
            .execute(&pool)
            .await
            .expect("insert archive");
        sqlx::query("INSERT INTO user_paths (user_id, path) VALUES ('user-1', '/comics/b/*')")
            .execute(&pool)
            .await
            .expect("insert path rule");

        let forbidden =
            authorize_archive_access(&pool, &test_auth_info("user-1", "user"), "archive-1").await;
        assert_eq!(forbidden, Err(StatusCode::FORBIDDEN));

        let allowed =
            authorize_archive_access(&pool, &test_auth_info("admin-1", "admin"), "archive-1")
                .await
                .expect("admin should bypass path restrictions");
        assert_eq!(allowed, "/comics/a/file.cbz");
    }
}
