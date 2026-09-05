use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReadingProgress {
    pub id: i32,
    #[serde(rename = "archiveId")]
    pub archive_id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "currentPage")]
    pub current_page: i32,
    #[serde(rename = "totalPages")]
    pub total_pages: i32,
    #[serde(rename = "progressPercentage")]
    pub progress_percentage: f64,
    #[serde(rename = "lastReadAt")]
    pub last_read_at: DateTime<Utc>,
    pub version: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProgressRequest {
    #[serde(rename = "currentPage")]
    pub current_page: i32,
    #[serde(rename = "expectedVersion")]
    pub expected_version: i64,
    #[serde(rename = "readerSessionId")]
    pub reader_session_id: Option<String>,
    #[serde(rename = "recommendationSessionId")]
    pub recommendation_session_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchProgressRequest {
    #[serde(rename = "archiveIds")]
    pub archive_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchProgressResponse {
    pub progress: std::collections::HashMap<String, ReadingProgress>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadingProgressHistoryItem {
    #[serde(flatten)]
    pub progress: ReadingProgress,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(rename = "subtitleLanguage", skip_serializing_if = "Option::is_none")]
    pub subtitle_language: Option<String>,
    #[serde(rename = "pageCount")]
    pub page_count: i32,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadingProgressListQuery {
    pub status: Option<String>,
}
