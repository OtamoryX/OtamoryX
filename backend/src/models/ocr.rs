use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrSettings {
    pub enabled: bool,
    pub active_model_id: String,
    pub image: OcrImageSettings,
    pub failure_policy: OcrFailurePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrImageSettings {
    pub target_long_edge: u32,
    pub preferred_decode_bytes: u64,
    pub jpeg_quality: u8,
    pub max_output_bytes: usize,
    pub large_image_long_edge: u32,
    pub large_image_decode_bytes: u64,
    pub large_image_jpeg_quality: u8,
    pub large_image_max_output_bytes: usize,
}

impl Default for OcrImageSettings {
    fn default() -> Self {
        Self {
            target_long_edge: 2048,
            preferred_decode_bytes: 96 * 1024 * 1024,
            jpeg_quality: 86,
            max_output_bytes: 2 * 1024 * 1024,
            large_image_long_edge: 2560,
            large_image_decode_bytes: 256 * 1024 * 1024,
            large_image_jpeg_quality: 88,
            large_image_max_output_bytes: 4 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrFailurePolicy {
    pub skip_unreadable_pages: bool,
    pub max_page_retries: u32,
}

impl Default for OcrFailurePolicy {
    fn default() -> Self {
        Self {
            skip_unreadable_pages: true,
            max_page_retries: 1,
        }
    }
}

impl Default for OcrSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            active_model_id: "ppocrv5-mobile-zh".to_string(),
            image: OcrImageSettings::default(),
            failure_policy: OcrFailurePolicy::default(),
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
    pub image: OcrImageSettings,
    pub failure_policy: OcrFailurePolicy,
    pub models: Vec<OcrModelStatus>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrSettingsUpdate {
    pub enabled: bool,
    pub image: OcrImageSettings,
    pub failure_policy: OcrFailurePolicy,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrOperationResponse {
    pub accepted: bool,
    pub message: String,
}
