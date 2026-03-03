use crate::middleware::auth::AuthInfo;
use axum::extract::Request as AxumRequest;
use axum::{http::StatusCode, middleware::Next, response::Response};

/// 管理员权限验证中间件
/// 必须在auth_middleware之后使用，因为需要AuthInfo
pub async fn admin_middleware(request: AxumRequest, next: Next) -> Result<Response, StatusCode> {
    // 从request扩展中获取AuthInfo（由auth_middleware设置）
    let auth_info = request
        .extensions()
        .get::<AuthInfo>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if auth_info.role != "admin" {
        tracing::warn!(
            "User {} attempted admin-only operation",
            auth_info.user_id
        );
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}
