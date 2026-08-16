use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAnalysisResult {
    pub themes: Vec<String>,
    pub concepts: Vec<ContentConcept>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentConcept {
    pub name: String,
    pub confidence: f32,
    pub evidence_pages: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAnalysisEvidence {
    pub page_number: i32,
    pub page_role: String,
    pub concepts: Vec<String>,
    pub confidence: Option<f32>,
    pub summary: String,
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
    pub concepts: Vec<ModelConcept>,
    pub evidence: Vec<ModelEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConcept {
    pub name: String,
    pub confidence: f32,
    pub evidence_pages: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEvidence {
    pub page: i32,
    pub role: String,
    pub concepts: Vec<String>,
    pub confidence: Option<f32>,
    pub summary: String,
}
