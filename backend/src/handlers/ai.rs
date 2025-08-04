use axum::{extract::State, http::StatusCode, response::Json};
use sqlx::{Pool, Sqlite};
use std::time::Duration;

use crate::models::{AIControlRequest, AIResourceLimits, AISchedule, AISettings, AIStatus};

pub struct AIHandler;

impl AIHandler {
    /// GET /api/v1/settings/ai - 获取AI配置
    pub async fn get_ai_settings(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<AISettings>, StatusCode> {
        // 从设置表获取AI配置
        let ai_config = sqlx::query!("SELECT value FROM settings WHERE key = 'ai_settings'")
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let settings = if let Some(config) = ai_config {
            serde_json::from_str(&config.value).unwrap_or_else(|_| Self::default_ai_settings())
        } else {
            Self::default_ai_settings()
        };

        Ok(Json(settings))
    }

    /// PUT /api/v1/settings/ai - 更新AI配置
    pub async fn update_ai_settings(
        State(pool): State<Pool<Sqlite>>,
        Json(settings): Json<AISettings>,
    ) -> Result<StatusCode, StatusCode> {
        let settings_json = serde_json::to_value(&settings).map_err(|_| StatusCode::BAD_REQUEST)?;

        sqlx::query!(
            r#"
            INSERT INTO settings (key, value, updated_at)
            VALUES ('ai_settings', ?, CURRENT_TIMESTAMP)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at
            "#,
            settings_json
        )
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // TODO: 应用新的AI设置到处理系统
        // 这里应该包括：
        // 1. 更新处理器配置
        // 2. 重启AI工作线程（如果需要）
        // 3. 验证模型可用性

        Ok(StatusCode::OK)
    }

    /// GET /api/v1/ai/status - 获取AI处理状态
    pub async fn get_ai_status(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<AIStatus>, StatusCode> {
        // 获取队列统计
        let queue_stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(CASE WHEN status = 'pending' THEN 1 END) as pending_count,
                COUNT(CASE WHEN status = 'processing' THEN 1 END) as processing_count,
                COUNT(CASE WHEN status = 'completed' AND DATE(created_at) = DATE('now') THEN 1 END) as completed_today,
                COUNT(CASE WHEN status = 'failed' AND DATE(created_at) = DATE('now') THEN 1 END) as failed_today
            FROM ai_processing_queue
            "#
        )
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // TODO: 计算平均处理时间和活跃模型
        let status = AIStatus {
            queue_size: queue_stats.pending_count as usize,
            processing_count: queue_stats.processing_count as usize,
            completed_today: queue_stats.completed_today as usize,
            failed_today: queue_stats.failed_today as usize,
            average_processing_time: Some(Duration::from_secs(120)), // 示例值
            active_models: vec!["local-classifier".to_string()],     // 示例值
        };

        Ok(Json(status))
    }

    /// PUT /api/v1/ai/control - 控制AI处理（暂停/恢复）
    pub async fn control_ai_processing(
        State(_pool): State<Pool<Sqlite>>,
        Json(request): Json<AIControlRequest>,
    ) -> Result<StatusCode, StatusCode> {
        // TODO: 实现AI处理控制逻辑
        match request.action {
            crate::models::AIControlAction::Pause => {
                // 暂停所有AI处理工作线程
                tracing::info!("Pausing AI processing");
            }
            crate::models::AIControlAction::Resume => {
                // 恢复AI处理工作线程
                tracing::info!("Resuming AI processing");
            }
            crate::models::AIControlAction::Stop => {
                // 停止所有AI处理并清空队列
                tracing::info!("Stopping AI processing");
            }
            crate::models::AIControlAction::Restart => {
                // 重启AI处理系统
                tracing::info!("Restarting AI processing");
            }
        }

        Ok(StatusCode::OK)
    }

    fn default_ai_settings() -> AISettings {
        AISettings {
            enabled: false,
            auto_apply_threshold: 0.8,
            processing_schedule: AISchedule {
                immediate: false,
                batch_processing: true,
                off_peak_hours: Some(vec![2, 3, 4, 5]), // 凌晨2-5点
            },
            resource_limits: AIResourceLimits {
                max_concurrent_tasks: 2,
                max_memory_usage: 1024 * 1024 * 1024, // 1GB
                timeout_seconds: 300,                 // 5分钟
                max_retries: 3,
            },
            enabled_analyzers: vec!["local-classifier".to_string()],
        }
    }
}
