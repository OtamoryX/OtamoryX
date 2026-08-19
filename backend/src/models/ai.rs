use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;

pub const AI_EXECUTOR_LANES: [&str; 4] = ["llm", "ocr", "plugin", "orchestration"];

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
    pub profiles: Vec<AIConnectionProfile>,
    pub active_profile_id: String,
    pub execution: AIExecutionSettings,
    pub features: AIFeatures,
}

impl Default for AISettings {
    fn default() -> Self {
        Self {
            connection: AIConnectionSettings::default(),
            profiles: Vec::new(),
            active_profile_id: "default".to_string(),
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
    /// Whether this profile accepts image content in OpenAI-compatible chat requests.
    /// Text-only profiles remain eligible for translation and metadata/OCR-based tagging.
    pub vision_capable: bool,
    pub auth_mode: AIAuthMode,
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
            vision_capable: true,
            auth_mode: AIAuthMode::Bearer,
            api_key: None,
            api_key_configured: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum AIAuthMode {
    #[default]
    Bearer,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIConnectionProfile {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub connection: AIConnectionSettings,
}

impl AIConnectionProfile {
    pub fn default_profile() -> Self {
        Self {
            id: "default".to_string(),
            name: "Default".to_string(),
            enabled: true,
            connection: AIConnectionSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AIExecutionSettings {
    pub lanes: AIExecutorConcurrencySettings,
    /// Read old persisted settings without exposing the retired global setting again.
    #[serde(skip_serializing)]
    pub max_concurrent_tasks: Option<usize>,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

impl Default for AIExecutionSettings {
    fn default() -> Self {
        Self {
            lanes: AIExecutorConcurrencySettings::default(),
            max_concurrent_tasks: None,
            timeout_seconds: 180,
            max_retries: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AIExecutorConcurrencySettings {
    pub llm: usize,
    pub ocr: usize,
    pub plugin: usize,
    pub orchestration: usize,
}

impl Default for AIExecutorConcurrencySettings {
    fn default() -> Self {
        Self {
            llm: 2,
            ocr: 1,
            plugin: 2,
            orchestration: 2,
        }
    }
}

impl AIExecutorConcurrencySettings {
    pub fn limit_for_lane(&self, lane: &str) -> Option<usize> {
        match lane {
            "llm" => Some(self.llm),
            "ocr" => Some(self.ocr),
            "plugin" => Some(self.plugin),
            "orchestration" => Some(self.orchestration),
            _ => None,
        }
    }

    pub fn apply_legacy_global_limit(&mut self, limit: usize) {
        self.llm = limit;
        self.ocr = 1;
        self.plugin = limit.min(2);
        self.orchestration = 1;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AIFeatures {
    pub title_translation: AITitleTranslationSettings,
    pub auto_tagging: AIAutoTaggingSettings,
    pub recommendations: AIRecommendationSettings,
}

impl Default for AIFeatures {
    fn default() -> Self {
        Self {
            title_translation: AITitleTranslationSettings::default(),
            auto_tagging: AIAutoTaggingSettings::default(),
            recommendations: AIRecommendationSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AIRecommendationSettings {
    /// Enables a small, stable comparison group when an installation opts into the experiment.
    pub multi_user_experiment_enabled: bool,
    /// A completed analysis is refreshed after this age only when new user feedback arrives.
    pub analysis_refresh_after_days: u16,
}

impl Default for AIRecommendationSettings {
    fn default() -> Self {
        Self {
            multi_user_experiment_enabled: false,
            analysis_refresh_after_days: 180,
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
    pub display_translated_title: bool,
}

impl Default for AITitleTranslationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            target_language: "zh-CN".to_string(),
            skip_if_target_language: true,
            retranslate_on_title_change: true,
            display_translated_title: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AIAutoTaggingSettings {
    pub enabled: bool,
    /// `suggestions` retains human review; `autoApplyReliable` applies suggestions with
    /// verified source evidence.
    pub mode: String,
    /// New archives enter the dependency workflow when tagging is enabled.
    pub auto_process_new_archives: bool,
}

impl Default for AIAutoTaggingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: "autoApplyReliable".to_string(),
            auto_process_new_archives: true,
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
    pub language_detection_pending: usize,
    pub retry_scheduled: usize,
    pub unresolved_failure_count: usize,
    pub provider_blocked_until: Option<String>,
    pub average_processing_time: Option<Duration>,
    pub active_models: Vec<String>,
    /// Pending and processing work grouped by durable queue executor lane.
    pub queue_by_lane: BTreeMap<String, usize>,
    /// Each executor lane has an independent worker limit and can progress independently.
    pub executor_lanes: Vec<AIExecutorLaneStatus>,
    /// Availability is attached to an individual configured model, never to the shared queue.
    pub model_states: Vec<AIModelStatus>,
    /// Every durable job type has its own independently controllable queue state.
    pub task_queues: Vec<AITaskQueueStatus>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIExecutorLaneStatus {
    pub executor_lane: String,
    pub pending_count: usize,
    pub processing_count: usize,
    pub max_concurrent_jobs: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIModelStatus {
    pub profile_id: String,
    pub profile_name: String,
    pub model: String,
    /// `available`, `rate_limited`, `unavailable`, or `disabled`.
    pub state: String,
    pub blocked_until: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AITaskQueueStatus {
    pub job_type: String,
    pub pending_count: usize,
    pub processing_count: usize,
    pub waiting_for_model_count: usize,
    pub manually_paused: bool,
    /// `running`, `manually_paused`, `waiting_for_model`, or `idle`.
    pub state: String,
    pub blocked_until: Option<String>,
    pub requires_model: bool,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AITaskQueueControlRequest {
    pub action: AITaskQueueControlAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AITaskQueueControlAction {
    Pause,
    Resume,
    ForceContinue,
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
