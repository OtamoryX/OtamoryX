use std::{collections::BTreeMap, sync::OnceLock};

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
use tokio::sync::Mutex;

use crate::middleware::{auth::AuthInfo, path_permission};
use crate::models::{
    AIControlRequest, AIExecutorLaneStatus, AIModelStatus, AISettings, AIStatus,
    AITaskQueueControlAction, AITaskQueueControlRequest, AITaskQueueStatus, AI_EXECUTOR_LANES,
};
use crate::services::{
    enqueue_suspicious_title_translation_repairs, enqueue_title_translation_backfill,
    enqueue_title_translation_retry, load_ai_settings, notify_ai_queue, preview_title_translation,
    provider_state_model, save_ai_settings, settings_for_connection_test, settings_for_profile,
    settings_for_response, test_connection, TitleTranslationPreview, FORCED_MODEL_RETRY_ATTEMPTS,
};

pub struct AIHandler;

// The guard remains owned by the spawned task until its SQLite writes finish.
static TITLE_TRANSLATION_BACKFILL_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

const TASK_QUEUE_TYPES: &[&str] = &[
    "title_translation",
    "title_language_detection",
    "content_analysis_reconcile",
    "content_analysis_synthesize",
    "ocr_extract",
    "metadata_extract",
    "auto_tagging",
    "tag_localization",
];
fn task_requires_model(job_type: &str) -> bool {
    matches!(
        job_type,
        "title_translation"
            | "title_language_detection"
            | "content_analysis_synthesize"
            | "auto_tagging"
            | "tag_localization"
    )
}

