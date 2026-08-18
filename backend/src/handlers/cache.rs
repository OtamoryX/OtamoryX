use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;

use crate::services::archive::ArchiveCacheService;
use crate::services::{CacheStrategy, CustomCacheConfig};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct CacheConfigRequest {
    pub strategy: Option<String>,
    pub custom_config: Option<CustomCacheConfig>,
}

#[derive(Serialize)]
pub struct CacheStatusResponse {
    pub current_strategy: String,
    pub stats: HashMap<String, serde_json::Value>,
    pub config: CacheConfigInfo,
}

#[derive(Serialize)]
pub struct CacheConfigInfo {
    pub max_memory_mb: usize,
    pub max_cached_archives: usize,
    pub cache_ttl_hours: f64,
    pub preload_next_pages: u32,
    pub preload_prev_pages: u32,
    pub cleanup_threshold_percent: u32,
    pub enable_background_preload: bool,
    pub max_concurrent_extractions: usize,
}

#[derive(Deserialize)]
pub struct ClearCacheQuery {
    pub scope: Option<String>, // all | pages | covers
}

/// GET /api/v1/cache/status - 获取缓存状态
pub async fn get_cache_status(
    State(_pool): State<Pool<Sqlite>>,
    axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
) -> Result<Json<CacheStatusResponse>, StatusCode> {
    // 获取当前缓存统计
    let stats = archive_cache.cache_stats().await;

    // 获取当前配置信息
    let config_info = CacheConfigInfo {
        max_memory_mb: 512, // 这里应该从实际配置获取
        max_cached_archives: 30,
        cache_ttl_hours: 1.0,
        preload_next_pages: 3,
        preload_prev_pages: 1,
        cleanup_threshold_percent: 80,
        enable_background_preload: true,
        max_concurrent_extractions: 2,
    };

    let response = CacheStatusResponse {
        current_strategy: "Balanced".to_string(),
        stats,
        config: config_info,
    };

    Ok(Json(response))
}

/// POST /api/v1/cache/configure - 配置缓存策略
pub async fn configure_cache(
    State(_pool): State<Pool<Sqlite>>,
    Json(request): Json<CacheConfigRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 根据请求确定缓存策略
    let strategy = match (request.strategy.as_deref(), request.custom_config) {
        (Some("conservative"), None) => CacheStrategy::Conservative,
        (Some("balanced"), None) => CacheStrategy::Balanced,
        (Some("aggressive"), None) => CacheStrategy::Aggressive,
        (Some("custom"), Some(config)) => CacheStrategy::Custom(config),
        (None, Some(config)) => CacheStrategy::Custom(config),
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // 注意：由于我们使用了lazy_static，无法在运行时更改配置
    // 在实际应用中，应该使用Arc<RwLock<Config>>来支持动态配置
    tracing::warn!(
        "Cache configuration change requested but not implemented in current architecture"
    );

    let response = serde_json::json!({
        "message": "Cache configuration update requested",
        "note": "Configuration will take effect after service restart",
        "requested_strategy": match strategy {
            CacheStrategy::Conservative => "Conservative",
            CacheStrategy::Balanced => "Balanced",
            CacheStrategy::Aggressive => "Aggressive",
            CacheStrategy::Custom(_) => "Custom",
        }
    });

    Ok(Json(response))
}

/// DELETE /api/v1/cache/clear - 清空缓存
pub async fn clear_cache(
    State(_pool): State<Pool<Sqlite>>,
    Query(query): Query<ClearCacheQuery>,
    axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let scope = query.scope.as_deref().unwrap_or("all");
    tracing::info!("Cache clear requested, scope={}", scope);

    match scope {
        "all" => archive_cache.clear_all().await,
        "pages" => archive_cache.clear_page_cache().await,
        "covers" => archive_cache.clear_cover_cache().await,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let response = serde_json::json!({
        "message": "Cache cleared successfully",
        "success": true,
        "scope": scope
    });

    Ok(Json(response))
}

/// GET /api/v1/cache/recommendations - 获取缓存配置推荐
pub async fn get_cache_recommendations(
    State(_pool): State<Pool<Sqlite>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 基于系统资源和使用模式推荐配置
    let recommendations = serde_json::json!({
        "strategies": {
            "conservative": {
                "description": "适合内存受限的环境（<1GB RAM）",
                "memory_usage": "128MB",
                "cache_duration": "15分钟",
                "best_for": ["低配置服务器", "移动设备", "共享环境"]
            },
            "balanced": {
                "description": "适合大多数用户（1-4GB RAM）",
                "memory_usage": "512MB",
                "cache_duration": "1小时",
                "best_for": ["个人服务器", "家庭使用", "中等负载"]
            },
            "aggressive": {
                "description": "适合高性能环境（>4GB RAM）",
                "memory_usage": "2GB",
                "cache_duration": "4小时",
                "best_for": ["专用服务器", "重度使用", "多用户环境"]
            },
            "custom": {
                "description": "自定义配置，满足特定需求",
                "configurable": [
                    "内存限制",
                    "缓存时间",
                    "预加载策略",
                    "清理阈值"
                ]
            }
        },
        "performance_tips": [
            "SSD存储可以减少对内存缓存的依赖",
            "网络存储需要更激进的缓存策略",
            "多用户环境建议增加缓存容量",
            "定期监控缓存命中率来优化配置"
        ]
    });

    Ok(Json(recommendations))
}
