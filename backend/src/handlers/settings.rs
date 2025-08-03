use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use sqlx::{Pool, Sqlite};
use crate::models::SystemSettings;

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
    let supported_formats_json = serde_json::to_string(&settings.supported_formats)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

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
    Ok(StatusCode::OK)
}

async fn insert_default_settings(pool: &Pool<Sqlite>, settings: &SystemSettings) -> Result<(), sqlx::Error> {
    let supported_formats_json = serde_json::to_string(&settings.supported_formats)
        .map_err(|e| sqlx::Error::ColumnDecode { index: "supported_formats".to_string(), source: Box::new(e) })?;
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