use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

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
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateProgressRequest {
    #[serde(rename = "currentPage")]
    pub current_page: i32,
}