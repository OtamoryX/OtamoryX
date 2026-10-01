use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use sqlx::{Pool, Row, Sqlite};
use uuid::Uuid;

use crate::models::{RecordBehaviorEventRequest, UserBehaviorEvent};
use crate::services::recommendations::weighted_graph::GraphFeedbackOutcome;
use crate::services::ContentAnalysisService;

const ALLOWED_EVENT_TYPES: &[&str] = &[
    "open",
    "page_turn",
    "exit",
    "continue_reading",
    "repeat_open",
    "manual_delete",
    "auto_delete",
    "restore",
    "rule_correction",
];

fn meets_effective_read_threshold(page: i64, total_pages: i64) -> bool {
    page >= 5 || (total_pages > 0 && (page as f64 / total_pages as f64) >= 0.5)
}

#[derive(Clone)]
pub struct CurationService {
    pool: Pool<Sqlite>,
}

impl CurationService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub fn validate_event_type(event_type: &str) -> Result<()> {
        if ALLOWED_EVENT_TYPES.contains(&event_type) {
            Ok(())
        } else {
            Err(anyhow!("unsupported behavior event type: {event_type}"))
        }
    }

    pub async fn record_event(
        &self,
        user_id: &str,
        request: &RecordBehaviorEventRequest,
    ) -> Result<(UserBehaviorEvent, bool)> {
        Self::validate_event_type(&request.event_type)?;
        if request.page.is_some_and(|page| page < 1) {
            return Err(anyhow!("page must be greater than zero"));
        }

        let metadata_json = serde_json::to_string(&request.metadata)
            .context("failed to serialize behavior metadata")?;
        let id = Uuid::new_v4().to_string();
        let occurred_at = request.occurred_at.unwrap_or_else(Utc::now);
        let result = sqlx::query(
            "INSERT OR IGNORE INTO user_behavior_events
             (id, user_id, archive_id, event_type, event_key, page, metadata_json, occurred_at, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)",
        )
        .bind(&id)
        .bind(user_id)
        .bind(&request.archive_id)
        .bind(&request.event_type)
        .bind(&request.event_key)
        .bind(request.page)
        .bind(metadata_json)
        .bind(occurred_at)
        .execute(&self.pool)
        .await
        .context("failed to record behavior event")?;

        let duplicate = result.rows_affected() == 0;
        let event = if duplicate {
            sqlx::query_as::<_, UserBehaviorEvent>(
                "SELECT id, user_id, archive_id, event_type, event_key, page, metadata_json, occurred_at, created_at
                 FROM user_behavior_events WHERE user_id = ? AND event_key = ?",
            )
            .bind(user_id)
            .bind(request.event_key.as_deref().unwrap_or_default())
            .fetch_one(&self.pool)
            .await
            .context("failed to load duplicate behavior event")?
        } else {
            sqlx::query_as::<_, UserBehaviorEvent>(
                "SELECT id, user_id, archive_id, event_type, event_key, page, metadata_json, occurred_at, created_at
                 FROM user_behavior_events WHERE id = ?",
            )
            .bind(&id)
            .fetch_one(&self.pool)
            .await
            .context("failed to load recorded behavior event")?
        };

        if let Err(error) = self.attribute_random_recommendation(user_id, &event).await {
            tracing::warn!(user_id, event_id = %event.id, %error, "random recommendation attribution failed");
        }
        if !duplicate {
            if let Err(error) = crate::services::PreferenceLearningService::new(self.pool.clone())
                .enqueue_event(&event.id, &event.user_id)
                .await
            {
                // Startup recovery will find events if an older or partial schema has no
                // learning queue yet. Behavior recording itself must remain independent of it.
                tracing::warn!(user_id, event_id = %event.id, %error, "preference learning event was not queued");
            }
            if let Some(archive_id) = event.archive_id.as_deref() {
                if let Err(error) = crate::services::ContentProfileService::new(self.pool.clone())
                    .enqueue_for_trigger(archive_id, &event.event_type)
                    .await
                {
                    tracing::warn!(user_id, event_id = %event.id, %error, "content profile was not queued for behavior event");
                }
            }
        }
        if !duplicate && feedback_event_can_refresh_analysis(&event.event_type) {
            if let Some(archive_id) = event.archive_id.as_deref() {
                if let Err(error) = ContentAnalysisService::new(self.pool.clone())
                    .enqueue_for_feedback(archive_id)
                    .await
                {
                    tracing::warn!(user_id, event_id = %event.id, %error, "feedback analysis refresh was not queued");
                }
            }
        }

        Ok((event, duplicate))
    }

    async fn attribute_random_recommendation(
        &self,
        user_id: &str,
        event: &UserBehaviorEvent,
    ) -> Result<()> {
        let Some(archive_id) = event.archive_id.as_deref() else {
            return Ok(());
        };
        let metadata: serde_json::Value =
            serde_json::from_str(&event.metadata_json).unwrap_or_else(|_| serde_json::json!({}));
        let session_id = metadata
            .get("recommendationSessionId")
            .or_else(|| metadata.get("recommendation_session_id"))
            .and_then(|value| value.as_str());
        let Some(session_id) = session_id else {
            return Ok(());
        };
        let item = sqlx::query(
            "SELECT id FROM random_recommendation_items
             WHERE session_id=? AND user_id=? AND archive_id=?
               AND EXISTS (SELECT 1 FROM random_recommendation_sessions s
                           WHERE s.id=session_id AND s.expires_at >= CURRENT_TIMESTAMP)
             LIMIT 1",
        )
        .bind(session_id)
        .bind(user_id)
        .bind(archive_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(item) = item else {
            return Ok(());
        };
        let item_id: String = item.get("id");
        let occurred = event.occurred_at;
        let mut graph_outcome = GraphFeedbackOutcome::Unobserved;
        match event.event_type.as_str() {
            "open" => {
                sqlx::query("UPDATE random_recommendation_items SET opened_at=COALESCE(opened_at, ?) WHERE id=?")
                    .bind(occurred).bind(&item_id).execute(&self.pool).await?;
            }
            "continue_reading" | "repeat_open" => {
                let page = event.page.unwrap_or(0) as i64;
                let total = metadata
                    .get("totalPages")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(0);
                if meets_effective_read_threshold(page, total) {
                    sqlx::query("UPDATE random_recommendation_items SET effective_read_at=COALESCE(effective_read_at, ?) WHERE id=?")
                        .bind(occurred).bind(&item_id).execute(&self.pool).await?;
                    graph_outcome = GraphFeedbackOutcome::Positive;
                }
            }
            "page_turn" => {
                let page = event.page.unwrap_or(0);
                let total = metadata
                    .get("totalPages")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                if meets_effective_read_threshold(page as i64, total) {
                    sqlx::query("UPDATE random_recommendation_items SET effective_read_at=COALESCE(effective_read_at, ?) WHERE id=?")
                        .bind(occurred).bind(&item_id).execute(&self.pool).await?;
                    graph_outcome = GraphFeedbackOutcome::Positive;
                }
            }
            "exit" => {
                let end = metadata
                    .get("endPage")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(event.page.unwrap_or(0) as i64);
                let total = metadata
                    .get("totalPages")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let duration = metadata
                    .get("durationMs")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(i64::MAX);
                let effective = meets_effective_read_threshold(end, total);
                let quick = duration < 30_000 && end <= 2;
                let mut query = String::from("UPDATE random_recommendation_items SET ");
                if effective {
                    query.push_str("effective_read_at=COALESCE(effective_read_at, ?), ");
                }
                if quick {
                    query.push_str("quick_exit_at=COALESCE(quick_exit_at, ?), ");
                }
                if query.ends_with(", ") {
                    query.truncate(query.len() - 2);
                } else {
                    return Ok(());
                }
                query.push_str(" WHERE id=?");
                let mut request = sqlx::query(&query);
                if effective {
                    request = request.bind(occurred);
                }
                if quick {
                    request = request.bind(occurred);
                }
                request.bind(&item_id).execute(&self.pool).await?;
                graph_outcome = if quick {
                    GraphFeedbackOutcome::Negative
                } else if effective {
                    GraphFeedbackOutcome::Positive
                } else {
                    GraphFeedbackOutcome::Unobserved
                };
            }
            "manual_delete" => {
                sqlx::query("UPDATE random_recommendation_items SET manual_delete_at=COALESCE(manual_delete_at, ?) WHERE id=?")
                    .bind(occurred).bind(&item_id).execute(&self.pool).await?;
                graph_outcome = GraphFeedbackOutcome::Negative;
            }
            _ => {}
        }
        crate::services::recommendations::weighted_graph::update_user_factors_for_item(
            &self.pool,
            &item_id,
            user_id,
            graph_outcome,
        )
        .await?;
        Ok(())
    }

    pub async fn record_disposition(
        &self,
        user_id: &str,
        archive_id: &str,
        disposition: &str,
        reason: Option<&str>,
        source: &str,
    ) -> Result<()> {
        let valid_disposition = [
            "keep",
            "downrank",
            "auto_delete",
            "manual_delete",
            "restored",
        ];
        if !valid_disposition.contains(&disposition) {
            return Err(anyhow!("unsupported archive disposition: {disposition}"));
        }

        sqlx::query(
            "INSERT INTO archive_dispositions
             (id, user_id, archive_id, disposition, reason, source, metadata_json, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, '{}', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(archive_id)
        .bind(disposition)
        .bind(reason)
        .bind(source)
        .execute(&self.pool)
        .await
        .context("failed to record archive disposition")?;
        Ok(())
    }

    pub async fn record_manual_delete_feedback(
        &self,
        user_id: &str,
        archive_id: &str,
        trash_entry_id: &str,
        reason: Option<&str>,
        source: &str,
    ) -> Result<()> {
        self.record_manual_delete_feedback_with_context(
            user_id,
            archive_id,
            trash_entry_id,
            reason,
            source,
            None,
            None,
        )
        .await
    }

    pub async fn record_manual_delete_feedback_with_context(
        &self,
        user_id: &str,
        archive_id: &str,
        trash_entry_id: &str,
        reason: Option<&str>,
        source: &str,
        recommendation_session_id: Option<&str>,
        recommendation_position: Option<i64>,
    ) -> Result<()> {
        let decision_key = format!("manual-delete:{trash_entry_id}");
        let mut metadata = serde_json::json!({
            "source": source,
            "trashEntryId": trash_entry_id,
        });
        if let Some(session_id) = recommendation_session_id {
            metadata["recommendationSessionId"] = serde_json::Value::String(session_id.to_string());
        }
        if let Some(position) = recommendation_position {
            metadata["recommendationPosition"] = serde_json::Value::Number(position.into());
        }
        let behavior = RecordBehaviorEventRequest {
            archive_id: Some(archive_id.to_string()),
            event_type: "manual_delete".to_string(),
            event_key: Some(decision_key.clone()),
            page: None,
            metadata: metadata.clone(),
            occurred_at: Some(Utc::now()),
        };
        self.record_event(user_id, &behavior).await?;
        self.record_disposition_with_metadata(
            user_id,
            archive_id,
            "manual_delete",
            reason,
            source,
            &metadata,
            Some(&decision_key),
        )
        .await
    }

    pub async fn record_disposition_with_metadata(
        &self,
        user_id: &str,
        archive_id: &str,
        disposition: &str,
        reason: Option<&str>,
        source: &str,
        metadata: &serde_json::Value,
        decision_key: Option<&str>,
    ) -> Result<()> {
        let valid_disposition = [
            "keep",
            "downrank",
            "auto_delete",
            "manual_delete",
            "restored",
        ];
        if !valid_disposition.contains(&disposition) {
            return Err(anyhow!("unsupported archive disposition: {disposition}"));
        }
        let metadata_json =
            serde_json::to_string(metadata).context("failed to serialize disposition metadata")?;
        sqlx::query(
            "INSERT OR IGNORE INTO archive_dispositions
             (id, user_id, archive_id, disposition, reason, source, metadata_json, decision_key,
              created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(archive_id)
        .bind(disposition)
        .bind(reason)
        .bind(source)
        .bind(metadata_json)
        .bind(decision_key)
        .execute(&self.pool)
        .await
        .context("failed to record archive disposition")?;
        Ok(())
    }

    pub async fn list_events(
        &self,
        user_id: &str,
        archive_id: Option<&str>,
        event_type: Option<&str>,
        limit: u32,
    ) -> Result<Vec<UserBehaviorEvent>> {
        let limit = limit.clamp(1, 200) as i64;
        let mut query = String::from(
            "SELECT id, user_id, archive_id, event_type, event_key, page, metadata_json, occurred_at, created_at
             FROM user_behavior_events WHERE user_id = ?",
        );
        if archive_id.is_some() {
            query.push_str(" AND archive_id = ?");
        }
        if let Some(event_type) = event_type {
            Self::validate_event_type(event_type)?;
            query.push_str(" AND event_type = ?");
        }
        query.push_str(" ORDER BY occurred_at DESC, created_at DESC LIMIT ?");

        let mut request = sqlx::query_as::<_, UserBehaviorEvent>(&query).bind(user_id);
        if let Some(archive_id) = archive_id {
            request = request.bind(archive_id);
        }
        if let Some(event_type) = event_type {
            request = request.bind(event_type);
        }
        Ok(request.bind(limit).fetch_all(&self.pool).await?)
    }
}

fn feedback_event_can_refresh_analysis(event_type: &str) -> bool {
    matches!(
        event_type,
        "open"
            | "page_turn"
            | "exit"
            | "continue_reading"
            | "repeat_open"
            | "manual_delete"
            | "restore"
            | "rule_correction"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite pool");
        sqlx::query(
            "CREATE TABLE user_behavior_events (
                id TEXT PRIMARY KEY, user_id TEXT NOT NULL, archive_id TEXT,
                event_type TEXT NOT NULL, event_key TEXT, page INTEGER,
                metadata_json TEXT NOT NULL, occurred_at DATETIME NOT NULL, created_at DATETIME NOT NULL,
                UNIQUE(user_id, event_key)
            )",
        )
        .execute(&pool)
        .await
        .expect("create behavior events table");
        sqlx::query(
            "CREATE TABLE archive_dispositions (
                id TEXT PRIMARY KEY, user_id TEXT NOT NULL, archive_id TEXT NOT NULL,
                disposition TEXT NOT NULL, reason TEXT, source TEXT NOT NULL,
                metadata_json TEXT NOT NULL, decision_key TEXT,
                created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("create archive dispositions table");
        sqlx::query(
            "CREATE UNIQUE INDEX archive_disposition_decision_key
             ON archive_dispositions(user_id, decision_key)
             WHERE decision_key IS NOT NULL",
        )
        .execute(&pool)
        .await
        .expect("create disposition decision key index");
        sqlx::query(
            "CREATE TABLE random_recommendation_sessions (
                id TEXT PRIMARY KEY, user_id TEXT NOT NULL, expires_at DATETIME NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("create recommendation sessions table");
        sqlx::query(
            "CREATE TABLE random_recommendation_items (
                id TEXT PRIMARY KEY, session_id TEXT NOT NULL, user_id TEXT NOT NULL,
                archive_id TEXT NOT NULL, opened_at DATETIME, effective_read_at DATETIME,
                quick_exit_at DATETIME, manual_delete_at DATETIME
            )",
        )
        .execute(&pool)
        .await
        .expect("create recommendation items table");
        sqlx::query(
            "CREATE TABLE random_recommendation_graph_trials (
                id TEXT PRIMARY KEY, item_id TEXT NOT NULL, user_id TEXT NOT NULL,
                tag_a_id TEXT NOT NULL, tag_b_id TEXT NOT NULL,
                signed_contribution REAL NOT NULL, source_archive_ids_json TEXT NOT NULL,
                positive_feedback_at DATETIME, negative_feedback_at DATETIME,
                UNIQUE(item_id, tag_a_id, tag_b_id)
            )",
        )
        .execute(&pool)
        .await
        .expect("create recommendation graph trials table");
        sqlx::query(
            "CREATE TABLE tag_relation_user_factors (
                user_id TEXT NOT NULL, tag_a_id TEXT NOT NULL, tag_b_id TEXT NOT NULL,
                influence REAL NOT NULL, updated_at DATETIME NOT NULL,
                PRIMARY KEY(user_id, tag_a_id, tag_b_id)
            )",
        )
        .execute(&pool)
        .await
        .expect("create relation user factors table");
        sqlx::query(
            "CREATE TABLE tag_relation_user_archive_feedback (
                user_id TEXT NOT NULL, archive_id TEXT NOT NULL,
                tag_a_id TEXT NOT NULL, tag_b_id TEXT NOT NULL,
                positive_feedback_at DATETIME, negative_feedback_at DATETIME,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY(user_id, archive_id, tag_a_id, tag_b_id)
            )",
        )
        .execute(&pool)
        .await
        .expect("create relation user archive feedback table");
        pool
    }

    #[tokio::test]
    async fn event_key_makes_writes_idempotent() {
        let pool = setup().await;
        let service = CurationService::new(pool.clone());
        let request = RecordBehaviorEventRequest {
            archive_id: Some("archive-1".to_string()),
            event_type: "open".to_string(),
            event_key: Some("reader-session-1".to_string()),
            page: None,
            metadata: serde_json::json!({"source": "reader"}),
            occurred_at: Some(Utc::now()),
        };

        let (_, first_duplicate) = service.record_event("user-1", &request).await.unwrap();
        let (second, second_duplicate) = service.record_event("user-1", &request).await.unwrap();

        assert!(!first_duplicate);
        assert!(second_duplicate);
        assert_eq!(second.event_key.as_deref(), Some("reader-session-1"));
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM user_behavior_events")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 1);
    }

    #[tokio::test]
    async fn manual_delete_feedback_is_idempotent_per_trash_entry() {
        let pool = setup().await;
        let service = CurationService::new(pool.clone());

        service
            .record_manual_delete_feedback(
                "user-1",
                "archive-1",
                "trash-1",
                Some("manual deletion"),
                "user",
            )
            .await
            .unwrap();
        service
            .record_manual_delete_feedback(
                "user-1",
                "archive-1",
                "trash-1",
                Some("manual deletion"),
                "user",
            )
            .await
            .unwrap();

        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM user_behavior_events WHERE event_type = 'manual_delete'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM archive_dispositions WHERE disposition = 'manual_delete'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
    }

    #[tokio::test]
    async fn manual_delete_feedback_attributes_random_context() {
        let pool = setup().await;
        sqlx::query(
            "INSERT INTO random_recommendation_sessions (id, user_id, expires_at)
             VALUES ('session-1', 'user-1', datetime('now', '+1 day'))",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO random_recommendation_items (id, session_id, user_id, archive_id)
             VALUES ('item-1', 'session-1', 'user-1', 'archive-1')",
        )
        .execute(&pool)
        .await
        .unwrap();

        CurationService::new(pool.clone())
            .record_manual_delete_feedback_with_context(
                "user-1",
                "archive-1",
                "trash-1",
                Some("manual deletion"),
                "user",
                Some("session-1"),
                Some(3),
            )
            .await
            .unwrap();

        let attributed: Option<String> = sqlx::query_scalar(
            "SELECT manual_delete_at FROM random_recommendation_items WHERE id = 'item-1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(attributed.is_some());

        let metadata: String = sqlx::query_scalar(
            "SELECT metadata_json FROM user_behavior_events WHERE event_type = 'manual_delete'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let metadata: serde_json::Value = serde_json::from_str(&metadata).unwrap();
        assert_eq!(metadata["recommendationSessionId"], "session-1");
        assert_eq!(metadata["recommendationPosition"], 3);
    }

    #[tokio::test]
    async fn attributed_manual_delete_reduces_the_edge_factor() {
        let pool = setup().await;
        sqlx::query(
            "INSERT INTO random_recommendation_sessions (id, user_id, expires_at)
             VALUES ('session-graph', 'user-1', datetime('now', '+1 day'))",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO random_recommendation_items (id, session_id, user_id, archive_id)
             VALUES ('item-graph', 'session-graph', 'user-1', 'archive-1')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO random_recommendation_graph_trials
             (id, item_id, user_id, tag_a_id, tag_b_id, signed_contribution,
              source_archive_ids_json)
             VALUES ('trial-1', 'item-graph', 'user-1', 'tag-a', 'tag-b', 0.3, '[]')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let effective_read = RecordBehaviorEventRequest {
            archive_id: Some("archive-1".to_string()),
            event_type: "page_turn".to_string(),
            event_key: Some("effective-read-before-delete".to_string()),
            page: Some(10),
            metadata: serde_json::json!({
                "recommendationSessionId": "session-graph",
                "totalPages": 20,
            }),
            occurred_at: Some(Utc::now()),
        };
        CurationService::new(pool.clone())
            .record_event("user-1", &effective_read)
            .await
            .unwrap();

        let request = RecordBehaviorEventRequest {
            archive_id: Some("archive-1".to_string()),
            event_type: "manual_delete".to_string(),
            event_key: Some("delete-graph-item".to_string()),
            page: None,
            metadata: serde_json::json!({"recommendationSessionId": "session-graph"}),
            occurred_at: Some(Utc::now()),
        };
        CurationService::new(pool.clone())
            .record_event("user-1", &request)
            .await
            .unwrap();

        let late_page_turn = RecordBehaviorEventRequest {
            archive_id: Some("archive-1".to_string()),
            event_type: "page_turn".to_string(),
            event_key: Some("late-page-turn-after-delete".to_string()),
            page: Some(10),
            metadata: serde_json::json!({
                "recommendationSessionId": "session-graph",
                "totalPages": 20,
            }),
            occurred_at: Some(Utc::now() + chrono::Duration::seconds(1)),
        };
        CurationService::new(pool.clone())
            .record_event("user-1", &late_page_turn)
            .await
            .unwrap();
        CurationService::new(pool.clone())
            .record_event("user-1", &late_page_turn)
            .await
            .unwrap();

        let influence: f64 = sqlx::query_scalar(
            "SELECT influence FROM tag_relation_user_factors
             WHERE user_id='user-1' AND tag_a_id='tag-a' AND tag_b_id='tag-b'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(influence, 0.7);
        let feedback: (Option<String>, Option<String>) = sqlx::query_as(
            "SELECT positive_feedback_at, negative_feedback_at
             FROM random_recommendation_graph_trials
             WHERE id='trial-1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(feedback.0.is_some());
        assert!(feedback.1.is_some());
    }

    #[tokio::test]
    async fn continue_and_repeat_only_train_the_edge_after_effective_read_depth() {
        let pool = setup().await;
        for (session_id, item_id, trial_id, archive_id) in [
            (
                "session-continue",
                "item-continue",
                "trial-continue",
                "archive-continue",
            ),
            (
                "session-repeat",
                "item-repeat",
                "trial-repeat",
                "archive-repeat",
            ),
            (
                "session-shallow",
                "item-shallow",
                "trial-shallow",
                "archive-shallow",
            ),
        ] {
            sqlx::query(
                "INSERT INTO random_recommendation_sessions (id, user_id, expires_at)
                 VALUES (?, 'user-1', datetime('now', '+1 day'))",
            )
            .bind(session_id)
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO random_recommendation_items (id, session_id, user_id, archive_id)
                 VALUES (?, ?, 'user-1', ?)",
            )
            .bind(item_id)
            .bind(session_id)
            .bind(archive_id)
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO random_recommendation_graph_trials
                 (id, item_id, user_id, tag_a_id, tag_b_id, signed_contribution,
                  source_archive_ids_json)
                 VALUES (?, ?, 'user-1', 'tag-a', 'tag-b', 0.4, '[]')",
            )
            .bind(trial_id)
            .bind(item_id)
            .execute(&pool)
            .await
            .unwrap();
        }
        sqlx::query(
            "INSERT INTO tag_relation_user_factors
             (user_id, tag_a_id, tag_b_id, influence, updated_at)
             VALUES ('user-1', 'tag-a', 'tag-b', 0.5, CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let curation = CurationService::new(pool.clone());
        for (archive_id, event_type, event_key, page) in [
            (
                "archive-continue",
                "continue_reading",
                "continue-effective",
                10,
            ),
            ("archive-repeat", "repeat_open", "repeat-effective", 10),
            ("archive-shallow", "continue_reading", "continue-shallow", 2),
        ] {
            curation
                .record_event(
                    "user-1",
                    &RecordBehaviorEventRequest {
                        archive_id: Some(archive_id.to_string()),
                        event_type: event_type.to_string(),
                        event_key: Some(event_key.to_string()),
                        page: Some(page),
                        metadata: serde_json::json!({
                            "recommendationSessionId": format!("session-{}", archive_id.strip_prefix("archive-").unwrap()),
                            "totalPages": 20,
                        }),
                        occurred_at: Some(Utc::now()),
                    },
                )
                .await
                .unwrap();
        }

        let marked: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM random_recommendation_graph_trials
             WHERE positive_feedback_at IS NOT NULL",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(marked, 2);
        let shallow_marked: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM random_recommendation_items
             WHERE id='item-shallow' AND effective_read_at IS NOT NULL",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(shallow_marked, 0);
        let influence: f64 = sqlx::query_scalar(
            "SELECT influence FROM tag_relation_user_factors
             WHERE user_id='user-1' AND tag_a_id='tag-a' AND tag_b_id='tag-b'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!((influence - 0.82).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn quick_exit_blocks_a_late_effective_page_turn_for_the_same_trial() {
        let pool = setup().await;
        sqlx::query(
            "INSERT INTO random_recommendation_sessions (id, user_id, expires_at)
             VALUES ('session-quick', 'user-1', datetime('now', '+1 day'))",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO random_recommendation_items (id, session_id, user_id, archive_id)
             VALUES ('item-quick', 'session-quick', 'user-1', 'archive-quick')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO random_recommendation_graph_trials
             (id, item_id, user_id, tag_a_id, tag_b_id, signed_contribution,
              source_archive_ids_json)
             VALUES ('trial-quick', 'item-quick', 'user-1', 'tag-a', 'tag-b', 0.3, '[]')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let curation = CurationService::new(pool.clone());
        let quick_exit = RecordBehaviorEventRequest {
            archive_id: Some("archive-quick".to_string()),
            event_type: "exit".to_string(),
            event_key: Some("quick-exit".to_string()),
            page: Some(1),
            metadata: serde_json::json!({
                "recommendationSessionId": "session-quick",
                "totalPages": 20,
                "durationMs": 1000,
                "endPage": 1,
            }),
            occurred_at: Some(Utc::now()),
        };
        curation.record_event("user-1", &quick_exit).await.unwrap();
        curation
            .record_event(
                "user-1",
                &RecordBehaviorEventRequest {
                    archive_id: Some("archive-quick".to_string()),
                    event_type: "page_turn".to_string(),
                    event_key: Some("late-page-turn-after-quick-exit".to_string()),
                    page: Some(10),
                    metadata: serde_json::json!({
                        "recommendationSessionId": "session-quick",
                        "totalPages": 20,
                    }),
                    occurred_at: Some(Utc::now() + chrono::Duration::seconds(1)),
                },
            )
            .await
            .unwrap();
        curation
            .record_event(
                "user-1",
                &RecordBehaviorEventRequest {
                    archive_id: Some("archive-quick".to_string()),
                    event_type: "manual_delete".to_string(),
                    event_key: Some("manual-delete-after-quick-exit".to_string()),
                    page: None,
                    metadata: serde_json::json!({
                        "recommendationSessionId": "session-quick",
                    }),
                    occurred_at: Some(Utc::now() + chrono::Duration::seconds(2)),
                },
            )
            .await
            .unwrap();

        let influence: f64 = sqlx::query_scalar(
            "SELECT influence FROM tag_relation_user_factors
             WHERE user_id='user-1' AND tag_a_id='tag-a' AND tag_b_id='tag-b'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(influence, 0.7);
        let positive_feedback: Option<String> = sqlx::query_scalar(
            "SELECT positive_feedback_at FROM random_recommendation_graph_trials
             WHERE id='trial-quick'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(positive_feedback.is_none());
    }

    #[tokio::test]
    async fn open_without_reading_does_not_update_edge_factor() {
        let pool = setup().await;
        sqlx::query(
            "INSERT INTO random_recommendation_sessions (id, user_id, expires_at)
             VALUES ('session-graph', 'user-1', datetime('now', '+1 day'))",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO random_recommendation_items (id, session_id, user_id, archive_id)
             VALUES ('item-graph', 'session-graph', 'user-1', 'archive-1')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO random_recommendation_graph_trials
             (id, item_id, user_id, tag_a_id, tag_b_id, signed_contribution,
              source_archive_ids_json)
             VALUES ('trial-1', 'item-graph', 'user-1', 'tag-a', 'tag-b', 0.3, '[]')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let request = RecordBehaviorEventRequest {
            archive_id: Some("archive-1".to_string()),
            event_type: "open".to_string(),
            event_key: Some("open-graph-item".to_string()),
            page: None,
            metadata: serde_json::json!({"recommendationSessionId": "session-graph"}),
            occurred_at: Some(Utc::now()),
        };
        CurationService::new(pool.clone())
            .record_event("user-1", &request)
            .await
            .unwrap();

        let factor_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tag_relation_user_factors WHERE user_id='user-1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let positive_feedback: Option<String> = sqlx::query_scalar(
            "SELECT positive_feedback_at FROM random_recommendation_graph_trials
             WHERE id='trial-1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(factor_count, 0);
        assert!(positive_feedback.is_none());
    }

    #[test]
    fn rejects_unknown_event_types() {
        assert!(CurationService::validate_event_type("rating").is_err());
    }
}
