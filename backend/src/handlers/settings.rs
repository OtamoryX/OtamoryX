use crate::models::SystemSettings;
use crate::services::archive_processing_service::ArchiveProcessingService;
use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use sqlx::{Pool, Sqlite};
use std::path::Path;
use tracing::{info, warn};

pub async fn get_settings(
    State(pool): State<Pool<Sqlite>>,
) -> Result<Json<SystemSettings>, StatusCode> {
    let row = sqlx::query!(
        "SELECT comics_path, supported_formats, max_file_size, image_cache_size, scan_on_startup 
         FROM system_settings 
         WHERE id = 'default'"
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting settings: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let settings = if let Some(settings_row) = row {
        let supported_formats: Vec<String> = serde_json::from_str(&settings_row.supported_formats)
            .unwrap_or_else(|_| SystemSettings::default().supported_formats);

        SystemSettings {
            comics_path: settings_row.comics_path,
            supported_formats,
            max_file_size: settings_row.max_file_size as u64,
            image_cache_size: settings_row.image_cache_size as u64,
            scan_on_startup: settings_row.scan_on_startup,
        }
    } else {
        // 如果没有设置记录，返回默认设置并插入到数据库
        let default_settings = SystemSettings::default();
        let _ = insert_default_settings(&pool, &default_settings).await;
        default_settings
    };

    Ok(Json(settings))
}

pub async fn update_settings(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(_user_id): axum::extract::Extension<String>, // 需要管理员权限
    Json(settings): Json<SystemSettings>,
) -> Result<StatusCode, StatusCode> {
    // 获取当前设置以检查路径是否有变化
    let current_settings = get_current_settings(&pool).await.ok();
    let supported_formats_json =
        serde_json::to_string(&settings.supported_formats).map_err(|_| StatusCode::BAD_REQUEST)?;

    let now = chrono::Utc::now();
    let max_file_size = settings.max_file_size as i64;
    let image_cache_size = settings.image_cache_size as i64;

    sqlx::query!(
        r#"
        INSERT INTO system_settings (id, comics_path, supported_formats, max_file_size, image_cache_size, scan_on_startup, updated_at)
        VALUES ('default', ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            comics_path = excluded.comics_path,
            supported_formats = excluded.supported_formats,
            max_file_size = excluded.max_file_size,
            image_cache_size = excluded.image_cache_size,
            scan_on_startup = excluded.scan_on_startup,
            updated_at = excluded.updated_at
        "#,
        settings.comics_path,
        supported_formats_json,
        max_file_size,
        image_cache_size,
        settings.scan_on_startup,
        now
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error updating settings: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    tracing::info!("Updated system settings successfully");

    // 检查漫画路径是否有变化，如果有则触发重新扫描
    if let Some(ref current) = current_settings {
        if current.comics_path != settings.comics_path {
            info!(
                "Comics path changed from '{}' to '{}', triggering rescan",
                current.comics_path, settings.comics_path
            );

            let new_path = Path::new(&settings.comics_path);
            if new_path.exists() && new_path.is_dir() {
                let _processing_service = ArchiveProcessingService::new(pool.clone());

                // 异步触发扫描，不阻塞设置保存
                let path_for_scan = settings.comics_path.clone();
                let scan_pool = pool.clone();
                tokio::spawn(async move {
                    let service = ArchiveProcessingService::new(scan_pool);
                    match service.scan_directory(&path_for_scan).await {
                        Ok(new_archives) => {
                            info!(
                                "Automatic rescan completed: {} new archives found",
                                new_archives.len()
                            );
                        }
                        Err(e) => {
                            warn!("Automatic rescan failed: {}", e);
                        }
                    }
                });
            } else {
                warn!(
                    "New comics path does not exist or is not a directory: {}",
                    settings.comics_path
                );
            }
        }
    }

    Ok(StatusCode::OK)
}

async fn insert_default_settings(
    pool: &Pool<Sqlite>,
    settings: &SystemSettings,
) -> Result<(), sqlx::Error> {
    let supported_formats_json =
        serde_json::to_string(&settings.supported_formats).map_err(|e| {
            sqlx::Error::ColumnDecode {
                index: "supported_formats".to_string(),
                source: Box::new(e),
            }
        })?;
    let now = chrono::Utc::now();
    let max_file_size = settings.max_file_size as i64;
    let image_cache_size = settings.image_cache_size as i64;

    sqlx::query!(
        "INSERT OR IGNORE INTO system_settings 
         (id, comics_path, supported_formats, max_file_size, image_cache_size, scan_on_startup, created_at, updated_at)
         VALUES ('default', ?, ?, ?, ?, ?, ?, ?)",
        settings.comics_path,
        supported_formats_json,
        max_file_size,
        image_cache_size,
        settings.scan_on_startup,
        now,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn get_current_settings(pool: &Pool<Sqlite>) -> Result<SystemSettings, StatusCode> {
    let row = sqlx::query!(
        "SELECT comics_path, supported_formats, max_file_size, image_cache_size, scan_on_startup 
         FROM system_settings 
         WHERE id = 'default'"
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting current settings: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some(settings_row) = row {
        let supported_formats: Vec<String> = serde_json::from_str(&settings_row.supported_formats)
            .unwrap_or_else(|_| SystemSettings::default().supported_formats);

        Ok(SystemSettings {
            comics_path: settings_row.comics_path,
            supported_formats,
            max_file_size: settings_row.max_file_size as u64,
            image_cache_size: settings_row.image_cache_size as u64,
            scan_on_startup: settings_row.scan_on_startup,
        })
    } else {
        Ok(SystemSettings::default())
    }
}

#[derive(Serialize)]
pub struct ScanResponse {
    message: String,
    new_archives_count: usize,
}

/// 手动触发漫画库扫描
/// POST /api/v1/settings/scan
pub async fn trigger_scan(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(_user_id): axum::extract::Extension<String>, // 需要管理员权限
) -> Result<Json<ScanResponse>, StatusCode> {
    info!("Manual scan triggered by admin");

    // 获取当前的漫画库路径
    let settings = get_current_settings(&pool).await?;
    let comics_path = Path::new(&settings.comics_path);

    if !comics_path.exists() {
        warn!("Comics path does not exist: {}", settings.comics_path);
        return Err(StatusCode::BAD_REQUEST);
    }

    if !comics_path.is_dir() {
        warn!("Comics path is not a directory: {}", settings.comics_path);
        return Err(StatusCode::BAD_REQUEST);
    }

    let processing_service = ArchiveProcessingService::new(pool);

    match processing_service.scan_directory(comics_path).await {
        Ok(new_archives) => {
            let count = new_archives.len();
            info!(
                "Manual scan completed successfully: {} new archives found",
                count
            );

            Ok(Json(ScanResponse {
                message: format!("扫描完成，发现 {} 个新漫画", count),
                new_archives_count: count,
            }))
        }
        Err(e) => {
            warn!("Manual scan failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
