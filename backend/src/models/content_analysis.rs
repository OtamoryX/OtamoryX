use serde::{Deserialize, Serialize};

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
