use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAnalysisResult {
    pub themes: Vec<String>,
    pub selected_tags: Vec<ContentSelectedTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSelectedTag {
    pub name: String,
    pub namespace: String,
    pub confidence: f32,
}

/// Deterministic, non-semantic content measurements used by recommendation
/// learning. Values are kept generic so the learning code does not need a
/// vocabulary of user-provided subject preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentProfileFeature {
    pub key: String,
    pub value: f64,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveContentProfileDocument {
    pub profile_version: String,
    pub content_fingerprint: String,
    pub expected_page_count: i32,
    pub actual_page_count: i32,
    pub sampled_page_count: i32,
    pub decoded_page_count: i32,
    pub coverage: f64,
    pub features: Vec<ContentProfileFeature>,
    #[serde(default)]
    pub measurements: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAnalysisEvidence {
    pub page_number: i32,
    pub page_role: String,
    pub themes: Vec<String>,
    pub confidence: Option<f32>,
    pub summary: String,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAnalysisResponse {
    pub id: String,
    pub archive_id: String,
    pub content_fingerprint: String,
    pub status: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt_version: String,
    pub result: Option<ContentAnalysisResult>,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub evidence: Vec<ContentAnalysisEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelContentAnalysis {
    pub themes: Vec<String>,
    #[serde(default)]
    pub selected_tags: Vec<ContentSelectedTag>,
    pub evidence: Vec<ModelEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEvidence {
    pub page: Option<i32>,
    pub role: String,
    pub themes: Vec<String>,
    pub confidence: Option<f32>,
    pub summary: String,
    #[serde(default)]
    pub sources: Vec<String>,
}
