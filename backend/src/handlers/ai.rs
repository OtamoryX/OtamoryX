use std::sync::OnceLock;

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
use tokio::sync::Mutex;

use crate::middleware::{auth::AuthInfo, path_permission};
use crate::models::{AIControlRequest, AISettings, AIStatus};
use crate::services::{
    enqueue_suspicious_title_translation_repairs, enqueue_title_translation_backfill,
    enqueue_title_translation_retry, load_ai_settings, save_ai_settings, settings_for_response,
    test_connection,
};

pub struct AIHandler;

// The guard remains owned by the spawned task until its SQLite writes finish.
static TITLE_TRANSLATION_BACKFILL_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIConnectionTestResponse {
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AITitleTranslationBackfillResponse {
    pub started: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AITitleTranslationBackfillQuery {
    pub force: bool,
    pub repair: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AITitleTranslationRetryResponse {
    pub queued: bool,
}

impl AIHandler {
    pub async fn get_ai_settings(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<AISettings>, StatusCode> {
        let settings = load_ai_settings(&pool).await.map_err(|err| {
            tracing::error!("Failed to load AI settings: {err:#}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        Ok(Json(settings_for_response(settings)))
    }

    pub async fn update_ai_settings(
        State(pool): State<Pool<Sqlite>>,
        Json(settings): Json<AISettings>,
    ) -> Result<StatusCode, StatusCode> {
        save_ai_settings(&pool, settings).await.map_err(|err| {
            tracing::warn!("Rejected AI settings update: {err:#}");
            StatusCode::BAD_REQUEST
        })?;
        Ok(StatusCode::OK)
    }

    pub async fn test_ai_connection(
        State(pool): State<Pool<Sqlite>>,
        settings: Option<Json<AISettings>>,
    ) -> Json<AIConnectionTestResponse> {
        let mut effective = match load_ai_settings(&pool).await {
            Ok(settings) => settings,
            Err(err) => {
                return Json(AIConnectionTestResponse {
                    success: false,
                    message: Some(format!("无法读取 AI 设置: {err}")),
                });
            }
        };
        if let Some(Json(provided)) = settings {
            // The submitted write-only key is allowed for this probe but is never echoed back.
            if provided.connection.api_key.is_some() {
                effective.connection.api_key = provided.connection.api_key;
            }
            effective.connection.provider = provided.connection.provider;
            effective.connection.base_url = provided.connection.base_url;
            effective.connection.model = provided.connection.model;
            effective.execution = provided.execution;
        }
        match test_connection(&effective).await {
            Ok(()) => Json(AIConnectionTestResponse {
                success: true,
                message: None,
            }),
            Err(err) => {
                tracing::warn!("AI connection test failed: {err:#}");
                Json(AIConnectionTestResponse {
                    success: false,
                    message: Some(err.to_string()),
                })
            }
        }
    }

    pub async fn backfill_title_translations(
        State(pool): State<Pool<Sqlite>>,
        Query(query): Query<AITitleTranslationBackfillQuery>,
    ) -> Result<(StatusCode, Json<AITitleTranslationBackfillResponse>), StatusCode> {
        let settings = load_ai_settings(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !settings.features.title_translation.enabled {
            return Err(StatusCode::CONFLICT);
        }
        let lock = TITLE_TRANSLATION_BACKFILL_LOCK.get_or_init(|| Mutex::new(()));
        let guard = lock.try_lock().map_err(|_| StatusCode::CONFLICT)?;
        let task_pool = pool.clone();
        tokio::spawn(async move {
            let _guard = guard;
            let result = if query.repair {
                enqueue_suspicious_title_translation_repairs(&task_pool).await
            } else {
                enqueue_title_translation_backfill(&task_pool, query.force).await
            };
            match result {
                Ok(result) => tracing::info!(
                    force = query.force,
                    repair = query.repair,
                    queued = result.queued,
                    skipped = result.skipped,
                    "Title translation backfill completed"
                ),
                Err(err) => tracing::error!("Title translation backfill failed: {err:#}"),
            }
        });

        Ok((
            StatusCode::ACCEPTED,
            Json(AITitleTranslationBackfillResponse { started: true }),
        ))
    }

    pub async fn retry_archive_title_translation(
        State(pool): State<Pool<Sqlite>>,
        Path(archive_id): Path<String>,
        Extension(auth): Extension<AuthInfo>,
    ) -> Result<(StatusCode, Json<AITitleTranslationRetryResponse>), StatusCode> {
        let archive_path =
            sqlx::query_scalar::<_, String>("SELECT path FROM archives WHERE id = ?")
                .bind(&archive_id)
                .fetch_optional(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .ok_or(StatusCode::NOT_FOUND)?;
        if !path_permission::has_path_permission(&pool, &auth, &archive_path).await? {
            return Err(StatusCode::FORBIDDEN);
        }
        let queued = enqueue_title_translation_retry(&pool, &archive_id)
            .await
            .map_err(|err| {
                tracing::warn!(
                    archive_id,
                    "Failed to queue title translation retry: {err:#}"
                );
                StatusCode::CONFLICT
            })?;
        Ok((
            StatusCode::ACCEPTED,
            Json(AITitleTranslationRetryResponse { queued }),
        ))
    }

    pub async fn get_ai_status(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<AIStatus>, StatusCode> {
        let stats = sqlx::query(
            r#"
            SELECT
                COUNT(CASE WHEN status = 'pending' THEN 1 END) as pending_count,
                COUNT(CASE WHEN status = 'processing' THEN 1 END) as processing_count,
                COUNT(CASE WHEN status = 'completed' AND DATE(completed_at) = DATE('now') THEN 1 END) as completed_today,
                COUNT(CASE WHEN status = 'failed' AND DATE(completed_at) = DATE('now') THEN 1 END) as failed_today
            FROM ai_processing_queue
            WHERE job_type = 'title_translation'
            "#,
        )
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let settings = load_ai_settings(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let active_models = if settings.features.title_translation.enabled {
            vec![settings.connection.model]
        } else {
            Vec::new()
        };
        Ok(Json(AIStatus {
            queue_size: stats.get::<i64, _>("pending_count") as usize,
            processing_count: stats.get::<i64, _>("processing_count") as usize,
            completed_today: stats.get::<i64, _>("completed_today") as usize,
            failed_today: stats.get::<i64, _>("failed_today") as usize,
            average_processing_time: None,
            active_models,
        }))
    }

    pub async fn control_ai_processing(
        State(_pool): State<Pool<Sqlite>>,
        Json(request): Json<AIControlRequest>,
    ) -> Result<StatusCode, StatusCode> {
        tracing::info!("AI processing control request: {:?}", request.action);
        // The worker is intentionally settings-driven. A future multi-worker scheduler can map
        // these actions to persisted pause state without changing the public AI configuration.
        Ok(StatusCode::NOT_IMPLEMENTED)
    }
}

#[cfg(test)]
mod tests {
    use axum::extract::State;
    use sqlx::sqlite::SqlitePoolOptions;

    use super::AIHandler;

    #[tokio::test]
    async fn ai_status_counts_only_title_translation_jobs() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect sqlite memory pool");
        sqlx::query(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at DATETIME)",
        )
        .execute(&pool)
        .await
        .expect("create settings table");
        sqlx::query(
            "CREATE TABLE ai_processing_queue (status TEXT NOT NULL, job_type TEXT NOT NULL, completed_at DATETIME)",
        )
        .execute(&pool)
        .await
        .expect("create queue table");
        sqlx::query(
            r#"
            INSERT INTO ai_processing_queue (status, job_type, completed_at) VALUES
                ('pending', 'title_translation', NULL),
                ('processing', 'title_translation', NULL),
                ('completed', 'title_translation', CURRENT_TIMESTAMP),
                ('failed', 'title_translation', CURRENT_TIMESTAMP),
                ('pending', 'auto_tagging', NULL),
                ('processing', 'auto_tagging', NULL),
                ('completed', 'auto_tagging', CURRENT_TIMESTAMP),
                ('failed', 'auto_tagging', CURRENT_TIMESTAMP)
            "#,
        )
        .execute(&pool)
        .await
        .expect("seed queue rows");

        let status = AIHandler::get_ai_status(State(pool))
            .await
            .expect("load AI status")
            .0;

        assert_eq!(status.queue_size, 1);
        assert_eq!(status.processing_count, 1);
        assert_eq!(status.completed_today, 1);
        assert_eq!(status.failed_today, 1);
    }
}
