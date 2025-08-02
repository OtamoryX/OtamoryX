use axum::{
    extract::State,
    response::Json, 
    http::StatusCode
};
use chrono::Utc;
use sqlx::{Pool, Sqlite};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use crate::models::{HealthStatus, SystemHealth, DatabaseHealth, ServicesHealth, SystemMetrics};

/// GET /health - 基础健康检查端点
pub async fn health_check() -> Result<Json<HealthStatus>, StatusCode> {
    let health = HealthStatus {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
    };
    
    Ok(Json(health))
}

/// GET /api/v1/system/health - 详细系统健康检查
pub async fn system_health(
    State(pool): State<Pool<Sqlite>>,
) -> Result<Json<SystemHealth>, StatusCode> {
    let start_time = Instant::now();
    
    // 数据库健康检查
    let db_health = check_database_health(&pool).await?;
    
    // 服务健康检查
    let services_health = check_services_health().await;
    
    // 系统指标收集
    let system_metrics = collect_system_metrics(&pool).await?;
    
    let overall_status = if db_health.status == "healthy" && 
                           services_health.cache_service == "healthy" &&
                           services_health.archive_service == "healthy" &&
                           services_health.auth_service == "healthy" {
        "healthy"
    } else {
        "degraded"
    };
    
    let health = SystemHealth {
        status: overall_status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
        database: db_health,
        services: services_health,
        system: system_metrics,
    };
    
    Ok(Json(health))
}

async fn check_database_health(pool: &Pool<Sqlite>) -> Result<DatabaseHealth, StatusCode> {
    let start = Instant::now();
    
    // 执行简单查询测试数据库连接
    let result = sqlx::query!("SELECT 1 as test")
        .fetch_one(pool)
        .await;
    
    let response_time = start.elapsed().as_millis() as u64;
    
    match result {
        Ok(_) => Ok(DatabaseHealth {
            status: "healthy".to_string(),
            connection_count: 1, // SQLite doesn't have connection pooling like PostgreSQL
            response_time_ms: response_time,
        }),
        Err(e) => {
            tracing::error!("Database health check failed: {}", e);
            Ok(DatabaseHealth {
                status: "unhealthy".to_string(),
                connection_count: 0,
                response_time_ms: response_time,
            })
        }
    }
}

async fn check_services_health() -> ServicesHealth {
    // 在实际实现中，这里会检查各个服务的状态
    // 目前返回基础状态
    ServicesHealth {
        cache_service: "healthy".to_string(),
        archive_service: "healthy".to_string(),
        auth_service: "healthy".to_string(),
    }
}

async fn collect_system_metrics(pool: &Pool<Sqlite>) -> Result<SystemMetrics, StatusCode> {
    // 获取各种统计数据
    let archives_count = sqlx::query!("SELECT COUNT(*) as count FROM archives")
        .fetch_one(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .count as u32;
    
    let users_count = sqlx::query!("SELECT COUNT(*) as count FROM users")
        .fetch_one(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .count as u32;
    
    let categories_count = sqlx::query!("SELECT COUNT(*) as count FROM categories")
        .fetch_one(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .count as u32;
    
    let tags_count = sqlx::query!("SELECT COUNT(*) as count FROM tags")
        .fetch_one(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .count as u32;
    
    // 计算系统运行时间（简化版本）
    let uptime = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    // 获取内存使用情况（简化版本）
    let memory_usage = get_memory_usage();
    
    Ok(SystemMetrics {
        total_archives: archives_count,
        total_users: users_count,
        total_categories: categories_count,
        total_tags: tags_count,
        uptime_seconds: uptime,
        memory_usage_mb: memory_usage,
    })
}

fn get_memory_usage() -> u64 {
    // 简化的内存使用获取
    // 在实际实现中，可以使用 sysinfo 或其他系统监控库
    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = std::fs::read_to_string("/proc/self/status") {
            for line in contents.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb / 1024; // 转换为MB
                        }
                    }
                }
            }
        }
    }
    
    // 默认返回值
    0
}