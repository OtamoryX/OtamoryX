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
    AIControlRequest, AIExecutorLaneStatus, AIFailureSummary, AIFailureSummaryItem,
    AIJobAttemptDiagnostic, AIModelStatus, AISettings, AIStatus, AITaskDiagnostic,
    AITaskDiagnosticPage, AITaskQueueControlAction, AITaskQueueControlRequest, AITaskQueueStatus,
    AI_EXECUTOR_LANES,
};
use crate::services::{
    enqueue_suspicious_title_translation_repairs, enqueue_title_translation_backfill,
    enqueue_title_translation_retry, load_ai_settings, notify_ai_queue, preview_title_translation,
    provider_state_model, save_ai_settings, settings_for_connection_test, settings_for_profile,
    settings_for_response, test_connection, TitleTranslationPreview, FORCED_MODEL_RETRY_ATTEMPTS,
    MODEL_AVAILABILITY_WAIT_ERROR,
};

pub struct AIHandler;

#[derive(Default)]
struct TaskQueueCounts {
    pending: usize,
    processing: usize,
    ready: usize,
    waiting_for_model: usize,
    waiting_for_dependency: usize,
    retry_scheduled: usize,
}

fn task_queue_state(counts: &TaskQueueCounts, manually_paused: bool) -> &'static str {
    if manually_paused {
        "manually_paused"
    } else if counts.processing > 0 {
        "running"
    } else if counts.ready > 0 {
        "queued"
    } else if counts.waiting_for_model > 0 {
        "waiting_for_model"
    } else if counts.waiting_for_dependency > 0 {
        "waiting_for_dependency"
    } else if counts.retry_scheduled > 0 {
        "retry_scheduled"
    } else if counts.pending > 0 {
        "queued"
    } else {
        "idle"
    }
}

fn task_queue_actions(state: &str) -> (Option<String>, Option<String>, Vec<String>) {
    match state {
        "running" | "queued" => (None, None, vec!["pause".into()]),
        "manually_paused" => (
            Some("user".into()),
            Some("manually_paused".into()),
            vec!["resume".into()],
        ),
        "waiting_for_model" => (
            Some("model".into()),
            Some("model_unavailable".into()),
            vec!["pause".into()],
        ),
        "waiting_for_dependency" => (
            Some("task".into()),
            Some("dependency_wait".into()),
            vec!["forceContinue".into(), "pause".into()],
        ),
        "retry_scheduled" => (
            Some("task".into()),
            Some("retry_backoff".into()),
            vec!["forceContinue".into(), "pause".into()],
        ),
        _ => (None, None, Vec::new()),
    }
}

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

fn failure_code(error: Option<&str>, outcome: Option<&str>) -> Option<String> {
    let value = error.or(outcome)?.to_ascii_lowercase();
    let code = if value.contains("archive title changed") {
        "stale_input"
    } else if value.contains("invalid page binding") {
        "invalid_page_binding"
    } else if value.contains("unsupported source") {
        "invalid_evidence_source"
    } else if value.contains("content analysis evidence references unsupported") {
        "content_evidence_invalid"
    } else if value.contains("unsupported theme")
        || value.contains("content analysis evidence is incomplete")
        || value.contains("content analysis response is missing")
    {
        "insufficient_content_analysis"
    } else if value.contains("output budget") {
        "output_budget_exhausted"
    } else if value.contains("database is locked") {
        "database_locked"
    } else if value.contains("invalid structured")
        || value.contains("invalid content analysis json")
    {
        "invalid_json"
    } else if value.contains("task quality") || value.contains("quality_retry_scheduled") {
        "task_quality_retry"
    } else if value.contains("timeout") {
        "provider_timeout"
    } else if value.contains("429") || value.contains("rate limit") {
        "rate_limited"
    } else if value.contains("cooldown") || value.contains("unavailable") {
        "provider_unavailable"
    } else if value.contains("no assistant") || value.contains("empty") {
        "empty_assistant_output"
    } else if value.contains("context") && value.contains("length") {
        "context_overflow"
    } else if value.contains("length") || value.contains("token") {
        "output_budget_exhausted"
    } else if value.contains("invalid ai provider response") {
        "provider_invalid_response"
    } else if value.contains("ai title translation request failed") {
        "provider_request_failed"
    } else if value.contains("depend") {
        "dependency_wait"
    } else {
        "unknown"
    };
    Some(code.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AITaskDiagnosticsQuery {
    pub status: Option<String>,
    pub job_type: Option<String>,
    pub failure_code: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub include_payload: Option<bool>,
}

fn parse_cursor(cursor: Option<&str>) -> Option<(String, String)> {
    let (created, id) = cursor?.split_once('|')?;
    Some((created.to_string(), id.to_string()))
}

async fn task_attempts(
    pool: &Pool<Sqlite>,
    job_id: &str,
) -> Result<Vec<AIJobAttemptDiagnostic>, StatusCode> {
    let rows = sqlx::query(
        "SELECT id, attempt_number, started_at, finished_at, outcome, error FROM ai_job_attempts WHERE job_id = ? ORDER BY attempt_number ASC",
    )
    .bind(job_id)
    .fetch_all(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let outcome = row.get::<Option<String>, _>("outcome");
            let error = row.get::<Option<String>, _>("error");
            AIJobAttemptDiagnostic {
                id: row.get("id"),
                attempt_number: row.get("attempt_number"),
                started_at: row.get("started_at"),
                finished_at: row.get("finished_at"),
                failure_code: failure_code(error.as_deref(), outcome.as_deref()),
                outcome,
                error,
            }
        })
        .collect())
}

