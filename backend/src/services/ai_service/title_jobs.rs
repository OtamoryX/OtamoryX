use super::*;

pub async fn enqueue_title_translation(pool: &Pool<Sqlite>, archive_id: &str) -> Result<bool> {
    let settings = load_ai_settings(pool).await?;
    enqueue_title_translation_with_settings(pool, archive_id, &settings, false, true, true).await
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
    enqueue_title_translation_with_settings(pool, archive_id, &settings, true, false, true).await
}

async fn enqueue_title_translation_with_settings(
    pool: &Pool<Sqlite>,
    archive_id: &str,
    settings: &AISettings,
    force: bool,
    clear_existing_subtitle: bool,
    flush_language_detection: bool,
) -> Result<bool> {
    let feature = &settings.features.title_translation;
    if !feature.enabled {
        return Ok(false);
    }
    let profile_id = select_enabled_profile_id(settings, false)
        .ok_or_else(|| anyhow!("No enabled AI profile is configured"))?;
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

    if feature.skip_if_target_language {
        let stored_decision = stored_title_language_decision(
            &mut transaction,
            archive_id,
            &source_hash,
            &feature.target_language,
        )
        .await?;
        let decision = if let Some(decision) = stored_decision {
            decision
        } else {
            let decision = classify_title_language_locally(&title, &feature.target_language);
            if decision != TitleLanguageDecision::Ambiguous {
                record_local_title_language_decision(
                    &mut transaction,
                    archive_id,
                    &source_hash,
                    &feature.target_language,
                    decision,
                    local_title_language_decision_source(&title, &feature.target_language),
                )
                .await?;
            }
            decision
        };
        match decision {
            TitleLanguageDecision::Target => {
                transaction.commit().await?;
                return Ok(false);
            }
            TitleLanguageDecision::Ambiguous => {
                let inserted = create_title_language_detection(
                    &mut transaction,
                    archive_id,
                    &source_hash,
                    &feature.target_language,
                )
                .await?;
                if clear_existing_subtitle && subtitle.is_some() {
                    sqlx::query(
                        "UPDATE archives SET subtitle = NULL, subtitle_language = NULL, subtitle_source_hash = NULL, updated_at = ? WHERE id = ?",
                    )
                    .bind(Utc::now())
                    .bind(archive_id)
                    .execute(&mut *transaction)
                    .await?;
                }
                transaction.commit().await?;
                if inserted && flush_language_detection {
                    enqueue_pending_title_language_detection_batches(pool, settings).await?;
                }
                return Ok(inserted);
            }
            TitleLanguageDecision::NonTarget => {}
        }
    }
    if !force {
        let already_confirmed_as_target = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM archive_title_translations WHERE archive_id = ? AND target_language = ? AND source_hash = ? AND status = 'completed' AND translated_title IS NULL",
        )
        .bind(archive_id)
        .bind(&feature.target_language)
        .bind(&source_hash)
        .fetch_one(&mut *transaction)
        .await?
            > 0;
        if already_confirmed_as_target {
            return Ok(false);
        }
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

    let legacy_dedupe_key = format!("{TITLE_TRANSLATION_JOB}:{archive_id}:{source_hash}");
    let legacy_job_active = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM ai_processing_queue WHERE dedupe_key = ? AND status IN ('pending', 'processing')",
    )
    .bind(&legacy_dedupe_key)
    .fetch_one(&mut *transaction)
    .await?
        > 0;
    if legacy_job_active {
        return Ok(false);
    }

    let dedupe_key = format!(
        "{TITLE_TRANSLATION_JOB}:{archive_id}:{source_hash}:{}",
        feature.target_language
    );
    let now = Utc::now();
    let queue_result = sqlx::query(
        r#"
        INSERT OR IGNORE INTO ai_processing_queue (
            id, archive_id, status, priority, attempts, job_type, payload, source_hash, dedupe_key,
            profile_id, created_at, next_run_at
        ) VALUES (?, ?, 'pending', 0, 0, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(archive_id)
    .bind(TITLE_TRANSLATION_JOB)
    .bind(serde_json::to_string(&TitleTranslationPayload {
        target_language: feature.target_language.clone(),
    })?)
    .bind(&source_hash)
    .bind(&dedupe_key)
    .bind(&profile_id)
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
    notify_ai_queue();
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
        if enqueue_title_translation_with_settings(pool, &archive_id, &settings, force, true, false)
            .await?
        {
            result.queued += 1;
        } else {
            result.skipped += 1;
        }
    }
    // Ambiguous Han titles have only created persisted detection records so far. Combine them
    // into compact requests after the scan rather than issuing one confirmation per archive.
    enqueue_pending_title_language_detection_batches(pool, &settings).await?;
    Ok(result)
}

