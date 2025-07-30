use axum::{
    routing::{get, post, put},
    Router,
    Json,
    http::StatusCode,
};
use serde::Serialize;
use tower_http::cors::CorsLayer;
use tracing::info;

mod config;
mod models;
mod handlers;
mod services;
mod utils;
mod database;

use config::Config;
use handlers::{archives, search, settings, progress, auth, categories};

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    timestamp: String,
}

async fn health_check() -> Result<Json<HealthResponse>, StatusCode> {
    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_target(false)
        .init();

    info!("Starting OtamoryX server v{}", env!("CARGO_PKG_VERSION"));

    // 构建路由
    let app = Router::new()
        // 健康检查
        .route("/health", get(health_check))
        
        // 系统初始化
        .route("/api/v1/system/status", get(auth::get_system_status))
        .route("/api/v1/system/initialize", post(auth::initialize_system))
        
        // 认证相关
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/logout", post(auth::logout))
        
        // 漫画管理
        .route("/api/v1/archives", get(archives::get_archives))
        .route("/api/v1/archives/:id", get(archives::get_archive))
        .route("/api/v1/archives/:id/thumbnail", get(archives::get_archive_thumbnail))
        .route("/api/v1/archives/:id/pages/:page", get(archives::get_archive_page))
        
        // 阅读进度
        .route("/api/v1/archives/:id/progress", get(progress::get_progress))
        .route("/api/v1/archives/:id/progress", post(progress::update_progress))
        
        // 搜索和标签
        .route("/api/v1/search", get(search::search_archives))
        .route("/api/v1/tags", get(search::get_tags))
        
        // 分类管理
        .route("/api/v1/categories", get(categories::get_categories))
        .route("/api/v1/categories", post(categories::create_category))
        .route("/api/v1/categories/dynamic", post(categories::create_dynamic_category))
        .route("/api/v1/categories/:id", put(categories::update_category))
        .route("/api/v1/categories/:id", axum::routing::delete(categories::delete_category))
        .route("/api/v1/categories/:id/archives", get(categories::get_category_archives))
        .route("/api/v1/categories/:id/archives", post(categories::add_archives_to_category))
        .route("/api/v1/categories/:id/archives", axum::routing::delete(categories::remove_archives_from_category))
        
        // 系统设置
        .route("/api/v1/settings", get(settings::get_settings))
        .route("/api/v1/settings", put(settings::update_settings))
        
        .layer(CorsLayer::very_permissive());

    // 启动服务器
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Server running on http://0.0.0.0:3000");
    
    axum::serve(listener, app).await?;

    Ok(())
}
