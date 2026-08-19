use super::*;

const MODEL_AVAILABILITY_WAIT_ERROR: &str = "waiting for AI model availability";
pub(crate) const FORCED_MODEL_RETRY_ATTEMPTS: i64 = 3;

/// What an enqueue request may change when the durable queue has already accepted the same
/// active work item. The unique active dedupe index remains the authority for coalescing.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ActiveQueueConflict<'a> {
    Ignore,
    RaisePriority,
    RaisePriorityAndReplacePayload(&'a str),
}

/// Enqueues work through the common durable queue. An active task with the same dedupe key is
/// never duplicated; selected callers may only raise its urgency or upgrade its payload.
pub(crate) async fn enqueue_pipeline_job(
    pool: &Pool<Sqlite>,
    archive_id: &str,
    fingerprint: &str,
    job_type: &str,
    payload: &str,
    executor_lane: &str,
    profile_id: Option<&str>,
    priority: i32,
    dedupe_key: &str,
    on_active_conflict: ActiveQueueConflict<'_>,
) -> Result<bool> {
    let mut transaction = pool.begin().await?;
    let inserted = sqlx::query(
        "INSERT OR IGNORE INTO ai_processing_queue \
         (id, archive_id, status, priority, attempts, job_type, payload, source_hash, dedupe_key, profile_id, executor_lane, created_at, next_run_at) \
         VALUES (?, ?, 'pending', ?, 0, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(archive_id)
    .bind(priority)
    .bind(job_type)
    .bind(payload)
    .bind(fingerprint)
    .bind(dedupe_key)
    .bind(profile_id)
    .bind(executor_lane)
    .execute(&mut *transaction)
    .await?;

    if inserted.rows_affected() == 0 {
        match on_active_conflict {
            ActiveQueueConflict::Ignore => {}
            ActiveQueueConflict::RaisePriority => {
                sqlx::query(
                    "UPDATE ai_processing_queue SET priority = MAX(priority, ?) \
                     WHERE job_type = ? AND dedupe_key = ? AND status IN ('pending', 'processing')",
                )
                .bind(priority)
                .bind(job_type)
                .bind(dedupe_key)
                .execute(&mut *transaction)
                .await?;
            }
            ActiveQueueConflict::RaisePriorityAndReplacePayload(payload) => {
                sqlx::query(
                    "UPDATE ai_processing_queue SET priority = MAX(priority, ?), payload = ? \
                     WHERE job_type = ? AND dedupe_key = ? AND status IN ('pending', 'processing')",
                )
                .bind(priority)
                .bind(payload)
                .bind(job_type)
                .bind(dedupe_key)
                .execute(&mut *transaction)
                .await?;
            }
        }
    }
    transaction.commit().await?;

    if inserted.rows_affected() > 0 {
        notify_ai_queue();
    }
    Ok(inserted.rows_affected() > 0)
}

/// Starts the durable project-wide worker pool. The historical table is retained as
/// `ai_processing_queue` for migration compatibility, but it now schedules LLM, OCR, plugin,
/// and orchestration work through one lease/retry state machine.
pub fn spawn_job_worker(pool: Pool<Sqlite>) {
    let signal = ai_queue_signal().clone();
    let reaper_pool = pool.clone();
    tokio::spawn(async move {
        run_ai_lease_reaper(reaper_pool).await;
    });

    let scheduler_pool = pool.clone();
    let scheduler_signal = signal.clone();
    tokio::spawn(async move {
        run_ai_retry_scheduler(scheduler_pool, scheduler_signal).await;
    });

    for executor_lane in crate::models::AI_EXECUTOR_LANES {
        for slot in 0..MAX_AI_WORKERS_PER_LANE {
            let supervisor_pool = pool.clone();
            let worker_signal = signal.clone();
            tokio::spawn(async move {
                loop {
                    let worker_pool = supervisor_pool.clone();
                    let signal = worker_signal.clone();
                    match tokio::spawn(async move {
                        run_ai_worker(worker_pool, executor_lane, slot, signal).await
                    })
                    .await
                    {
                        Ok(()) => warn!(
                            executor_lane,
                            slot, "AI worker stopped unexpectedly; restarting"
                        ),
                        Err(err) => {
                            tracing::error!(executor_lane, slot, error = %err, "AI worker panicked; restarting")
                        }
                    }
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            });
        }
    }
}

/// Compatibility name for callers compiled against the former translation-only worker.
pub fn spawn_ai_worker(pool: Pool<Sqlite>) {
    spawn_job_worker(pool);
}

async fn run_ai_worker(
    pool: Pool<Sqlite>,
    executor_lane: &'static str,
    slot: usize,
    signal: Arc<AiQueueSignal>,
) {
    // Drain work that was persisted before this process started, then block until a producer or
    // the retry scheduler signals new work. The first pass is intentionally unconditional so a
    // restart never depends on an in-memory notification that no longer exists.
    loop {
        let notified = signal.work.notified();
        tokio::pin!(notified);
        // Register before querying SQLite so an enqueue racing with an empty claim cannot be lost.
        notified.as_mut().enable();

        let settings = match load_ai_settings(&pool).await {
            Ok(settings) => settings,
            Err(error) => {
                warn!("AI worker settings load failed: {error:#}");
                tokio::select! {
                    _ = notified => {}
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                }
                continue;
            }
        };
        let worker_limit = settings
            .execution
            .lanes
            .limit_for_lane(executor_lane)
            .expect("worker was started for a known executor lane")
            .clamp(1, MAX_AI_WORKERS_PER_LANE);
        if slot >= worker_limit {
            // Configuration changes are picked up on the next queue wakeup without making idle
            // disabled slots issue their own database polling queries.
            notified.await;
            continue;
        }

        match process_next_job_for_lane_with_settings(&pool, &settings, Some(executor_lane)).await {
            Ok(true) => continue,
            Ok(false) => notified.await,
            Err(err) => {
                warn!("AI worker iteration failed: {err:#}");
                tokio::select! {
                    _ = notified => {}
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                }
            }
        }
    }
}

/// Runs one job at most. Public to make the queue behavior testable without a background worker.
pub async fn process_next_job(pool: &Pool<Sqlite>) -> Result<bool> {
    let settings = load_ai_settings(pool).await?;
    process_next_job_with_settings(pool, &settings).await
}

async fn process_next_job_with_settings(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
) -> Result<bool> {
    process_next_job_for_lane_with_settings(pool, settings, None).await
}

async fn process_next_job_for_lane_with_settings(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    executor_lane: Option<&str>,
) -> Result<bool> {
    let Some(job) = claim_next_job_for_lane(pool, executor_lane).await? else {
        return Ok(false);
    };
    enum QueueOutcome {
        Complete,
        Deferred(i64),
        Failed(TitleTranslationJobError),
    }

    let mut execution_settings = None;
    let outcome = match job.job_type.as_str() {
        TITLE_TRANSLATION_JOB | TITLE_LANGUAGE_DETECTION_JOB => {
            if !settings.features.title_translation.enabled {
                QueueOutcome::Failed(TitleTranslationJobError::permanent(
                    "Title translation is disabled",
                ))
            } else {
                let Some(job_settings) = select_available_job_settings(
                    pool,
                    settings,
                    job.profile_id.as_deref(),
                    &job.job_type,
                )
                .await?
                else {
                    defer_job_type_for_unavailable_models(pool, settings, &job).await?;
                    return Ok(false);
                };
                update_job_profile(pool, &job.id, &job_settings.active_profile_id).await?;
                execution_settings = Some(job_settings.clone());
                if job.job_type == TITLE_TRANSLATION_JOB {
                    process_title_translation_job(pool, &job_settings, &job)
                        .await
                        .map(|_| QueueOutcome::Complete)
                        .unwrap_or_else(QueueOutcome::Failed)
                } else {
                    process_title_language_detection_job(pool, &job_settings, &job)
                        .await
                        .map(|_| QueueOutcome::Complete)
                        .unwrap_or_else(QueueOutcome::Failed)
                }
            }
        }
        CONTENT_ANALYSIS_RECONCILE_JOB
        | CONTENT_ANALYSIS_SYNTHESIZE_JOB
        | OCR_EXTRACT_JOB
        | METADATA_EXTRACT_JOB
        | AUTO_TAGGING_JOB => {
            let uses_provider = matches!(
                job.job_type.as_str(),
                CONTENT_ANALYSIS_SYNTHESIZE_JOB | AUTO_TAGGING_JOB
            );
            let job_settings = if uses_provider {
                let Some(selected) = select_available_job_settings(
                    pool,
                    settings,
                    job.profile_id.as_deref(),
                    &job.job_type,
                )
                .await?
                else {
                    defer_job_type_for_unavailable_models(pool, settings, &job).await?;
                    return Ok(false);
                };
                update_job_profile(pool, &job.id, &selected.active_profile_id).await?;
                execution_settings = Some(selected.clone());
                selected
            } else {
                settings.clone()
            };
            match crate::services::content_analysis::service::process_workflow_job(
                pool,
                &job_settings,
                &job.id,
                &job.archive_id,
                job.source_hash.as_deref(),
                &job.job_type,
            )
            .await
            {
                Ok(crate::services::content_analysis::service::WorkflowJobResult::Completed) => {
                    QueueOutcome::Complete
                }
                Ok(crate::services::content_analysis::service::WorkflowJobResult::Deferred(
                    seconds,
                )) => QueueOutcome::Deferred(seconds),
                Err(err) => {
                    let queue_error = err
                        .downcast_ref::<ProviderRequestError>()
                        .map(|provider_error| {
                            TitleTranslationJobError::provider_unavailable(
                                provider_error.to_string(),
                                provider_error.retry_after_seconds(),
                            )
                        })
                        .unwrap_or_else(|| TitleTranslationJobError::retryable(err.to_string()));
                    QueueOutcome::Failed(queue_error)
                }
            }
        }
        unexpected => QueueOutcome::Failed(TitleTranslationJobError::permanent(format!(
            "unsupported background job type `{unexpected}`"
        ))),
    };
    match outcome {
        QueueOutcome::Complete => {
            if let Some(execution_settings) = execution_settings.as_ref() {
                clear_provider_cooldown_after_success(pool, execution_settings).await?;
            }
            complete_job(pool, &job.id).await?
        }
        QueueOutcome::Deferred(seconds) => {
            defer_job_for_dependency(pool, &job.id, seconds).await?;
        }
        QueueOutcome::Failed(err) => {
            fail_or_retry_job(
                pool,
                settings,
                execution_settings.as_ref(),
                &job.id,
                &job.job_type,
                &job.archive_id,
                job.source_hash.as_deref(),
                &err,
            )
            .await?
        }
    }
    Ok(true)
}

/// The task's current profile is preferred so a retry stays with the fallback that was selected
/// for it. Other enabled profiles follow the configured list order.
fn enabled_profile_ids_in_failover_order(
    settings: &AISettings,
    preferred_profile_id: Option<&str>,
) -> Vec<String> {
    let preferred_profile_id = preferred_profile_id.unwrap_or(&settings.active_profile_id);
    let mut profile_ids = Vec::with_capacity(settings.profiles.len());
    if settings
        .profiles
        .iter()
        .any(|profile| profile.id == preferred_profile_id && profile.enabled)
    {
        profile_ids.push(preferred_profile_id.to_string());
    }
    profile_ids.extend(
        settings
            .profiles
            .iter()
            .filter(|profile| profile.enabled && profile.id != preferred_profile_id)
            .map(|profile| profile.id.clone()),
    );
    profile_ids
}

async fn select_available_job_settings(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    preferred_profile_id: Option<&str>,
    job_type: &str,
) -> Result<Option<AISettings>> {
    let profile_ids = enabled_profile_ids_in_failover_order(settings, preferred_profile_id);
    for profile_id in &profile_ids {
        let profile_settings = settings_for_profile(settings, Some(&profile_id))?;
        if provider_is_available(pool, &profile_settings).await? {
            clear_forced_model_attempt(pool, job_type).await?;
            return Ok(Some(profile_settings));
        }
    }
    // A forced continue is deliberately one probe only. It does not clear the model cooldown or
    // release the rest of the queue, so an operator cannot accidentally replay a whole backlog.
    if consume_forced_model_attempt(pool, job_type).await? {
        if let Some(profile_id) =
            earliest_recovering_profile_id(pool, settings, &profile_ids).await?
        {
            return settings_for_profile(settings, Some(&profile_id)).map(Some);
        }
    }
    Ok(None)
}

async fn update_job_profile(pool: &Pool<Sqlite>, job_id: &str, profile_id: &str) -> Result<()> {
    sqlx::query("UPDATE ai_processing_queue SET profile_id = ? WHERE id = ?")
        .bind(profile_id)
        .bind(job_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn defer_job_type_for_unavailable_models(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    job: &ClaimedJob,
) -> Result<()> {
    let available_at = earliest_model_recheck_at(pool, settings, job.profile_id.as_deref()).await?;
    let error = format!("{MODEL_AVAILABILITY_WAIT_ERROR} until {available_at}");
    // Do not let every worker claim and defer a different item from the same task type. All work
    // that is ready now waits together until a model can be tried again.
    sqlx::query(
        "UPDATE ai_processing_queue SET next_run_at = ?, last_error = ? \
         WHERE job_type = ? AND status = 'pending' \
           AND (next_run_at IS NULL OR julianday(next_run_at) <= julianday('now'))",
    )
    .bind(available_at)
    .bind(&error)
    .bind(&job.job_type)
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE ai_processing_queue SET status = 'pending', started_at = NULL, lease_expires_at = NULL, \
         next_run_at = ?, last_error = ? WHERE id = ?",
    )
    .bind(available_at)
    .bind(&error)
    .bind(&job.id)
    .execute(pool)
    .await?;
    finish_job_attempt(pool, &job.id, "waiting_model", Some(&error)).await?;
    ai_queue_signal().scheduler.notify_one();
    Ok(())
}

async fn defer_job_for_dependency(pool: &Pool<Sqlite>, job_id: &str, seconds: i64) -> Result<()> {
    let available_at = Utc::now() + ChronoDuration::seconds(seconds.clamp(5, 3_600));
    sqlx::query(
        "UPDATE ai_processing_queue SET status = 'pending', attempts = CASE WHEN attempts > 0 THEN attempts - 1 ELSE 0 END, \
         started_at = NULL, lease_expires_at = NULL, next_run_at = ?, last_error = 'waiting for dependency' \
         WHERE id = ?",
    )
    .bind(available_at)
    .bind(job_id)
    .execute(pool)
    .await?;
    finish_job_attempt(pool, job_id, "waiting_dependency", None).await?;
    ai_queue_signal().scheduler.notify_one();
    Ok(())
}

pub(crate) async fn release_expired_leases(pool: &Pool<Sqlite>) -> Result<u64> {
    // A process can disappear after the provider accepted a request. Keep the attempt audit
    // explicit and retry the idempotent work item once its lease expires.
    sqlx::query(
        "UPDATE ai_job_attempts SET finished_at = CURRENT_TIMESTAMP, outcome = 'lease_expired', \
         error = 'worker lease expired before recording an outcome' \
         WHERE finished_at IS NULL AND job_id IN ( \
             SELECT id FROM ai_processing_queue WHERE status = 'processing' \
             AND lease_expires_at IS NOT NULL AND julianday(lease_expires_at) < julianday('now') \
         )",
    )
    .execute(pool)
    .await?;
    let released = sqlx::query(
        "UPDATE ai_processing_queue SET status = 'pending', started_at = NULL, lease_expires_at = NULL \
         WHERE status = 'processing' AND lease_expires_at IS NOT NULL \
           AND julianday(lease_expires_at) < julianday('now')",
    )
    .execute(pool)
    .await?;
    Ok(released.rows_affected())
}

async fn run_ai_lease_reaper(pool: Pool<Sqlite>) {
    // Lease recovery is deliberately isolated from the worker hot path. It is a safety net for
    // crashed or stuck requests, not the mechanism used to notice ordinary new work.
    loop {
        match release_expired_leases(&pool).await {
            Ok(released) if released > 0 => notify_ai_queue(),
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "AI lease reaper iteration failed"),
        }
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

async fn next_retry_delay(pool: &Pool<Sqlite>) -> Result<Option<Duration>> {
    let delay_seconds: Option<f64> = sqlx::query_scalar(
        "SELECT MIN(julianday(next_run_at) - julianday('now')) \
         FROM ai_processing_queue \
         WHERE status = 'pending' AND next_run_at IS NOT NULL",
    )
    .fetch_one(pool)
    .await?;
    Ok(delay_seconds
        .map(|seconds| Duration::from_secs_f64((seconds.max(0.0) * 86_400.0).min(86_400.0))))
}

async fn run_ai_retry_scheduler(pool: Pool<Sqlite>, signal: Arc<AiQueueSignal>) {
    loop {
        let notified = signal.scheduler.notified();
        tokio::pin!(notified);
        // The scheduler can be woken by a newly scheduled retry while it is reading the current
        // minimum due time. Enable the future first so that update cannot be missed.
        notified.as_mut().enable();
        let delay = match next_retry_delay(&pool).await {
            Ok(delay) => delay,
            Err(error) => {
                tracing::warn!(%error, "AI retry scheduler query failed");
                Some(Duration::from_secs(30))
            }
        };
        match delay {
            Some(delay) if delay.is_zero() => {
                // A due retry is a work signal, not a reason to spin. If workers are disabled,
                // wait for a settings/queue event before checking this due row again.
                signal.work.notify_waiters();
                notified.await;
            }
            Some(delay) => {
                tokio::select! {
                    _ = notified => {}
                    _ = tokio::time::sleep(delay) => notify_ai_queue(),
                }
            }
            None => notified.await,
        }
    }
}

pub(crate) async fn claim_next_job(pool: &Pool<Sqlite>) -> Result<Option<ClaimedJob>> {
    claim_next_job_for_lane(pool, None).await
}

async fn claim_next_job_for_lane(
    pool: &Pool<Sqlite>,
    executor_lane: Option<&str>,
) -> Result<Option<ClaimedJob>> {
    let lease_expires_at = Utc::now() + ChronoDuration::minutes(10);
    let mut query = sqlx::QueryBuilder::<Sqlite>::new(
        "UPDATE ai_processing_queue \
         SET status = 'processing', attempts = attempts + 1, started_at = CURRENT_TIMESTAMP, lease_expires_at = ",
    );
    query.push_bind(lease_expires_at).push(
        " WHERE id = ( \
         SELECT id FROM ai_processing_queue \
         WHERE status = 'pending' \
           AND (next_run_at IS NULL OR julianday(next_run_at) <= julianday('now')) \
           AND NOT EXISTS ( \
               SELECT 1 FROM ai_queue_controls control \
               WHERE control.job_type = ai_processing_queue.job_type \
                 AND control.manually_paused = 1 \
           )",
    );
    if let Some(executor_lane) = executor_lane {
        query.push(" AND executor_lane = ").push_bind(executor_lane);
    }
    query
        .push(" ORDER BY CASE WHEN job_type = ")
        .push_bind(TITLE_LANGUAGE_DETECTION_JOB)
        .push(
            " AND EXISTS ( \
            SELECT 1 FROM ai_queue_scheduler_state \
            WHERE id = 'default' AND last_job_type = ",
        )
        .push_bind(TITLE_LANGUAGE_DETECTION_JOB)
        .push(
            ") THEN 1 ELSE 0 END, priority DESC, created_at ASC LIMIT 1 \
         ) AND status = 'pending' \
         RETURNING id, archive_id, source_hash, job_type, payload, profile_id",
        );
    let row = query.build().fetch_optional(pool).await?;
    let Some(row) = row else { return Ok(None) };
    let job = ClaimedJob {
        id: row.get("id"),
        archive_id: row.get("archive_id"),
        source_hash: row.try_get("source_hash")?,
        job_type: row.get("job_type"),
        payload: row.try_get("payload")?,
        profile_id: row.try_get("profile_id")?,
    };
    sqlx::query(
        "INSERT INTO ai_job_attempts (id, job_id, attempt_number, started_at) \
         SELECT ?, id, attempts, CURRENT_TIMESTAMP FROM ai_processing_queue WHERE id = ?",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&job.id)
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE ai_queue_scheduler_state SET last_job_type = ?, updated_at = CURRENT_TIMESTAMP WHERE id = 'default'",
    )
    .bind(&job.job_type)
    .execute(pool)
    .await?;
    Ok(Some(job))
}

async fn complete_job(pool: &Pool<Sqlite>, job_id: &str) -> Result<()> {
    sqlx::query(
        "UPDATE ai_processing_queue SET status = 'completed', completed_at = CURRENT_TIMESTAMP, lease_expires_at = NULL, last_error = NULL WHERE id = ?",
    )
    .bind(job_id)
    .execute(pool)
    .await?;
    finish_job_attempt(pool, job_id, "completed", None).await?;
    Ok(())
}

async fn fail_or_retry_job(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    execution_settings: Option<&AISettings>,
    job_id: &str,
    job_type: &str,
    archive_id: &str,
    source_hash: Option<&str>,
    error: &TitleTranslationJobError,
) -> Result<()> {
    let attempts: i64 = sqlx::query_scalar("SELECT attempts FROM ai_processing_queue WHERE id = ?")
        .bind(job_id)
        .fetch_one(pool)
        .await?;
    // Short provider outages must not become terminal jobs merely because they last longer than
    // a few seconds. Only malformed work and repeatedly invalid model output enter the dead
    // letter state; transport/provider retries remain durable work items.
    let final_failure = error.retry_policy == RetryPolicy::Permanent
        || (error.retry_policy == RetryPolicy::Limited
            && attempts > settings.execution.max_retries as i64);
    let status = if final_failure { "failed" } else { "pending" };
    let retry_delay = error
        .retry_after_seconds
        .unwrap_or_else(|| durable_retry_delay_seconds(attempts));
    let retry_at = Utc::now() + ChronoDuration::seconds(retry_delay.clamp(60, 86_400));
    let failover_profile_id = if error.retry_policy == RetryPolicy::ProviderCooldown {
        let Some(execution_settings) = execution_settings else {
            return Err(anyhow!(
                "provider failure did not include its execution profile"
            ));
        };
        block_provider_until(pool, execution_settings, retry_at, &error.message).await?;
        select_available_job_settings(
            pool,
            settings,
            Some(&execution_settings.active_profile_id),
            job_type,
        )
        .await?
        .map(|next| next.active_profile_id)
    } else {
        None
    };
    let next_run_at = if failover_profile_id.is_some() {
        Utc::now()
    } else {
        retry_at
    };
    sqlx::query(
        "UPDATE ai_processing_queue SET status = ?, profile_id = COALESCE(?, profile_id), last_error = ?, next_run_at = ?, completed_at = CASE WHEN ? = 'failed' THEN CURRENT_TIMESTAMP ELSE NULL END, lease_expires_at = NULL WHERE id = ?",
    )
    .bind(status)
    .bind(&failover_profile_id)
    .bind(&error.message)
    .bind(next_run_at)
    .bind(status)
    .bind(job_id)
    .execute(pool)
    .await?;
    finish_job_attempt(
        pool,
        job_id,
        if final_failure {
            "dead_letter"
        } else if failover_profile_id.is_some() {
            "failover"
        } else {
            "retry_scheduled"
        },
        Some(&error.message),
    )
    .await?;
    if final_failure && job_is_title_language_detection(pool, job_id).await? {
        mark_title_language_detection_batch_failed(pool, job_id, &error.message).await?;
    }
    if let (Some(source_hash), Ok(target_language)) = (
        source_hash,
        title_translation_target_from_raw_job(pool, job_id).await,
    ) {
        sqlx::query(
            "UPDATE archive_title_translations SET status = ?, last_error = ?, updated_at = CURRENT_TIMESTAMP WHERE archive_id = ? AND source_hash = ? AND target_language = ?",
        )
        .bind(status)
        .bind(&error.message)
        .bind(archive_id)
        .bind(source_hash)
        .bind(target_language)
        .execute(pool)
        .await?;
    }
    if status == "pending" {
        if failover_profile_id.is_some() {
            notify_ai_queue();
        } else {
            ai_queue_signal().scheduler.notify_one();
        }
    }
    Ok(())
}

fn durable_retry_delay_seconds(attempts: i64) -> i64 {
    // The deterministic offset prevents workers from retrying every item at precisely the same
    // instant while making next_run_at explainable from the attempt count.
    let base = match attempts {
        1 => 60,
        2 => 5 * 60,
        3 => 30 * 60,
        4 => 2 * 60 * 60,
        5 => 12 * 60 * 60,
        _ => 24 * 60 * 60,
    };
    base + (attempts.rem_euclid(17) * 7)
}

async fn finish_job_attempt(
    pool: &Pool<Sqlite>,
    job_id: &str,
    outcome: &str,
    error: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "UPDATE ai_job_attempts SET finished_at = CURRENT_TIMESTAMP, outcome = ?, error = ? \
         WHERE job_id = ? AND finished_at IS NULL",
    )
    .bind(outcome)
    .bind(error)
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn provider_is_available(pool: &Pool<Sqlite>, settings: &AISettings) -> Result<bool> {
    // Reserve a manually-authorized retry before bypassing cooldown. The update is atomic so
    // concurrently running queue workers can collectively issue at most the configured probes.
    let force_reserved = sqlx::query(
        "UPDATE ai_provider_states SET force_attempts_remaining = force_attempts_remaining - 1, \
         updated_at = CURRENT_TIMESTAMP WHERE provider = ? AND model = ? \
           AND blocked_until IS NOT NULL AND julianday(blocked_until) > julianday('now') \
           AND force_attempts_remaining > 0",
    )
    .bind(&settings.connection.provider)
    .bind(provider_state_model(settings))
    .execute(pool)
    .await?;
    if force_reserved.rows_affected() == 1 {
        return Ok(true);
    }
    let blocked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_provider_states WHERE provider = ? AND model = ? \
         AND blocked_until IS NOT NULL AND julianday(blocked_until) > julianday('now')",
    )
    .bind(&settings.connection.provider)
    .bind(provider_state_model(settings))
    .fetch_one(pool)
    .await?;
    Ok(blocked == 0)
}

async fn earliest_model_recheck_at(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    preferred_profile_id: Option<&str>,
) -> Result<chrono::DateTime<Utc>> {
    let mut earliest = None;
    for profile_id in enabled_profile_ids_in_failover_order(settings, preferred_profile_id) {
        let profile_settings = settings_for_profile(settings, Some(&profile_id))?;
        let blocked_until = sqlx::query_scalar::<_, Option<chrono::DateTime<Utc>>>(
            "SELECT blocked_until FROM ai_provider_states WHERE provider = ? AND model = ? \
             AND blocked_until IS NOT NULL AND julianday(blocked_until) > julianday('now')",
        )
        .bind(&profile_settings.connection.provider)
        .bind(provider_state_model(&profile_settings))
        .fetch_optional(pool)
        .await?
        .flatten();
        if let Some(blocked_until) = blocked_until {
            earliest = Some(match earliest {
                Some(current) if current <= blocked_until => current,
                _ => blocked_until,
            });
        }
    }
    // A disabled or newly misconfigured model has no provider-provided recovery time. Keep the
    // queue dormant briefly, while settings changes wake it immediately through notify_ai_queue.
    Ok(earliest.unwrap_or_else(|| Utc::now() + ChronoDuration::minutes(1)))
}

async fn earliest_recovering_profile_id(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    profile_ids: &[String],
) -> Result<Option<String>> {
    let mut earliest = None;
    for profile_id in profile_ids {
        let profile_settings = settings_for_profile(settings, Some(profile_id))?;
        let blocked_until = sqlx::query_scalar::<_, Option<chrono::DateTime<Utc>>>(
            "SELECT blocked_until FROM ai_provider_states WHERE provider = ? AND model = ? \
             AND blocked_until IS NOT NULL AND julianday(blocked_until) > julianday('now')",
        )
        .bind(&profile_settings.connection.provider)
        .bind(provider_state_model(&profile_settings))
        .fetch_optional(pool)
        .await?
        .flatten();
        match (earliest.as_ref(), blocked_until) {
            (None, Some(blocked_until)) => earliest = Some((blocked_until, profile_id.clone())),
            (Some((current, _)), Some(blocked_until)) if blocked_until < *current => {
                earliest = Some((blocked_until, profile_id.clone()))
            }
            _ => {}
        }
    }
    Ok(earliest
        .map(|(_, profile_id)| profile_id)
        .or_else(|| profile_ids.first().cloned()))
}

async fn clear_forced_model_attempt(pool: &Pool<Sqlite>, job_type: &str) -> Result<()> {
    sqlx::query(
        "UPDATE ai_queue_controls SET force_next_model_attempt = 0, updated_at = CURRENT_TIMESTAMP \
         WHERE job_type = ? AND force_next_model_attempt = 1",
    )
    .bind(job_type)
    .execute(pool)
    .await?;
    Ok(())
}

async fn consume_forced_model_attempt(pool: &Pool<Sqlite>, job_type: &str) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE ai_queue_controls SET force_next_model_attempt = 0, updated_at = CURRENT_TIMESTAMP \
         WHERE job_type = ? AND force_next_model_attempt = 1",
    )
    .bind(job_type)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

async fn block_provider_until(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    blocked_until: chrono::DateTime<Utc>,
    error: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO ai_provider_states (provider, model, blocked_until, last_error, updated_at) \
         VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP) \
         ON CONFLICT(provider, model) DO UPDATE SET \
             blocked_until = CASE WHEN ai_provider_states.blocked_until IS NULL \
                                       OR julianday(excluded.blocked_until) > julianday(ai_provider_states.blocked_until) \
                                  THEN excluded.blocked_until ELSE ai_provider_states.blocked_until END, \
             last_error = excluded.last_error, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(&settings.connection.provider)
    .bind(provider_state_model(settings))
    .bind(blocked_until)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}

