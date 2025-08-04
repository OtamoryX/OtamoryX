use axum::{
    middleware as axum_middleware,
    routing::{delete, get, post, put},
    Router,
};
use image::imageops::thumbnail;
use tower_http::cors::CorsLayer;
use tracing::info;

mod config;
mod database;
mod handlers;
mod middleware;
mod models;
mod services;
mod utils;

use handlers::{
    ai, archives, auth, cache, categories, filesystem, health, plugins, progress, search, settings,
    tags, users,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt().with_target(false).init();

    info!("Starting OtamoryX server v{}", env!("CARGO_PKG_VERSION"));

    // 初始化数据库连接池
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:otamoryx.db".to_string());
    let pool = database::create_pool(&database_url).await?;

    // 公开路由（不需要认证）
    let open_routes = Router::new()
        // 健康检查
        .route("/health", get(health::health_check))
        // 系统初始化
        .route("/api/v1/system/status", get(auth::get_system_status))
        .route("/api/v1/system/health", get(health::system_health))
        .route("/api/v1/system/initialize", post(auth::initialize_system))
        // 认证相关
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/logout", post(auth::logout));

    // 需要认证的路由（普通用户权限）
    let protected_routes = Router::new()
        // 漫画管理
        .route("/api/v1/archives", get(archives::get_archives))
        .route(
            "/api/v1/archives/{id}/thumbnail",
            get(archives::get_archive_thumbnail),
        )
        .route(
            "/api/v1/archives/random",
            get(archives::get_random_archives),
        )
        .route("/api/v1/archives/{id}", get(archives::get_archive))
        .route(
            "/api/v1/archives/{id}/pages/{page}",
            get(archives::get_archive_page),
        )
        .route(
            "/api/v1/archives/{id}/tags",
            post(archives::add_tag_to_archive),
        )
        .route(
            "/api/v1/archives/{id}/tags/{tag_id}",
            delete(archives::remove_tag_from_archive),
        )
        // 阅读进度
        .route(
            "/api/v1/archives/{id}/progress",
            get(progress::get_progress),
        )
        .route(
            "/api/v1/archives/{id}/progress",
            post(progress::update_progress),
        )
        // 搜索和标签
        .route("/api/v1/search", get(search::search_archives))
        .route("/api/v1/tags", get(tags::list_tags))
        // 分类管理
        .route("/api/v1/categories", get(categories::get_categories))
        .route(
            "/api/v1/categories/{id}/archives",
            get(categories::get_category_archives),
        )
        // 系统设置（只读）
        .route("/api/v1/settings", get(settings::get_settings))
        // 缓存状态查看
        .route("/api/v1/cache/status", get(cache::get_cache_status))
        // 用户权限查看
        .route(
            "/api/v1/users/me/permissions",
            get(users::UserHandler::get_my_permissions),
        )
        .layer(axum_middleware::from_fn_with_state(
            pool.clone(),
            middleware::auth::auth_middleware,
        ));

    // 需要管理员权限的路由
    let admin_routes = Router::new()
        // 用户管理（管理员专用）
        .route("/api/v1/users", get(users::UserHandler::list_users))
        .route("/api/v1/users", post(users::UserHandler::create_user))
        .route(
            "/api/v1/users/batch-delete",
            delete(users::UserHandler::batch_delete_users),
        )
        .route(
            "/api/v1/users/admins",
            get(users::UserHandler::get_admin_users),
        )
        .route("/api/v1/users/{id}", get(users::UserHandler::get_user))
        .route("/api/v1/users/{id}", put(users::UserHandler::update_user))
        .route(
            "/api/v1/users/{id}",
            delete(users::UserHandler::delete_user),
        )
        .route(
            "/api/v1/users/{id}/promote",
            put(users::UserHandler::promote_to_admin),
        )
        .route(
            "/api/v1/users/{id}/demote",
            put(users::UserHandler::demote_from_admin),
        )
        .route(
            "/api/v1/users/{id}/paths",
            get(users::UserHandler::get_user_paths),
        )
        .route(
            "/api/v1/users/{id}/paths",
            put(users::UserHandler::update_user_paths),
        )
        // 系统统计（管理员专用）
        .route(
            "/api/v1/system/stats",
            get(users::UserHandler::get_system_stats),
        )
        // 系统管理操作
        .route("/api/v1/settings", put(settings::update_settings))
        .route("/api/v1/settings/scan", post(settings::trigger_scan))
        .route(
            "/api/v1/archives/batch-delete",
            delete(archives::batch_delete_archives),
        )
        // 分类管理（创建、修改、删除）
        .route("/api/v1/categories", post(categories::create_category))
        .route(
            "/api/v1/categories/dynamic",
            post(categories::create_dynamic_category),
        )
        .route(
            "/api/v1/categories/prune",
            delete(categories::prune_empty_categories),
        )
        .route("/api/v1/categories/{id}", put(categories::update_category))
        .route(
            "/api/v1/categories/{id}",
            delete(categories::delete_category),
        )
        .route(
            "/api/v1/categories/{id}/archives",
            post(categories::add_archives_to_category),
        )
        .route(
            "/api/v1/categories/{id}/archives",
            delete(categories::remove_archives_from_category),
        )
        .route(
            "/api/v1/categories/{id}/archives/batch-delete",
            delete(categories::batch_delete_category_archives),
        )
        // 标签管理
        .route("/api/v1/tags/prune", delete(tags::prune_unused_tags))
        .route(
            "/api/v1/tags/{id}/archives/batch-delete",
            delete(tags::batch_delete_tag_archives),
        )
        // 插件管理
        .route("/api/v1/plugins", get(plugins::PluginHandler::list_plugins))
        .route(
            "/api/v1/plugins/install",
            post(plugins::PluginHandler::install_plugin),
        )
        .route(
            "/api/v1/plugins/{id}/toggle",
            put(plugins::PluginHandler::toggle_plugin),
        )
        .route(
            "/api/v1/plugins/{id}/config",
            put(plugins::PluginHandler::configure_plugin),
        )
        // AI自动标签
        .route("/api/v1/settings/ai", get(ai::AIHandler::get_ai_settings))
        .route(
            "/api/v1/settings/ai",
            put(ai::AIHandler::update_ai_settings),
        )
        .route("/api/v1/ai/status", get(ai::AIHandler::get_ai_status))
        .route(
            "/api/v1/ai/control",
            put(ai::AIHandler::control_ai_processing),
        )
        .route("/api/v1/ai/tags/review", post(tags::review_ai_tags))
        // 缓存管理
        .route("/api/v1/cache/configure", post(cache::configure_cache))
        .route("/api/v1/cache/clear", delete(cache::clear_cache))
        .route(
            "/api/v1/cache/recommendations",
            get(cache::get_cache_recommendations),
        )
        // 文件系统浏览（管理员专用）
        .route(
            "/api/v1/filesystem/directories",
            get(filesystem::list_directories),
        );

    // Windows专用路由
    #[cfg(windows)]
    let admin_routes =
        admin_routes.route("/api/v1/filesystem/drives", get(filesystem::list_drives));

    let admin_routes = admin_routes
        .layer(axum_middleware::from_fn_with_state(
            pool.clone(),
            middleware::admin::admin_middleware,
        ))
        .layer(axum_middleware::from_fn_with_state(
            pool.clone(),
            middleware::auth::auth_middleware,
        ));

    // 合并路由
    let app = Router::new()
        .merge(open_routes)
        .merge(protected_routes)
        .merge(admin_routes)
        .with_state(pool)
        .layer(CorsLayer::very_permissive());

    // 启动服务器
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Server running on http://0.0.0.0:3000");

    axum::serve(listener, app).await?;

    Ok(())
}
