use crate::services::auth_service::validate_jwt;
use axum::extract::Request as AxumRequest;
use axum::{http::StatusCode, middleware::Next, response::Response};

/// Authentication info extracted from JWT, injected into request extensions.
/// All downstream handlers/middleware can read this without additional DB queries.
#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub user_id: String,
    pub role: String,
}

pub async fn auth_middleware(mut request: AxumRequest, next: Next) -> Result<Response, StatusCode> {
    // 获取 Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok());

    // 检查是否存在 Bearer token
    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => match header.strip_prefix("Bearer ") {
            Some(token) => token,
            None => return Err(StatusCode::UNAUTHORIZED),
        },
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    // 验证 JWT token（纯本地校验，无需数据库查询）
    match validate_jwt(token) {
        Ok(claims) => {
            // 将 AuthInfo 添加到 request 扩展中，供后续处理器使用
            request.extensions_mut().insert(AuthInfo {
                user_id: claims.sub,
                role: claims.role,
            });
            Ok(next.run(request).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}
