use anyhow::{anyhow, Context, Result};
use chrono::{Duration as ChronoDuration, Utc};
use reqwest::Client;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};
use std::time::Duration;
use tracing::warn;
use uuid::Uuid;

use crate::models::AISettings;

const SETTINGS_KEY: &str = "ai_settings";
const API_KEY_SETTINGS_KEY: &str = "ai_connection_api_key";
const TITLE_TRANSLATION_JOB: &str = "title_translation";
const MAX_AI_WORKERS: usize = 16;

#[derive(Debug, Clone, Copy, Default)]
pub struct BackfillResult {
    pub queued: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone)]
struct ClaimedJob {
    id: String,
    archive_id: String,
    source_hash: Option<String>,
}

#[derive(Debug)]
struct TitleTranslationJobError {
    message: String,
    retryable: bool,
    retry_after_seconds: Option<i64>,
}

impl TitleTranslationJobError {
    fn retryable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: true,
            retry_after_seconds: None,
        }
    }

    fn retryable_after(message: impl Into<String>, retry_after_seconds: Option<i64>) -> Self {
        Self {
            message: message.into(),
            retryable: true,
            retry_after_seconds,
        }
    }

    fn permanent(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
            retry_after_seconds: None,
        }
    }
}

impl std::fmt::Display for TitleTranslationJobError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TitleTranslationJobError {}

pub async fn load_ai_settings(pool: &Pool<Sqlite>) -> Result<AISettings> {
    let mut settings: AISettings =
        sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
            .bind(SETTINGS_KEY)
            .fetch_optional(pool)
            .await?
            .map(|raw| deserialize_stored_settings(&raw))
            .unwrap_or_default();

    let stored_key = sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(API_KEY_SETTINGS_KEY)
        .fetch_optional(pool)
        .await?;
    settings.connection.api_key = stored_key;
    settings.connection.api_key_configured = configured_api_key(&settings).is_some();
    Ok(settings)
}

fn deserialize_stored_settings(raw: &str) -> AISettings {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return AISettings::default();
    };
    if value.get("connection").is_some() {
        return serde_json::from_value(value).unwrap_or_default();
    }

    // Preserve the subset of the original flat settings schema that still has a destination in
    // the shared AI configuration. The old scheduler fields had no executing implementation.
    let mut settings = AISettings::default();
    settings.features.auto_tagging.enabled = value
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    settings.features.auto_tagging.auto_apply_threshold = value
        .get("auto_apply_threshold")
        .or_else(|| value.get("autoApplyThreshold"))
        .and_then(Value::as_f64)
        .filter(|threshold| (0.0..=1.0).contains(threshold))
        .map(|threshold| threshold as f32)
        .unwrap_or(settings.features.auto_tagging.auto_apply_threshold);

    let limits = value
        .get("resource_limits")
        .or_else(|| value.get("resourceLimits"));
    if let Some(limits) = limits {
        settings.execution.max_concurrent_tasks = limits
            .get("max_concurrent_tasks")
            .or_else(|| limits.get("maxConcurrentTasks"))
            .and_then(Value::as_u64)
            .filter(|count| (1..=MAX_AI_WORKERS as u64).contains(count))
            .map(|count| count as usize)
            .unwrap_or(settings.execution.max_concurrent_tasks);
        settings.execution.timeout_seconds = limits
            .get("timeout_seconds")
            .or_else(|| limits.get("timeoutSeconds"))
            .and_then(Value::as_u64)
            .filter(|timeout| (5..=1_800).contains(timeout))
            .unwrap_or(settings.execution.timeout_seconds);
        settings.execution.max_retries = limits
            .get("max_retries")
            .or_else(|| limits.get("maxRetries"))
            .and_then(Value::as_u64)
            .filter(|retries| *retries <= 10)
            .map(|retries| retries as u32)
            .unwrap_or(settings.execution.max_retries);
    }
    settings
}

