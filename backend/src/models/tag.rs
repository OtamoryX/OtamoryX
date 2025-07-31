use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ArchiveTag {
    pub archive_id: String,
    pub tag_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AIGeneratedTag {
    pub id: String,
    pub archive_id: String,
    pub tag_id: String,
    pub confidence_score: f32,
    pub approved: Option<bool>, // NULL=pending, true=approved, false=rejected
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "reviewedAt")]
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reviewed_by: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AITagReview {
    pub id: String,
    pub archive_title: String,
    pub tag_name: String,
    pub namespace: String,
    pub confidence: f32,
    pub preview_images: Vec<String>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AITagDecision {
    pub tag_id: String,
    pub action: ReviewAction,
    pub edited_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum ReviewAction {
    Approve,
    Reject,
    Edit,
}