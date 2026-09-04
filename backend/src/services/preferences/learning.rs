//! Incremental, evidence-backed preference learning.
//!
//! The learner starts at the migration boundary. It never replays legacy
//! behavior and never derives rules from a fixed semantic vocabulary. Content
//! features come from the deterministic profile document; behavior comes from
//! one idempotent aggregate per user/archive pair.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use sqlx::{Pool, Row, Sqlite};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::{Arc, OnceLock},
    time::Duration as StdDuration,
};
use tokio::sync::Notify;
use uuid::Uuid;

use crate::models::{
    ArchiveContentProfileDocument, ContentProfileFeature, PreferenceInsightCandidate,
    CANONICAL_THEME_FEATURE_KIND,
};
use crate::services::content_profile::CONTENT_PROFILE_VERSION;

const COLD_START_VERSION: &str = "cold-start-v1";
const CANDIDATE_SOURCE: &str = "cold_start_v1";
const LEARNED_RULE_SOURCE: &str = "learned_cold_start";
const MIN_OBSERVING_ARCHIVES: usize = 3;
const MIN_FORMAL_ARCHIVES: usize = 8;
const MIN_FORMAL_RESULTS: usize = 12;
const MIN_LIFT: f64 = 0.10;
const SIGNAL_HALF_LIFE_DAYS: f64 = 30.0;
const MAX_RETRIES: i64 = 5;

static PREFERENCE_LEARNING_SIGNAL: OnceLock<Arc<Notify>> = OnceLock::new();

fn preference_learning_signal() -> &'static Arc<Notify> {
    PREFERENCE_LEARNING_SIGNAL.get_or_init(|| Arc::new(Notify::new()))
}

pub fn notify_preference_learning_worker() {
    preference_learning_signal().notify_waiters();
}

#[derive(Clone)]
pub struct PreferenceLearningService {
    pool: Pool<Sqlite>,
}

#[derive(Debug, Clone, Default)]
struct FeedbackAggregate {
    archive_id: String,
    open_count: i64,
    page_turn_count: i64,
    exit_count: i64,
    continue_count: i64,
    repeat_open_count: i64,
    restore_count: i64,
    correction_count: i64,
    max_page: i64,
    max_progress_ratio: f64,
    effective_read: bool,
    deep_read: bool,
    completed_read: bool,
    quick_exit: bool,
    manual_delete: i64,
    delete_stage: Option<String>,
    total_duration_ms: i64,
    max_duration_ms: i64,
    recommendation_exposure_count: i64,
    first_recommendation_position: Option<i64>,
    visibility_confidence: f64,
    algorithm_variants: Vec<String>,
    last_event_at: DateTime<Utc>,
    profile_coverage: f64,
    profile_version: String,
    features: Vec<ContentProfileFeature>,
}

#[derive(Debug, Clone)]
struct EventMetrics {
    open_count: i64,
    page_turn_count: i64,
    exit_count: i64,
    continue_count: i64,
    repeat_open_count: i64,
    restore_count: i64,
    correction_count: i64,
    page: i64,
    progress_ratio: f64,
    effective_read: bool,
    deep_read: bool,
    completed_read: bool,
    quick_exit: bool,
    manual_delete: i64,
    duration_ms: i64,
    recommendation_exposure_count: i64,
    recommendation_position: Option<i64>,
    visibility_confidence: f64,
    algorithm_variant: Option<String>,
}

#[derive(Debug, Clone)]
struct CandidateStats {
    condition_key: String,
    conditions: Value,
    feature_kind: String,
    profile_version: String,
    sample_archives: BTreeSet<String>,
    positive_archives: HashSet<String>,
    negative_archives: HashSet<String>,
    positive_score: f64,
    negative_score: f64,
    effective_read_count: i64,
    deep_read_count: i64,
    manual_delete_count: i64,
    quick_exit_count: i64,
    conflict_count: i64,
    informative_result_count: i64,
    profile_coverage_sum: f64,
    open_count: i64,
    page_turn_count: i64,
    exit_count: i64,
    continue_count: i64,
    repeat_open_count: i64,
    restore_count: i64,
    correction_count: i64,
    exposure_archive_count: i64,
    position_sum: f64,
    visibility_confidence_sum: f64,
    progress_ratio_sum: f64,
    max_page: i64,
    total_duration_ms: i64,
    max_duration_ms: i64,
    delete_before_open_count: i64,
    delete_after_open_count: i64,
    delete_after_effective_read_count: i64,
    algorithm_variant_counts: BTreeMap<String, i64>,
}

impl CandidateStats {
    fn new(
        condition_key: String,
        conditions: Value,
        feature_kind: String,
        profile_version: String,
    ) -> Self {
        Self {
            condition_key,
            conditions,
            feature_kind,
            profile_version,
            sample_archives: BTreeSet::new(),
            positive_archives: HashSet::new(),
            negative_archives: HashSet::new(),
            positive_score: 0.0,
            negative_score: 0.0,
            effective_read_count: 0,
            deep_read_count: 0,
            manual_delete_count: 0,
            quick_exit_count: 0,
            conflict_count: 0,
            informative_result_count: 0,
            profile_coverage_sum: 0.0,
            open_count: 0,
            page_turn_count: 0,
            exit_count: 0,
            continue_count: 0,
            repeat_open_count: 0,
            restore_count: 0,
            correction_count: 0,
            exposure_archive_count: 0,
            position_sum: 0.0,
            visibility_confidence_sum: 0.0,
            progress_ratio_sum: 0.0,
            max_page: 0,
            total_duration_ms: 0,
            max_duration_ms: 0,
            delete_before_open_count: 0,
            delete_after_open_count: 0,
            delete_after_effective_read_count: 0,
            algorithm_variant_counts: BTreeMap::new(),
        }
    }

    fn record_metrics(&mut self, aggregate: &FeedbackAggregate) {
        self.open_count += aggregate.open_count;
        self.page_turn_count += aggregate.page_turn_count;
        self.exit_count += aggregate.exit_count;
        self.continue_count += aggregate.continue_count;
        self.repeat_open_count += aggregate.repeat_open_count;
        self.restore_count += aggregate.restore_count;
        self.correction_count += aggregate.correction_count;
        self.progress_ratio_sum += aggregate.max_progress_ratio;
        self.max_page = self.max_page.max(aggregate.max_page);
        self.total_duration_ms += aggregate.total_duration_ms;
        self.max_duration_ms = self.max_duration_ms.max(aggregate.max_duration_ms);
        if aggregate.recommendation_exposure_count > 0 {
            self.exposure_archive_count += 1;
            if let Some(position) = aggregate.first_recommendation_position {
                self.position_sum += position as f64;
                self.visibility_confidence_sum += aggregate.visibility_confidence;
            }
        }
        match aggregate.delete_stage.as_deref() {
            Some("before_open") => self.delete_before_open_count += 1,
            Some("after_open") => self.delete_after_open_count += 1,
            Some("after_effective_read") => self.delete_after_effective_read_count += 1,
            _ => {}
        }
        for variant in &aggregate.algorithm_variants {
            *self
                .algorithm_variant_counts
                .entry(variant.clone())
                .or_default() += 1;
        }
    }
}

