use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserBehaviorEvent {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "archiveId")]
    pub archive_id: Option<String>,
    #[serde(rename = "eventType")]
    pub event_type: String,
    #[serde(rename = "eventKey")]
    pub event_key: Option<String>,
    pub page: Option<i32>,
    #[serde(rename = "metadataJson")]
    pub metadata_json: String,
    #[serde(rename = "occurredAt")]
    pub occurred_at: DateTime<Utc>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecordBehaviorEventRequest {
    #[serde(rename = "archiveId")]
    pub archive_id: Option<String>,
    #[serde(rename = "eventType")]
    pub event_type: String,
    #[serde(rename = "eventKey")]
    pub event_key: Option<String>,
    pub page: Option<i32>,
    #[serde(default)]
    pub metadata: serde_json::Value,
    #[serde(rename = "occurredAt")]
    pub occurred_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordBehaviorEventResponse {
    pub event: UserBehaviorEvent,
    pub duplicate: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BehaviorEventQuery {
    #[serde(rename = "archiveId")]
    pub archive_id: Option<String>,
    #[serde(rename = "eventType")]
    pub event_type: Option<String>,
    pub limit: Option<u32>,
}
