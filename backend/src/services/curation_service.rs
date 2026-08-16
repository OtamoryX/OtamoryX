use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use sqlx::{Pool, Sqlite};
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

        Ok((event, duplicate))
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

    #[test]
    fn rejects_unknown_event_types() {
        assert!(CurationService::validate_event_type("rating").is_err());
    }
}
