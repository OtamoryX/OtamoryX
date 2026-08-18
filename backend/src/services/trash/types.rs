use crate::models::TrashEntry;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ArchiveSnapshot {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) subtitle: Option<String>,
    pub(crate) subtitle_language: Option<String>,
    pub(crate) path: String,
    pub(crate) file_hash: String,
    pub(crate) file_size: i64,
    pub(crate) page_count: i32,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) tags: Vec<TagSnapshot>,
    #[serde(default)]
    pub(crate) related_inserts: Vec<String>,
    #[serde(default)]
    pub(crate) related_updates: Vec<String>,
    pub(crate) source: Option<String>,
    #[serde(default)]
    pub(crate) evidence_pages: Vec<i32>,
    #[serde(default)]
    pub(crate) decision_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TagSnapshot {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) namespace: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct VersionRelationMigration {
    #[serde(default)]
    pub(crate) version: u8,
    pub(crate) keeper_archive_id: String,
    #[serde(default)]
    pub(crate) before_tag_ids: Vec<String>,
    #[serde(default)]
    pub(crate) after_tag_ids: Vec<String>,
    #[serde(default)]
    pub(crate) before_category_ids: Vec<String>,
    #[serde(default)]
    pub(crate) after_category_ids: Vec<String>,
    #[serde(default)]
    pub(crate) progress: Vec<VersionProgressMigration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct VersionProgressMigration {
    pub(crate) user_id: String,
    pub(crate) before: Option<ReadingProgressSnapshot>,
    pub(crate) after: ReadingProgressSnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
pub(crate) struct ReadingProgressSnapshot {
    pub(crate) id: String,
    pub(crate) user_id: String,
    pub(crate) archive_id: String,
    pub(crate) current_page: i32,
    pub(crate) total_pages: i32,
    pub(crate) progress_percentage: f64,
    pub(crate) last_read_at: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct VersionOperationMember {
    pub(crate) id: String,
    pub(crate) user_id: String,
    pub(crate) archive_id: String,
    pub(crate) original_path: String,
    pub(crate) trash_path: Option<String>,
    pub(crate) reason: Option<String>,
    pub(crate) rule_version: Option<String>,
    pub(crate) rule_id: Option<String>,
    pub(crate) evaluation_id: Option<String>,
    pub(crate) model_confidence: Option<f64>,
    pub(crate) metadata_json: String,
    pub(crate) operation_id: Option<String>,
    pub(crate) operation_type: Option<String>,
    pub(crate) status: String,
    pub(crate) deleted_at: chrono::DateTime<Utc>,
    pub(crate) expires_at: Option<chrono::DateTime<Utc>>,
    pub(crate) restored_at: Option<chrono::DateTime<Utc>>,
    pub(crate) cleanup_attempts: i64,
    pub(crate) last_cleanup_attempt_at: Option<chrono::DateTime<Utc>>,
    pub(crate) last_cleanup_error: Option<String>,
    pub(crate) expired_at: Option<chrono::DateTime<Utc>>,
    pub(crate) migration_snapshot_json: String,
}

impl VersionOperationMember {
    pub(crate) fn entry(&self) -> TrashEntry {
        TrashEntry {
            id: self.id.clone(),
            user_id: self.user_id.clone(),
            archive_id: self.archive_id.clone(),
            original_path: self.original_path.clone(),
            trash_path: self.trash_path.clone(),
            reason: self.reason.clone(),
            rule_version: self.rule_version.clone(),
            rule_id: self.rule_id.clone(),
            evaluation_id: self.evaluation_id.clone(),
            model_confidence: self.model_confidence,
            metadata_json: self.metadata_json.clone(),
            operation_id: self.operation_id.clone(),
            operation_type: self.operation_type.clone(),
            status: self.status.clone(),
            deleted_at: self.deleted_at,
            expires_at: self.expires_at,
            restored_at: self.restored_at,
            cleanup_attempts: self.cleanup_attempts,
            last_cleanup_attempt_at: self.last_cleanup_attempt_at,
            last_cleanup_error: self.last_cleanup_error.clone(),
            expired_at: self.expired_at,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TrashCleanupReport {
    pub claimed: u32,
    pub deleted_files: u32,
    pub missing_files: u32,
    pub failed: u32,
}

#[derive(sqlx::FromRow)]
pub(crate) struct TrashCleanupCandidate {
    pub(crate) id: String,
    pub(crate) trash_path: Option<String>,
}

pub(crate) enum TrashFileCleanupResult {
    Deleted,
    Missing,
}
