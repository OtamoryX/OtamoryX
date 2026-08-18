use super::*;

pub fn spawn_ai_worker(pool: Pool<Sqlite>) {
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

    for slot in 0..MAX_AI_WORKERS {
        let supervisor_pool = pool.clone();
        let worker_signal = signal.clone();
        tokio::spawn(async move {
            loop {
                let worker_pool = supervisor_pool.clone();
                let signal = worker_signal.clone();
                match tokio::spawn(async move { run_ai_worker(worker_pool, slot, signal).await })
                    .await
                {
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

async fn run_ai_worker(pool: Pool<Sqlite>, slot: usize, signal: Arc<AiQueueSignal>) {
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
            .max_concurrent_tasks
            .clamp(1, MAX_AI_WORKERS);
        if slot >= worker_limit {
            // Configuration changes are picked up on the next queue wakeup without making idle
            // disabled slots issue their own database polling queries.
            notified.await;
            continue;
        }

        match process_next_job_with_settings(&pool, &settings).await {
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
    if !settings.features.title_translation.enabled {
        return Ok(false);
    }
    active_enabled_profile_id(&settings)?;
    let Some(job) = claim_next_job(pool).await? else {
        return Ok(false);
    };
    let job_settings = settings_for_profile(settings, job.profile_id.as_deref());
    let outcome = match job_settings.as_ref() {
        Ok(job_settings) if active_enabled_profile_id(job_settings).is_ok() => {
            if !provider_is_available(pool, job_settings).await? {
                defer_job_for_provider_cooldown(pool, job_settings, &job.id).await?;
                return Ok(false);
            }
            match job.job_type.as_str() {
                TITLE_TRANSLATION_JOB => {
                    process_title_translation_job(pool, job_settings, &job).await
                }
                TITLE_LANGUAGE_DETECTION_JOB => {
                    process_title_language_detection_job(pool, job_settings, &job).await
                }
                unexpected => Err(TitleTranslationJobError::permanent(format!(
                    "unsupported AI job type `{unexpected}`"
                ))),
            }
        }
        Ok(_) => {
            defer_job_for_disabled_profile(pool, &job.id).await?;
            return Ok(false);
        }
        Err(err) => Err(TitleTranslationJobError::permanent(format!(
            "AI profile for this job is unavailable: {err}"
        ))),
    };
    let failure_settings = job_settings.as_ref().unwrap_or(settings);
    match outcome {
        Ok(()) => complete_job(pool, &job.id).await?,
        Err(err) => {
            fail_or_retry_job(
                pool,
                failure_settings,
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
         WHERE status = 'pending' AND job_type IN (?, ?) AND next_run_at IS NOT NULL",
    )
    .bind(TITLE_LANGUAGE_DETECTION_JOB)
    .bind(TITLE_TRANSLATION_JOB)
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
    let row = sqlx::query(
        r#"
        SELECT id, archive_id, source_hash, job_type, payload, profile_id
        FROM ai_processing_queue
        WHERE status = 'pending' AND job_type IN (?, ?)
          AND (next_run_at IS NULL OR julianday(next_run_at) <= julianday('now'))
        ORDER BY
          CASE WHEN job_type = ? AND EXISTS (
              SELECT 1 FROM ai_queue_scheduler_state
              WHERE id = 'default' AND last_job_type = ?
          ) THEN 1 ELSE 0 END,
          priority DESC, created_at ASC
        LIMIT 1
        "#,
    )
    .bind(TITLE_LANGUAGE_DETECTION_JOB)
    .bind(TITLE_TRANSLATION_JOB)
    .bind(TITLE_LANGUAGE_DETECTION_JOB)
    .bind(TITLE_LANGUAGE_DETECTION_JOB)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else { return Ok(None) };
    let job = ClaimedJob {
        id: row.get("id"),
        archive_id: row.get("archive_id"),
        source_hash: row.try_get("source_hash")?,
        job_type: row.get("job_type"),
        payload: row.try_get("payload")?,
        profile_id: row.try_get("profile_id")?,
    };
    let lease_expires_at = Utc::now() + ChronoDuration::minutes(10);
    let claimed = sqlx::query(
        "UPDATE ai_processing_queue SET status = 'processing', attempts = attempts + 1, started_at = CURRENT_TIMESTAMP, lease_expires_at = ? WHERE id = ? AND status = 'pending'",
    )
    .bind(lease_expires_at)
    .bind(&job.id)
    .execute(pool)
    .await?;
    if claimed.rows_affected() != 1 {
        return Ok(None);
    }
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
    job_id: &str,
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
    if error.retry_policy == RetryPolicy::ProviderCooldown {
        block_provider_until(pool, settings, retry_at, &error.message).await?;
    }
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
    finish_job_attempt(
        pool,
        job_id,
        if final_failure {
            "dead_letter"
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
        ai_queue_signal().scheduler.notify_one();
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

async fn defer_job_for_provider_cooldown(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    job_id: &str,
) -> Result<()> {
    let blocked_until = sqlx::query_scalar::<_, Option<String>>(
        "SELECT blocked_until FROM ai_provider_states WHERE provider = ? AND model = ?",
    )
    .bind(&settings.connection.provider)
    .bind(provider_state_model(settings))
    .fetch_optional(pool)
    .await?
    .flatten()
    .unwrap_or_else(|| (Utc::now() + ChronoDuration::minutes(1)).to_rfc3339());
    sqlx::query(
        "UPDATE ai_processing_queue SET status = 'pending', started_at = NULL, lease_expires_at = NULL, next_run_at = ? WHERE id = ?",
    )
    .bind(blocked_until)
    .bind(job_id)
    .execute(pool)
    .await?;
    finish_job_attempt(pool, job_id, "provider_cooldown", None).await?;
    ai_queue_signal().scheduler.notify_one();
    Ok(())
}

async fn defer_job_for_disabled_profile(pool: &Pool<Sqlite>, job_id: &str) -> Result<()> {
    sqlx::query(
        "UPDATE ai_processing_queue SET status = 'pending', started_at = NULL, lease_expires_at = NULL, next_run_at = ? WHERE id = ?",
    )
    .bind(Utc::now() + ChronoDuration::minutes(1))
    .bind(job_id)
    .execute(pool)
    .await?;
    finish_job_attempt(pool, job_id, "profile_disabled", None).await?;
    ai_queue_signal().scheduler.notify_one();
    Ok(())
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
