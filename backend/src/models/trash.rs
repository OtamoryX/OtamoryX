use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TrashEntry {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "archiveId")]
    pub archive_id: String,
    #[serde(rename = "originalPath")]
    pub original_path: String,
    #[serde(rename = "trashPath")]
    pub trash_path: Option<String>,
    pub reason: Option<String>,
    #[serde(rename = "ruleVersion")]
    pub rule_version: Option<String>,
    #[serde(rename = "modelConfidence")]
    pub model_confidence: Option<f64>,
    #[serde(rename = "metadataJson")]
    pub metadata_json: String,
    pub status: String,
    #[serde(rename = "deletedAt")]
    pub deleted_at: DateTime<Utc>,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(rename = "restoredAt")]
    pub restored_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrashQuery {
    pub status: Option<String>,
    pub limit: Option<u32>,
}
