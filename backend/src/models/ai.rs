use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AIProcessingQueue {
    pub id: String,
    pub archive_id: String,
    pub status: AIProcessingStatus,
    pub priority: i32,
    pub attempts: i32,
    pub last_error: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "startedAt")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(rename = "completedAt")]
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum AIProcessingStatus {
    #[sqlx(rename = "pending")]
    Pending,
    #[sqlx(rename = "processing")]
    Processing,
    #[sqlx(rename = "completed")]
    Completed,
    #[sqlx(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AISettings {
    pub connection: AIConnectionSettings,
    pub execution: AIExecutionSettings,
    pub features: AIFeatures,
}

impl Default for AISettings {
    fn default() -> Self {
        Self {
            connection: AIConnectionSettings::default(),
            execution: AIExecutionSettings::default(),
            features: AIFeatures::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AIConnectionSettings {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    /// This is accepted by PUT but deliberately omitted from every response.
    #[serde(skip_serializing)]
    pub api_key: Option<String>,
    #[serde(skip_deserializing)]
    pub api_key_configured: bool,
}

impl Default for AIConnectionSettings {
    fn default() -> Self {
        Self {
            provider: "openaiCompatible".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: None,
            api_key_configured: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AIExecutionSettings {
    pub max_concurrent_tasks: usize,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

impl Default for AIExecutionSettings {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 2,
            timeout_seconds: 60,
            max_retries: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AIFeatures {
    pub title_translation: AITitleTranslationSettings,
    pub auto_tagging: AIAutoTaggingSettings,
}

impl Default for AIFeatures {
    fn default() -> Self {
        Self {
            title_translation: AITitleTranslationSettings::default(),
            auto_tagging: AIAutoTaggingSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AITitleTranslationSettings {
    pub enabled: bool,
    pub target_language: String,
    pub skip_if_target_language: bool,
    pub retranslate_on_title_change: bool,
}

impl Default for AITitleTranslationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            target_language: "zh-CN".to_string(),
            skip_if_target_language: true,
            retranslate_on_title_change: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AIAutoTaggingSettings {
    pub enabled: bool,
    pub auto_apply_threshold: f32,
}

impl Default for AIAutoTaggingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_apply_threshold: 0.8,
        }
    }
}

// Retained for source compatibility with callers that used the earlier settings model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISchedule {
    pub immediate: bool,
    pub batch_processing: bool,
    pub off_peak_hours: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIResourceLimits {
    pub max_concurrent_tasks: usize,
    pub max_memory_usage: u64,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIStatus {
    pub queue_size: usize,
    pub processing_count: usize,
    pub completed_today: usize,
    pub failed_today: usize,
    pub average_processing_time: Option<Duration>,
    pub active_models: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AIControlRequest {
    pub action: AIControlAction,
}

#[derive(Debug, Clone, Deserialize)]
pub enum AIControlAction {
    Pause,
    Resume,
    Stop,
    Restart,
}

#[derive(Debug, Clone)]
pub struct AIAnalysisResult {
    pub suggested_tags: Vec<SuggestedTag>,
    pub confidence_summary: f32,
    pub processing_time: Duration,
    pub model_version: String,
}

#[derive(Debug, Clone)]
pub struct SuggestedTag {
    pub name: String,
    pub namespace: String,
    pub confidence: f32,
    pub reasoning: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AIModelType {
    LocalModel(String),
    CloudAPI(String),
    Plugin(String),
}
