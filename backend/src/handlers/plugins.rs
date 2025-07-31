use axum::{
    extract::{Path, State},
    response::Json,
    http::StatusCode,
};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;
use chrono::Utc;

use crate::models::{Plugin, InstallPluginRequest, PluginConfigRequest};

pub struct PluginHandler;

impl PluginHandler {
    /// GET /api/v1/plugins - 获取已安装插件列表
    pub async fn list_plugins(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<Vec<Plugin>>, StatusCode> {
        let plugins = sqlx::query_as::<_, Plugin>(
            "SELECT id, name, version, enabled, config, installed_at, updated_at FROM plugins ORDER BY name"
        )
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(plugins))
    }

    /// POST /api/v1/plugins/install - 安装插件
    pub async fn install_plugin(
        State(pool): State<Pool<Sqlite>>,
        Json(request): Json<InstallPluginRequest>,
    ) -> Result<Json<Plugin>, StatusCode> {
        // TODO: 实际的插件安装逻辑
        // 这里应该包括：
        // 1. 验证插件文件
        // 2. 解析插件元数据
        // 3. 检查权限和依赖
        // 4. 安装插件到系统

        let plugin_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let plugin = Plugin {
            id: plugin_id.clone(),
            name: request.name.clone(),
            version: "1.0.0".to_string(), // 从插件元数据获取
            enabled: false,
            config: None,
            installed_at: now,
            updated_at: now,
        };

        sqlx::query(
            r#"
            INSERT INTO plugins (id, name, version, enabled, config, installed_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
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
}