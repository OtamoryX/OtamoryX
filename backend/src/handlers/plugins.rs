use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use crate::models::{Plugin, PluginConfigRequest};

pub struct PluginHandler;

impl PluginHandler {
    /// GET /api/v1/plugins - 获取已安装插件列表
    pub async fn list_plugins(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<Vec<Plugin>>, StatusCode> {
        let plugins = sqlx::query_as::<_, Plugin>(
            "SELECT id, name, version, enabled, config, created_at AS installed_at, updated_at FROM plugins ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(plugins))
    }

    /// POST /api/v1/plugins/install - 安装插件
    pub async fn install_plugin(
        State(pool): State<Pool<Sqlite>>,
        mut multipart: Multipart,
    ) -> Result<Json<Plugin>, StatusCode> {
        // TODO: 实际的插件安装逻辑
        // 这里应该包括：
        // 1. 验证插件文件
        // 2. 解析插件元数据
        // 3. 检查权限和依赖
        // 4. 安装插件到系统

        let mut uploaded_filename: Option<String> = None;
        let mut has_plugin_payload = false;

        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?
        {
            if field.name() == Some("plugin") {
                uploaded_filename = field.file_name().map(|name| name.to_string());
                let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                if bytes.is_empty() {
                    return Err(StatusCode::BAD_REQUEST);
                }
                has_plugin_payload = true;
                break;
            }
        }

        if !has_plugin_payload {
            return Err(StatusCode::BAD_REQUEST);
        }

        let plugin_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let plugin_name = uploaded_filename
            .as_deref()
            .map(infer_plugin_name)
            .unwrap_or_else(|| format!("plugin-{}", &plugin_id[..8]));

        let plugin = Plugin {
            id: plugin_id.clone(),
            name: plugin_name,
            version: "1.0.0".to_string(), // 从插件元数据获取
            enabled: false,
            config: None,
            installed_at: now,
            updated_at: now,
        };

        sqlx::query(
            r#"
            INSERT INTO plugins (id, name, version, enabled, config, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&plugin.id)
        .bind(&plugin.name)
        .bind(&plugin.version)
        .bind(plugin.enabled)
        .bind(&plugin.config)
        .bind(plugin.installed_at)
        .bind(plugin.updated_at)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(plugin))
    }

    /// PUT /api/v1/plugins/:id/toggle - 启用/禁用插件
    pub async fn toggle_plugin(
        State(pool): State<Pool<Sqlite>>,
        Path(plugin_id): Path<String>,
    ) -> Result<StatusCode, StatusCode> {
        // 获取当前状态
        let current_enabled = sqlx::query!("SELECT enabled FROM plugins WHERE id = ?", plugin_id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?
            .enabled;

        // 切换状态
        let new_enabled = !current_enabled;

        sqlx::query("UPDATE plugins SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(new_enabled)
            .bind(Utc::now())
            .bind(&plugin_id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // TODO: 实际启用/禁用插件的逻辑
        // 这里应该包括：
        // 1. 如果启用：加载插件，初始化，注册钩子
        // 2. 如果禁用：卸载插件，清理资源

        Ok(StatusCode::OK)
    }

    /// PUT /api/v1/plugins/:id/config - 配置插件
    pub async fn configure_plugin(
        State(pool): State<Pool<Sqlite>>,
        Path(plugin_id): Path<String>,
        Json(request): Json<PluginConfigRequest>,
    ) -> Result<StatusCode, StatusCode> {
        // 验证插件存在
        let _plugin = sqlx::query!("SELECT id FROM plugins WHERE id = ?", plugin_id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        // TODO: 验证配置格式是否符合插件要求

        // 更新配置
        sqlx::query("UPDATE plugins SET config = ?, updated_at = ? WHERE id = ?")
            .bind(&request.config)
            .bind(Utc::now())
            .bind(&plugin_id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // TODO: 如果插件已启用，应该重新加载配置

        Ok(StatusCode::OK)
    }

    /// DELETE /api/v1/plugins/:id - 卸载插件
    pub async fn uninstall_plugin(
        State(pool): State<Pool<Sqlite>>,
        Path(plugin_id): Path<String>,
    ) -> Result<StatusCode, StatusCode> {
        let result = sqlx::query("DELETE FROM plugins WHERE id = ?")
            .bind(&plugin_id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if result.rows_affected() == 0 {
            return Err(StatusCode::NOT_FOUND);
        }

        Ok(StatusCode::NO_CONTENT)
    }
}

fn infer_plugin_name(filename: &str) -> String {
    let base = filename
        .strip_suffix(".tar.gz")
        .or_else(|| filename.strip_suffix(".tgz"))
        .unwrap_or(filename);

    let mut normalized = String::with_capacity(base.len());
    let mut prev_dash = false;

    for ch in base.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '-'
        };

        if mapped == '-' {
            if !prev_dash {
                normalized.push('-');
                prev_dash = true;
            }
        } else {
            normalized.push(mapped);
            prev_dash = false;
        }
    }

    let trimmed = normalized.trim_matches('-');
    if trimmed.is_empty() {
        "plugin".to_string()
    } else {
        trimmed.to_string()
    }
}