impl PreferenceLearningService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Enqueue only events that happened after the cold-start marker. Existing
    /// queue rows may be present from the old learner, so the claim query also
    /// repeats this boundary check.
    pub async fn enqueue_events(&self) -> Result<u64> {
        let result = sqlx::query(
            "INSERT OR IGNORE INTO preference_learning_events (id, behavior_event_id, user_id)
             SELECT lower(hex(randomblob(16))), event.id, event.user_id
             FROM user_behavior_events event
             WHERE event.event_type IN
                ('open','page_turn','exit','continue_reading','repeat_open',
                 'manual_delete','restore','rule_correction')
               AND event.occurred_at >= (
                   SELECT cold_start_started_at FROM preference_learning_state WHERE id = 'default'
               )",
        )
        .execute(&self.pool)
        .await?;
        let queued = result.rows_affected();
        if queued > 0 {
            notify_preference_learning_worker();
        }
        Ok(queued)
    }

    /// Enqueue one newly inserted event. The startup scan remains the recovery path for events
    /// written while this process was down or by an older caller.
    pub(crate) async fn enqueue_event(&self, event_id: &str, user_id: &str) -> Result<bool> {
        let result = sqlx::query(
            "INSERT OR IGNORE INTO preference_learning_events (id, behavior_event_id, user_id)
             SELECT lower(hex(randomblob(16))), event.id, event.user_id
             FROM user_behavior_events event
             WHERE event.id = ? AND event.user_id = ?
               AND event.event_type IN
                  ('open','page_turn','exit','continue_reading','repeat_open',
                   'manual_delete','restore','rule_correction')
               AND event.occurred_at >= (
                   SELECT cold_start_started_at FROM preference_learning_state WHERE id = 'default'
               )",
        )
        .bind(event_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        let queued = result.rows_affected() == 1;
        if queued {
            notify_preference_learning_worker();
        }
        Ok(queued)
    }

    pub async fn process_next(&self) -> Result<bool> {
        self.enqueue_events().await?;
        self.process_next_queued().await
    }

    /// Move only the events for an archive whose profile has just become usable back into the
    /// normal queue. Waiting events are otherwise dormant until this dependency signal arrives.
    pub(crate) async fn wake_waiting_for_archive(&self, archive_id: &str) -> Result<u64> {
        let updated = sqlx::query(
            "UPDATE preference_learning_events
             SET status = 'pending', next_attempt_at = NULL, last_error = NULL,
                 updated_at = CURRENT_TIMESTAMP
             WHERE status = 'waiting_analysis'
               AND behavior_event_id IN (
                   SELECT id FROM user_behavior_events WHERE archive_id = ?
               )",
        )
        .bind(archive_id)
        .execute(&self.pool)
        .await?;
        let count = updated.rows_affected();
        if count > 0 {
            notify_preference_learning_worker();
        }
        Ok(count)
    }

    /// Recover notifications that were lost while the process was down. This remains a low
    /// frequency recovery scan; normal profile completion uses `wake_waiting_for_archive`.
    async fn recover_waiting_with_ready_profiles(&self) -> Result<u64> {
        let updated = sqlx::query(
            "UPDATE preference_learning_events AS queue
             SET status = 'pending', next_attempt_at = NULL, last_error = NULL,
                 updated_at = CURRENT_TIMESTAMP
             WHERE queue.status = 'waiting_analysis'
               AND EXISTS (
                   SELECT 1
                   FROM user_behavior_events event
                   JOIN archives archive ON archive.id = event.archive_id
                   JOIN archive_content_profiles profile
                     ON profile.id = (
                         SELECT latest.id
                         FROM archive_content_profiles latest
                         WHERE latest.archive_id = archive.id
                           AND latest.content_fingerprint = archive.file_hash
                           AND latest.profile_version = ?
                           AND latest.status IN ('completed', 'partial')
                         ORDER BY latest.updated_at DESC, latest.id DESC
                         LIMIT 1
                     )
                   WHERE event.id = queue.behavior_event_id
                     AND profile.coverage >= 0.60
               )",
        )
        .bind(CONTENT_PROFILE_VERSION)
        .execute(&self.pool)
        .await?;
        let count = updated.rows_affected();
        if count > 0 {
            notify_preference_learning_worker();
        }
        Ok(count)
    }

    async fn process_next_queued(&self) -> Result<bool> {
        let row = sqlx::query(
            "SELECT queue.id, queue.behavior_event_id, queue.user_id, queue.attempts
             FROM preference_learning_events queue
             JOIN user_behavior_events event ON event.id = queue.behavior_event_id
             WHERE queue.status IN ('pending','retryable')
               AND event.occurred_at >= (
                   SELECT cold_start_started_at FROM preference_learning_state WHERE id = 'default'
               )
               AND (queue.next_attempt_at IS NULL OR queue.next_attempt_at <= CURRENT_TIMESTAMP)
             ORDER BY queue.updated_at ASC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(false);
        };
        let queue_id: String = row.get("id");
        let event_id: String = row.get("behavior_event_id");
        let user_id: String = row.get("user_id");
        let attempts: i64 = row.get("attempts");
        let claimed = sqlx::query(
            "UPDATE preference_learning_events
             SET status = 'running', attempts = attempts + 1, updated_at = CURRENT_TIMESTAMP
             WHERE id = ? AND status IN ('pending','retryable')
               AND (next_attempt_at IS NULL OR next_attempt_at <= CURRENT_TIMESTAMP)",
        )
        .bind(&queue_id)
        .execute(&self.pool)
        .await?;
        if claimed.rows_affected() != 1 {
            return Ok(true);
        }

        match self.process_event(&event_id, &user_id).await {
            Ok(()) => {
                sqlx::query(
                    "UPDATE preference_learning_events
                     SET status = 'completed', last_error = NULL, next_attempt_at = NULL,
                         updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                )
                .bind(&queue_id)
                .execute(&self.pool)
                .await?;
            }
            Err(error) if error.to_string() == "profile pending" => {
                sqlx::query(
                    "UPDATE preference_learning_events
                     SET status = 'waiting_analysis', next_attempt_at = NULL, last_error = ?,
                         updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                )
                .bind(error.to_string())
                .bind(&queue_id)
                .execute(&self.pool)
                .await?;
            }
            Err(error) => {
                let status = if attempts + 1 >= MAX_RETRIES {
                    "failed"
                } else {
                    "retryable"
                };
                let delay = 2_i64.pow((attempts as u32).min(5));
                sqlx::query(
                    "UPDATE preference_learning_events
                     SET status = ?, next_attempt_at = ?, last_error = ?,
                         updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                )
                .bind(status)
                .bind(Utc::now() + Duration::seconds(delay))
                .bind(error.to_string())
                .bind(&queue_id)
                .execute(&self.pool)
                .await?;
                tracing::warn!(%event_id, %error, "preference learning event failed");
            }
        }
        Ok(true)
    }

    async fn release_expired(&self) -> Result<u64> {
        let updated = sqlx::query(
            "UPDATE preference_learning_events
             SET status = 'retryable', next_attempt_at = CURRENT_TIMESTAMP,
                 last_error = 'expired worker lease', updated_at = CURRENT_TIMESTAMP
             WHERE status = 'running' AND updated_at < datetime('now', '-10 minutes')",
        )
        .execute(&self.pool)
        .await?;
        Ok(updated.rows_affected())
    }

    async fn next_retry_delay(&self) -> Result<Option<StdDuration>> {
        let seconds: Option<f64> = sqlx::query_scalar(
            "SELECT MIN(julianday(next_attempt_at) - julianday('now'))
             FROM preference_learning_events
             WHERE status = 'retryable' AND next_attempt_at IS NOT NULL",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(seconds
            .map(|value| StdDuration::from_secs_f64((value.max(0.0) * 86_400.0).min(86_400.0))))
    }

    async fn process_event(&self, event_id: &str, user_id: &str) -> Result<()> {
        let event = sqlx::query(
            "SELECT id, archive_id, event_type, page, metadata_json, occurred_at
             FROM user_behavior_events WHERE id = ? AND user_id = ?",
        )
        .bind(event_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow!("behavior event not found"))?;
        let Some(archive_id) = event.try_get::<Option<String>, _>("archive_id")? else {
            return Ok(());
        };
        let event_type: String = event.get("event_type");
        let metadata: Value =
            serde_json::from_str(event.get::<String, _>("metadata_json").as_str())
                .unwrap_or_else(|_| json!({}));
        let page = event.try_get::<Option<i64>, _>("page")?.unwrap_or(0);
        let occurred_at: DateTime<Utc> = event.get("occurred_at");
        self.apply_feedback_event(
            event_id,
            user_id,
            &archive_id,
            &event_type,
            page,
            &metadata,
            occurred_at,
        )
        .await?;

        let profile = self.load_profile(&archive_id).await?;
        let Some(profile) = profile else {
            // A deleted archive may still have a durable negative aggregate,
            // but it cannot produce a content rule anymore.
            let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM archives WHERE id = ?")
                .bind(&archive_id)
                .fetch_optional(&self.pool)
                .await?;
            if exists.is_none() {
                return Ok(());
            }
            if let Err(error) = crate::services::ContentProfileService::new(self.pool.clone())
                .enqueue_for_trigger(&archive_id, &event_type)
                .await
            {
                tracing::warn!(%archive_id, %error, "profile was not queued while learning waited");
            }
            return Err(anyhow!("profile pending"));
        };
        if profile.coverage < 0.60 {
            return Err(anyhow!("profile pending"));
        }
        self.rebuild_for_user(user_id, None).await
    }

    async fn apply_feedback_event(
        &self,
        event_id: &str,
        user_id: &str,
        archive_id: &str,
        event_type: &str,
        page: i64,
        metadata: &Value,
        occurred_at: DateTime<Utc>,
    ) -> Result<bool> {
        let total_pages: i64 =
            sqlx::query_scalar("SELECT COALESCE(page_count, 0) FROM archives WHERE id = ?")
                .bind(archive_id)
                .fetch_optional(&self.pool)
                .await?
                .flatten()
                .unwrap_or_else(|| {
                    value_i64(metadata, &["totalPages", "total_pages"]).unwrap_or(0)
                });
        let context = self
            .recommendation_context(user_id, archive_id, metadata)
            .await?;
        let event_metrics = event_metrics(event_type, page, metadata, total_pages, context);
        let mut tx = self.pool.begin().await?;
        let marker = sqlx::query(
            "INSERT OR IGNORE INTO preference_feedback_event_applied
             (behavior_event_id, user_id, archive_id, applied_at)
             VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
        )
        .bind(event_id)
        .bind(user_id)
        .bind(archive_id)
        .execute(&mut *tx)
        .await?;
        if marker.rows_affected() == 0 {
            tx.commit().await?;
            return Ok(false);
        }

        let previous = sqlx::query(
            "SELECT open_count, effective_read, delete_stage, algorithm_variants_json
             FROM preference_feedback_aggregates WHERE user_id = ? AND archive_id = ?",
        )
        .bind(user_id)
        .bind(archive_id)
        .fetch_optional(&mut *tx)
        .await?;
        let previous_open_count = previous
            .as_ref()
            .map(|row| row.get::<i64, _>("open_count"))
            .unwrap_or(0);
        let previous_effective = previous
            .as_ref()
            .map(|row| row.get::<i64, _>("effective_read") != 0)
            .unwrap_or(false);
        let previous_delete_stage: Option<String> = previous
            .as_ref()
            .and_then(|row| row.try_get("delete_stage").ok())
            .flatten();
        let delete_stage = if event_type == "manual_delete" && previous_delete_stage.is_none() {
            Some(if previous_effective || event_metrics.effective_read {
                "after_effective_read"
            } else if previous_open_count > 0 {
                "after_open"
            } else {
                "before_open"
            })
        } else {
            None
        };
        let mut variants: Vec<String> = previous
            .as_ref()
            .and_then(|row| row.try_get::<String, _>("algorithm_variants_json").ok())
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        if let Some(variant) = event_metrics.algorithm_variant.as_deref() {
            if !variants.iter().any(|value| value == variant) {
                variants.push(variant.to_string());
            }
        }

        sqlx::query(
            "INSERT INTO preference_feedback_aggregates
             (user_id, archive_id, open_count, page_turn_count, exit_count,
              continue_count, repeat_open_count, restore_count, correction_count,
              max_page, max_progress_ratio, effective_read, deep_read, completed_read,
              quick_exit, manual_delete, delete_stage, total_duration_ms, max_duration_ms,
              recommendation_exposure_count, first_recommendation_position,
              visibility_confidence, algorithm_variants_json, first_event_at, last_event_at,
              updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(user_id, archive_id) DO UPDATE SET
              open_count = preference_feedback_aggregates.open_count + excluded.open_count,
              page_turn_count = preference_feedback_aggregates.page_turn_count + excluded.page_turn_count,
              exit_count = preference_feedback_aggregates.exit_count + excluded.exit_count,
              continue_count = preference_feedback_aggregates.continue_count + excluded.continue_count,
              repeat_open_count = preference_feedback_aggregates.repeat_open_count + excluded.repeat_open_count,
              restore_count = preference_feedback_aggregates.restore_count + excluded.restore_count,
              correction_count = preference_feedback_aggregates.correction_count + excluded.correction_count,
              max_page = MAX(preference_feedback_aggregates.max_page, excluded.max_page),
              max_progress_ratio = MAX(preference_feedback_aggregates.max_progress_ratio, excluded.max_progress_ratio),
              effective_read = MAX(preference_feedback_aggregates.effective_read, excluded.effective_read),
              deep_read = MAX(preference_feedback_aggregates.deep_read, excluded.deep_read),
              completed_read = MAX(preference_feedback_aggregates.completed_read, excluded.completed_read),
              quick_exit = MAX(preference_feedback_aggregates.quick_exit, excluded.quick_exit),
              manual_delete = preference_feedback_aggregates.manual_delete + excluded.manual_delete,
              delete_stage = COALESCE(preference_feedback_aggregates.delete_stage, excluded.delete_stage),
              total_duration_ms = preference_feedback_aggregates.total_duration_ms + excluded.total_duration_ms,
              max_duration_ms = MAX(preference_feedback_aggregates.max_duration_ms, excluded.max_duration_ms),
              recommendation_exposure_count = preference_feedback_aggregates.recommendation_exposure_count + excluded.recommendation_exposure_count,
              first_recommendation_position = COALESCE(preference_feedback_aggregates.first_recommendation_position, excluded.first_recommendation_position),
              visibility_confidence = MAX(preference_feedback_aggregates.visibility_confidence, excluded.visibility_confidence),
              algorithm_variants_json = excluded.algorithm_variants_json,
              first_event_at = MIN(preference_feedback_aggregates.first_event_at, excluded.first_event_at),
              last_event_at = MAX(preference_feedback_aggregates.last_event_at, excluded.last_event_at),
              updated_at = CURRENT_TIMESTAMP",
        )
        .bind(user_id)
        .bind(archive_id)
        .bind(event_metrics.open_count)
        .bind(event_metrics.page_turn_count)
        .bind(event_metrics.exit_count)
        .bind(event_metrics.continue_count)
        .bind(event_metrics.repeat_open_count)
        .bind(event_metrics.restore_count)
        .bind(event_metrics.correction_count)
        .bind(event_metrics.page)
        .bind(event_metrics.progress_ratio)
        .bind(event_metrics.effective_read as i64)
        .bind(event_metrics.deep_read as i64)
        .bind(event_metrics.completed_read as i64)
        .bind(event_metrics.quick_exit as i64)
        .bind(event_metrics.manual_delete)
        .bind(delete_stage)
        .bind(event_metrics.duration_ms)
        .bind(event_metrics.duration_ms)
        .bind(event_metrics.recommendation_exposure_count)
        .bind(event_metrics.recommendation_position)
        .bind(event_metrics.visibility_confidence)
        .bind(serde_json::to_string(&variants)?)
        .bind(occurred_at)
        .bind(occurred_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    async fn recommendation_context(
        &self,
        user_id: &str,
        archive_id: &str,
        metadata: &Value,
    ) -> Result<(Option<i64>, Option<String>, f64)> {
        let session_id = metadata
            .get("recommendationSessionId")
            .or_else(|| metadata.get("recommendation_session_id"))
            .and_then(Value::as_str);
        let metadata_position = value_i64(
            metadata,
            &["recommendationPosition", "recommendation_position"],
        );
        let Some(session_id) = session_id else {
            return Ok((
                metadata_position,
                None,
                if metadata_position.is_some() {
                    0.5
                } else {
                    0.0
                },
            ));
        };
        let row = sqlx::query(
            "SELECT i.position, s.algorithm_variant
             FROM random_recommendation_items i
             JOIN random_recommendation_sessions s ON s.id = i.session_id
             WHERE i.session_id = ? AND i.user_id = ? AND i.archive_id = ?
             ORDER BY i.created_at DESC LIMIT 1",
        )
        .bind(session_id)
        .bind(user_id)
        .bind(archive_id)
        .fetch_optional(&self.pool)
        .await?;
        let position = row
            .as_ref()
            .and_then(|value| value.try_get::<i64, _>("position").ok())
            .or(metadata_position);
        let variant = row.and_then(|value| value.try_get("algorithm_variant").ok());
        let confidence = position
            .map(|position| (1.0 / (1.0 + position.max(0) as f64 / 3.0)).clamp(0.2, 1.0))
            .unwrap_or(0.5);
        Ok((position, variant, confidence))
    }

    async fn load_profile(
        &self,
        archive_id: &str,
    ) -> Result<Option<ArchiveContentProfileDocument>> {
        let row = sqlx::query(
            "SELECT p.profile_json, p.coverage FROM archive_content_profiles p
             JOIN archives a ON a.id = p.archive_id
                            AND a.file_hash = p.content_fingerprint
             WHERE p.archive_id = ? AND p.profile_version = ?
               AND p.status IN ('completed','partial')
             ORDER BY p.updated_at DESC LIMIT 1",
        )
        .bind(archive_id)
        .bind(CONTENT_PROFILE_VERSION)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let coverage: f64 = row.get("coverage");
        let document: ArchiveContentProfileDocument =
            serde_json::from_str(row.get::<String, _>("profile_json").as_str())
                .context("invalid deterministic content profile")?;
        if coverage < 0.60 || document.coverage < 0.60 {
            return Ok(None);
        }
        Ok(Some(document))
    }

    /// Rebuild only from the new aggregate table. This is called after a
    /// profile finishes so an event that was waiting on its profile can become
    /// useful without replaying the old event log.
    pub async fn rebuild_for_archive(&self, archive_id: &str) -> Result<()> {
        let users: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT user_id FROM preference_feedback_aggregates WHERE archive_id = ?",
        )
        .bind(archive_id)
        .fetch_all(&self.pool)
        .await?;
        for user_id in users {
            self.rebuild_for_user(&user_id, None).await?;
        }
        Ok(())
    }

    /// Refresh only the observation-only canonical-theme candidates after identity resolution.
    /// This keeps a metadata change in canonicalization from recalculating ordinary learned rules.
    pub async fn rebuild_observing_for_archive(&self, archive_id: &str) -> Result<()> {
        let users: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT user_id FROM preference_feedback_aggregates WHERE archive_id = ?",
        )
        .bind(archive_id)
        .fetch_all(&self.pool)
        .await?;
        for user_id in users {
            self.rebuild_for_user(&user_id, Some(CANONICAL_THEME_FEATURE_KIND))
                .await?;
        }
        Ok(())
    }

    async fn rebuild_for_user(
        &self,
        user_id: &str,
        feature_kind_filter: Option<&str>,
    ) -> Result<()> {
        let rows = sqlx::query(
            "SELECT f.user_id, f.archive_id, f.open_count, f.page_turn_count, f.exit_count,
                    f.continue_count, f.repeat_open_count, f.restore_count, f.correction_count,
                    f.max_page, f.max_progress_ratio, f.effective_read, f.deep_read,
                    f.completed_read, f.quick_exit, f.manual_delete, f.delete_stage,
                    f.total_duration_ms, f.max_duration_ms, f.recommendation_exposure_count,
                    f.first_recommendation_position, f.visibility_confidence,
                    f.algorithm_variants_json, f.last_event_at,
                    p.profile_json, p.coverage, p.profile_version
             FROM preference_feedback_aggregates f
             JOIN archive_content_profiles p ON p.archive_id = f.archive_id
             WHERE f.user_id = ?
               AND f.last_event_at >= (
                   SELECT cold_start_started_at FROM preference_learning_state WHERE id = 'default'
               )
               AND p.profile_version = ?
               AND p.status IN ('completed','partial')
               AND p.id = (
                   SELECT latest.id FROM archive_content_profiles latest
                   WHERE latest.archive_id = f.archive_id
                     AND latest.profile_version = ?
                     AND latest.status IN ('completed','partial')
                   ORDER BY latest.updated_at DESC, latest.id DESC LIMIT 1
               )",
        )
        .bind(user_id)
        .bind(CONTENT_PROFILE_VERSION)
        .bind(CONTENT_PROFILE_VERSION)
        .fetch_all(&self.pool)
        .await?;
        if rows.is_empty() {
            return Ok(());
        }

        let mut aggregates = Vec::with_capacity(rows.len());
        for row in rows {
            let document: ArchiveContentProfileDocument =
                serde_json::from_str(row.get::<String, _>("profile_json").as_str())
                    .context("invalid profile while rebuilding preference candidates")?;
            let coverage: f64 = row.get("coverage");
            if coverage < 0.60 || document.coverage < 0.60 {
                continue;
            }
            aggregates.push(FeedbackAggregate {
                archive_id: row.get("archive_id"),
                open_count: row.get("open_count"),
                page_turn_count: row.get("page_turn_count"),
                exit_count: row.get("exit_count"),
                continue_count: row.get("continue_count"),
                repeat_open_count: row.get("repeat_open_count"),
                restore_count: row.get("restore_count"),
                correction_count: row.get("correction_count"),
                max_page: row.get("max_page"),
                max_progress_ratio: row.get("max_progress_ratio"),
                effective_read: row.get::<i64, _>("effective_read") != 0,
                deep_read: row.get::<i64, _>("deep_read") != 0,
                completed_read: row.get::<i64, _>("completed_read") != 0,
                quick_exit: row.get::<i64, _>("quick_exit") != 0,
                manual_delete: row.get("manual_delete"),
                delete_stage: row.try_get("delete_stage")?,
                total_duration_ms: row.get("total_duration_ms"),
                max_duration_ms: row.get("max_duration_ms"),
                recommendation_exposure_count: row.get("recommendation_exposure_count"),
                first_recommendation_position: row.try_get("first_recommendation_position")?,
                visibility_confidence: row.get("visibility_confidence"),
                algorithm_variants: serde_json::from_str(
                    row.get::<String, _>("algorithm_variants_json").as_str(),
                )
                .unwrap_or_default(),
                last_event_at: row.get("last_event_at"),
                profile_coverage: document.coverage,
                profile_version: row.get("profile_version"),
                features: document.features,
            });
        }
        if aggregates.is_empty() {
            return Ok(());
        }

        let now = Utc::now();
        let mut stats: HashMap<String, CandidateStats> = HashMap::new();
        let mut baseline_positive = 0.0;
        let mut baseline_negative = 0.0;
        for aggregate in &aggregates {
            let outcome = outcome_for_aggregate(aggregate);
            let visibility_weight = visibility_weight(aggregate.visibility_confidence);
            let decay = decayed_score(aggregate.last_event_at, now);
            let positive_score = outcome.positive_score * visibility_weight * decay;
            let negative_score = outcome.negative_score * visibility_weight * decay;
            baseline_positive += positive_score;
            baseline_negative += negative_score;
            for feature in &aggregate.features {
                if feature_kind_filter.is_some_and(|kind| feature.kind != kind) {
                    continue;
                }
                let (condition_key, conditions) = feature_condition(feature);
                let candidate = stats.entry(condition_key.clone()).or_insert_with(|| {
                    CandidateStats::new(
                        condition_key,
                        conditions,
                        feature.kind.clone(),
                        aggregate.profile_version.clone(),
                    )
                });
                candidate
                    .sample_archives
                    .insert(aggregate.archive_id.clone());
                candidate.profile_coverage_sum += aggregate.profile_coverage;
                candidate.record_metrics(aggregate);
                candidate.effective_read_count += aggregate.effective_read as i64;
                candidate.deep_read_count += aggregate.deep_read as i64;
                candidate.manual_delete_count += aggregate.manual_delete;
                candidate.quick_exit_count += aggregate.quick_exit as i64;
                if aggregate.manual_delete > 0 && aggregate.effective_read {
                    candidate.conflict_count += 1;
                }
                if positive_score > 0.0 {
                    candidate.positive_score += positive_score;
                    candidate
                        .positive_archives
                        .insert(aggregate.archive_id.clone());
                }
                if negative_score > 0.0 {
                    candidate.negative_score += negative_score;
                    candidate
                        .negative_archives
                        .insert(aggregate.archive_id.clone());
                }
                if positive_score > 0.0 || negative_score > 0.0 {
                    candidate.informative_result_count += 1;
                }
            }
        }
        let baseline_total = (baseline_positive + baseline_negative).max(1.0);
        let baseline_net = (baseline_positive - baseline_negative) / baseline_total;
        for candidate in stats.values() {
            self.persist_candidate(user_id, candidate, baseline_net, now)
                .await?;
        }
        Ok(())
    }

    async fn persist_candidate(
        &self,
        user_id: &str,
        candidate: &CandidateStats,
        baseline_net: f64,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let total = (candidate.positive_score + candidate.negative_score).max(1.0);
        let net = (candidate.positive_score - candidate.negative_score) / total;
        let lift = net - baseline_net;
        let direction_probability =
            direction_probability(candidate.positive_score, candidate.negative_score);
        let unique_archive_count = candidate.sample_archives.len();
        let observing_only = candidate.feature_kind == CANONICAL_THEME_FEATURE_KIND;
        let evidence_state = if observing_only {
            "observing"
        } else if unique_archive_count < MIN_OBSERVING_ARCHIVES {
            "insufficient_evidence"
        } else if unique_archive_count >= MIN_FORMAL_ARCHIVES
            && candidate.informative_result_count >= MIN_FORMAL_RESULTS as i64
            && direction_probability >= 0.95
            && lift.abs() >= MIN_LIFT
        {
            "eligible"
        } else {
            "observing"
        };
        let status = if evidence_state == "eligible" && !observing_only {
            "promoted"
        } else {
            "observing"
        };
        let sample_archives: Vec<String> = candidate
            .sample_archives
            .iter()
            .take(100)
            .cloned()
            .collect();
        let evidence = json!({
            "algorithmVersion": COLD_START_VERSION,
            "independentArchiveCount": unique_archive_count,
            "informativeResultCount": candidate.informative_result_count,
            "positiveRate": candidate.positive_score / total,
            "negativeRate": candidate.negative_score / total,
            "baselineNet": baseline_net,
            "candidateNet": net,
            "lift": lift,
            "directionProbability": direction_probability,
            "positionCorrection": "visibility_confidence_weight",
            "timeDecayHalfLifeDays": SIGNAL_HALF_LIFE_DAYS,
            "positiveArchives": candidate.positive_archives.len(),
            "negativeArchives": candidate.negative_archives.len(),
            "metrics": {
                "openCount": candidate.open_count,
                "pageTurnCount": candidate.page_turn_count,
                "exitCount": candidate.exit_count,
                "continueCount": candidate.continue_count,
                "repeatOpenCount": candidate.repeat_open_count,
                "restoreCount": candidate.restore_count,
                "correctionCount": candidate.correction_count,
                "effectiveReadCount": candidate.effective_read_count,
                "deepReadCount": candidate.deep_read_count,
                "manualDeleteCount": candidate.manual_delete_count,
                "quickExitCount": candidate.quick_exit_count,
                "deleteBeforeOpenCount": candidate.delete_before_open_count,
                "deleteAfterOpenCount": candidate.delete_after_open_count,
                "deleteAfterEffectiveReadCount": candidate.delete_after_effective_read_count,
                "exposureArchiveCount": candidate.exposure_archive_count,
                "meanRecommendationPosition": if candidate.exposure_archive_count == 0 {
                    Value::Null
                } else {
                    json!(candidate.position_sum / candidate.exposure_archive_count as f64)
                },
                "meanVisibilityConfidence": if candidate.exposure_archive_count == 0 {
                    Value::Null
                } else {
                    json!(candidate.visibility_confidence_sum / candidate.exposure_archive_count as f64)
                },
                "meanMaximumProgressRatio": if unique_archive_count == 0 {
                    0.0
                } else {
                    candidate.progress_ratio_sum / unique_archive_count as f64
                },
                "maximumPage": candidate.max_page,
                "totalDurationMs": candidate.total_duration_ms,
                "maximumDurationMs": candidate.max_duration_ms,
                "algorithmVariantCounts": &candidate.algorithm_variant_counts,
            },
        });
        let evidence_json = serde_json::to_string(&evidence)?;
        let sample_archives_json = serde_json::to_string(&sample_archives)?;
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO preference_rule_candidates
             (id, user_id, condition_key, conditions_json, positive_score, negative_score,
              positive_support, negative_support, sample_archives_json, confidence, status,
              last_learned_at, updated_at, evidence_state, source, feature_kind,
              profile_version, unique_archive_count, informative_result_count,
              effective_read_count, deep_read_count, manual_delete_count, quick_exit_count,
              conflict_count, baseline_rate, lift, direction_probability, profile_coverage,
              evidence_json)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(user_id, condition_key) DO UPDATE SET
              conditions_json = excluded.conditions_json,
              positive_score = excluded.positive_score,
              negative_score = excluded.negative_score,
              positive_support = excluded.positive_support,
              negative_support = excluded.negative_support,
              sample_archives_json = excluded.sample_archives_json,
              confidence = excluded.confidence,
              status = excluded.status,
              last_learned_at = excluded.last_learned_at,
              updated_at = excluded.updated_at,
              evidence_state = excluded.evidence_state,
              source = excluded.source,
              feature_kind = excluded.feature_kind,
              profile_version = excluded.profile_version,
              unique_archive_count = excluded.unique_archive_count,
              informative_result_count = excluded.informative_result_count,
              effective_read_count = excluded.effective_read_count,
              deep_read_count = excluded.deep_read_count,
              manual_delete_count = excluded.manual_delete_count,
              quick_exit_count = excluded.quick_exit_count,
              conflict_count = excluded.conflict_count,
              baseline_rate = excluded.baseline_rate,
              lift = excluded.lift,
              direction_probability = excluded.direction_probability,
              profile_coverage = excluded.profile_coverage,
              evidence_json = excluded.evidence_json",
        )
        .bind(id)
        .bind(user_id)
        .bind(&candidate.condition_key)
        .bind(serde_json::to_string(&candidate.conditions)?)
        .bind(candidate.positive_score)
        .bind(candidate.negative_score)
        .bind(candidate.positive_archives.len() as i64)
        .bind(candidate.negative_archives.len() as i64)
        .bind(sample_archives_json)
        .bind(direction_probability)
        .bind(status)
        .bind(now)
        .bind(evidence_state)
        .bind(CANDIDATE_SOURCE)
        .bind(&candidate.feature_kind)
        .bind(&candidate.profile_version)
        .bind(unique_archive_count as i64)
        .bind(candidate.informative_result_count as i64)
        .bind(candidate.effective_read_count)
        .bind(candidate.deep_read_count)
        .bind(candidate.manual_delete_count)
        .bind(candidate.quick_exit_count)
        .bind(candidate.conflict_count)
        .bind(baseline_net)
        .bind(lift)
        .bind(direction_probability)
        .bind(candidate.profile_coverage_sum / unique_archive_count.max(1) as f64)
        .bind(evidence_json)
        .execute(&self.pool)
        .await?;

        if evidence_state == "eligible" && !observing_only {
            self.promote_rule(user_id, candidate, direction_probability, lift)
                .await?;
        } else {
            self.disable_rule(user_id, &candidate.conditions).await?;
        }
        Ok(())
    }

    async fn promote_rule(
        &self,
        user_id: &str,
        candidate: &CandidateStats,
        confidence: f64,
        lift: f64,
    ) -> Result<()> {
        if candidate.feature_kind == CANONICAL_THEME_FEATURE_KIND {
            self.disable_rule(user_id, &candidate.conditions).await?;
            return Ok(());
        }
        let condition_json = serde_json::to_string(&candidate.conditions)?;
        let action = if lift >= 0.0 { "keep" } else { "downrank" };
        let weight = (1.0 + lift.abs() * 2.0).clamp(0.5, 2.0);
        let rule_version = format!("{COLD_START_VERSION}-{}", Utc::now().timestamp_millis());
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT id FROM preference_rules
             WHERE user_id = ? AND source = ? AND conditions_json = ? LIMIT 1",
        )
        .bind(user_id)
        .bind(LEARNED_RULE_SOURCE)
        .bind(&condition_json)
        .fetch_optional(&self.pool)
        .await?;
        if let Some(id) = existing {
            sqlx::query(
                "UPDATE preference_rules
                 SET rule_version = ?, action = ?, enabled = 1, auto_paused = 0,
                     confidence_threshold = ?, preference_weight = ?,
                     positive_support = ?, negative_support = ?,
                     last_learned_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?",
            )
            .bind(rule_version)
            .bind(action)
            .bind(confidence)
            .bind(weight)
            .bind(candidate.positive_archives.len() as i64)
            .bind(candidate.negative_archives.len() as i64)
            .bind(id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                "INSERT INTO preference_rules
                 (id, user_id, name, rule_version, conditions_json, exceptions_json,
                  action, confidence_threshold, enabled, owner_role, source,
                  preference_weight, positive_support, negative_support, last_learned_at)
                 VALUES (?, ?, ?, ?, ?, '{}', ?, ?, 1, 'user', ?, ?, ?, ?, CURRENT_TIMESTAMP)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(user_id)
            .bind(format!(
                "Learned feature evidence: {}",
                candidate.condition_key
            ))
            .bind(rule_version)
            .bind(condition_json)
            .bind(action)
            .bind(confidence)
            .bind(LEARNED_RULE_SOURCE)
            .bind(weight)
            .bind(candidate.positive_archives.len() as i64)
            .bind(candidate.negative_archives.len() as i64)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn disable_rule(&self, user_id: &str, conditions: &Value) -> Result<()> {
        sqlx::query(
            "UPDATE preference_rules SET enabled = 0, updated_at = CURRENT_TIMESTAMP
             WHERE user_id = ? AND source = ? AND conditions_json = ?",
        )
        .bind(user_id)
        .bind(LEARNED_RULE_SOURCE)
        .bind(serde_json::to_string(conditions)?)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_candidates(&self, user_id: &str) -> Result<Vec<PreferenceInsightCandidate>> {
        let rows = sqlx::query(
            "SELECT id, user_id, condition_key, conditions_json, feature_kind,
                    positive_score, negative_score, positive_support, negative_support,
                    unique_archive_count, informative_result_count, effective_read_count,
                    deep_read_count, manual_delete_count, quick_exit_count, conflict_count,
                    baseline_rate, lift, direction_probability, profile_coverage,
                    evidence_state, status, source, sample_archives_json, evidence_json,
                    last_learned_at
             FROM preference_rule_candidates
             WHERE user_id = ? AND source = ?
             ORDER BY CASE evidence_state WHEN 'eligible' THEN 0 WHEN 'observing' THEN 1 ELSE 2 END,
                      direction_probability DESC, updated_at DESC",
        )
        .bind(user_id)
        .bind(CANDIDATE_SOURCE)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(PreferenceInsightCandidate {
                    id: row.get("id"),
                    user_id: row.get("user_id"),
                    condition_key: row.get("condition_key"),
                    conditions: serde_json::from_str(
                        row.get::<String, _>("conditions_json").as_str(),
                    )?,
                    feature_kind: row.try_get("feature_kind")?,
                    positive_score: row.get("positive_score"),
                    negative_score: row.get("negative_score"),
                    positive_support: row.get("positive_support"),
                    negative_support: row.get("negative_support"),
                    unique_archive_count: row.get("unique_archive_count"),
                    informative_result_count: row.get("informative_result_count"),
                    effective_read_count: row.get("effective_read_count"),
                    deep_read_count: row.get("deep_read_count"),
                    manual_delete_count: row.get("manual_delete_count"),
                    quick_exit_count: row.get("quick_exit_count"),
                    conflict_count: row.get("conflict_count"),
                    baseline_rate: row.get("baseline_rate"),
                    lift: row.get("lift"),
                    direction_probability: row.get("direction_probability"),
                    profile_coverage: row.get("profile_coverage"),
                    evidence_state: row.get("evidence_state"),
                    status: row.get("status"),
                    source: row.get("source"),
                    sample_archives: serde_json::from_str(
                        row.get::<String, _>("sample_archives_json").as_str(),
                    )
                    .unwrap_or_default(),
                    evidence: serde_json::from_str(row.get::<String, _>("evidence_json").as_str())?,
                    last_learned_at: row.try_get("last_learned_at")?,
                })
            })
            .collect()
    }
}

fn event_metrics(
    event_type: &str,
    page: i64,
    metadata: &Value,
    total_pages: i64,
    context: (Option<i64>, Option<String>, f64),
) -> EventMetrics {
    let end_page = value_i64(metadata, &["endPage", "end_page"])
        .unwrap_or(page)
        .max(0);
    let total_pages = value_i64(metadata, &["totalPages", "total_pages"])
        .unwrap_or(total_pages)
        .max(0);
    let ratio = if total_pages > 0 {
        (end_page as f64 / total_pages as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let effective = matches!(event_type, "page_turn" | "exit") && (end_page >= 5 || ratio >= 0.5);
    let deep = effective && (ratio >= 0.75 || end_page >= 20);
    let completed = effective && ratio >= 0.90;
    let duration_ms = value_i64(metadata, &["durationMs", "duration_ms"])
        .unwrap_or(0)
        .max(0);
    let start_page = value_i64(metadata, &["startPage", "start_page"])
        .unwrap_or(end_page)
        .max(1);
    let quick_exit = event_type == "exit"
        && duration_ms > 0
        && duration_ms <= 30_000
        && (end_page <= start_page + 1 || ratio < 0.10);
    let has_recommendation_context = context.0.is_some();
    EventMetrics {
        open_count: (event_type == "open") as i64,
        page_turn_count: (event_type == "page_turn") as i64,
        exit_count: (event_type == "exit") as i64,
        continue_count: (event_type == "continue_reading") as i64,
        repeat_open_count: (event_type == "repeat_open") as i64,
        restore_count: (event_type == "restore") as i64,
        correction_count: (event_type == "rule_correction") as i64,
        page: end_page,
        progress_ratio: ratio,
        effective_read: effective,
        deep_read: deep,
        completed_read: completed,
        quick_exit,
        manual_delete: (event_type == "manual_delete") as i64,
        duration_ms: if event_type == "exit" { duration_ms } else { 0 },
        recommendation_exposure_count: (has_recommendation_context
            && matches!(event_type, "open" | "repeat_open" | "manual_delete"))
            as i64,
        recommendation_position: context.0,
        visibility_confidence: if has_recommendation_context {
            context.2
        } else {
            0.0
        },
        algorithm_variant: context.1,
    }
}

fn value_i64(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_i64))
}

struct AggregateOutcome {
    positive_score: f64,
    negative_score: f64,
}

fn outcome_for_aggregate(aggregate: &FeedbackAggregate) -> AggregateOutcome {
    // A manual deletion is always negative, including a deletion after a deep
    // read. The latter is retained as a conflict metric, never reinterpreted
    // as a positive label.
    if aggregate.manual_delete > 0 {
        return AggregateOutcome {
            positive_score: 0.0,
            negative_score: 3.0,
        };
    }
    let sustained_session =
        aggregate.open_count > 0 && aggregate.max_duration_ms >= 30_000 && !aggregate.quick_exit;
    if aggregate.effective_read
        || aggregate.deep_read
        || aggregate.continue_count > 0
        || aggregate.repeat_open_count > 0
        || sustained_session
    {
        return AggregateOutcome {
            positive_score: 1.0
                + if aggregate.deep_read { 0.5 } else { 0.0 }
                + if aggregate.completed_read { 0.25 } else { 0.0 }
                + (aggregate.repeat_open_count.min(2) as f64 * 0.15)
                + (aggregate.continue_count.min(2) as f64 * 0.10)
                + (aggregate.max_progress_ratio.clamp(0.0, 1.0) * 0.20)
                + (aggregate.page_turn_count.min(20) as f64 * 0.01)
                + if sustained_session { 0.15 } else { 0.0 },
            negative_score: 0.0,
        };
    }
    if aggregate.quick_exit {
        return AggregateOutcome {
            positive_score: 0.0,
            negative_score: 0.5,
        };
    }
    AggregateOutcome {
        positive_score: 0.0,
        negative_score: 0.0,
    }
}

fn visibility_weight(confidence: f64) -> f64 {
    if confidence <= 0.0 {
        1.0
    } else {
        confidence.clamp(0.25, 1.0)
    }
}

fn decayed_score(occurred_at: DateTime<Utc>, now: DateTime<Utc>) -> f64 {
    let age_days = (now - occurred_at).num_milliseconds().max(0) as f64 / 86_400_000.0;
    2_f64.powf(-age_days / SIGNAL_HALF_LIFE_DAYS)
}

fn direction_probability(positive: f64, negative: f64) -> f64 {
    let total = positive + negative;
    if total <= 0.0 {
        return 0.5;
    }
    // Beta(1 + positive, 1 + negative), approximated around p=0.5. The
    // prior prevents one small sample from becoming a 0/1 certainty.
    let alpha = 1.0 + positive;
    let beta = 1.0 + negative;
    let n = alpha + beta;
    let mean = alpha / n;
    let variance = alpha * beta / (n * n * (n + 1.0));
    if variance <= f64::EPSILON {
        return if mean >= 0.5 { 1.0 } else { 0.0 };
    }
    let z = (mean - 0.5) / variance.sqrt();
    normal_cdf(z).max(1.0 - normal_cdf(z)).clamp(0.5, 1.0)
}

fn normal_cdf(value: f64) -> f64 {
    0.5 * (1.0 + erf(value / 2.0_f64.sqrt()))
}

fn erf(value: f64) -> f64 {
    // Abramowitz and Stegun 7.1.26, sufficient for a bounded confidence
    // display and avoiding a new numerical dependency.
    let sign = if value < 0.0 { -1.0 } else { 1.0 };
    let x = value.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let polynomial = (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736)
        * t
        + 0.254829592)
        * t;
    sign * (1.0 - polynomial * (-x * x).exp())
}

