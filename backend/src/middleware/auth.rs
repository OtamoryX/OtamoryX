use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use axum::extract::Request as AxumRequest;
use sqlx::{Pool, Sqlite};
use crate::services::auth_service::AuthService;

pub async fn auth_middleware(
    State(pool): State<Pool<Sqlite>>,
    mut request: AxumRequest,
    next: Next,
) -> Result<Response, StatusCode> {
    // 获取 Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok());

    // 检查是否存在 Bearer token
    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            header.strip_prefix("Bearer ").unwrap()
        }
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    // 验证 JWT token
    let auth_service = AuthService::new(pool);
    match auth_service.validate_token(token).await {
        Ok(user_id) => {
            // 将用户ID添加到request扩展中，供后续处理器使用
            request.extensions_mut().insert(user_id);
            Ok(next.run(request).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}