async fn diagnostic_from_row(
    pool: &Pool<Sqlite>,
    row: &sqlx::sqlite::SqliteRow,
    include_payload: bool,
) -> Result<AITaskDiagnostic, StatusCode> {
    let id = row.get::<String, _>("id");
    let last_error = row.get::<Option<String>, _>("last_error");
    Ok(AITaskDiagnostic {
        attempts: task_attempts(pool, &id).await?,
        id,
        archive_id: row.get("archive_id"),
        job_type: row.get("job_type"),
        status: row.get("status"),
        executor_lane: row.get("executor_lane"),
        priority: row.get("priority"),
        attempts_count: row.get("attempts"),
        profile_id: row.get("profile_id"),
        payload: include_payload
            .then(|| row.get::<Option<String>, _>("payload"))
            .flatten(),
        failure_code: failure_code(last_error.as_deref(), None),
        last_error,
        created_at: row.get("created_at"),
        started_at: row.get("started_at"),
        completed_at: row.get("completed_at"),
        next_run_at: row.get("next_run_at"),
        lease_expires_at: row.get("lease_expires_at"),
    })
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
                COUNT(CASE WHEN status = 'pending' AND (next_run_at IS NULL OR julianday(next_run_at) <= julianday('now')) THEN 1 END) AS ready_count,
                COUNT(CASE WHEN status = 'pending' AND last_error LIKE ? THEN 1 END) AS waiting_for_model_count,
                COUNT(CASE WHEN status = 'pending' AND last_error = 'waiting for dependency' THEN 1 END) AS waiting_for_dependency_count,
                COUNT(CASE WHEN status = 'pending' AND next_run_at IS NOT NULL AND julianday(next_run_at) > julianday('now') AND (last_error IS NULL OR (last_error NOT LIKE ? AND last_error != 'waiting for dependency')) THEN 1 END) AS retry_scheduled_count,
                MIN(CASE WHEN status = 'pending' AND last_error LIKE ? THEN next_run_at END) AS blocked_until,
                MIN(CASE WHEN status = 'pending' AND next_run_at IS NOT NULL AND julianday(next_run_at) > julianday('now') THEN next_run_at END) AS next_run_at,
                (
                    SELECT detail.last_error
                    FROM ai_processing_queue detail
                    WHERE detail.job_type = queue.job_type
                      AND detail.status = 'pending'
                      AND detail.last_error IS NOT NULL
                    ORDER BY CASE WHEN detail.next_run_at IS NULL THEN 0 ELSE 1 END,
                             detail.next_run_at ASC
                    LIMIT 1
                ) AS last_error
            FROM ai_processing_queue queue
            WHERE job_type IN (
                'title_translation', 'title_language_detection', 'content_analysis_reconcile',
                'content_analysis_synthesize', 'ocr_extract', 'metadata_extract', 'auto_tagging',
                'tag_localization'
            )
            GROUP BY job_type
            "#,
        )
        .bind(format!("{MODEL_AVAILABILITY_WAIT_ERROR}%"))
        .bind(format!("{MODEL_AVAILABILITY_WAIT_ERROR}%"))
        .bind(format!("{MODEL_AVAILABILITY_WAIT_ERROR}%"))
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
                let counts = TaskQueueCounts {
                    pending: row
                        .map(|row| row.get::<i64, _>("pending_count") as usize)
                        .unwrap_or_default(),
                    processing: row
                        .map(|row| row.get::<i64, _>("processing_count") as usize)
                        .unwrap_or_default(),
                    ready: row
                        .map(|row| row.get::<i64, _>("ready_count") as usize)
                        .unwrap_or_default(),
                    waiting_for_model: row
                        .map(|row| row.get::<i64, _>("waiting_for_model_count") as usize)
                        .unwrap_or_default(),
                    waiting_for_dependency: row
                        .map(|row| row.get::<i64, _>("waiting_for_dependency_count") as usize)
                        .unwrap_or_default(),
                    retry_scheduled: row
                        .map(|row| row.get::<i64, _>("retry_scheduled_count") as usize)
                        .unwrap_or_default(),
                };
                let blocked_until = row
                    .and_then(|row| row.try_get::<Option<String>, _>("blocked_until").ok())
                    .flatten();
                let next_run_at = row
                    .and_then(|row| row.try_get::<Option<String>, _>("next_run_at").ok())
                    .flatten();
                let last_error = row
                    .and_then(|row| row.try_get::<Option<String>, _>("last_error").ok())
                    .flatten();
                let manually_paused = manually_paused.get(*job_type).copied().unwrap_or(false);
                let state = task_queue_state(&counts, manually_paused);
                let (blocking_scope, blocking_reason, available_actions) =
                    task_queue_actions(state);
                AITaskQueueStatus {
                    job_type: (*job_type).to_string(),
                    pending_count: counts.pending,
                    processing_count: counts.processing,
                    waiting_for_model_count: counts.waiting_for_model,
                    waiting_for_dependency_count: counts.waiting_for_dependency,
                    retry_scheduled_count: counts.retry_scheduled,
                    manually_paused,
                    state: state.to_string(),
                    blocked_until,
                    next_run_at,
                    last_error,
                    requires_model: task_requires_model(job_type),
                    blocking_scope,
                    blocking_reason,
                    available_actions,
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

    pub async fn list_ai_tasks(
        State(pool): State<Pool<Sqlite>>,
        Query(query): Query<AITaskDiagnosticsQuery>,
    ) -> Result<Json<AITaskDiagnosticPage>, StatusCode> {
        let limit = query.limit.unwrap_or(50).clamp(1, 200) as usize;
        let (cursor_created, cursor_id) = parse_cursor(query.cursor.as_deref())
            .map(|(created, id)| (Some(created), Some(id)))
            .unwrap_or((None, None));
        let rows = sqlx::query(
            r#"
            SELECT * FROM (
                SELECT queue.*,
                    CASE
                        WHEN lower(COALESCE(last_error, '')) LIKE '%archive title changed%' THEN 'stale_input'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%invalid page binding%' THEN 'invalid_page_binding'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%unsupported source%' THEN 'invalid_evidence_source'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%content analysis evidence references unsupported%' THEN 'content_evidence_invalid'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%unsupported theme%' OR lower(COALESCE(last_error, '')) LIKE '%content analysis evidence is incomplete%' OR lower(COALESCE(last_error, '')) LIKE '%content analysis response is missing%' THEN 'insufficient_content_analysis'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%output budget%' THEN 'output_budget_exhausted'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%database is locked%' THEN 'database_locked'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%invalid structured%' OR lower(COALESCE(last_error, '')) LIKE '%invalid content analysis json%' THEN 'invalid_json'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%task quality%' OR lower(COALESCE(last_error, '')) LIKE '%quality_retry_scheduled%' THEN 'task_quality_retry'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%timeout%' THEN 'provider_timeout'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%429%' OR lower(COALESCE(last_error, '')) LIKE '%rate limit%' THEN 'rate_limited'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%cooldown%' OR lower(COALESCE(last_error, '')) LIKE '%unavailable%' THEN 'provider_unavailable'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%no assistant%' OR lower(COALESCE(last_error, '')) LIKE '%empty%' THEN 'empty_assistant_output'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%context%length%' THEN 'context_overflow'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%length%' OR lower(COALESCE(last_error, '')) LIKE '%token%' THEN 'output_budget_exhausted'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%invalid ai provider response%' THEN 'provider_invalid_response'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%ai title translation request failed%' THEN 'provider_request_failed'
                        WHEN lower(COALESCE(last_error, '')) LIKE '%depend%' THEN 'dependency_wait'
                        WHEN last_error IS NOT NULL THEN 'unknown'
                        ELSE NULL
                    END AS normalized_failure_code
                FROM ai_processing_queue queue
            ) diagnostic
            WHERE (? IS NULL OR status = ?)
              AND (? IS NULL OR job_type = ?)
              AND (? IS NULL OR normalized_failure_code = ?)
              AND (? IS NULL OR created_at > ? OR (created_at = ? AND id > ?))
            ORDER BY created_at ASC, id ASC
            LIMIT ?
            "#,
        )
        .bind(query.status.as_deref())
        .bind(query.status.as_deref())
        .bind(query.job_type.as_deref())
        .bind(query.job_type.as_deref())
        .bind(query.failure_code.as_deref())
        .bind(query.failure_code.as_deref())
        .bind(cursor_created.as_deref())
        .bind(cursor_created.as_deref())
        .bind(cursor_created.as_deref())
        .bind(cursor_id.as_deref())
        .bind((limit + 1) as i64)
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let has_more = rows.len() > limit;
        let mut items = Vec::with_capacity(limit.min(rows.len()));
        for row in rows.iter().take(limit) {
            items.push(
                diagnostic_from_row(&pool, row, query.include_payload.unwrap_or(false)).await?,
            );
        }
        let next_cursor = has_more.then(|| {
            let last = items.last().expect("a paginated result has a last item");
            format!("{}|{}", last.created_at, last.id)
        });
        Ok(Json(AITaskDiagnosticPage { items, next_cursor }))
    }

    pub async fn get_ai_task(
        State(pool): State<Pool<Sqlite>>,
        Path(task_id): Path<String>,
    ) -> Result<Json<AITaskDiagnostic>, StatusCode> {
        let row = sqlx::query("SELECT * FROM ai_processing_queue WHERE id = ?")
            .bind(task_id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;
        Ok(Json(diagnostic_from_row(&pool, &row, true).await?))
    }

    pub async fn get_ai_failure_summary(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<AIFailureSummary>, StatusCode> {
        let rows = sqlx::query(
            "SELECT id, job_type, last_error, COALESCE(completed_at, started_at, created_at) AS failed_at FROM ai_processing_queue WHERE last_error IS NOT NULL ORDER BY failed_at DESC, id DESC",
        )
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut groups: BTreeMap<(String, String), AIFailureSummaryItem> = BTreeMap::new();
        for row in rows {
            let job_type = row.get::<String, _>("job_type");
            let error = row.get::<Option<String>, _>("last_error");
            let code = failure_code(error.as_deref(), None).unwrap_or_else(|| "unknown".into());
            let entry = groups
                .entry((job_type.clone(), code.clone()))
                .or_insert_with(|| AIFailureSummaryItem {
                    job_type,
                    failure_code: code,
                    count: 0,
                    latest_at: row.get("failed_at"),
                    example_task_ids: Vec::new(),
                });
            entry.count += 1;
            if entry.example_task_ids.len() < 3 {
                entry.example_task_ids.push(row.get("id"));
            }
        }
        let mut groups = groups.into_values().collect::<Vec<_>>();
        groups.sort_by(|left, right| right.count.cmp(&left.count));
        Ok(Json(AIFailureSummary { groups }))
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
                     VALUES (?, 0, 0, CURRENT_TIMESTAMP) \
                     ON CONFLICT(job_type) DO UPDATE SET manually_paused = 0, force_next_model_attempt = 0, updated_at = CURRENT_TIMESTAMP",
                )
                .bind(&job_type)
                .execute(&mut *transaction)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                sqlx::query(
                    "UPDATE ai_processing_queue SET next_run_at = CURRENT_TIMESTAMP \
                     WHERE job_type = ? AND status = 'pending' \
                       AND (last_error IS NULL OR last_error NOT LIKE ?)",
                )
                .bind(&job_type)
                .bind(format!("{MODEL_AVAILABILITY_WAIT_ERROR}%"))
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

    use super::{failure_code, task_queue_actions, task_queue_state, AIHandler, TaskQueueCounts};

    #[test]
    fn task_queue_actions_do_not_offer_task_force_continue_for_model_waits() {
        let (scope, reason, actions) = task_queue_actions("waiting_for_model");
        assert_eq!(scope.as_deref(), Some("model"));
        assert_eq!(reason.as_deref(), Some("model_unavailable"));
        assert_eq!(actions, vec!["pause"]);

        let (scope, reason, actions) = task_queue_actions("retry_scheduled");
        assert_eq!(scope.as_deref(), Some("task"));
        assert_eq!(reason.as_deref(), Some("retry_backoff"));
        assert_eq!(actions, vec!["forceContinue", "pause"]);
        assert_eq!(
            failure_code(Some("waiting for AI task quality recovery until ..."), None),
            Some("task_quality_retry".to_string())
        );
        assert_eq!(
            failure_code(
                Some("invalid content-understanding output: content analysis evidence references unsupported themes, pages, or sources"),
                None,
            ),
            Some("content_evidence_invalid".to_string())
        );
        assert_eq!(
            failure_code(
                Some("waiting for AI task quality recovery: invalid page binding"),
                None,
            ),
            Some("invalid_page_binding".to_string())
        );
        assert_eq!(
            failure_code(
                Some("invalid content-understanding output: content analysis evidence references an unsupported source"),
                None,
            ),
            Some("invalid_evidence_source".to_string())
        );
        assert_eq!(
            failure_code(
                Some("invalid content-understanding output: content analysis response is missing 2-5 themes or evidence"),
                None,
            ),
            Some("insufficient_content_analysis".to_string())
        );
        assert_eq!(
            failure_code(Some("invalid content analysis JSON: expected value"), None),
            Some("invalid_json".to_string())
        );
        assert_eq!(
            failure_code(
                Some("AI structured response recovery failed after AI provider exhausted the configured output budget"),
                None,
            ),
            Some("output_budget_exhausted".to_string())
        );
        assert_eq!(
            failure_code(Some("archive title changed before translation"), None),
            Some("stale_input".to_string())
        );
    }

    #[test]
    fn task_queue_state_only_reports_running_for_claimed_work() {
        let queued = TaskQueueCounts {
            pending: 2,
            ready: 2,
            ..Default::default()
        };
        assert_eq!(task_queue_state(&queued, false), "queued");
        let waiting_model = TaskQueueCounts {
            pending: 2,
            waiting_for_model: 2,
            ..Default::default()
        };
        assert_eq!(task_queue_state(&waiting_model, false), "waiting_for_model");
        let waiting_dependency = TaskQueueCounts {
            pending: 1,
            waiting_for_dependency: 1,
            ..Default::default()
        };
        assert_eq!(
            task_queue_state(&waiting_dependency, false),
            "waiting_for_dependency"
        );
        let retry_scheduled = TaskQueueCounts {
            pending: 1,
            retry_scheduled: 1,
            ..Default::default()
        };
        assert_eq!(task_queue_state(&retry_scheduled, false), "retry_scheduled");
        assert_eq!(task_queue_state(&queued, true), "manually_paused");
        let running = TaskQueueCounts {
            processing: 1,
            waiting_for_model: 2,
            ..Default::default()
        };
        assert_eq!(task_queue_state(&running, false), "running");
        assert_eq!(task_queue_state(&running, true), "manually_paused");
        assert_eq!(task_queue_actions("manually_paused").2, vec!["resume"]);
        assert_eq!(task_queue_state(&TaskQueueCounts::default(), false), "idle");
    }

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
            "CREATE TABLE ai_provider_states (provider TEXT NOT NULL, model TEXT NOT NULL, blocked_until DATETIME, last_error TEXT, force_attempts_remaining INTEGER NOT NULL DEFAULT 0, failure_count INTEGER NOT NULL DEFAULT 0, probe_reserved_until DATETIME, updated_at DATETIME, PRIMARY KEY (provider, model))",
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
