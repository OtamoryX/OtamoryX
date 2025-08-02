use axum::{
    extract::State,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use axum::extract::Request as AxumRequest;
use sqlx::{Pool, Sqlite};

/// 管理员权限验证中间件
/// 必须在auth_middleware之后使用，因为需要用户ID
pub async fn admin_middleware(
    State(pool): State<Pool<Sqlite>>,
    mut request: AxumRequest,
    next: Next,
) -> Result<Response, StatusCode> {
    // 从request扩展中获取用户ID（由auth_middleware设置）
    let user_id = request
        .extensions()
        .get::<String>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 验证用户是否为管理员
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

    if user.role != "admin" {
        tracing::warn!("User {} attempted admin-only operation", user_id);
        return Err(StatusCode::FORBIDDEN);
    }

    // 将用户角色添加到request扩展中，供后续处理器使用
    request.extensions_mut().insert("admin".to_string());
    
    Ok(next.run(request).await)
}

/// 获取当前用户角色的辅助函数
pub async fn get_user_role(pool: &Pool<Sqlite>, user_id: &str) -> Result<String, StatusCode> {
    let user = sqlx::query!(
        "SELECT role FROM users WHERE id = ?",
        user_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting user role: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(user.role)
}

/// 检查用户是否有管理员权限
pub async fn is_admin(pool: &Pool<Sqlite>, user_id: &str) -> Result<bool, StatusCode> {
    let role = get_user_role(pool, user_id).await?;
    Ok(role == "admin")
}