fn feature_condition(feature: &ContentProfileFeature) -> (String, Value) {
    if feature.kind == "binary" || feature.kind == CANONICAL_THEME_FEATURE_KIND {
        let key = format!("profile:{CONTENT_PROFILE_VERSION}:{}:eq:1", feature.key);
        let condition = json!({
            "all": [{"feature": feature.key, "operator": "eq", "value": 1.0}]
        });
        return (key, condition);
    }
    let width = bucket_width(feature.value);
    let start = (feature.value / width).floor() * width;
    let end = start + width;
    let key = format!(
        "profile:{CONTENT_PROFILE_VERSION}:{}:between:{start:.4}:{end:.4}",
        feature.key
    );
    let condition = json!({
        "all": [{
            "feature": feature.key,
            "operator": "between",
            "min": start,
            "max": end
        }]
    });
    (key, condition)
}

fn bucket_width(value: f64) -> f64 {
    let absolute = value.abs();
    if absolute >= 20.0 {
        10.0
    } else if absolute >= 2.0 {
        1.0
    } else {
        0.1
    }
}

/// Match the generic profile conditions used by learned recommendation rules.
/// Manual legacy theme rules continue to be evaluated by the content analysis
/// decision service; this matcher has no semantic vocabulary.
pub fn profile_condition_matches(
    condition: &Value,
    profile: &ArchiveContentProfileDocument,
) -> bool {
    if let Some(all) = condition.get("all").and_then(Value::as_array) {
        return all
            .iter()
            .all(|child| profile_condition_matches(child, profile));
    }
    if let Some(any) = condition.get("any").and_then(Value::as_array) {
        return any
            .iter()
            .any(|child| profile_condition_matches(child, profile));
    }
    if let Some(not) = condition.get("not") {
        return !profile_condition_matches(not, profile);
    }
    let Some(feature_key) = condition.get("feature").and_then(Value::as_str) else {
        return false;
    };
    let Some(feature) = profile
        .features
        .iter()
        .find(|feature| feature.key == feature_key)
    else {
        return false;
    };
    match condition.get("operator").and_then(Value::as_str) {
        Some("eq") => condition
            .get("value")
            .and_then(Value::as_f64)
            .is_some_and(|value| (feature.value - value).abs() <= 1.0e-6),
        Some("between") => {
            let min = condition
                .get("min")
                .and_then(Value::as_f64)
                .unwrap_or(f64::MIN);
            let max = condition
                .get("max")
                .and_then(Value::as_f64)
                .unwrap_or(f64::MAX);
            feature.value >= min && feature.value < max
        }
        _ => false,
    }
}

