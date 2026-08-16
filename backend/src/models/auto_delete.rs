use serde::{Deserialize, Serialize};

/// A rule engine decision that has already passed upstream content and
/// preference checks. This model is intentionally not exposed by a handler.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutoDeleteDecision {
    #[serde(rename = "archiveId")]
    pub archive_id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    pub reason: String,
    #[serde(rename = "ruleVersion")]
    pub rule_version: String,
    #[serde(rename = "modelConfidence")]
    pub model_confidence: f64,
    #[serde(rename = "evidencePages")]
    pub evidence_pages: Vec<i32>,
    #[serde(rename = "decisionKey")]
    pub decision_key: String,
}