pub async fn save_ai_settings(pool: &Pool<Sqlite>, mut settings: AISettings) -> Result<()> {
    validate_settings(&settings)?;
    let submitted_key = settings
        .connection
        .api_key
        .take()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let stored_json = serde_json::to_string(&settings)?;
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(SETTINGS_KEY)
    .bind(stored_json)
    .execute(pool)
    .await?;

    // Persist API keys independently so ordinary settings reads and responses cannot expose them.
    if let Some(api_key) = submitted_key {
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(API_KEY_SETTINGS_KEY)
        .bind(api_key)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub fn settings_for_response(mut settings: AISettings) -> AISettings {
    settings.connection.api_key_configured = configured_api_key(&settings).is_some();
    settings.connection.api_key = None;
    settings
}

pub async fn enqueue_title_translation(pool: &Pool<Sqlite>, archive_id: &str) -> Result<bool> {
    let settings = load_ai_settings(pool).await?;
    enqueue_title_translation_with_settings(pool, archive_id, &settings, false, true).await
}

pub async fn enqueue_title_translation_retry(
    pool: &Pool<Sqlite>,
    archive_id: &str,
) -> Result<bool> {
    let settings = load_ai_settings(pool).await?;
    if !settings.features.title_translation.enabled {
        return Err(anyhow!("Title translation is disabled"));
    }
    // Keep the visible subtitle until the replacement succeeds. The queued record still tracks
    // the retry so a second click cannot create another active request.
    enqueue_title_translation_with_settings(pool, archive_id, &settings, true, false).await
}

async fn enqueue_title_translation_with_settings(
    pool: &Pool<Sqlite>,
    archive_id: &str,
    settings: &AISettings,
    force: bool,
    clear_existing_subtitle: bool,
) -> Result<bool> {
    let feature = &settings.features.title_translation;
    if !feature.enabled {
        return Ok(false);
    }
    let mut transaction = pool.begin().await?;
    let row = sqlx::query(
        "SELECT title, subtitle, subtitle_language, subtitle_source_hash FROM archives WHERE id = ? LIMIT 1",
    )
    .bind(archive_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| anyhow!("archive `{archive_id}` not found"))?;
    let title: String = row.get("title");
    let subtitle: Option<String> = row.get("subtitle");
    let subtitle_language: Option<String> = row.get("subtitle_language");
    let active_hash: Option<String> = row.get("subtitle_source_hash");
    let source_hash = title_hash(&title);

    if feature.skip_if_target_language
        && title_looks_like_target_language(&title, &feature.target_language)
    {
        return Ok(false);
    }
    if !force {
        if active_hash.as_deref() == Some(&source_hash)
            && subtitle.is_some()
            && subtitle_language.as_deref() == Some(feature.target_language.as_str())
        {
            return Ok(false);
        }
        if !feature.retranslate_on_title_change && subtitle.is_some() {
            return Ok(false);
        }
    }

    let dedupe_key = format!("{TITLE_TRANSLATION_JOB}:{archive_id}:{source_hash}");
    let now = Utc::now();
    let queue_result = sqlx::query(
        r#"
        INSERT OR IGNORE INTO ai_processing_queue (
            id, archive_id, status, priority, attempts, job_type, payload, source_hash, dedupe_key,
            created_at, next_run_at
        ) VALUES (?, ?, 'pending', 0, 0, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(archive_id)
    .bind(TITLE_TRANSLATION_JOB)
    .bind(json!({ "targetLanguage": feature.target_language }).to_string())
    .bind(&source_hash)
    .bind(&dedupe_key)
    .bind(now)
    .bind(now)
    .execute(&mut *transaction)
    .await?;
    if queue_result.rows_affected() == 0 {
        return Ok(false);
    }

    // A subtitle belongs to both a source title and a target language. Do not show a stale
    // translation while the current title or language is waiting in the AI queue.
    if clear_existing_subtitle && subtitle.is_some() {
        sqlx::query(
            "UPDATE archives SET subtitle = NULL, subtitle_language = NULL, subtitle_source_hash = NULL, updated_at = ? WHERE id = ?",
        )
        .bind(Utc::now())
        .bind(archive_id)
        .execute(&mut *transaction)
        .await?;
    }

    let translation_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO archive_title_translations (
            id, archive_id, source_title, source_hash, target_language, status, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, 'pending', ?, ?)
        ON CONFLICT(archive_id, target_language, source_hash) DO UPDATE SET
            status = CASE WHEN archive_title_translations.status = 'completed' THEN 'completed' ELSE 'pending' END,
            last_error = NULL,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(translation_id)
    .bind(archive_id)
    .bind(&title)
    .bind(&source_hash)
    .bind(&feature.target_language)
    .bind(now)
    .bind(now)
    .execute(&mut *transaction)
    .await?;
    if force {
        sqlx::query(
            "UPDATE archive_title_translations SET status = 'pending', translated_title = NULL, last_error = NULL, completed_at = NULL, updated_at = ? WHERE archive_id = ? AND target_language = ? AND source_hash = ?",
        )
        .bind(now)
        .bind(archive_id)
        .bind(&feature.target_language)
        .bind(&source_hash)
        .execute(&mut *transaction)
        .await?;
    }

    transaction.commit().await?;
    Ok(true)
}

pub async fn enqueue_title_translation_backfill(
    pool: &Pool<Sqlite>,
    force: bool,
) -> Result<BackfillResult> {
    let settings = load_ai_settings(pool).await?;
    if !settings.features.title_translation.enabled {
        return Err(anyhow!("Title translation is disabled"));
    }
    let ids = sqlx::query_scalar::<_, String>("SELECT id FROM archives ORDER BY created_at ASC")
        .fetch_all(pool)
        .await?;
    let mut result = BackfillResult::default();
    for archive_id in ids {
        if enqueue_title_translation_with_settings(pool, &archive_id, &settings, force, true)
            .await?
        {
            result.queued += 1;
        } else {
            result.skipped += 1;
        }
    }
    Ok(result)
}

pub async fn enqueue_suspicious_title_translation_repairs(
    pool: &Pool<Sqlite>,
) -> Result<BackfillResult> {
    let settings = load_ai_settings(pool).await?;
    let feature = &settings.features.title_translation;
    if !feature.enabled {
        return Err(anyhow!("Title translation is disabled"));
    }

    let rows = sqlx::query(
        r#"
        SELECT
            a.id,
            a.title,
            a.subtitle,
            a.subtitle_language,
            a.subtitle_source_hash,
            t.status AS translation_status,
            t.translated_title,
            t.source_hash AS translation_source_hash
        FROM archives a
        LEFT JOIN archive_title_translations t
            ON t.archive_id = a.id AND t.target_language = ?
        ORDER BY a.created_at ASC
        "#,
    )
    .bind(&feature.target_language)
    .fetch_all(pool)
    .await?;

    let mut result = BackfillResult::default();
    for row in rows {
        let archive_id: String = row.get("id");
        let title: String = row.get("title");
        let source_hash = title_hash(&title);
        let translation_source_hash: Option<String> = row.get("translation_source_hash");
        if translation_source_hash.as_deref() != Some(source_hash.as_str()) {
            continue;
        }

        let translation_status: Option<String> = row.get("translation_status");
        let translated_title: Option<String> = row.get("translated_title");
        let subtitle: Option<String> = row.get("subtitle");
        let subtitle_language: Option<String> = row.get("subtitle_language");
        let subtitle_source_hash: Option<String> = row.get("subtitle_source_hash");
        let stored_subtitle = (subtitle_language.as_deref()
            == Some(feature.target_language.as_str())
            && subtitle_source_hash.as_deref() == Some(source_hash.as_str()))
        .then_some(subtitle)
        .flatten();
        let has_quality_issue = translated_title
            .as_deref()
            .or(stored_subtitle.as_deref())
            .and_then(|value| translation_quality_issue(&title, value, &feature.target_language))
            .is_some();
        let should_retry = translation_status.as_deref() == Some("failed") || has_quality_issue;
        if !should_retry {
            continue;
        }

        if enqueue_title_translation_with_settings(pool, &archive_id, &settings, true, false)
            .await?
        {
            result.queued += 1;
        } else {
            result.skipped += 1;
        }
    }
    Ok(result)
}

pub fn spawn_ai_worker(pool: Pool<Sqlite>) {
    for slot in 0..MAX_AI_WORKERS {
        let supervisor_pool = pool.clone();
        tokio::spawn(async move {
            loop {
                let worker_pool = supervisor_pool.clone();
                match tokio::spawn(async move { run_ai_worker(worker_pool, slot).await }).await {
                    Ok(()) => warn!(slot, "AI worker stopped unexpectedly; restarting"),
                    Err(err) => {
                        tracing::error!(slot, error = %err, "AI worker panicked; restarting")
                    }
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
    }
}

async fn run_ai_worker(pool: Pool<Sqlite>, slot: usize) {
    loop {
        let worker_limit = load_ai_settings(&pool)
            .await
            .map(|settings| {
                settings
                    .execution
                    .max_concurrent_tasks
                    .clamp(1, MAX_AI_WORKERS)
            })
            .unwrap_or(1);
        if slot >= worker_limit {
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        match process_next_job(&pool).await {
            Ok(true) => continue,
            Ok(false) => tokio::time::sleep(Duration::from_secs(2)).await,
            Err(err) => {
                warn!("AI worker iteration failed: {err:#}");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

/// Runs one job at most. Public to make the queue behavior testable without a background worker.
pub async fn process_next_job(pool: &Pool<Sqlite>) -> Result<bool> {
    release_expired_leases(pool).await?;
    let Some(job) = claim_next_job(pool).await? else {
        return Ok(false);
    };
    let settings = load_ai_settings(pool).await?;
    if !settings.features.title_translation.enabled {
        // Do not consume jobs while the feature is disabled; it can be re-enabled later.
        sqlx::query("UPDATE ai_processing_queue SET status = 'pending', started_at = NULL, lease_expires_at = NULL WHERE id = ?")
            .bind(&job.id)
            .execute(pool)
            .await?;
        return Ok(false);
    }

    match process_title_translation_job(pool, &settings, &job).await {
        Ok(()) => complete_job(pool, &job.id).await?,
        Err(err) => {
            fail_or_retry_job(
                pool,
                &settings,
                &job.id,
                &job.archive_id,
                job.source_hash.as_deref(),
                &err,
            )
            .await?
        }
    }
    Ok(true)
}

async fn release_expired_leases(pool: &Pool<Sqlite>) -> Result<()> {
    sqlx::query(
        "UPDATE ai_processing_queue SET status = 'pending', started_at = NULL, lease_expires_at = NULL \
         WHERE status = 'processing' AND lease_expires_at IS NOT NULL \
           AND julianday(lease_expires_at) < julianday('now')",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn claim_next_job(pool: &Pool<Sqlite>) -> Result<Option<ClaimedJob>> {
    let row = sqlx::query(
        r#"
        SELECT id, archive_id, source_hash
        FROM ai_processing_queue
        WHERE status = 'pending' AND job_type = ?
          AND (next_run_at IS NULL OR julianday(next_run_at) <= julianday('now'))
        ORDER BY priority DESC, created_at ASC
        LIMIT 1
        "#,
    )
    .bind(TITLE_TRANSLATION_JOB)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else { return Ok(None) };
    let job = ClaimedJob {
        id: row.get("id"),
        archive_id: row.get("archive_id"),
        source_hash: row.try_get("source_hash")?,
    };
    let lease_expires_at = Utc::now() + ChronoDuration::minutes(10);
    let claimed = sqlx::query(
        "UPDATE ai_processing_queue SET status = 'processing', attempts = attempts + 1, started_at = CURRENT_TIMESTAMP, lease_expires_at = ? WHERE id = ? AND status = 'pending'",
    )
    .bind(lease_expires_at)
    .bind(&job.id)
    .execute(pool)
    .await?;
    Ok((claimed.rows_affected() == 1).then_some(job))
}

async fn process_title_translation_job(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    job: &ClaimedJob,
) -> std::result::Result<(), TitleTranslationJobError> {
    let source_hash = job.source_hash.as_deref().ok_or_else(|| {
        TitleTranslationJobError::permanent("title translation job has no source hash")
    })?;
    let row = sqlx::query("SELECT title FROM archives WHERE id = ? LIMIT 1")
        .bind(&job.archive_id)
        .fetch_optional(pool)
        .await
        .map_err(|err| {
            TitleTranslationJobError::retryable(format!("failed to load archive: {err}"))
        })?
        .ok_or_else(|| TitleTranslationJobError::permanent("archive deleted before translation"))?;
    let title: String = row.get("title");
    if title_hash(&title) != source_hash {
        return Err(TitleTranslationJobError::permanent(
            "archive title changed before translation",
        ));
    }

    let translated = translate_title(settings, &title).await?;
    if translated.trim().is_empty() || translated.len() > 1_000 {
        return Err(TitleTranslationJobError::retryable(
            "model returned an invalid translated title",
        ));
    }
    let feature = &settings.features.title_translation;
    let now = Utc::now();
    sqlx::query(
        r#"
        UPDATE archive_title_translations
        SET translated_title = ?, status = 'completed', provider = ?, model = ?, last_error = NULL,
            completed_at = ?, updated_at = ?
        WHERE archive_id = ? AND source_hash = ? AND target_language = ?
        "#,
    )
    .bind(&translated)
    .bind(&settings.connection.provider)
    .bind(&settings.connection.model)
    .bind(now)
    .bind(now)
    .bind(&job.archive_id)
    .bind(source_hash)
    .bind(&feature.target_language)
    .execute(pool)
    .await
    .map_err(|err| {
        TitleTranslationJobError::retryable(format!("failed to save translated title: {err}"))
    })?;
    let updated = sqlx::query(
        "UPDATE archives SET subtitle = ?, subtitle_language = ?, subtitle_source_hash = ?, updated_at = ? WHERE id = ? AND title = ?",
    )
    .bind(translated)
    .bind(&feature.target_language)
    .bind(source_hash)
    .bind(now)
    .bind(&job.archive_id)
    .bind(&title)
    .execute(pool)
    .await
    .map_err(|err| {
        TitleTranslationJobError::retryable(format!("failed to update archive subtitle: {err}"))
    })?;
    if updated.rows_affected() != 1 {
        return Err(TitleTranslationJobError::permanent(
            "archive title changed while writing translation",
        ));
    }
    Ok(())
}

async fn complete_job(pool: &Pool<Sqlite>, job_id: &str) -> Result<()> {
    sqlx::query(
        "UPDATE ai_processing_queue SET status = 'completed', completed_at = CURRENT_TIMESTAMP, lease_expires_at = NULL, last_error = NULL WHERE id = ?",
    )
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn fail_or_retry_job(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    job_id: &str,
    archive_id: &str,
    source_hash: Option<&str>,
    error: &TitleTranslationJobError,
) -> Result<()> {
    let attempts: i64 = sqlx::query_scalar("SELECT attempts FROM ai_processing_queue WHERE id = ?")
        .bind(job_id)
        .fetch_one(pool)
        .await?;
    // `max_retries` is retries after the first request. This gives the default value of 3 a
    // predictable four total attempts, each of which is a fresh provider routing attempt.
    let final_failure = !error.retryable || attempts > settings.execution.max_retries as i64;
    let status = if final_failure { "failed" } else { "pending" };
    let retry_delay = error.retry_after_seconds.unwrap_or_else(|| match attempts {
        1 => 2,
        2 => 5,
        _ => 10,
    });
    let retry_at = Utc::now() + ChronoDuration::seconds(retry_delay.clamp(1, 300));
    sqlx::query(
        "UPDATE ai_processing_queue SET status = ?, last_error = ?, next_run_at = ?, completed_at = CASE WHEN ? = 'failed' THEN CURRENT_TIMESTAMP ELSE NULL END, lease_expires_at = NULL WHERE id = ?",
    )
    .bind(status)
    .bind(&error.message)
    .bind(retry_at)
    .bind(status)
    .bind(job_id)
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE archive_title_translations SET status = ?, last_error = ?, updated_at = CURRENT_TIMESTAMP WHERE archive_id = ? AND source_hash = ?",
    )
    .bind(status)
    .bind(&error.message)
    .bind(archive_id)
    .bind(source_hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn test_connection(settings: &AISettings) -> Result<()> {
    let key = configured_api_key(settings).ok_or_else(|| anyhow!("No AI API key is configured"))?;
    let endpoint = chat_completions_endpoint(&settings.connection.base_url)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(
            settings.execution.timeout_seconds.clamp(5, 300),
        ))
        .build()?;
    let response = client
        .post(endpoint)
        .bearer_auth(key)
        .json(&json!({
            "model": settings.connection.model,
            "temperature": 0,
            "max_tokens": 4,
            "messages": [{ "role": "user", "content": "Reply exactly: OK" }]
        }))
        .send()
        .await
        .context("AI connection request failed")?;
    if !response.status().is_success() {
        return Err(anyhow!("AI provider returned HTTP {}", response.status()));
    }
    Ok(())
}

async fn translate_title(
    settings: &AISettings,
    title: &str,
) -> std::result::Result<String, TitleTranslationJobError> {
    let key = configured_api_key(settings)
        .ok_or_else(|| TitleTranslationJobError::permanent("No AI API key is configured"))?;
    let endpoint = chat_completions_endpoint(&settings.connection.base_url)
        .map_err(|err| TitleTranslationJobError::permanent(err.to_string()))?;
    let client = Client::builder()
        .timeout(Duration::from_secs(
            settings.execution.timeout_seconds.clamp(5, 300),
        ))
        .build()
        .map_err(|err| {
            TitleTranslationJobError::permanent(format!("failed to build AI client: {err}"))
        })?;
    let target = settings.features.title_translation.target_language.trim();
    let target_name = target_language_name(target);
    let response = client
        .post(&endpoint)
        .bearer_auth(&key)
        .json(&json!({
            "model": settings.connection.model,
            "temperature": 0.1,
            "max_tokens": 256,
            "messages": [
                {
                    "role": "system",
                    "content": "You translate bibliographic comic-title strings. Transform only the supplied title; do not expand, evaluate, or describe its content. Return exactly one translated title. If you cannot translate it, return exactly [[REFUSED]]."
                },
                { "role": "user", "content": title_translation_prompt(title, target, &target_name) }
            ]
        }))
        .send()
        .await
        .map_err(|err| {
            TitleTranslationJobError::retryable(format!("AI title translation request failed: {err}"))
        })?;
    let status = response.status();
    if !status.is_success() {
        let retry_after_seconds = retry_after_seconds(&response);
        let body = response.text().await.unwrap_or_default();
        let message = format!(
            "AI provider returned HTTP {}: {}",
            status,
            compact_error_body(&body)
        );
        return if is_retryable_http_response(status.as_u16(), &body) {
            Err(TitleTranslationJobError::retryable_after(
                message,
                retry_after_seconds,
            ))
        } else {
            Err(TitleTranslationJobError::permanent(message))
        };
    }
    let body: Value = response.json().await.map_err(|err| {
        TitleTranslationJobError::retryable(format!("Invalid AI provider response: {err}"))
    })?;
    let content = body
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            TitleTranslationJobError::retryable("AI provider response has no assistant content")
        })?;
    let translated = normalize_model_title(content).map_err(|err| {
        TitleTranslationJobError::retryable(format!("Invalid translated title: {err}"))
    })?;
    if let Some(issue) = translation_quality_issue(title, &translated, target) {
        return Err(TitleTranslationJobError::retryable(format!(
            "AI translation failed validation: {issue}"
        )));
    }
    Ok(translated)
}

fn title_translation_prompt(title: &str, target: &str, target_name: &str) -> String {
    format!(
        "Translate the comic title below from whatever source language it uses into {target_name} ({target}).\n\
         Translate all natural-language words, particles, counters, volume/chapter labels, and translatable text inside brackets. \
         Preserve bracket characters, numbers, rating markers, and the identity of proper names, authors, and circles. \
         Render names using an established target-language form when one exists; otherwise transliterate them into the target language's normal writing system. \
         Do not leave source-language grammar or writing-system fragments in the result merely because they occur in a name or brackets. \
         Return exactly one translated title with no explanation, label, quotation marks, or JSON.\n\nSource title: {title}"
    )
}

fn retry_after_seconds(response: &reqwest::Response) -> Option<i64> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<i64>().ok())
        .filter(|seconds| *seconds > 0)
}

fn compact_error_body(body: &str) -> String {
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        "no response body".to_string()
    } else {
        compact.chars().take(240).collect()
    }
}

fn is_retryable_http_response(status: u16, body: &str) -> bool {
    matches!(status, 408 | 409 | 425 | 429) || status >= 500 || is_safety_block_response(body)
}

fn is_safety_block_response(body: &str) -> bool {
    let normalized = body.to_ascii_lowercase();
    [
        "moderation",
        "safety",
        "content policy",
        "policy violation",
        "blocked",
        "refused",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
        || body.contains("内容安全")
        || body.contains("内容政策")
        || body.contains("安全策略")
}

fn target_language_name(language: &str) -> String {
    let normalized = language.to_ascii_lowercase();
    let name = if normalized == "zh-cn" || normalized == "zh-hans" {
        "Simplified Chinese"
    } else if normalized == "zh-tw" || normalized == "zh-hant" || normalized == "zh-hk" {
        "Traditional Chinese"
    } else if normalized.starts_with("zh") {
        "Chinese"
    } else if normalized.starts_with("ja") {
        "Japanese"
    } else if normalized.starts_with("ko") {
        "Korean"
    } else if normalized.starts_with("en") {
        "English"
    } else if normalized.starts_with("fr") {
        "French"
    } else if normalized.starts_with("de") {
        "German"
    } else if normalized.starts_with("es") {
        "Spanish"
    } else if normalized.starts_with("pt") {
        "Portuguese"
    } else if normalized.starts_with("it") {
        "Italian"
    } else if normalized.starts_with("ru") {
        "Russian"
    } else if normalized.starts_with("uk") {
        "Ukrainian"
    } else {
        return language.to_string();
    };
    name.to_string()
}

fn configured_api_key(settings: &AISettings) -> Option<String> {
    std::env::var("AI_PROVIDER_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            settings
                .connection
                .api_key
                .clone()
                .filter(|value| !value.trim().is_empty())
        })
}

fn chat_completions_endpoint(base_url: &str) -> Result<String> {
    let base = base_url.trim().trim_end_matches('/');
    if !(base.starts_with("https://") || base.starts_with("http://")) {
        return Err(anyhow!("AI base URL must use http:// or https://"));
    }
    Ok(if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{base}/chat/completions")
    })
}

fn normalize_model_title(content: &str) -> Result<String> {
    let stripped = content.trim().trim_matches('`').trim();
    if let Ok(value) = serde_json::from_str::<Value>(stripped) {
        if let Some(title) = value.get("title").and_then(Value::as_str) {
            return normalize_model_title(title);
        }
    }
    let single_line = stripped.lines().next().unwrap_or("").trim();
    if single_line.is_empty() {
        return Err(anyhow!("AI provider returned an empty title"));
    }
    Ok(single_line.to_string())
}

fn translation_quality_issue(source: &str, translated: &str, target: &str) -> Option<String> {
    let translated = translated.trim();
    if translated.is_empty() {
        return Some("the result is empty".to_string());
    }
    if is_title_translation_refusal(translated) {
        return Some("the model refused to translate the title".to_string());
    }
    if translated.len() > 1_000 {
        return Some("the result is longer than 1000 bytes".to_string());
    }
    if source.trim().eq_ignore_ascii_case(translated)
        && !title_looks_like_target_language(source, target)
    {
        return Some("the result repeats the source title unchanged".to_string());
    }

    let target = target.to_ascii_lowercase();
    let has_letters = source.chars().any(char::is_alphabetic);
    if target.starts_with("zh") {
        if translated.chars().any(is_japanese_kana) {
            return Some("a Chinese result still contains Japanese kana".to_string());
        }
        if translated.chars().any(is_hangul) {
            return Some("a Chinese result still contains Hangul".to_string());
        }
        if translated.chars().any(is_cyrillic) {
            return Some("a Chinese result still contains Cyrillic letters".to_string());
        }
        if has_letters && !translated.chars().any(is_han) {
            return Some("a Chinese result contains no Chinese characters".to_string());
        }
    } else if target.starts_with("ja") {
        if translated.chars().any(is_hangul) {
            return Some("a Japanese result still contains Hangul".to_string());
        }
        if translated.chars().any(is_cyrillic) {
            return Some("a Japanese result still contains Cyrillic letters".to_string());
        }
        if has_letters && !translated.chars().any(|c| is_japanese_kana(c) || is_han(c)) {
            return Some("a Japanese result contains no Japanese writing".to_string());
        }
    } else if target.starts_with("ko") {
        if translated.chars().any(is_japanese_kana) {
            return Some("a Korean result still contains Japanese kana".to_string());
        }
        if translated.chars().any(is_cyrillic) {
            return Some("a Korean result still contains Cyrillic letters".to_string());
        }
        if has_letters && !translated.chars().any(is_hangul) {
            return Some("a Korean result contains no Hangul".to_string());
        }
    } else if is_latin_target(&target) {
        if translated
            .chars()
            .any(|c| is_han(c) || is_japanese_kana(c) || is_hangul(c) || is_cyrillic(c))
        {
            return Some("a Latin-script result still contains another writing system".to_string());
        }
        if has_letters && !translated.chars().any(is_latin) {
            return Some("a Latin-script result contains no Latin letters".to_string());
        }
    } else if is_cyrillic_target(&target) {
        if translated
            .chars()
            .any(|c| is_han(c) || is_japanese_kana(c) || is_hangul(c))
        {
            return Some(
                "a Cyrillic result still contains an East Asian writing system".to_string(),
            );
        }
        if has_letters && !translated.chars().any(is_cyrillic) {
            return Some("a Cyrillic result contains no Cyrillic letters".to_string());
        }
    }
    None
}

fn is_title_translation_refusal(value: &str) -> bool {
    let normalized = value
        .trim()
        .to_ascii_lowercase()
        .replace(['\u{2018}', '\u{2019}'], "'");
    if normalized == "[[refused]]"
        || [
            "as an ai",
            "i'm sorry",
            "i cannot",
            "i can't",
            "i'm unable",
            "cannot assist",
            "can't assist",
            "unable to translate",
            "content policy",
            "safety policy",
            "policy violation",
        ]
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return true;
    }

    (value.contains("抱歉")
        && ["不能", "无法", "不可以", "拒绝"]
            .iter()
            .any(|marker| value.contains(marker)))
        || (value.contains("无法")
            && ["翻译", "提供", "协助", "处理"]
                .iter()
                .any(|marker| value.contains(marker)))
        || (value.contains("作为 AI")
            && ["不能", "无法"].iter().any(|marker| value.contains(marker)))
}

fn is_han(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF)
}

fn is_japanese_kana(c: char) -> bool {
    matches!(c as u32, 0x3040..=0x30FF | 0x31F0..=0x31FF | 0xFF66..=0xFF9D)
}

fn is_hangul(c: char) -> bool {
    matches!(c as u32, 0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xAC00..=0xD7AF | 0xD7B0..=0xD7FF)
}

fn is_latin(c: char) -> bool {
    c.is_ascii_alphabetic()
        || matches!(c as u32, 0x00C0..=0x02AF | 0x1D00..=0x1D7F | 0x1E00..=0x1EFF)
}

fn is_cyrillic(c: char) -> bool {
    matches!(c as u32, 0x0400..=0x052F | 0x2DE0..=0x2DFF | 0xA640..=0xA69F)
}

fn language_matches(language: &str, prefix: &str) -> bool {
    language == prefix
        || language
            .strip_prefix(prefix)
            .is_some_and(|suffix| suffix.starts_with('-'))
}

fn is_latin_target(language: &str) -> bool {
    [
        "en", "fr", "de", "es", "pt", "it", "nl", "pl", "cs", "sk", "hu", "ro", "tr", "vi", "id",
        "ms", "sv", "no", "da", "fi",
    ]
    .iter()
    .any(|prefix| language_matches(language, prefix))
}

fn is_cyrillic_target(language: &str) -> bool {
    ["ru", "uk", "be", "bg", "mk"]
        .iter()
        .any(|prefix| language_matches(language, prefix))
}

fn validate_settings(settings: &AISettings) -> Result<()> {
    if settings.connection.provider != "openaiCompatible" {
        return Err(anyhow!(
            "Only the openaiCompatible provider is currently supported"
        ));
    }
    chat_completions_endpoint(&settings.connection.base_url)?;
    if settings.connection.model.trim().is_empty() {
        return Err(anyhow!("AI model must not be empty"));
    }
    if settings
        .features
        .title_translation
        .target_language
        .trim()
        .is_empty()
    {
        return Err(anyhow!(
            "Title translation target language must not be empty"
        ));
    }
    if settings.execution.max_concurrent_tasks == 0 || settings.execution.max_concurrent_tasks > 16
    {
        return Err(anyhow!("AI maxConcurrentTasks must be between 1 and 16"));
    }
    Ok(())
}

pub fn title_hash(title: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(title.trim().as_bytes());
    format!("{:x}", hasher.finalize())
}

fn title_looks_like_target_language(title: &str, language: &str) -> bool {
    let letters: Vec<char> = title.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return false;
    }
    let language = language.to_ascii_lowercase();
    if language.starts_with("zh") {
        return letters
            .iter()
            .filter(|c| matches!(**c as u32, 0x4E00..=0x9FFF))
            .count()
            * 2
            >= letters.len();
    }
    if language.starts_with("ja") {
        return letters
            .iter()
            .filter(|c| matches!(**c as u32, 0x3040..=0x30FF | 0x4E00..=0x9FFF))
            .count()
            * 2
            >= letters.len();
    }
    if language.starts_with("ko") {
        return letters
            .iter()
            .filter(|c| matches!(**c as u32, 0xAC00..=0xD7AF))
            .count()
            * 2
            >= letters.len();
    }
    language.starts_with("en") && letters.iter().all(|c| c.is_ascii())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn hashes_trimmed_title_and_detects_target_scripts() {
        assert_eq!(title_hash(" title "), title_hash("title"));
        assert!(title_looks_like_target_language("中文标题 Vol. 1", "zh-CN"));
        assert!(!title_looks_like_target_language("English title", "zh-CN"));
    }

    #[test]
    fn normalizes_plain_or_json_model_responses() {
        assert_eq!(
            normalize_model_title("  translated title  ").unwrap(),
            "translated title"
        );
        assert_eq!(
            normalize_model_title(r#"{"title":"译名"}"#).unwrap(),
            "译名"
        );
        assert!(chat_completions_endpoint("example.com").is_err());
    }

    #[test]
    fn validates_target_writing_system_for_multiple_source_languages() {
        let japanese_source = "月の花嫁ちゃんと冒険!その3";
        assert!(
            translation_quality_issue(japanese_source, "月之新娘ちゃんと冒险！その3", "zh-CN")
                .is_some()
        );
        assert!(
            translation_quality_issue(japanese_source, "月之新娘的冒险！第3篇", "zh-CN").is_none()
        );

        assert!(translation_quality_issue("The Moon Bride", "The Moon Bride", "zh-CN").is_some());
        assert!(translation_quality_issue("The Moon Bride", "月之新娘", "zh-CN").is_none());
        assert!(translation_quality_issue("달빛 신부", "달빛新娘", "zh-CN").is_some());
        assert!(translation_quality_issue("달빛 신부", "月光新娘", "zh-CN").is_none());

        assert!(translation_quality_issue("月光新娘", "Moonlight Bride", "en").is_none());
        assert!(translation_quality_issue("月光新娘", "월빛 신부", "ko").is_none());
        assert!(translation_quality_issue("月光新娘", "Лунная невеста", "ru").is_none());
    }

    #[test]
    fn rejects_empty_and_refusal_responses_before_saving_them_as_titles() {
        assert!(translation_quality_issue("Original title", "", "zh-CN").is_some());
        assert!(translation_quality_issue("Original title", "[[REFUSED]]", "zh-CN").is_some());
        assert!(translation_quality_issue(
            "Original title",
            "抱歉，我无法协助处理这个标题。",
            "zh-CN",
        )
        .is_some());
        assert!(translation_quality_issue(
            "Original title",
            "I'm sorry, but I can't assist with that.",
            "en",
        )
        .is_some());
    }

    #[test]
    fn retries_transient_and_safety_provider_failures_only() {
        assert!(is_retryable_http_response(429, "rate limit"));
        assert!(is_retryable_http_response(503, "upstream unavailable"));
        assert!(is_retryable_http_response(
            400,
            "provider moderation policy blocked this request",
        ));
        assert!(!is_retryable_http_response(401, "invalid API key"));
        assert!(!is_retryable_http_response(404, "model not found"));
    }

    #[test]
    fn builds_source_language_agnostic_translation_prompts() {
        let prompt =
            title_translation_prompt("The Moon Bride", "zh-CN", &target_language_name("zh-CN"));
        assert!(prompt.contains("whatever source language"));
        assert!(prompt.contains("Simplified Chinese (zh-CN)"));
        assert!(prompt.contains("translatable text inside brackets"));
        assert!(prompt.contains("transliterate"));
    }

    #[test]
    fn settings_responses_never_serialize_api_keys() {
        let mut settings = AISettings::default();
        settings.connection.api_key = Some("secret-value".to_string());
        settings.connection.api_key_configured = true;
        let response = serde_json::to_string(&settings_for_response(settings)).unwrap();
        assert!(!response.contains("secret-value"));
        assert!(response.contains("apiKeyConfigured"));
    }

    #[test]
    fn preserves_legacy_ai_settings_when_reading_the_new_schema() {
        let settings = deserialize_stored_settings(
            r#"{
                "enabled": true,
                "auto_apply_threshold": 0.65,
                "resource_limits": {
                    "max_concurrent_tasks": 4,
                    "timeout_seconds": 180,
                    "max_retries": 5
                }
            }"#,
        );
        assert!(settings.features.auto_tagging.enabled);
        assert_eq!(settings.features.auto_tagging.auto_apply_threshold, 0.65);
        assert_eq!(settings.execution.max_concurrent_tasks, 4);
        assert_eq!(settings.execution.timeout_seconds, 180);
        assert_eq!(settings.execution.max_retries, 5);
    }

    #[tokio::test]
    async fn queues_a_single_job_for_an_unchanged_title_without_network() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for statement in [
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at DATETIME)",
            "CREATE TABLE archives (id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT, subtitle_source_hash TEXT, updated_at DATETIME)",
            "CREATE TABLE archive_title_translations (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, source_title TEXT NOT NULL, source_hash TEXT NOT NULL, target_language TEXT NOT NULL, translated_title TEXT, status TEXT NOT NULL, provider TEXT, model TEXT, last_error TEXT, created_at DATETIME, updated_at DATETIME, completed_at DATETIME, UNIQUE(archive_id, target_language, source_hash))",
            "CREATE TABLE ai_processing_queue (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, status TEXT NOT NULL, priority INTEGER NOT NULL, attempts INTEGER NOT NULL, last_error TEXT, created_at DATETIME, started_at DATETIME, completed_at DATETIME, job_type TEXT NOT NULL, payload TEXT, source_hash TEXT, dedupe_key TEXT, next_run_at DATETIME, lease_expires_at DATETIME)",
            "CREATE UNIQUE INDEX ai_jobs_active_dedupe ON ai_processing_queue (dedupe_key) WHERE dedupe_key IS NOT NULL AND status IN ('pending', 'processing')",
        ] {
            sqlx::query(statement).execute(&pool).await.unwrap();
        }
        let mut settings = AISettings::default();
        settings.features.title_translation.enabled = true;
        save_ai_settings(&pool, settings).await.unwrap();
        sqlx::query("INSERT INTO archives (id, title) VALUES ('archive-1', 'Original title')")
            .execute(&pool)
            .await
            .unwrap();

        assert!(enqueue_title_translation(&pool, "archive-1").await.unwrap());
        assert!(!enqueue_title_translation(&pool, "archive-1").await.unwrap());
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ai_processing_queue")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn force_backfill_requeues_completed_translation_but_preserves_active_one() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for statement in [
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at DATETIME)",
            "CREATE TABLE archives (id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT, subtitle_source_hash TEXT, created_at DATETIME, updated_at DATETIME)",
            "CREATE TABLE archive_title_translations (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, source_title TEXT NOT NULL, source_hash TEXT NOT NULL, target_language TEXT NOT NULL, translated_title TEXT, status TEXT NOT NULL, provider TEXT, model TEXT, last_error TEXT, created_at DATETIME, updated_at DATETIME, completed_at DATETIME, UNIQUE(archive_id, target_language, source_hash))",
            "CREATE TABLE ai_processing_queue (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, status TEXT NOT NULL, priority INTEGER NOT NULL, attempts INTEGER NOT NULL, last_error TEXT, created_at DATETIME, started_at DATETIME, completed_at DATETIME, job_type TEXT NOT NULL, payload TEXT, source_hash TEXT, dedupe_key TEXT, next_run_at DATETIME, lease_expires_at DATETIME)",
            "CREATE UNIQUE INDEX ai_jobs_active_dedupe ON ai_processing_queue (dedupe_key) WHERE dedupe_key IS NOT NULL AND status IN ('pending', 'processing')",
        ] {
            sqlx::query(statement).execute(&pool).await.unwrap();
        }
        let mut settings = AISettings::default();
        settings.features.title_translation.enabled = true;
        save_ai_settings(&pool, settings).await.unwrap();

        let completed_hash = title_hash("The Moon Bride");
        let active_hash = title_hash("The Snow Bride");
        sqlx::query(
            "INSERT INTO archives (id, title, subtitle, subtitle_language, subtitle_source_hash, created_at) VALUES ('completed', 'The Moon Bride', '旧译文', 'zh-CN', ?, CURRENT_TIMESTAMP), ('active', 'The Snow Bride', '进行中的旧译文', 'zh-CN', ?, CURRENT_TIMESTAMP)",
        )
        .bind(&completed_hash)
        .bind(&active_hash)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO archive_title_translations (id, archive_id, source_title, source_hash, target_language, translated_title, status, created_at, updated_at, completed_at) VALUES ('translation-completed', 'completed', 'The Moon Bride', ?, 'zh-CN', '旧译文', 'completed', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(&completed_hash)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO ai_processing_queue (id, archive_id, status, priority, attempts, job_type, source_hash, dedupe_key, created_at, next_run_at) VALUES ('active-job', 'active', 'pending', 0, 0, 'title_translation', ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(&active_hash)
        .bind(format!("{TITLE_TRANSLATION_JOB}:active:{active_hash}"))
        .execute(&pool)
        .await
        .unwrap();

        let result = enqueue_title_translation_backfill(&pool, true)
            .await
            .unwrap();

        assert_eq!(result.queued, 1);
        assert_eq!(result.skipped, 1);
        let completed: (Option<String>, String, Option<String>) = sqlx::query_as(
            "SELECT a.subtitle, t.status, t.translated_title FROM archives a JOIN archive_title_translations t ON t.archive_id = a.id WHERE a.id = 'completed'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(completed, (None, "pending".to_string(), None));
        let active_subtitle: Option<String> =
            sqlx::query_scalar("SELECT subtitle FROM archives WHERE id = 'active'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(active_subtitle.as_deref(), Some("进行中的旧译文"));
    }

    #[tokio::test]
    async fn malformed_title_job_does_not_terminate_queue_processing() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for statement in [
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at DATETIME)",
            "CREATE TABLE archives (id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT, subtitle_source_hash TEXT, updated_at DATETIME)",
            "CREATE TABLE archive_title_translations (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, source_title TEXT NOT NULL, source_hash TEXT NOT NULL, target_language TEXT NOT NULL, translated_title TEXT, status TEXT NOT NULL, provider TEXT, model TEXT, last_error TEXT, created_at DATETIME, updated_at DATETIME, completed_at DATETIME, UNIQUE(archive_id, target_language, source_hash))",
            "CREATE TABLE ai_processing_queue (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, status TEXT NOT NULL, priority INTEGER NOT NULL, attempts INTEGER NOT NULL, last_error TEXT, created_at DATETIME, started_at DATETIME, completed_at DATETIME, job_type TEXT NOT NULL, payload TEXT, source_hash TEXT, dedupe_key TEXT, next_run_at DATETIME, lease_expires_at DATETIME)",
        ] {
            sqlx::query(statement).execute(&pool).await.unwrap();
        }
        let mut settings = AISettings::default();
        settings.features.title_translation.enabled = true;
        settings.execution.max_retries = 1;
        save_ai_settings(&pool, settings).await.unwrap();
        sqlx::query("INSERT INTO ai_processing_queue (id, archive_id, status, priority, attempts, job_type, source_hash) VALUES ('malformed-job', 'archive-1', 'pending', 0, 0, 'title_translation', NULL)")
            .execute(&pool)
            .await
            .unwrap();

        assert!(process_next_job(&pool).await.unwrap());
        let status: String =
            sqlx::query_scalar("SELECT status FROM ai_processing_queue WHERE id = 'malformed-job'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(status, "failed");
    }

    #[tokio::test]
    async fn claims_ready_rfc3339_job_without_claiming_future_job() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE ai_processing_queue (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, status TEXT NOT NULL, priority INTEGER NOT NULL, attempts INTEGER NOT NULL, started_at DATETIME, job_type TEXT NOT NULL, source_hash TEXT, created_at DATETIME, next_run_at DATETIME, lease_expires_at DATETIME)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let today = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let tomorrow = today + ChronoDuration::days(1);
        for (id, priority, next_run_at) in [("ready", 0, today), ("future", 10, tomorrow)] {
            sqlx::query(
                "INSERT INTO ai_processing_queue (id, archive_id, status, priority, attempts, job_type, source_hash, created_at, next_run_at) VALUES (?, ?, 'pending', ?, 0, 'title_translation', 'hash', CURRENT_TIMESTAMP, ?)",
            )
            .bind(id)
            .bind(id)
            .bind(priority)
            .bind(next_run_at)
            .execute(&pool)
            .await
            .unwrap();
        }

        let claimed = claim_next_job(&pool).await.unwrap().unwrap();

        assert_eq!(claimed.id, "ready");
        let future_status: String =
            sqlx::query_scalar("SELECT status FROM ai_processing_queue WHERE id = 'future'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(future_status, "pending");
    }

    #[tokio::test]
    async fn releases_expired_rfc3339_lease_without_releasing_future_lease() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE ai_processing_queue (id TEXT PRIMARY KEY, status TEXT NOT NULL, started_at DATETIME, lease_expires_at DATETIME)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let today = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let tomorrow = today + ChronoDuration::days(1);
        for (id, lease_expires_at) in [("expired", today), ("future", tomorrow)] {
            sqlx::query(
                "INSERT INTO ai_processing_queue (id, status, started_at, lease_expires_at) VALUES (?, 'processing', CURRENT_TIMESTAMP, ?)",
            )
            .bind(id)
            .bind(lease_expires_at)
            .execute(&pool)
            .await
            .unwrap();
        }

        release_expired_leases(&pool).await.unwrap();

        let statuses: Vec<(String, String)> =
            sqlx::query_as("SELECT id, status FROM ai_processing_queue ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(
            statuses,
            vec![
                ("expired".to_string(), "pending".to_string()),
                ("future".to_string(), "processing".to_string()),
            ]
        );
    }
}