pub fn spawn_preference_learning_worker(pool: Pool<Sqlite>) {
    let recovery_pool = pool.clone();
    tokio::spawn(async move {
        let service = PreferenceLearningService::new(recovery_pool);
        if let Err(error) = service.enqueue_events().await {
            tracing::warn!(%error, "preference learning startup recovery failed");
        }
        match service.recover_waiting_with_ready_profiles().await {
            Ok(recovered) if recovered > 0 => notify_preference_learning_worker(),
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "preference learning dependency recovery failed"),
        }
        loop {
            match service.recover_waiting_with_ready_profiles().await {
                Ok(recovered) if recovered > 0 => notify_preference_learning_worker(),
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(%error, "preference learning dependency recovery failed")
                }
            }
            match service.release_expired().await {
                Ok(released) if released > 0 => notify_preference_learning_worker(),
                Ok(_) => {}
                Err(error) => tracing::warn!(%error, "preference learning lease recovery failed"),
            }
            tokio::time::sleep(StdDuration::from_secs(10 * 60)).await;
        }
    });

    let signal = preference_learning_signal().clone();
    tokio::spawn(async move {
        let service = PreferenceLearningService::new(pool);
        loop {
            let notified = signal.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            match service.process_next_queued().await {
                Ok(true) => {}
                Ok(false) => match service.next_retry_delay().await {
                    Ok(Some(delay)) if delay.is_zero() => continue,
                    Ok(Some(delay)) => {
                        tokio::select! {
                            _ = notified => {}
                            _ = tokio::time::sleep(delay) => {}
                        }
                    }
                    Ok(None) => notified.await,
                    Err(error) => {
                        tracing::warn!(%error, "preference learning retry timer query failed");
                        tokio::select! {
                            _ = notified => {}
                            _ = tokio::time::sleep(StdDuration::from_secs(30)) => {}
                        }
                    }
                },
                Err(error) => {
                    tracing::warn!(%error, "preference learning worker iteration failed");
                    tokio::select! {
                        _ = notified => {}
                        _ = tokio::time::sleep(StdDuration::from_secs(30)) => {}
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn learning_test_pool(with_profile: bool) -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("SQLite test database should connect");
        crate::database::run_sqlite_migrations(&pool)
            .await
            .expect("learning test migrations should succeed");
        sqlx::query(
            "INSERT INTO users (id, username, password_hash, api_key)
             VALUES ('user-1', 'learning-user', 'test-hash', 'test-key')",
        )
        .execute(&pool)
        .await
        .expect("learning test user should be inserted");
        sqlx::query(
            "INSERT INTO archives (id, title, path, file_hash, file_size, page_count)
             VALUES ('archive-1', 'test archive', '/tmp/test-archive.cbz', 'hash-1', 1, 60)",
        )
        .execute(&pool)
        .await
        .expect("learning test archive should be inserted");
        sqlx::query(
            "UPDATE preference_learning_state
             SET cold_start_started_at = datetime('now', '-1 hour')
             WHERE id = 'default'",
        )
        .execute(&pool)
        .await
        .expect("learning marker should be adjustable in the test");
        if with_profile {
            insert_completed_profile(&pool).await;
        }
        pool
    }

    fn profile_json() -> String {
        serde_json::to_string(&ArchiveContentProfileDocument {
            profile_version: CONTENT_PROFILE_VERSION.to_string(),
            content_fingerprint: "hash-1".to_string(),
            expected_page_count: 60,
            actual_page_count: 60,
            sampled_page_count: 10,
            decoded_page_count: 10,
            coverage: 1.0,
            features: vec![
                ContentProfileFeature {
                    key: "page_count".to_string(),
                    value: 60.0,
                    kind: "numeric".to_string(),
                },
                ContentProfileFeature {
                    key: "color_fraction".to_string(),
                    value: 0.7,
                    kind: "numeric".to_string(),
                },
                ContentProfileFeature {
                    key: "page_similarity_p50".to_string(),
                    value: 0.8,
                    kind: "numeric".to_string(),
                },
            ],
            measurements: json!({"source": "test"}),
        })
        .expect("profile should serialize")
    }

    async fn insert_completed_profile(pool: &Pool<Sqlite>) {
        sqlx::query(
            "INSERT INTO archive_content_profiles
             (id, archive_id, content_fingerprint, profile_version, status, profile_json,
             expected_page_count, actual_page_count, sampled_page_count, decoded_page_count,
             coverage, method_json, completed_at)
             VALUES ('profile-1', 'archive-1', 'hash-1', ?, 'completed', ?,
                     60, 60, 10, 10, 1.0, '{}', CURRENT_TIMESTAMP)
             ON CONFLICT(archive_id, content_fingerprint, profile_version) DO UPDATE SET
                 status = excluded.status, profile_json = excluded.profile_json,
                 expected_page_count = excluded.expected_page_count,
                 actual_page_count = excluded.actual_page_count,
                 sampled_page_count = excluded.sampled_page_count,
                 decoded_page_count = excluded.decoded_page_count,
                 coverage = excluded.coverage, method_json = excluded.method_json,
                 completed_at = excluded.completed_at, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(CONTENT_PROFILE_VERSION)
        .bind(profile_json())
        .execute(pool)
        .await
        .expect("completed profile should be inserted");
    }

    async fn insert_event(pool: &Pool<Sqlite>, id: &str, event_type: &str, page: Option<i32>) {
        sqlx::query(
            "INSERT INTO user_behavior_events
             (id, user_id, archive_id, event_type, event_key, page, metadata_json, occurred_at)
             VALUES (?, 'user-1', 'archive-1', ?, ?, ?, '{}', datetime('now'))",
        )
        .bind(id)
        .bind(event_type)
        .bind(format!("key-{id}"))
        .bind(page)
        .execute(pool)
        .await
        .expect("behavior event should be inserted");
    }

    #[test]
    fn manual_delete_is_always_negative_even_after_deep_read() {
        let aggregate = FeedbackAggregate {
            manual_delete: 1,
            effective_read: true,
            deep_read: true,
            ..Default::default()
        };
        let outcome = outcome_for_aggregate(&aggregate);
        assert_eq!(outcome.positive_score, 0.0);
        assert!(outcome.negative_score >= 3.0);
    }

    #[test]
    fn only_meaningful_reading_or_quick_exit_is_informative() {
        let neutral = FeedbackAggregate::default();
        assert_eq!(outcome_for_aggregate(&neutral).positive_score, 0.0);
        assert_eq!(outcome_for_aggregate(&neutral).negative_score, 0.0);
        let quick = FeedbackAggregate {
            quick_exit: true,
            ..Default::default()
        };
        assert_eq!(outcome_for_aggregate(&quick).negative_score, 0.5);
    }

    #[test]
    fn profile_conditions_match_numeric_and_binary_features() {
        let profile = ArchiveContentProfileDocument {
            profile_version: CONTENT_PROFILE_VERSION.to_string(),
            content_fingerprint: "hash".to_string(),
            expected_page_count: 60,
            actual_page_count: 60,
            sampled_page_count: 10,
            decoded_page_count: 10,
            coverage: 1.0,
            features: vec![
                ContentProfileFeature {
                    key: "page_count".to_string(),
                    value: 60.0,
                    kind: "numeric".to_string(),
                },
                ContentProfileFeature {
                    key: "tag:namespace:value".to_string(),
                    value: 1.0,
                    kind: "binary".to_string(),
                },
            ],
            measurements: json!({}),
        };
        let (_, numeric) = feature_condition(&profile.features[0]);
        let (_, binary) = feature_condition(&profile.features[1]);
        assert!(profile_condition_matches(&numeric, &profile));
        assert!(profile_condition_matches(&binary, &profile));
    }

    #[test]
    fn canonical_theme_features_have_an_observing_only_kind() {
        let feature = ContentProfileFeature {
            key: "theme:theme-id".to_string(),
            value: 1.0,
            kind: CANONICAL_THEME_FEATURE_KIND.to_string(),
        };
        let (condition_key, condition) = feature_condition(&feature);

        assert_eq!(condition_key, "profile:profile-v1:theme:theme-id:eq:1");
        assert_eq!(condition["all"][0]["feature"], "theme:theme-id");
        assert_eq!(feature.kind, CANONICAL_THEME_FEATURE_KIND);
    }

    #[test]
    fn posterior_confidence_has_a_prior() {
        assert_eq!(direction_probability(0.0, 0.0), 0.5);
        assert!(direction_probability(1.0, 0.0) < 1.0);
        assert!(direction_probability(20.0, 1.0) > 0.95);
    }

    #[test]
    fn low_visibility_positions_are_downweighted() {
        assert_eq!(visibility_weight(0.0), 1.0);
        assert!(visibility_weight(0.3) < visibility_weight(1.0));
    }

    #[tokio::test]
    async fn historical_events_before_the_marker_are_not_enqueued() {
        let pool = learning_test_pool(false).await;
        sqlx::query(
            "INSERT INTO user_behavior_events
             (id, user_id, archive_id, event_type, event_key, metadata_json, occurred_at)
             VALUES ('old-event', 'user-1', 'archive-1', 'open', 'old-key', '{}',
                     datetime('now', '-2 hours'))",
        )
        .execute(&pool)
        .await
        .expect("historical event should be inserted");
        let service = PreferenceLearningService::new(pool.clone());
        assert_eq!(service.enqueue_events().await.unwrap(), 0);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM preference_learning_events",)
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );

        insert_event(&pool, "new-event", "open", None).await;
        assert_eq!(service.enqueue_events().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn manual_delete_before_open_is_retained_as_strong_negative_feedback() {
        let pool = learning_test_pool(true).await;
        insert_event(&pool, "delete-event", "manual_delete", None).await;
        let service = PreferenceLearningService::new(pool.clone());
        assert!(service.process_next().await.unwrap());
        assert!(!service.process_next().await.unwrap());

        let aggregate = sqlx::query(
            "SELECT manual_delete, delete_stage FROM preference_feedback_aggregates
             WHERE user_id = 'user-1' AND archive_id = 'archive-1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(aggregate.get::<i64, _>("manual_delete"), 1);
        assert_eq!(aggregate.get::<String, _>("delete_stage"), "before_open");
        let candidates = service.list_candidates("user-1").await.unwrap();
        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|candidate| {
            candidate.negative_score > 0.0 && candidate.positive_score == 0.0
        }));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM preference_feedback_event_applied",)
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn page_turns_are_aggregated_once_per_event_and_keep_maximum_depth() {
        let pool = learning_test_pool(true).await;
        insert_event(&pool, "page-1", "page_turn", Some(1)).await;
        insert_event(&pool, "page-5", "page_turn", Some(5)).await;
        insert_event(&pool, "page-20", "page_turn", Some(20)).await;
        let service = PreferenceLearningService::new(pool.clone());
        for _ in 0..3 {
            assert!(service.process_next().await.unwrap());
        }
        assert_eq!(service.enqueue_events().await.unwrap(), 0);

        let aggregate = sqlx::query(
            "SELECT page_turn_count, max_page, max_progress_ratio, effective_read, deep_read
             FROM preference_feedback_aggregates
             WHERE user_id = 'user-1' AND archive_id = 'archive-1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(aggregate.get::<i64, _>("page_turn_count"), 3);
        assert_eq!(aggregate.get::<i64, _>("max_page"), 20);
        assert!((aggregate.get::<f64, _>("max_progress_ratio") - (20.0 / 60.0)).abs() < 1e-6);
        assert_eq!(aggregate.get::<i64, _>("effective_read"), 1);
        assert_eq!(aggregate.get::<i64, _>("deep_read"), 1);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM preference_feedback_event_applied",)
                .fetch_one(&pool)
                .await
                .unwrap(),
            3
        );
    }

    #[tokio::test]
    async fn profile_pending_waits_and_completion_rebuilds_without_replaying_events() {
        let pool = learning_test_pool(false).await;
        insert_event(&pool, "waiting-page", "page_turn", Some(20)).await;
        let service = PreferenceLearningService::new(pool.clone());
        assert!(service.process_next().await.unwrap());
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT status FROM preference_learning_events
                 WHERE behavior_event_id = 'waiting-page'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "waiting_analysis"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM preference_feedback_event_applied",)
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert!(!service.process_next().await.unwrap());

        insert_completed_profile(&pool).await;
        assert_eq!(
            service.wake_waiting_for_archive("archive-1").await.unwrap(),
            1
        );
        assert!(service.process_next().await.unwrap());
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT status FROM preference_learning_events
                 WHERE behavior_event_id = 'waiting-page'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "completed"
        );
        let candidates = service.list_candidates("user-1").await.unwrap();
        assert!(!candidates.is_empty());
        assert!(candidates
            .iter()
            .all(|candidate| candidate.evidence_state == "insufficient_evidence"));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM preference_feedback_event_applied",)
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn canonical_theme_candidates_remain_observing_even_at_promotion_threshold() {
        let pool = learning_test_pool(false).await;
        let feature = ContentProfileFeature {
            key: "theme:theme-id".to_string(),
            value: 1.0,
            kind: CANONICAL_THEME_FEATURE_KIND.to_string(),
        };
        let (condition_key, condition) = feature_condition(&feature);
        let condition_json = serde_json::to_string(&condition).unwrap();
        let ordinary_feature = ContentProfileFeature {
            key: "color_fraction".to_string(),
            value: 0.7,
            kind: "numeric".to_string(),
        };
        let (ordinary_condition_key, ordinary_condition) = feature_condition(&ordinary_feature);
        let ordinary_condition_json = serde_json::to_string(&ordinary_condition).unwrap();

        sqlx::query(
            "INSERT INTO preference_rules
             (id, user_id, name, rule_version, conditions_json, action,
              confidence_threshold, enabled, owner_role, source, preference_weight)
             VALUES ('old-theme-rule', 'user-1', 'old theme rule', 'old', ?, 'keep',
                     0.85, 1, 'user', 'learned_cold_start', 1.0)",
        )
        .bind(&condition_json)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO preference_rule_candidates
             (id, user_id, condition_key, conditions_json, status, evidence_state,
              source, feature_kind, profile_version, unique_archive_count,
              informative_result_count)
             VALUES ('ordinary-candidate', 'user-1', ?, ?, 'promoted', 'eligible',
                     'cold_start_v1', 'numeric', 'profile-v1', 12, 12)",
        )
        .bind(&ordinary_condition_key)
        .bind(&ordinary_condition_json)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO preference_rules
             (id, user_id, name, rule_version, conditions_json, action,
              confidence_threshold, enabled, owner_role, source, preference_weight)
             VALUES ('ordinary-rule', 'user-1', 'ordinary rule', 'rule-v1', ?, 'keep',
                     0.95, 1, 'user', 'learned_cold_start', 1.0)",
        )
        .bind(&ordinary_condition_json)
        .execute(&pool)
        .await
        .unwrap();

        let profile_json = serde_json::to_string(&ArchiveContentProfileDocument {
            profile_version: CONTENT_PROFILE_VERSION.to_string(),
            content_fingerprint: String::new(),
            expected_page_count: 60,
            actual_page_count: 60,
            sampled_page_count: 1,
            decoded_page_count: 1,
            coverage: 1.0,
            features: vec![feature, ordinary_feature],
            measurements: json!({}),
        })
        .unwrap();

        for index in 1..=12 {
            let archive_id = format!("archive-{index}");
            let fingerprint = format!("hash-{index}");
            if index > 1 {
                sqlx::query(
                    "INSERT INTO archives
                     (id, title, path, file_hash, file_size, page_count)
                     VALUES (?, ?, ?, ?, 1, 60)",
                )
                .bind(&archive_id)
                .bind(&archive_id)
                .bind(format!("/tmp/{archive_id}.cbz"))
                .bind(&fingerprint)
                .execute(&pool)
                .await
                .unwrap();
            }

            let archive_profile_json = profile_json.replace(
                "\"contentFingerprint\":\"\"",
                &format!("\"contentFingerprint\":\"{fingerprint}\""),
            );
            sqlx::query(
                "INSERT INTO archive_content_profiles
                 (id, archive_id, content_fingerprint, profile_version, status, profile_json,
                  expected_page_count, actual_page_count, sampled_page_count, decoded_page_count,
                  coverage, method_json, completed_at)
                 VALUES (?, ?, ?, ?, 'completed', ?, 60, 60, 1, 1, 1.0, '{}', CURRENT_TIMESTAMP)",
            )
            .bind(format!("profile-{index}"))
            .bind(&archive_id)
            .bind(&fingerprint)
            .bind(CONTENT_PROFILE_VERSION)
            .bind(archive_profile_json)
            .execute(&pool)
            .await
            .unwrap();

            sqlx::query(
                "INSERT INTO preference_feedback_aggregates
                 (user_id, archive_id, effective_read, deep_read, completed_read,
                  max_page, max_progress_ratio, first_event_at, last_event_at)
                 VALUES ('user-1', ?, 1, 1, 1, 60, 1.0, datetime('now'), datetime('now'))",
            )
            .bind(&archive_id)
            .execute(&pool)
            .await
            .unwrap();
        }

        PreferenceLearningService::new(pool.clone())
            .rebuild_observing_for_archive("archive-1")
            .await
            .unwrap();

        let candidates = PreferenceLearningService::new(pool.clone())
            .list_candidates("user-1")
            .await
            .unwrap();
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.condition_key == condition_key)
            .expect("canonical theme candidate should be auditable");
        assert_eq!(
            candidate.feature_kind.as_deref(),
            Some(CANONICAL_THEME_FEATURE_KIND)
        );
        assert_eq!(candidate.evidence_state, "observing");
        assert_eq!(candidate.status, "observing");
        assert_eq!(candidate.unique_archive_count, 12);
        let ordinary_candidate = candidates
            .iter()
            .find(|candidate| candidate.condition_key == ordinary_condition_key)
            .expect("ordinary candidate should remain available");
        assert_eq!(ordinary_candidate.feature_kind.as_deref(), Some("numeric"));
        assert_eq!(ordinary_candidate.evidence_state, "eligible");
        assert_eq!(ordinary_candidate.status, "promoted");
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT enabled FROM preference_rules WHERE id = 'old-theme-rule'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT enabled FROM preference_rules WHERE id = 'ordinary-rule'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
    }
}
