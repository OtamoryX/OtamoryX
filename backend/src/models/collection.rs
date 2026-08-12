use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionSummary {
    pub id: String,
    pub display_title: String,
    pub subtitle: Option<String>,
    pub cover_archive_id: Option<String>,
    pub status: String,
    pub is_manual_locked: bool,
    pub member_count: i64,
    pub content_count: i64,
    pub variant_group_count: i64,
    pub variant_count: i64,
    pub review_count: i64,
    pub matched_member_count: i64,
    pub progress_percentage: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionMember {
    pub archive: crate::models::Archive,
    pub matches_filter: bool,
    pub unit_type: String,
    pub volume_number: Option<String>,
    pub chapter_number: Option<String>,
    pub issue_number: Option<String>,
    pub raw_number: Option<String>,
    pub sort_key: f64,
    pub variant_group_key: Option<String>,
    pub confidence: f64,
    pub membership_source: String,
    pub is_manual_locked: bool,
    pub review: Option<CollectionMemberReview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionMemberReview {
    pub id: String,
    pub reason: String,
    pub evidence: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionDetail {
    pub collection: CollectionSummary,
    pub members: Vec<CollectionMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionCandidate {
    pub archive: crate::models::Archive,
    pub matches_filter: bool,
    pub confidence: f64,
    pub is_recommended: bool,
    pub recommendation_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionGroup {
    pub id: String,
    pub group_key: String,
    pub display_title: String,
    pub subtitle: Option<String>,
    pub collection_id: Option<String>,
    pub collection_title: Option<String>,
    pub unit_label: String,
    pub confidence: f64,
    pub status: String,
    pub recommended_archive_id: Option<String>,
    pub reclaimable_size: i64,
    pub matched_member_count: i64,
    pub members: Vec<VersionCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionReviewItem {
    pub id: String,
    pub archive: crate::models::Archive,
    pub collection: CollectionSummary,
    pub reason: String,
    pub evidence: serde_json::Value,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionReviewAction {
    pub action: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCollectionRequest {
    pub display_title: String,
    pub archive_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCollectionMemberRequest {
    pub archive_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCollectionRequest {
    pub display_title: Option<String>,
    pub subtitle: Option<String>,
    pub is_manual_locked: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionCleanupRequest {
    pub keep_archive_id: String,
    pub delete_archive_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionCleanupResponse {
    pub kept_archive_id: String,
    pub deleted: usize,
    pub failed_archive_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionRebuildResponse {
    pub parsed_archives: i64,
    pub created_collections: i64,
    pub grouped_archives: i64,
    pub pending_reviews: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionRebuildPreviewItem {
    pub display_title: String,
    pub member_count: i64,
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionRebuildPreview {
    pub parsed_archives: i64,
    pub collection_candidates: Vec<CollectionRebuildPreviewItem>,
    pub version_candidates: Vec<CollectionRebuildPreviewItem>,
    pub pending_review_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionDeletionResponse {
    pub collection_id: String,
    pub deleted_archives: u64,
}
