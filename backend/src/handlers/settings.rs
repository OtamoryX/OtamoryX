use crate::middleware::auth::AuthInfo;
use crate::models::{ScanSettings, SystemSettings};
use crate::services::{ArchiveProcessingService, FileMonitorService};
use axum::{extract::State, http::StatusCode, Extension, Json};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

pub async fn get_settings(
    State(pool): State<Pool<Sqlite>>,
) -> Result<Json<SystemSettings>, StatusCode> {
    let row = sqlx::query!(
        "SELECT comics_path, supported_formats, max_file_size, image_cache_size, image_cache_path, scan_on_startup, scan_settings
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

        let scan_settings: crate::models::ScanSettings =
            serde_json::from_str(&settings_row.scan_settings).unwrap_or_default();

        SystemSettings {
            comics_path: settings_row.comics_path,
            supported_formats,
            max_file_size: settings_row.max_file_size as u64,
            image_cache_size: settings_row.image_cache_size as u64,
            image_cache_path: settings_row.image_cache_path,
            scan_on_startup: settings_row.scan_on_startup,
            scan_settings,
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
    Extension(file_monitor): Extension<Arc<FileMonitorService>>,
    axum::extract::Extension(_auth): axum::extract::Extension<AuthInfo>, // 需要管理员权限
    Json(settings): Json<SystemSettings>,
) -> Result<StatusCode, StatusCode> {
    // 获取当前设置以检查路径是否有变化
    let current_settings = get_current_settings(&pool).await.ok();
    let supported_formats_json =
        serde_json::to_string(&settings.supported_formats).map_err(|_| StatusCode::BAD_REQUEST)?;

    let now = chrono::Utc::now();
    let max_file_size = settings.max_file_size as i64;
    let image_cache_size = settings.image_cache_size as i64;

    let scan_settings_json =
        serde_json::to_string(&settings.scan_settings).map_err(|_| StatusCode::BAD_REQUEST)?;

    sqlx::query!(
        r#"
        INSERT INTO system_settings (id, comics_path, supported_formats, max_file_size, image_cache_size, image_cache_path, scan_on_startup, scan_settings, updated_at)
        VALUES ('default', ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            comics_path = excluded.comics_path,
            supported_formats = excluded.supported_formats,
            max_file_size = excluded.max_file_size,
            image_cache_size = excluded.image_cache_size,
            image_cache_path = excluded.image_cache_path,
            scan_on_startup = excluded.scan_on_startup,
            scan_settings = excluded.scan_settings,
            updated_at = excluded.updated_at
        "#,
        settings.comics_path,
        supported_formats_json,
        max_file_size,
        image_cache_size,
        settings.image_cache_path,
        settings.scan_on_startup,
        scan_settings_json,
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

    // 更新文件监控服务的设置
    if let Err(e) = file_monitor
        .update_settings(&settings.comics_path, settings.scan_settings.clone())
        .await
    {
        tracing::error!("Failed to update file monitor settings: {}", e);
        // 不返回错误，因为设置已经保存成功，只是监控更新失败
        tracing::warn!("File monitoring may not reflect the latest settings");
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

    let scan_settings_json =
        serde_json::to_string(&settings.scan_settings).map_err(|e| sqlx::Error::ColumnDecode {
            index: "scan_settings".to_string(),
            source: Box::new(e),
        })?;

    sqlx::query!(
        "INSERT OR IGNORE INTO system_settings 
         (id, comics_path, supported_formats, max_file_size, image_cache_size, image_cache_path, scan_on_startup, scan_settings, created_at, updated_at)
         VALUES ('default', ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        settings.comics_path,
        supported_formats_json,
        max_file_size,
        image_cache_size,
        settings.image_cache_path,
        settings.scan_on_startup,
        scan_settings_json,
        now,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_current_settings(pool: &Pool<Sqlite>) -> Result<SystemSettings, StatusCode> {
    let row = sqlx::query!(
        "SELECT comics_path, supported_formats, max_file_size, image_cache_size, image_cache_path, scan_on_startup, scan_settings
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

        let scan_settings: crate::models::ScanSettings =
            serde_json::from_str(&settings_row.scan_settings).unwrap_or_default();

        Ok(SystemSettings {
            comics_path: settings_row.comics_path,
            supported_formats,
            max_file_size: settings_row.max_file_size as u64,
            image_cache_size: settings_row.image_cache_size as u64,
            image_cache_path: settings_row.image_cache_path,
            scan_on_startup: settings_row.scan_on_startup,
            scan_settings,
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

/// 手动触发漫画库扫描（异步后台执行）
/// POST /api/v1/settings/scan
pub async fn trigger_scan(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(_auth): axum::extract::Extension<AuthInfo>, // 需要管理员权限
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

    // 异步后台执行扫描，立即返回响应
    let scan_pool = pool.clone();
    let path_for_scan = settings.comics_path.clone();
    tokio::spawn(async move {
        let service = ArchiveProcessingService::new(scan_pool);
        match service.scan_directory(&path_for_scan).await {
            Ok(new_archives) => {
                info!(
                    "Background scan completed: {} new archives found",
                    new_archives.len()
                );
            }
            Err(e) => {
                warn!("Background scan failed: {}", e);
            }
        }
    });

    Ok(Json(ScanResponse {
        message: "扫描已启动，正在后台处理中...".to_string(),
        new_archives_count: 0,
    }))
}

#[derive(Deserialize)]
pub struct UpdateScanSettingsRequest {
    #[serde(rename = "scanSettings")]
    pub scan_settings: ScanSettings,
}

#[derive(Serialize)]
pub struct ScanSettingsResponse {
    #[serde(rename = "scanSettings")]
    pub scan_settings: ScanSettings,
    pub monitoring_status: bool,
}

/// 获取扫描设置
/// GET /api/v1/settings/scan-settings
pub async fn get_scan_settings(
    State(pool): State<Pool<Sqlite>>,
    Extension(file_monitor): Extension<Arc<FileMonitorService>>,
) -> Result<Json<ScanSettingsResponse>, StatusCode> {
    let settings = get_current_settings(&pool).await?;
    let monitoring_status = file_monitor.is_monitoring().await;

    Ok(Json(ScanSettingsResponse {
        scan_settings: settings.scan_settings,
        monitoring_status,
    }))
}

/// 更新扫描设置
/// POST /api/v1/settings/scan-settings
pub async fn update_scan_settings(
    State(pool): State<Pool<Sqlite>>,
    Extension(file_monitor): Extension<Arc<FileMonitorService>>,
    axum::extract::Extension(_auth): axum::extract::Extension<AuthInfo>, // 需要管理员权限
    Json(request): Json<UpdateScanSettingsRequest>,
) -> Result<Json<ScanSettingsResponse>, StatusCode> {
    info!("Updating scan settings: {:?}", request.scan_settings);

    // 获取当前设置
    let mut current_settings = get_current_settings(&pool).await?;
    current_settings.scan_settings = request.scan_settings.clone();

    // 更新数据库中的设置
    let scan_settings_json =
        serde_json::to_string(&request.scan_settings).map_err(|_| StatusCode::BAD_REQUEST)?;
    let supported_formats_json = serde_json::to_string(&current_settings.supported_formats)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let now = chrono::Utc::now();
    let max_file_size = current_settings.max_file_size as i64;
    let image_cache_size = current_settings.image_cache_size as i64;

    sqlx::query!(
        r#"
        INSERT INTO system_settings (id, comics_path, supported_formats, max_file_size, image_cache_size, scan_on_startup, scan_settings, updated_at)
        VALUES ('default', ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            scan_settings = excluded.scan_settings,
            updated_at = excluded.updated_at
        "#,
        current_settings.comics_path,
        supported_formats_json,
        max_file_size,
        image_cache_size,
        current_settings.scan_on_startup,
        scan_settings_json,
        now
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error updating scan settings: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // 更新文件监控服务的设置
    if let Err(e) = file_monitor
        .update_settings(&current_settings.comics_path, request.scan_settings.clone())
        .await
    {
        tracing::error!("Failed to update file monitor settings: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let monitoring_status = file_monitor.is_monitoring().await;

    info!(
        "Scan settings updated successfully, monitoring status: {}",
        monitoring_status
    );

    Ok(Json(ScanSettingsResponse {
        scan_settings: request.scan_settings,
        monitoring_status,
    }))
}
