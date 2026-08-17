use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use sqlx::{Pool, Row, Sqlite};
use uuid::Uuid;

use crate::models::{RecordBehaviorEventRequest, UserBehaviorEvent};

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
        match event.event_type.as_str() {
            "open" => {
                sqlx::query("UPDATE random_recommendation_items SET opened_at=COALESCE(opened_at, ?) WHERE id=?")
                    .bind(occurred).bind(item_id).execute(&self.pool).await?;
            }
            "page_turn" => {
                let page = event.page.unwrap_or(0);
                let total = metadata
                    .get("totalPages")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                if page >= 5 || (total > 0 && (page as f64 / total as f64) >= 0.5) {
                    sqlx::query("UPDATE random_recommendation_items SET effective_read_at=COALESCE(effective_read_at, ?) WHERE id=?")
                        .bind(occurred).bind(item_id).execute(&self.pool).await?;
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
                let effective = end >= 5 || (total > 0 && (end as f64 / total as f64) >= 0.5);
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
                request.bind(item_id).execute(&self.pool).await?;
            }
            "manual_delete" => {
                sqlx::query("UPDATE random_recommendation_items SET manual_delete_at=COALESCE(manual_delete_at, ?) WHERE id=?")
                    .bind(occurred).bind(item_id).execute(&self.pool).await?;
            }
            _ => {}
        }
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
        let decision_key = format!("manual-delete:{trash_entry_id}");
        let metadata = serde_json::json!({
            "source": source,
            "trashEntryId": trash_entry_id,
        });
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

    #[test]
    fn rejects_unknown_event_types() {
        assert!(CurationService::validate_event_type("rating").is_err());
    }
}
