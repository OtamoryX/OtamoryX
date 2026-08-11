use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionSummary {
    pub id: String,
    pub display_title: String,
    pub cover_archive_id: Option<String>,
    pub status: String,
    pub is_manual_locked: bool,
    pub member_count: i64,
    pub variant_count: i64,
    pub review_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionMember {
    pub archive: crate::models::Archive,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionDetail {
    pub collection: CollectionSummary,
    pub members: Vec<CollectionMember>,
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
    pub is_manual_locked: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionRebuildResponse {
    pub parsed_archives: i64,
    pub created_collections: i64,
    pub grouped_archives: i64,
    pub pending_reviews: i64,
}