async fn stored_title_language_decision(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    archive_id: &str,
    source_hash: &str,
    target_language: &str,
) -> Result<Option<TitleLanguageDecision>> {
    let stored = sqlx::query_scalar::<_, Option<bool>>(
        "SELECT is_target_language FROM archive_title_language_detections \
         WHERE archive_id = ? AND source_hash = ? AND target_language = ? AND status = 'completed'",
    )
    .bind(archive_id)
    .bind(source_hash)
    .bind(target_language)
    .fetch_optional(&mut **transaction)
    .await?;
    if let Some(Some(is_target)) = stored {
        return Ok(Some(if is_target {
            TitleLanguageDecision::Target
        } else {
            TitleLanguageDecision::NonTarget
        }));
    }
    Ok(None)
}

async fn record_local_title_language_decision(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    archive_id: &str,
    source_hash: &str,
    target_language: &str,
    decision: TitleLanguageDecision,
    source: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO archive_title_language_detections \
         (archive_id, target_language, source_hash, status, is_target_language, decision_source, created_at, updated_at, completed_at) \
         VALUES (?, ?, ?, 'completed', ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) \
         ON CONFLICT(archive_id, target_language, source_hash) DO UPDATE SET \
             status = 'completed', is_target_language = excluded.is_target_language, \
             decision_source = excluded.decision_source, last_error = NULL, \
             updated_at = CURRENT_TIMESTAMP, completed_at = CURRENT_TIMESTAMP",
    )
    .bind(archive_id)
    .bind(target_language)
    .bind(source_hash)
    .bind(decision == TitleLanguageDecision::Target)
    .bind(source)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn create_title_language_detection(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    archive_id: &str,
    source_hash: &str,
    target_language: &str,
) -> Result<bool> {
    let retried = sqlx::query(
        "UPDATE archive_title_language_detections SET status = 'pending', last_error = NULL, updated_at = CURRENT_TIMESTAMP \
         WHERE archive_id = ? AND target_language = ? AND source_hash = ? AND status = 'failed'",
    )
    .bind(archive_id)
    .bind(target_language)
    .bind(source_hash)
    .execute(&mut **transaction)
    .await?;
    if retried.rows_affected() == 1 {
        return Ok(true);
    }
    let inserted = sqlx::query(
        "INSERT OR IGNORE INTO archive_title_language_detections \
         (archive_id, target_language, source_hash, status, created_at, updated_at) \
         VALUES (?, ?, ?, 'pending', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind(archive_id)
    .bind(target_language)
    .bind(source_hash)
    .execute(&mut **transaction)
    .await?;
    Ok(inserted.rows_affected() == 1)
}

async fn enqueue_pending_title_language_detection_batches(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
) -> Result<usize> {
    let target = &settings.features.title_translation.target_language;
    let profile_id = select_enabled_profile_id(settings, false)
        .ok_or_else(|| anyhow!("No enabled AI profile is configured"))?;
    let mut queued = 0;
    loop {
        let mut transaction = pool.begin().await?;
        let rows = sqlx::query(
            "SELECT d.archive_id, d.source_hash, a.title FROM archive_title_language_detections d \
             JOIN archives a ON a.id = d.archive_id \
             WHERE d.target_language = ? AND d.status = 'pending' \
             ORDER BY d.created_at ASC LIMIT ?",
        )
        .bind(target)
        .bind(TITLE_LANGUAGE_DETECTION_BATCH_SIZE)
        .fetch_all(&mut *transaction)
        .await?;
        if rows.is_empty() {
            transaction.commit().await?;
            break;
        }
        let items: Vec<TitleLanguageBatchItem> = rows
            .iter()
            .map(|row| TitleLanguageBatchItem {
                archive_id: row.get("archive_id"),
                source_hash: row.get("source_hash"),
                title: row.get("title"),
            })
            .collect();
        let first_archive_id = items[0].archive_id.clone();
        let payload = serde_json::to_string(&TitleLanguageBatchPayload {
            target_language: target.clone(),
            items,
        })?;
        let dedupe_key = format!("{TITLE_LANGUAGE_DETECTION_JOB}:{}", title_hash(&payload));
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO ai_processing_queue \
             (id, archive_id, status, priority, attempts, job_type, payload, dedupe_key, profile_id, created_at, next_run_at) \
             VALUES (?, ?, 'pending', 1, 0, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(first_archive_id)
        .bind(TITLE_LANGUAGE_DETECTION_JOB)
        .bind(&payload)
        .bind(&dedupe_key)
        .bind(&profile_id)
        .bind(now)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        let payload: TitleLanguageBatchPayload = serde_json::from_str(&payload)?;
        for item in &payload.items {
            sqlx::query(
                "UPDATE archive_title_language_detections SET status = 'queued', updated_at = ? \
                 WHERE archive_id = ? AND source_hash = ? AND target_language = ? AND status = 'pending'",
            )
            .bind(now)
            .bind(&item.archive_id)
            .bind(&item.source_hash)
            .bind(target)
            .execute(&mut *transaction)
            .await?;
        }
        transaction.commit().await?;
        queued += 1;
    }
    if queued > 0 {
        notify_ai_queue();
    }
    Ok(queued)
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

        if enqueue_title_translation_with_settings(pool, &archive_id, &settings, true, false, true)
            .await?
        {
            result.queued += 1;
        } else {
            result.skipped += 1;
        }
    }
    Ok(result)
}

async fn enqueue_title_translation_in_transaction(
    transaction: &mut sqlx::Transaction<'_, Sqlite>,
    archive_id: &str,
    source_hash: &str,
    target_language: &str,
    profile_id: &str,
) -> std::result::Result<(), TitleTranslationJobError> {
    let row = sqlx::query("SELECT title FROM archives WHERE id = ? LIMIT 1")
        .bind(archive_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(|err| {
            TitleTranslationJobError::retryable(format!("failed to load archive: {err}"))
        })?;
    let Some(row) = row else { return Ok(()) };
    let title: String = row.get("title");
    if title_hash(&title) != source_hash {
        return Ok(());
    }
    let now = Utc::now();
    sqlx::query(
        "INSERT OR IGNORE INTO ai_processing_queue \
         (id, archive_id, status, priority, attempts, job_type, payload, source_hash, dedupe_key, profile_id, created_at, next_run_at) \
         VALUES (?, ?, 'pending', 0, 0, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(archive_id)
    .bind(TITLE_TRANSLATION_JOB)
    .bind(
        serde_json::to_string(&TitleTranslationPayload {
            target_language: target_language.to_string(),
        })
        .map_err(|err| {
            TitleTranslationJobError::permanent(format!(
                "failed to encode translation payload: {err}"
            ))
        })?,
    )
    .bind(source_hash)
    .bind(format!(
        "{TITLE_TRANSLATION_JOB}:{archive_id}:{source_hash}:{target_language}"
    ))
    .bind(profile_id)
    .bind(now)
    .bind(now)
    .execute(&mut **transaction)
    .await
    .map_err(|err| TitleTranslationJobError::retryable(format!("failed to queue translation: {err}")))?;
    sqlx::query(
        "INSERT INTO archive_title_translations \
         (id, archive_id, source_title, source_hash, target_language, status, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, 'pending', ?, ?) \
         ON CONFLICT(archive_id, target_language, source_hash) DO UPDATE SET \
             status = 'pending', last_error = NULL, updated_at = excluded.updated_at",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(archive_id)
    .bind(title)
    .bind(source_hash)
    .bind(target_language)
    .bind(now)
    .bind(now)
    .execute(&mut **transaction)
    .await
    .map_err(|err| TitleTranslationJobError::retryable(format!("failed to track translation: {err}")))?;
    Ok(())
}

fn title_translation_target(
    payload: Option<&str>,
) -> std::result::Result<String, TitleTranslationJobError> {
    let payload = payload.ok_or_else(|| {
        TitleTranslationJobError::permanent("title translation job has no payload")
    })?;
    let target = serde_json::from_str::<TitleTranslationPayload>(payload)
        .map_err(|err| {
            TitleTranslationJobError::permanent(format!("invalid title translation payload: {err}"))
        })?
        .target_language;
    if target.trim().is_empty() {
        return Err(TitleTranslationJobError::permanent(
            "title translation job has no target language",
        ));
    }
    Ok(target)
}

pub(super) async fn process_title_translation_job(
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

    let target_language = title_translation_target(job.payload.as_deref())?;
    let translated = translate_title(settings, &title, &target_language).await?;
    if matches!(translated, TitleTranslationOutput::AlreadyInTargetLanguage) {
        sqlx::query(
            r#"
            UPDATE archive_title_translations
            SET translated_title = NULL, status = 'completed', provider = ?, model = ?, last_error = NULL,
                completed_at = ?, updated_at = ?
            WHERE archive_id = ? AND source_hash = ? AND target_language = ?
            "#,
        )
        .bind(&settings.connection.provider)
        .bind(&settings.connection.model)
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(&job.archive_id)
        .bind(source_hash)
        .bind(&target_language)
        .execute(pool)
        .await
        .map_err(|err| {
            TitleTranslationJobError::retryable(format!(
                "failed to record target-language title: {err}"
            ))
        })?;
        return Ok(());
    }
    let TitleTranslationOutput::Translated(translated) = translated else {
        unreachable!("already-target titles return above");
    };
    if translated.trim().is_empty() || translated.len() > 1_000 {
        return Err(TitleTranslationJobError::retryable(
            "model returned an invalid translated title",
        ));
    }
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
    .bind(&target_language)
    .execute(pool)
    .await
    .map_err(|err| {
        TitleTranslationJobError::retryable(format!("failed to save translated title: {err}"))
    })?;
    let updated = sqlx::query(
        "UPDATE archives SET subtitle = ?, subtitle_language = ?, subtitle_source_hash = ?, updated_at = ? WHERE id = ? AND title = ?",
    )
    .bind(translated)
    .bind(&target_language)
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

pub(super) async fn title_translation_target_from_raw_job(
    pool: &Pool<Sqlite>,
    job_id: &str,
) -> Result<String> {
    let payload = sqlx::query_scalar::<_, Option<String>>(
        "SELECT payload FROM ai_processing_queue WHERE id = ?",
    )
    .bind(job_id)
    .fetch_one(pool)
    .await?;
    title_translation_target(payload.as_deref()).map_err(anyhow::Error::new)
}

pub(super) async fn process_title_language_detection_job(
    pool: &Pool<Sqlite>,
    settings: &AISettings,
    job: &ClaimedJob,
) -> std::result::Result<(), TitleTranslationJobError> {
    let payload = job.payload.as_deref().ok_or_else(|| {
        TitleTranslationJobError::permanent("title language detection job has no payload")
    })?;
    let batch = parse_title_language_batch_payload(pool, payload).await?;
    let target = batch.target_language.trim().to_string();
    let items = batch.items;
    if items.is_empty() || items.len() > TITLE_LANGUAGE_DETECTION_BATCH_SIZE as usize {
        return Err(TitleTranslationJobError::permanent(
            "title language detection batch has an invalid size",
        ));
    }
    if target.is_empty() {
        return Err(TitleTranslationJobError::permanent(
            "title language detection job has no target language",
        ));
    }
    let decisions = detect_title_languages_with_model(settings, &items, &target).await?;
    let submitted: HashSet<(&str, &str)> = items
        .iter()
        .map(|item| (item.archive_id.as_str(), item.source_hash.as_str()))
        .collect();
    let returned: HashSet<(&str, &str)> = decisions
        .iter()
        .map(|decision| (decision.archive_id.as_str(), decision.source_hash.as_str()))
        .collect();
    if decisions.len() != items.len() || returned.len() != items.len() || returned != submitted {
        return Err(TitleTranslationJobError::retryable(
            "AI title-language response does not cover exactly the submitted titles",
        ));
    }

    let mut transaction = pool.begin().await.map_err(|err| {
        TitleTranslationJobError::retryable(format!("failed to begin detection transaction: {err}"))
    })?;
    let now = Utc::now();
    for decision in decisions {
        let updated = sqlx::query(
            "UPDATE archive_title_language_detections \
             SET status = 'completed', is_target_language = ?, decision_source = 'model_batch', \
                 last_error = NULL, completed_at = ?, updated_at = ? \
             WHERE archive_id = ? AND source_hash = ? AND target_language = ?",
        )
        .bind(decision.is_target_language)
        .bind(now)
        .bind(now)
        .bind(&decision.archive_id)
        .bind(&decision.source_hash)
        .bind(&target)
        .execute(&mut *transaction)
        .await
        .map_err(|err| {
            TitleTranslationJobError::retryable(format!(
                "failed to store title-language decision: {err}"
            ))
        })?;
        if updated.rows_affected() != 1 {
            return Err(TitleTranslationJobError::permanent(
                "title-language record changed before the batch completed",
            ));
        }
        if !decision.is_target_language {
            enqueue_title_translation_in_transaction(
                &mut transaction,
                &decision.archive_id,
                &decision.source_hash,
                &target,
                &settings.active_profile_id,
            )
            .await?;
        }
    }
    transaction.commit().await.map_err(|err| {
        TitleTranslationJobError::retryable(format!(
            "failed to commit title-language decisions: {err}"
        ))
    })?;
    notify_ai_queue();
    Ok(())
}

pub(super) async fn parse_title_language_batch_payload(
    pool: &Pool<Sqlite>,
    payload: &str,
) -> std::result::Result<TitleLanguageBatchPayload, TitleTranslationJobError> {
    if let Ok(batch) = serde_json::from_str::<TitleLanguageBatchPayload>(payload) {
        return Ok(batch);
    }

    let items = serde_json::from_str::<Vec<TitleLanguageBatchItem>>(payload).map_err(|err| {
        TitleTranslationJobError::permanent(format!(
            "invalid title language detection payload: {err}"
        ))
    })?;
    let Some(first) = items.first() else {
        return Ok(TitleLanguageBatchPayload {
            target_language: String::new(),
            items,
        });
    };
    let target_language = sqlx::query_scalar::<_, String>(
        "SELECT target_language FROM archive_title_language_detections \
         WHERE archive_id = ? AND source_hash = ? AND status = 'queued' \
         ORDER BY updated_at DESC LIMIT 1",
    )
    .bind(&first.archive_id)
    .bind(&first.source_hash)
    .fetch_optional(pool)
    .await
    .map_err(|err| {
        TitleTranslationJobError::retryable(format!(
            "failed to recover legacy title-language target: {err}"
        ))
    })?
    .ok_or_else(|| {
        TitleTranslationJobError::permanent(
            "legacy title language detection job has no queued target record",
        )
    })?;
    Ok(TitleLanguageBatchPayload {
        target_language,
        items,
    })
}