fn is_rate_limit_error(error: Option<&str>) -> bool {
    error.is_some_and(|message| message.contains("HTTP 429"))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIConnectionTestResponse {
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AITitleDisplayPreference {
    pub display_translated_title: bool,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AITitleTranslationPreviewRequest {
    pub title: String,
    pub target_language: Option<String>,
    pub settings: Option<AISettings>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AITitleTranslationPreviewResponse {
    pub success: bool,
    pub message: Option<String>,
    pub preview: Option<TitleTranslationPreview>,
}

impl AIHandler {
    pub async fn get_title_display_preference(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<AITitleDisplayPreference>, StatusCode> {
        let settings = load_ai_settings(&pool).await.map_err(|err| {
            tracing::error!("Failed to load title display preference: {err:#}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        Ok(Json(AITitleDisplayPreference {
            display_translated_title: settings.features.title_translation.display_translated_title,
        }))
    }

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
            effective = match settings_for_connection_test(&effective, provided) {
                Ok(settings) => settings,
                Err(err) => {
                    return Json(AIConnectionTestResponse {
                        success: false,
                        message: Some(err.to_string()),
                    });
                }
            };
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

    pub async fn preview_title_translation(
        State(pool): State<Pool<Sqlite>>,
        Json(request): Json<AITitleTranslationPreviewRequest>,
    ) -> Json<AITitleTranslationPreviewResponse> {
        let stored = match load_ai_settings(&pool).await {
            Ok(settings) => settings,
            Err(err) => {
                return Json(AITitleTranslationPreviewResponse {
                    success: false,
                    message: Some(format!("无法读取 AI 设置: {err}")),
                    preview: None,
                });
            }
        };
        let effective = match request.settings {
            Some(provided) => match settings_for_connection_test(&stored, provided) {
                Ok(settings) => settings,
                Err(err) => {
                    return Json(AITitleTranslationPreviewResponse {
                        success: false,
                        message: Some(err.to_string()),
                        preview: None,
                    });
                }
            },
            None => stored,
        };
        let target_language = request
            .target_language
            .unwrap_or_else(|| effective.features.title_translation.target_language.clone());
        match preview_title_translation(&effective, &request.title, &target_language).await {
            Ok(preview) => Json(AITitleTranslationPreviewResponse {
                success: true,
                message: None,
                preview: Some(preview),
            }),
            Err(err) => {
                tracing::warn!("AI title translation preview failed: {err:#}");
                Json(AITitleTranslationPreviewResponse {
                    success: false,
                    message: Some(err.to_string()),
                    preview: None,
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
        let settings = load_ai_settings(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let stats = sqlx::query(
            r#"
            SELECT
                COUNT(CASE WHEN status = 'pending' THEN 1 END) as pending_count,
                COUNT(CASE WHEN status = 'processing' THEN 1 END) as processing_count,
                COUNT(CASE WHEN status = 'completed' AND DATE(completed_at) = DATE('now') THEN 1 END) as completed_today,
                COUNT(CASE WHEN status = 'failed' AND DATE(completed_at) = DATE('now') THEN 1 END) as failed_today,
                COUNT(CASE WHEN job_type = 'title_language_detection' AND status IN ('pending', 'processing') THEN 1 END) as language_detection_pending,
                COUNT(CASE WHEN status = 'pending' AND next_run_at IS NOT NULL AND julianday(next_run_at) > julianday('now') THEN 1 END) as retry_scheduled,
                (
                    SELECT COUNT(*)
                    FROM archive_title_translations
                    WHERE status = 'failed' AND target_language = ?
                ) + (
                    SELECT COUNT(*)
                    FROM archive_title_language_detections
                    WHERE status = 'failed' AND target_language = ?
                ) as unresolved_failure_count
            FROM ai_processing_queue
            "#,
        )
        .bind(&settings.features.title_translation.target_language)
        .bind(&settings.features.title_translation.target_language)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let active_models = settings
            .profiles
            .iter()
            .filter(|profile| profile.enabled)
            .map(|profile| profile.connection.model.clone())
            .chain((settings.profiles.is_empty()).then(|| settings.connection.model.clone()))
            .collect();
        let lane_rows = sqlx::query(
            "SELECT executor_lane, \
                COUNT(CASE WHEN status = 'pending' THEN 1 END) AS pending_count, \
                COUNT(CASE WHEN status = 'processing' THEN 1 END) AS processing_count \
             FROM ai_processing_queue \
             WHERE status IN ('pending', 'processing') GROUP BY executor_lane",
        )
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let lane_counts = lane_rows
            .into_iter()
            .map(|row| {
                (
                    row.get::<String, _>("executor_lane"),
                    (
                        row.get::<i64, _>("pending_count") as usize,
                        row.get::<i64, _>("processing_count") as usize,
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let queue_by_lane = lane_counts
            .iter()
            .map(|(lane, (pending_count, processing_count))| {
                (lane.clone(), pending_count + processing_count)
            })
            .collect::<BTreeMap<_, _>>();
        let executor_lanes = AI_EXECUTOR_LANES
            .iter()
            .map(|executor_lane| {
                let (pending_count, processing_count) =
                    lane_counts.get(*executor_lane).copied().unwrap_or_default();
                AIExecutorLaneStatus {
                    executor_lane: (*executor_lane).to_string(),
                    pending_count,
                    processing_count,
                    max_concurrent_jobs: settings
                        .execution
                        .lanes
                        .limit_for_lane(executor_lane)
                        .expect("known executor lane has a configured limit"),
                }
            })
            .collect::<Vec<_>>();
        let control_rows = sqlx::query("SELECT job_type, manually_paused FROM ai_queue_controls")
            .fetch_all(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let manually_paused = control_rows
            .into_iter()
            .map(|row| {
                (
                    row.get::<String, _>("job_type"),
                    row.get::<i64, _>("manually_paused") != 0,
                )
            })
            .collect::<BTreeMap<_, _>>();
        let task_rows = sqlx::query(
            r#"
            SELECT
                job_type,
                COUNT(CASE WHEN status = 'pending' THEN 1 END) AS pending_count,
                COUNT(CASE WHEN status = 'processing' THEN 1 END) AS processing_count,
                COUNT(CASE WHEN status = 'pending' AND last_error LIKE 'waiting for AI model availability%' THEN 1 END) AS waiting_for_model_count,
                MIN(CASE WHEN status = 'pending' AND last_error LIKE 'waiting for AI model availability%' THEN next_run_at END) AS blocked_until
            FROM ai_processing_queue
            WHERE job_type IN (
                'title_translation', 'title_language_detection', 'content_analysis_reconcile',
                'content_analysis_synthesize', 'ocr_extract', 'metadata_extract', 'auto_tagging',
                'tag_localization'
            )
            GROUP BY job_type
            "#,
        )
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let task_rows = task_rows
            .into_iter()
            .map(|row| (row.get::<String, _>("job_type"), row))
            .collect::<BTreeMap<_, _>>();
        let task_queues = TASK_QUEUE_TYPES
            .iter()
            .map(|job_type| {
                let row = task_rows.get(*job_type);
                let pending_count = row
                    .map(|row| row.get::<i64, _>("pending_count") as usize)
                    .unwrap_or_default();
                let processing_count = row
                    .map(|row| row.get::<i64, _>("processing_count") as usize)
                    .unwrap_or_default();
                let waiting_for_model_count = row
                    .map(|row| row.get::<i64, _>("waiting_for_model_count") as usize)
                    .unwrap_or_default();
                let blocked_until = row
                    .and_then(|row| row.try_get::<Option<String>, _>("blocked_until").ok())
                    .flatten();
                let manually_paused = manually_paused.get(*job_type).copied().unwrap_or(false);
                let state = if manually_paused {
                    "manually_paused"
                } else if waiting_for_model_count > 0 {
                    "waiting_for_model"
                } else if pending_count > 0 || processing_count > 0 {
                    "running"
                } else {
                    "idle"
                };
                AITaskQueueStatus {
                    job_type: (*job_type).to_string(),
                    pending_count,
                    processing_count,
                    waiting_for_model_count,
                    manually_paused,
                    state: state.to_string(),
                    blocked_until,
                    requires_model: task_requires_model(job_type),
                }
            })
            .collect::<Vec<_>>();
        let mut model_states = Vec::with_capacity(settings.profiles.len());
        for profile in &settings.profiles {
            let profile_settings = settings_for_profile(&settings, Some(&profile.id))
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let provider_state = sqlx::query(
                "SELECT blocked_until, last_error, force_attempts_remaining FROM ai_provider_states WHERE provider = ? AND model = ? \
                 AND blocked_until IS NOT NULL AND julianday(blocked_until) > julianday('now')",
            )
            .bind(&profile_settings.connection.provider)
            .bind(provider_state_model(&profile_settings))
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let blocked_until = provider_state
                .as_ref()
                .and_then(|row| row.try_get::<Option<String>, _>("blocked_until").ok())
                .flatten();
            let last_error = provider_state
                .as_ref()
                .and_then(|row| row.try_get::<Option<String>, _>("last_error").ok())
                .flatten();
            let force_attempts_remaining = provider_state
                .as_ref()
                .and_then(|row| row.try_get::<i64, _>("force_attempts_remaining").ok())
                .unwrap_or_default()
                .max(0) as u32;
            let state = if !profile.enabled {
                "disabled"
            } else if blocked_until.is_some() && force_attempts_remaining > 0 {
                "force_retrying"
            } else if blocked_until.is_some() && is_rate_limit_error(last_error.as_deref()) {
                "rate_limited"
            } else if blocked_until.is_some() {
                "unavailable"
            } else {
                "available"
            };
            model_states.push(AIModelStatus {
                profile_id: profile.id.clone(),
                profile_name: profile.name.clone(),
                model: profile.connection.model.clone(),
                state: state.to_string(),
                blocked_until,
                last_error,
                force_attempts_remaining,
            });
        }
        let provider_blocked_until = model_states
            .iter()
            .find(|state| state.profile_id == settings.active_profile_id)
            .and_then(|state| state.blocked_until.clone());
        Ok(Json(AIStatus {
            queue_size: stats.get::<i64, _>("pending_count") as usize,
            processing_count: stats.get::<i64, _>("processing_count") as usize,
            completed_today: stats.get::<i64, _>("completed_today") as usize,
            failed_today: stats.get::<i64, _>("failed_today") as usize,
            language_detection_pending: stats.get::<i64, _>("language_detection_pending") as usize,
            retry_scheduled: stats.get::<i64, _>("retry_scheduled") as usize,
            unresolved_failure_count: stats.get::<i64, _>("unresolved_failure_count") as usize,
            provider_blocked_until,
            average_processing_time: None,
            active_models,
            queue_by_lane,
            executor_lanes,
            model_states,
            task_queues,
        }))
    }

    pub async fn control_ai_task_queue(
        State(pool): State<Pool<Sqlite>>,
        Path(job_type): Path<String>,
        Json(request): Json<AITaskQueueControlRequest>,
    ) -> Result<StatusCode, StatusCode> {
        if !TASK_QUEUE_TYPES.contains(&job_type.as_str()) {
            return Err(StatusCode::NOT_FOUND);
        }
        match request.action {
            AITaskQueueControlAction::Pause => {
                sqlx::query(
                    "INSERT INTO ai_queue_controls (job_type, manually_paused, force_next_model_attempt, updated_at) \
                     VALUES (?, 1, 0, CURRENT_TIMESTAMP) \
                     ON CONFLICT(job_type) DO UPDATE SET manually_paused = 1, force_next_model_attempt = 0, updated_at = CURRENT_TIMESTAMP",
                )
                .bind(&job_type)
                .execute(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            AITaskQueueControlAction::Resume => {
                sqlx::query(
                    "INSERT INTO ai_queue_controls (job_type, manually_paused, force_next_model_attempt, updated_at) \
                     VALUES (?, 0, 0, CURRENT_TIMESTAMP) \
                     ON CONFLICT(job_type) DO UPDATE SET manually_paused = 0, force_next_model_attempt = 0, updated_at = CURRENT_TIMESTAMP",
                )
                .bind(&job_type)
                .execute(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            AITaskQueueControlAction::ForceContinue => {
                let mut transaction = pool
                    .begin()
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                sqlx::query(
                    "INSERT INTO ai_queue_controls (job_type, manually_paused, force_next_model_attempt, updated_at) \
                     VALUES (?, 0, 1, CURRENT_TIMESTAMP) \
                     ON CONFLICT(job_type) DO UPDATE SET manually_paused = 0, force_next_model_attempt = 1, updated_at = CURRENT_TIMESTAMP",
                )
                .bind(&job_type)
                .execute(&mut *transaction)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                sqlx::query(
                    "UPDATE ai_processing_queue SET next_run_at = CURRENT_TIMESTAMP, last_error = NULL \
                     WHERE job_type = ? AND status = 'pending'",
                )
                .bind(&job_type)
                .execute(&mut *transaction)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                transaction
                    .commit()
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
        }
        notify_ai_queue();
        Ok(StatusCode::NO_CONTENT)
    }

    pub async fn force_continue_ai_model(
        State(pool): State<Pool<Sqlite>>,
        Path(profile_id): Path<String>,
    ) -> Result<StatusCode, StatusCode> {
        let settings = load_ai_settings(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let profile = settings
            .profiles
            .iter()
            .find(|profile| profile.id == profile_id)
            .filter(|profile| profile.enabled)
            .ok_or(StatusCode::NOT_FOUND)?;
        let profile_settings = settings_for_profile(&settings, Some(&profile.id))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let updated = sqlx::query(
            "UPDATE ai_provider_states SET force_attempts_remaining = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE provider = ? AND model = ? AND blocked_until IS NOT NULL \
               AND julianday(blocked_until) > julianday('now')",
        )
        .bind(FORCED_MODEL_RETRY_ATTEMPTS)
        .bind(&profile_settings.connection.provider)
        .bind(provider_state_model(&profile_settings))
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if updated.rows_affected() == 0 {
            return Err(StatusCode::CONFLICT);
        }

        sqlx::query(
            "UPDATE ai_processing_queue SET next_run_at = CURRENT_TIMESTAMP, last_error = NULL \
             WHERE status = 'pending' AND last_error LIKE 'waiting for AI model availability%'",
        )
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        notify_ai_queue();
        Ok(StatusCode::NO_CONTENT)
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
    async fn ai_status_counts_current_unresolved_title_failures() {
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
            "CREATE TABLE ai_processing_queue (status TEXT NOT NULL, job_type TEXT NOT NULL, completed_at DATETIME, next_run_at DATETIME, last_error TEXT, executor_lane TEXT NOT NULL DEFAULT 'llm')",
        )
        .execute(&pool)
        .await
        .expect("create queue table");
        sqlx::query(
            "CREATE TABLE archive_title_translations (status TEXT NOT NULL, target_language TEXT NOT NULL)",
        )
        .execute(&pool)
        .await
        .expect("create title translation table");
        sqlx::query(
            "CREATE TABLE archive_title_language_detections (status TEXT NOT NULL, target_language TEXT NOT NULL)",
        )
        .execute(&pool)
        .await
        .expect("create title language detection table");
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
        sqlx::query(
            r#"
            INSERT INTO archive_title_translations (status, target_language) VALUES
                ('failed', 'zh-CN'),
                ('pending', 'zh-CN'),
                ('failed', 'en')
            "#,
        )
        .execute(&pool)
        .await
        .expect("seed translation statuses");
        sqlx::query(
            r#"
            INSERT INTO archive_title_language_detections (status, target_language) VALUES
                ('failed', 'zh-CN'),
                ('completed', 'zh-CN'),
                ('failed', 'en')
            "#,
        )
        .execute(&pool)
        .await
        .expect("seed language detection statuses");
        sqlx::query(
            "CREATE TABLE ai_provider_states (provider TEXT NOT NULL, model TEXT NOT NULL, blocked_until DATETIME, last_error TEXT, force_attempts_remaining INTEGER NOT NULL DEFAULT 0, updated_at DATETIME, PRIMARY KEY (provider, model))",
        )
        .execute(&pool)
        .await
        .expect("create provider state table");
        sqlx::query(
            "CREATE TABLE ai_queue_controls (job_type TEXT PRIMARY KEY, manually_paused INTEGER NOT NULL DEFAULT 0, force_next_model_attempt INTEGER NOT NULL DEFAULT 0, updated_at DATETIME)",
        )
        .execute(&pool)
        .await
        .expect("create queue controls table");

        let status = AIHandler::get_ai_status(State(pool.clone()))
            .await
            .expect("load AI status")
            .0;

        assert_eq!(status.queue_size, 2);
        assert_eq!(status.processing_count, 2);
        assert_eq!(status.completed_today, 2);
        assert_eq!(status.failed_today, 2);
        assert_eq!(status.queue_by_lane.get("llm"), Some(&4));
        assert_eq!(status.unresolved_failure_count, 2);

        sqlx::query("UPDATE archive_title_translations SET status = 'pending' WHERE status = 'failed' AND target_language = 'zh-CN'")
            .execute(&pool)
            .await
            .expect("requeue translation failure");
        sqlx::query("UPDATE archive_title_language_detections SET status = 'completed' WHERE status = 'failed' AND target_language = 'zh-CN'")
            .execute(&pool)
            .await
            .expect("complete language detection failure");

        let status = AIHandler::get_ai_status(State(pool))
            .await
            .expect("reload AI status")
            .0;
        assert_eq!(status.unresolved_failure_count, 0);
    }
}
