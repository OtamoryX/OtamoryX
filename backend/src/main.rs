use axum::{
    http::Request,
    middleware::{self as axum_middleware, Next},
    routing::{delete, get, post, put},
    Router,
};
use database::DatabasePool;
use services::{
    bootstrap_seed_plugins, init_jwt_secret, spawn_ai_worker, ArchiveCacheConfig,
    ArchiveCacheService, ArchiveProcessingService, CacheStrategy, FileMonitorService,
};
use std::path::Path;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::info;

// 应用状态结构
#[derive(Clone)]
pub struct AppState {
    pub db: DatabasePool,
    pub file_monitor: Arc<FileMonitorService>,
    pub archive_cache: Arc<ArchiveCacheService>,
}

mod config;
mod database;
mod handlers;
mod middleware;
mod models;
mod plugins;
mod services;
mod utils;

use handlers::{
    ai, archives, auth, cache, categories, filesystem, health, opds, plugins as plugin_handlers,
    progress, search, settings, tags, users,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_target(true)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // 初始化 JWT 密钥（若环境未配置则为本进程自动生成）
    let _ = init_jwt_secret();

    info!("Starting OtamoryX server v{}", env!("CARGO_PKG_VERSION"));

    // 初始化数据库连接池
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./data/otamoryx.db".to_string());
    let pool = database::create_database_pool(&database_url).await?;
    let sqlite_pool = get_sqlite_pool(&pool)?;
    let seeded_count = bootstrap_seed_plugins(&sqlite_pool).await?;
    info!(
        "Plugin bootstrap completed on startup: {} seed plugins ensured",
        seeded_count
    );
    spawn_ai_worker(sqlite_pool.clone());

    // 初始化缓存服务（从数据库读取配置）
    let cache_strategy = CacheStrategy::Balanced; // 可以从配置文件或环境变量读取
    let cache_config =
        ArchiveCacheConfig::from_strategy_with_db(cache_strategy, &sqlite_pool).await;
    let archive_cache = Arc::new(ArchiveCacheService::new(cache_config));

    // 初始化文件监控服务
    let file_monitor = Arc::new(FileMonitorService::new(
        sqlite_pool.clone(),
        archive_cache.clone(),
    ));

    // 初始化登录限流器
    let rate_limiter = Arc::new(services::rate_limiter::LoginRateLimiter::new());
    rate_limiter.start_cleanup_task();

    // 启动文件监控（如果启用）
    if let Ok(settings) = handlers::settings::get_current_settings(&sqlite_pool).await {
        if settings.scan_settings.realtime_monitoring {
            info!("Starting file system monitoring on startup");
            if let Err(e) = file_monitor
                .start_monitoring(&settings.comics_path, settings.scan_settings)
                .await
            {
                tracing::warn!("Failed to start file monitoring: {}", e);
            }
        }

        if settings.scan_on_startup {
            let startup_scan_path = settings.comics_path.clone();
            if Path::new(&startup_scan_path).exists() && Path::new(&startup_scan_path).is_dir() {
                info!(
                    "scan_on_startup enabled, starting background library scan: {}",
                    startup_scan_path
                );
                let scan_pool = sqlite_pool.clone();
                tokio::spawn(async move {
                    let service = ArchiveProcessingService::new(scan_pool);
                    match service.scan_directory(&startup_scan_path).await {
                        Ok(new_archives) => {
                            info!(
                                "Startup scan completed: {} new archives discovered",
                                new_archives.len()
                            );
                        }
                        Err(e) => {
                            tracing::warn!("Startup scan failed: {}", e);
                        }
                    }
                });
            } else {
                tracing::warn!(
                    "scan_on_startup is enabled but comics path is invalid: {}",
                    startup_scan_path
                );
            }
        }
    }

    // 为了简化路由，我们将继续使用pool作为State，并将file_monitor作为扩展

    // 公开路由（不需要认证）
    let open_routes = Router::new()
        // 健康检查
        .route("/health", get(health::health_check))
        // 系统初始化
        .route("/api/v1/system/status", get(auth::get_system_status))
        .route("/api/v1/system/health", get(health::system_health))
        .route("/api/v1/system/initialize", post(auth::initialize_system))
        // 认证相关
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/logout", post(auth::logout));

    // 需要认证的路由（普通用户权限）
    let protected_routes = Router::new()
        // 注册新用户（需要认证）
        .route("/api/v1/auth/register", post(auth::register))
        .route(
            "/api/v1/archives/{id}/thumbnail",
            get(archives::get_archive_thumbnail),
        )
        .route(
            "/api/v1/archives/random",
            get(archives::get_random_archives),
        )
        .route(
            "/api/v1/archives/{id}",
            get(archives::get_archive).delete(archives::delete_archive),
        )
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
        .route(
            "/api/v1/archives/{id}/categories",
            get(categories::get_archive_categories),
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
        .route("/api/v1/progress/batch", post(progress::get_batch_progress))
        // 搜索和标签
        .route("/api/v1/search", get(search::search_archives))
        .route("/api/v1/tags", get(tags::list_tags).post(tags::create_tag))
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
        // OPDS feeds (require authentication for metadata protection)
        .route("/opds", get(opds::opds_root))
        .route("/opds/archives", get(opds::opds_archives))
        .route("/opds/search", get(opds::opds_search))
        .layer(axum_middleware::from_fn(middleware::auth::auth_middleware));

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
            "/api/v1/settings/scan-settings",
            get(settings::get_scan_settings),
        )
        .route(
            "/api/v1/settings/scan-settings",
            post(settings::update_scan_settings),
        )
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
        .route(
            "/api/v1/categories/{id}/archives/delete-preview",
            get(categories::preview_category_archive_deletion),
        )
        // 标签管理
        .route("/api/v1/tags/prune", delete(tags::prune_unused_tags))
        .route(
            "/api/v1/tags/{id}/archives/batch-delete",
            delete(tags::batch_delete_tag_archives),
        )
        // 插件管理
        .route(
            "/api/v1/plugins",
            get(plugin_handlers::PluginHandler::list_plugins),
        )
        .route(
            "/api/v1/plugins/install",
            post(plugin_handlers::PluginHandler::install_plugin),
        )
        .route(
            "/api/v1/plugins/{id}",
            get(plugin_handlers::PluginHandler::get_plugin)
                .delete(plugin_handlers::PluginHandler::uninstall_plugin),
        )
        .route(
            "/api/v1/plugins/{id}/toggle",
            put(plugin_handlers::PluginHandler::toggle_plugin),
        )
        .route(
            "/api/v1/plugins/{id}/config",
            put(plugin_handlers::PluginHandler::configure_plugin),
        )
        .route(
            "/api/v1/plugins/{id}/config/schema",
            get(plugin_handlers::PluginHandler::get_plugin_config_schema),
        )
        .route(
            "/api/v1/plugins/{id}/execute",
            post(plugin_handlers::PluginHandler::execute_plugin),
        )
        .route(
            "/api/v1/plugins/{id}/execute/{archive_id}",
            post(plugin_handlers::PluginHandler::execute_plugin_for_archive),
        )
        .route(
            "/api/v1/plugins/{id}/executions",
            get(plugin_handlers::PluginHandler::list_plugin_executions),
        )
        .route(
            "/api/v1/plugin-executions",
            get(plugin_handlers::PluginHandler::list_all_plugin_executions),
        )
        // AI自动标签
        .route("/api/v1/settings/ai", get(ai::AIHandler::get_ai_settings))
        .route(
            "/api/v1/settings/ai",
            put(ai::AIHandler::update_ai_settings),
        )
        .route(
            "/api/v1/settings/ai/test-connection",
            post(ai::AIHandler::test_ai_connection),
        )
        .route("/api/v1/ai/status", get(ai::AIHandler::get_ai_status))
        .route(
            "/api/v1/ai/control",
            put(ai::AIHandler::control_ai_processing),
        )
        .route(
            "/api/v1/ai/title-translations/backfill",
            post(ai::AIHandler::backfill_title_translations),
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
        .layer(axum_middleware::from_fn(
            middleware::admin::admin_middleware,
        ))
        .layer(axum_middleware::from_fn(middleware::auth::auth_middleware));

    // 合并路由
    let app = Router::new()
        .merge(open_routes)
        .merge(protected_routes)
        .merge(admin_routes)
        .with_state(get_sqlite_pool(&pool)?)
        .layer(axum_middleware::from_fn(
            move |mut req: Request<axum::body::Body>, next: Next| {
                let file_monitor = file_monitor.clone();
                let archive_cache = archive_cache.clone();
                let db_pool = pool.clone();
                let rate_limiter = rate_limiter.clone();
                async move {
                    req.extensions_mut().insert(file_monitor);
                    req.extensions_mut().insert(archive_cache);
                    req.extensions_mut().insert(db_pool);
                    req.extensions_mut().insert(rate_limiter);
                    next.run(req).await
                }
            },
        ))
        .layer({
            let origins = std::env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:5173".to_string());
            let origins: Vec<_> = origins
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::DELETE,
                ])
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                ])
        });

    // 启动服务器
    let bind_address = std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    info!("Server running on http://{}", bind_address);

    axum::serve(listener, app).await?;

    Ok(())
}

// Helper function to extract SQLite pool for services that haven't been updated yet
fn get_sqlite_pool(
    pool: &DatabasePool,
) -> Result<sqlx::Pool<sqlx::Sqlite>, Box<dyn std::error::Error>> {
    match pool {
        DatabasePool::Sqlite(sqlite_pool) => Ok(sqlite_pool.clone()),
        DatabasePool::Postgres(_) => {
            Err("Service requires SQLite but PostgreSQL pool provided".into())
        }
    }
}
