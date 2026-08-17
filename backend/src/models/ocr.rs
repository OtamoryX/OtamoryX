use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OcrSettings {
    pub enabled: bool,
    pub active_model_id: String,
}

impl Default for OcrSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            active_model_id: "ppocrv5-mobile-zh".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrModelStatus {
    pub id: String,
    pub name: String,
    pub language: String,
    pub version: String,
    pub downloaded: bool,
    pub active: bool,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrSettingsResponse {
    pub enabled: bool,
    pub active_model_id: String,
    pub cache_path: String,
    pub models: Vec<OcrModelStatus>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrSettingsUpdate {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrOperationResponse {
    pub accepted: bool,
    pub message: String,
}
