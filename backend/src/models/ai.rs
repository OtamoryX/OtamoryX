use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;

pub const AI_EXECUTOR_LANES: [&str; 4] = ["llm", "ocr", "plugin", "orchestration"];
pub const AI_SETTINGS_VERSION: u32 = 4;
/// Base provider output reservation for structured requests without native reasoning.
pub const DEFAULT_OUTPUT_TOKEN_LIMIT: u64 = 2_048;
/// Base provider output reservation for native reasoning. Ollama counts reasoning and the final
/// structured answer against one `num_predict` budget.
pub const DEFAULT_THINKING_OUTPUT_TOKEN_LIMIT: u64 = 8_192;
pub(crate) const LEGACY_DEFAULT_OUTPUT_TOKEN_LIMIT: u64 = 1_024;
pub(crate) const LEGACY_DEFAULT_THINKING_OUTPUT_TOKEN_LIMIT: u64 = 4_096;

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
    pub settings_version: u32,
    pub connection: AIConnectionSettings,
    pub profiles: Vec<AIConnectionProfile>,
    pub active_profile_id: String,
    pub execution: AIExecutionSettings,
    pub features: AIFeatures,
}

impl Default for AISettings {
    fn default() -> Self {
        Self {
            settings_version: AI_SETTINGS_VERSION,
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
    /// Overall timeout for one request to this profile, in seconds.
    pub timeout_seconds: u64,
    /// Requests Server-Sent Events from OpenAI-compatible Chat Completions providers.
    pub stream_response: bool,
    /// Maximum time to wait for the first model output token when streaming is enabled.
    pub first_token_timeout_seconds: u64,
    /// Minimum delay between request starts for this model profile.
    pub request_interval_seconds: u64,
    /// For the native Ollama API, offload all available model layers to GPU (`num_gpu: -1`).
    pub ollama_use_gpu: bool,
    /// Native Ollama context window. Values such as `16k` are normalized before persistence.
    #[serde(deserialize_with = "deserialize_num_ctx")]
    pub ollama_max_num_ctx: u64,
    /// Context window used to budget image-heavy requests for every provider. Native Ollama also
    /// receives `ollama_max_num_ctx`; OpenAI-compatible providers use this value for planning.
    pub context_window_tokens: u64,
    /// Whether native Ollama should expose its reasoning/thinking channel.
    pub ollama_thinking: bool,
    /// Native Ollama repetition penalty shared by structured tasks using this model profile.
    pub ollama_repeat_penalty: f64,
    /// Native Ollama repetition window shared by structured tasks using this model profile.
    pub ollama_repeat_last_n: u64,
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

fn deserialize_num_ctx<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumCtx {
        Number(u64),
        Text(String),
    }
    let value = Option::<NumCtx>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(0);
    };
    let raw = match value {
        NumCtx::Number(value) => return Ok(value),
        NumCtx::Text(value) => value,
    };
    let compact = raw.trim().to_ascii_lowercase().replace('_', "");
    let (digits, multiplier) = if let Some(value) = compact.strip_suffix('k') {
        (value, 1_024_u64)
    } else if let Some(value) = compact.strip_suffix('m') {
        (value, 1_048_576_u64)
    } else {
        (compact.as_str(), 1)
    };
    let parsed = digits.parse::<u64>().map_err(|_| {
        serde::de::Error::custom("ollamaMaxNumCtx must be an integer or use k/M suffix")
    })?;
    parsed
        .checked_mul(multiplier)
        .ok_or_else(|| serde::de::Error::custom("ollamaMaxNumCtx is too large"))
}

impl Default for AIConnectionSettings {
    fn default() -> Self {
        Self {
            provider: "openaiCompatible".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            timeout_seconds: 300,
            stream_response: false,
            first_token_timeout_seconds: 30,
            request_interval_seconds: 0,
            ollama_use_gpu: false,
            ollama_max_num_ctx: 16_384,
            context_window_tokens: 16_384,
            ollama_thinking: true,
            ollama_repeat_penalty: 1.15,
            ollama_repeat_last_n: 256,
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
    /// Maximum number of images a vision task may attach after budget planning.
    pub max_images_per_task: usize,
    /// Conservative token cost reserved for one normalized page image.
    pub image_token_budget: u64,
    /// Output reservation sent to providers for structured tasks.
    pub output_token_limit: u64,
    /// Output reservation used when native Ollama reasoning is enabled. Reasoning tokens consume
    /// the same provider budget as the structured result, so this needs a larger default.
    pub thinking_output_token_limit: u64,
    /// Task resolution stores its selected output budget here without mutating the persisted
    /// global defaults. Providers and prompt planners use this value when it is present.
    #[serde(skip)]
    pub resolved_output_token_limit: Option<u64>,
    /// Task resolution stores the selected sampling temperature here for the provider boundary.
    #[serde(skip)]
    pub resolved_temperature: Option<f64>,
    /// Tokens kept unused to absorb tokenizer/provider differences.
    pub prompt_safety_margin: u64,
    /// Number of additional attempts after a provider context overflow.
    pub adaptive_context_retries: u32,
    /// Maximum OCR pages included in a planned vision prompt.
    pub ocr_max_pages: usize,
    /// Maximum OCR characters included per page in a planned vision prompt.
    pub ocr_chars_per_page: usize,
}

impl Default for AIExecutionSettings {
    fn default() -> Self {
        Self {
            lanes: AIExecutorConcurrencySettings::default(),
            max_concurrent_tasks: None,
            timeout_seconds: 180,
            max_retries: 3,
            max_images_per_task: 20,
            image_token_budget: 1_800,
            output_token_limit: DEFAULT_OUTPUT_TOKEN_LIMIT,
            thinking_output_token_limit: DEFAULT_THINKING_OUTPUT_TOKEN_LIMIT,
            resolved_output_token_limit: None,
            resolved_temperature: None,
            prompt_safety_margin: 1_024,
            adaptive_context_retries: 2,
            ocr_max_pages: 8,
            ocr_chars_per_page: 600,
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
    pub tag_localization: AITagLocalizationSettings,
    pub content_understanding: AIContentUnderstandingSettings,
    pub auto_tagging: AIAutoTaggingSettings,
    pub recommendations: AIRecommendationSettings,
}

impl Default for AIFeatures {
    fn default() -> Self {
        Self {
            title_translation: AITitleTranslationSettings::default(),
            tag_localization: AITagLocalizationSettings::default(),
            content_understanding: AIContentUnderstandingSettings::default(),
            auto_tagging: AIAutoTaggingSettings::default(),
            recommendations: AIRecommendationSettings::default(),
        }
    }
}

/// A user-facing AI workflow. Queue job names are deliberately not exposed as configuration
/// concepts: several implementation jobs can form one workflow from the user's perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AIWorkflowTask {
    TitleLocalization,
    TagLocalization,
    ContentUnderstanding,
    TagGeneration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AITaskExecutionSettings {
    /// `auto` uses the active compatible profile; a profile ID pins the first attempt to it.
    pub profile_id: String,
    /// `inherit`, `disabled`, or `enabled`. Tasks inherit the selected model's thinking policy by
    /// default, while an explicit override can bound or enable reasoning for a specific workflow.
    pub thinking_mode: String,
    /// An optional task-specific output reservation used when reasoning is disabled.
    pub output_token_limit: Option<u64>,
    /// An optional task-specific output reservation used when reasoning is enabled. This keeps
    /// short structured outputs from competing with a model's internal reasoning budget.
    pub thinking_output_token_limit: Option<u64>,
    /// Native Ollama context window used while this task has reasoning enabled. `None` keeps the
    /// selected model profile's context setting instead.
    #[serde(default = "default_thinking_context_window_tokens")]
    pub thinking_context_window_tokens: Option<u64>,
    /// Sampling temperature for this structured workflow.
    pub temperature: f64,
    /// `jsonObject` asks the provider to constrain JSON, while `promptOnly` relies on the
    /// application-owned prompt. Title localization additionally supports `jsonSchema`.
    pub structured_output_mode: String,
    /// Optional image cap for vision workflows. Text-only workflows leave this unset.
    pub max_images_per_request: Option<usize>,
    /// An optional task-specific request timeout in seconds.
    pub timeout_seconds: Option<u64>,
    /// An optional task-specific first-token timeout in seconds. `None` keeps the selected
    /// profile's first-token budget, while a value overrides it for this workflow. It is always
    /// bounded by the task's effective overall request timeout.
    pub first_token_timeout_seconds: Option<u64>,
    /// Administrator-authored guidance appended without replacing the application-owned schema
    /// and data-boundary rules.
    pub additional_instructions: String,
}

fn default_thinking_context_window_tokens() -> Option<u64> {
    Some(32_768)
}

impl Default for AITaskExecutionSettings {
    fn default() -> Self {
        Self {
            profile_id: "auto".to_string(),
            thinking_mode: "inherit".to_string(),
            output_token_limit: None,
            thinking_output_token_limit: None,
            thinking_context_window_tokens: default_thinking_context_window_tokens(),
            temperature: 0.0,
            structured_output_mode: "jsonObject".to_string(),
            max_images_per_request: None,
            timeout_seconds: None,
            first_token_timeout_seconds: None,
            additional_instructions: String::new(),
        }
    }
}

impl AITaskExecutionSettings {
    fn title_localization_default() -> Self {
        Self {
            temperature: 0.1,
            structured_output_mode: "promptOnly".to_string(),
            ..Self::default()
        }
    }

    fn vision_workflow_default(max_images: usize) -> Self {
        Self {
            max_images_per_request: Some(max_images),
            ..Self::default()
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
    /// Retained for compatibility with settings written before task-level temperature existed.
    pub temperature: f64,
    /// Retained for compatibility with settings written before repetition controls moved to a
    /// model profile.
    pub ollama_repeat_penalty: f64,
    /// Retained for compatibility with settings written before repetition controls moved to a
    /// model profile.
    pub ollama_repeat_last_n: u64,
    /// Retained for compatibility with settings written before structured output moved to the
    /// task execution block.
    pub structured_output_mode: String,
    pub execution: AITaskExecutionSettings,
}

impl Default for AITitleTranslationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            target_language: "zh-CN".to_string(),
            skip_if_target_language: true,
            retranslate_on_title_change: true,
            display_translated_title: false,
            temperature: 0.1,
            ollama_repeat_penalty: 1.15,
            ollama_repeat_last_n: 256,
            structured_output_mode: "promptOnly".to_string(),
            execution: AITaskExecutionSettings::title_localization_default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AITagLocalizationSettings {
    pub enabled: bool,
    pub execution: AITaskExecutionSettings,
}

impl Default for AITagLocalizationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            execution: AITaskExecutionSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AIContentUnderstandingSettings {
    pub execution: AITaskExecutionSettings,
}

impl Default for AIContentUnderstandingSettings {
    fn default() -> Self {
        Self {
            execution: AITaskExecutionSettings::vision_workflow_default(4),
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
    pub execution: AITaskExecutionSettings,
}

impl Default for AIAutoTaggingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: "autoApplyReliable".to_string(),
            auto_process_new_archives: true,
            execution: AITaskExecutionSettings::vision_workflow_default(6),
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
    /// Bounded manual retries that may run while this model is still in cooldown.
    pub force_attempts_remaining: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AITaskQueueStatus {
    pub job_type: String,
    pub pending_count: usize,
    pub processing_count: usize,
    pub waiting_for_model_count: usize,
    pub waiting_for_dependency_count: usize,
    pub retry_scheduled_count: usize,
    pub manually_paused: bool,
    /// `running`, `queued`, `manually_paused`, `waiting_for_model`,
    /// `waiting_for_dependency`, `retry_scheduled`, or `idle`.
    pub state: String,
    pub blocked_until: Option<String>,
    pub next_run_at: Option<String>,
    pub last_error: Option<String>,
    pub requires_model: bool,
    /// `task`, `model`, or `user` when progress is currently blocked.
    pub blocking_scope: Option<String>,
    /// Stable machine-readable reason matching the queue state.
    pub blocking_reason: Option<String>,
    /// Actions currently accepted for this task queue.
    pub available_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIJobAttemptDiagnostic {
    pub id: String,
    pub attempt_number: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub outcome: Option<String>,
    pub failure_code: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AITaskDiagnostic {
    pub id: String,
    pub archive_id: String,
    pub job_type: String,
    pub status: String,
    pub executor_lane: String,
    pub priority: i64,
    pub attempts_count: i64,
    pub profile_id: Option<String>,
    pub payload: Option<String>,
    pub failure_code: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub next_run_at: Option<String>,
    pub lease_expires_at: Option<String>,
    pub attempts: Vec<AIJobAttemptDiagnostic>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AITaskDiagnosticPage {
    pub items: Vec<AITaskDiagnostic>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIFailureSummaryItem {
    pub job_type: String,
    pub failure_code: String,
    pub count: usize,
    pub latest_at: String,
    pub example_task_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AIFailureSummary {
    pub groups: Vec<AIFailureSummaryItem>,
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