async fn clear_provider_cooldown_after_success(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
) -> Result<()> {
    sqlx::query(
        "UPDATE ai_provider_states SET blocked_until = NULL, last_error = NULL, \
         force_attempts_remaining = 0, updated_at = CURRENT_TIMESTAMP \
         WHERE provider = ? AND model = ?",
    )
    .bind(&settings.connection.provider)
    .bind(provider_state_model(settings))
    .execute(pool)
    .await?;
    Ok(())
}

async fn job_is_title_language_detection(pool: &Pool<Sqlite>, job_id: &str) -> Result<bool> {
    let job_type =
        sqlx::query_scalar::<_, String>("SELECT job_type FROM ai_processing_queue WHERE id = ?")
            .bind(job_id)
            .fetch_one(pool)
            .await?;
    Ok(job_type == TITLE_LANGUAGE_DETECTION_JOB)
}

async fn mark_title_language_detection_batch_failed(
    pool: &Pool<Sqlite>,
    job_id: &str,
    error: &str,
) -> Result<()> {
    let payload = sqlx::query_scalar::<_, Option<String>>(
        "SELECT payload FROM ai_processing_queue WHERE id = ?",
    )
    .bind(job_id)
    .fetch_one(pool)
    .await?;
    let Some(payload) = payload else {
        return Ok(());
    };
    let batch = parse_title_language_batch_payload(pool, &payload)
        .await
        .map_err(anyhow::Error::new)?;
    for item in batch.items {
        sqlx::query(
            "UPDATE archive_title_language_detections SET status = 'failed', last_error = ?, updated_at = CURRENT_TIMESTAMP \
             WHERE archive_id = ? AND source_hash = ? AND target_language = ? AND status = 'queued'",
        )
        .bind(error)
        .bind(item.archive_id)
        .bind(item.source_hash)
        .bind(&batch.target_language)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_profiles_use_the_job_profile_then_configured_order() {
        let mut settings = AISettings::default();
        let mut primary = AIConnectionProfile::default_profile();
        primary.id = "primary".to_string();
        let mut disabled = AIConnectionProfile::default_profile();
        disabled.id = "disabled".to_string();
        disabled.enabled = false;
        let mut fallback = AIConnectionProfile::default_profile();
        fallback.id = "fallback".to_string();
        settings.profiles = vec![primary, disabled, fallback];
        settings.active_profile_id = "primary".to_string();

        assert_eq!(
            enabled_profile_ids_in_failover_order(&settings, None),
            vec!["primary", "fallback"]
        );
        assert_eq!(
            enabled_profile_ids_in_failover_order(&settings, Some("fallback")),
            vec!["fallback", "primary"]
        );
    }

    #[tokio::test]
    async fn unavailable_provider_moves_job_to_next_enabled_profile() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for statement in [
            "CREATE TABLE ai_processing_queue (id TEXT PRIMARY KEY, attempts INTEGER NOT NULL, status TEXT NOT NULL, profile_id TEXT, last_error TEXT, next_run_at DATETIME, completed_at DATETIME, lease_expires_at DATETIME)",
            "CREATE TABLE ai_provider_states (provider TEXT NOT NULL, model TEXT NOT NULL, blocked_until DATETIME, last_error TEXT, force_attempts_remaining INTEGER NOT NULL DEFAULT 0, updated_at DATETIME, PRIMARY KEY (provider, model))",
            "CREATE TABLE ai_job_attempts (id TEXT PRIMARY KEY, job_id TEXT NOT NULL, attempt_number INTEGER NOT NULL, started_at DATETIME, finished_at DATETIME, outcome TEXT, error TEXT)",
            "CREATE TABLE ai_queue_controls (job_type TEXT PRIMARY KEY, manually_paused INTEGER NOT NULL DEFAULT 0, force_next_model_attempt INTEGER NOT NULL DEFAULT 0, updated_at DATETIME)",
        ] {
            sqlx::query(statement).execute(&pool).await.unwrap();
        }
        sqlx::query(
            "INSERT INTO ai_processing_queue (id, attempts, status, profile_id) VALUES ('job', 1, 'processing', 'primary')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let mut settings = AISettings::default();
        let mut primary = AIConnectionProfile::default_profile();
        primary.id = "primary".to_string();
        let mut fallback = AIConnectionProfile::default_profile();
        fallback.id = "fallback".to_string();
        settings.profiles = vec![primary, fallback];
        settings.active_profile_id = "primary".to_string();
        let primary_settings = settings_for_profile(&settings, Some("primary")).unwrap();

        fail_or_retry_job(
            &pool,
            &settings,
            Some(&primary_settings),
            "job",
            TITLE_TRANSLATION_JOB,
            "archive",
            None,
            &TitleTranslationJobError::provider_unavailable("primary unavailable", Some(120)),
        )
        .await
        .unwrap();

        let profile_id: String =
            sqlx::query_scalar("SELECT profile_id FROM ai_processing_queue WHERE id = 'job'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(profile_id, "fallback");
        let blocked: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ai_provider_states WHERE provider = ? AND model = ?",
        )
        .bind(&primary_settings.connection.provider)
        .bind(provider_state_model(&primary_settings))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(blocked, 1);
    }

    #[tokio::test]
    async fn forced_model_continue_recools_after_the_configured_number_of_failed_probes() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE ai_provider_states (provider TEXT NOT NULL, model TEXT NOT NULL, blocked_until DATETIME, last_error TEXT, force_attempts_remaining INTEGER NOT NULL DEFAULT 0, updated_at DATETIME, PRIMARY KEY (provider, model))",
        )
        .execute(&pool)
        .await
        .unwrap();
        let settings = AISettings::default();
        block_provider_until(
            &pool,
            &settings,
            Utc::now() + ChronoDuration::minutes(5),
            "AI provider returned HTTP 429: rate limit",
        )
        .await
        .unwrap();
        sqlx::query(
            "UPDATE ai_provider_states SET force_attempts_remaining = ? WHERE provider = ? AND model = ?",
        )
        .bind(FORCED_MODEL_RETRY_ATTEMPTS)
        .bind(&settings.connection.provider)
        .bind(provider_state_model(&settings))
        .execute(&pool)
        .await
        .unwrap();

        for _ in 0..FORCED_MODEL_RETRY_ATTEMPTS {
            assert!(provider_is_available(&pool, &settings).await.unwrap());
            block_provider_until(
                &pool,
                &settings,
                Utc::now() + ChronoDuration::minutes(5),
                "AI provider returned HTTP 429: rate limit",
            )
            .await
            .unwrap();
        }
        assert!(!provider_is_available(&pool, &settings).await.unwrap());
        let remaining: i64 = sqlx::query_scalar(
            "SELECT force_attempts_remaining FROM ai_provider_states WHERE provider = ? AND model = ?",
        )
        .bind(&settings.connection.provider)
        .bind(provider_state_model(&settings))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(remaining, 0);
        let still_blocked: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ai_provider_states WHERE provider = ? AND model = ? \
             AND blocked_until IS NOT NULL AND julianday(blocked_until) > julianday('now')",
        )
        .bind(&settings.connection.provider)
        .bind(provider_state_model(&settings))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(still_blocked, 1);
    }

    #[tokio::test]
    async fn unavailable_models_defer_only_the_affected_task_queue() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for statement in [
            "CREATE TABLE ai_processing_queue (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, status TEXT NOT NULL, priority INTEGER NOT NULL, attempts INTEGER NOT NULL, job_type TEXT NOT NULL, source_hash TEXT, payload TEXT, profile_id TEXT, last_error TEXT, next_run_at DATETIME, started_at DATETIME, lease_expires_at DATETIME, created_at DATETIME)",
            "CREATE TABLE ai_provider_states (provider TEXT NOT NULL, model TEXT NOT NULL, blocked_until DATETIME, last_error TEXT, force_attempts_remaining INTEGER NOT NULL DEFAULT 0, updated_at DATETIME, PRIMARY KEY (provider, model))",
            "CREATE TABLE ai_job_attempts (id TEXT PRIMARY KEY, job_id TEXT NOT NULL, attempt_number INTEGER NOT NULL, started_at DATETIME, finished_at DATETIME, outcome TEXT, error TEXT)",
            "CREATE TABLE ai_queue_controls (job_type TEXT PRIMARY KEY, manually_paused INTEGER NOT NULL DEFAULT 0, force_next_model_attempt INTEGER NOT NULL DEFAULT 0, updated_at DATETIME)",
        ] {
            sqlx::query(statement).execute(&pool).await.unwrap();
        }

        let settings = AISettings::default();
        block_provider_until(
            &pool,
            &settings,
            Utc::now() + ChronoDuration::minutes(5),
            "AI provider returned HTTP 429: rate limit",
        )
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO ai_processing_queue (id, archive_id, status, priority, attempts, job_type, next_run_at, created_at) VALUES \
             ('claimed', 'archive-a', 'processing', 0, 1, 'title_translation', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP), \
             ('same-task', 'archive-b', 'pending', 0, 0, 'title_translation', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP), \
             ('other-task', 'archive-c', 'pending', 0, 0, 'ocr_extract', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .unwrap();

        defer_job_type_for_unavailable_models(
            &pool,
            &settings,
            &ClaimedJob {
                id: "claimed".to_string(),
                archive_id: "archive-a".to_string(),
                source_hash: None,
                job_type: TITLE_TRANSLATION_JOB.to_string(),
                payload: None,
                profile_id: None,
            },
        )
        .await
        .unwrap();

        let deferred: Vec<(String, String, Option<String>)> = sqlx::query_as(
            "SELECT id, status, last_error FROM ai_processing_queue WHERE id IN ('claimed', 'same-task') ORDER BY id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(deferred.len(), 2);
        assert!(deferred.iter().all(|(_, status, error)| {
            status == "pending"
                && error
                    .as_deref()
                    .is_some_and(|message| message.starts_with(MODEL_AVAILABILITY_WAIT_ERROR))
        }));
        let other_task: (String, Option<String>) = sqlx::query_as(
            "SELECT status, last_error FROM ai_processing_queue WHERE id = 'other-task'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(other_task, ("pending".to_string(), None));
    }

    #[tokio::test]
    async fn lane_worker_only_claims_its_own_executor_lane() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for statement in [
            "CREATE TABLE ai_processing_queue (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, status TEXT NOT NULL, priority INTEGER NOT NULL, attempts INTEGER NOT NULL, job_type TEXT NOT NULL, payload TEXT, source_hash TEXT, profile_id TEXT, executor_lane TEXT NOT NULL, created_at DATETIME NOT NULL, next_run_at DATETIME, started_at DATETIME, lease_expires_at DATETIME)",
            "CREATE TABLE ai_queue_controls (job_type TEXT PRIMARY KEY, manually_paused INTEGER NOT NULL DEFAULT 0, force_next_model_attempt INTEGER NOT NULL DEFAULT 0, updated_at DATETIME)",
            "CREATE TABLE ai_queue_scheduler_state (id TEXT PRIMARY KEY, last_job_type TEXT, updated_at DATETIME)",
            "CREATE TABLE ai_job_attempts (id TEXT PRIMARY KEY, job_id TEXT NOT NULL, attempt_number INTEGER NOT NULL, started_at DATETIME, finished_at DATETIME, outcome TEXT, error TEXT)",
        ] {
            sqlx::query(statement).execute(&pool).await.unwrap();
        }
        sqlx::query("INSERT INTO ai_queue_scheduler_state (id) VALUES ('default')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO ai_processing_queue (id, archive_id, status, priority, attempts, job_type, executor_lane, created_at, next_run_at) VALUES \
             ('llm-first', 'archive-a', 'pending', 100, 0, 'title_translation', 'llm', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP), \
             ('ocr-work', 'archive-b', 'pending', 1, 0, 'ocr_extract', 'ocr', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let ocr_job = claim_next_job_for_lane(&pool, Some("ocr"))
            .await
            .unwrap()
            .expect("OCR worker should claim its own queued work");
        assert_eq!(ocr_job.id, "ocr-work");

        let llm_job = claim_next_job_for_lane(&pool, Some("llm"))
            .await
            .unwrap()
            .expect("LLM worker should still claim its own queued work");
        assert_eq!(llm_job.id, "llm-first");
    }
}
