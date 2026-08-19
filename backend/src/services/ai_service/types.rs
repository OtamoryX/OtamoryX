use super::*;

pub(super) const SETTINGS_KEY: &str = "ai_settings";
pub(super) const API_KEY_SETTINGS_KEY: &str = "ai_connection_api_key";
pub(super) const PROFILE_API_KEY_PREFIX: &str = "ai_connection_api_key:";
pub(super) const TITLE_TRANSLATION_JOB: &str = "title_translation";
pub(super) const TITLE_LANGUAGE_DETECTION_JOB: &str = "title_language_detection";
pub(super) const CONTENT_ANALYSIS_RECONCILE_JOB: &str = "content_analysis_reconcile";
pub(super) const CONTENT_ANALYSIS_SYNTHESIZE_JOB: &str = "content_analysis_synthesize";
pub(super) const OCR_EXTRACT_JOB: &str = "ocr_extract";
pub(super) const METADATA_EXTRACT_JOB: &str = "metadata_extract";
pub(super) const AUTO_TAGGING_JOB: &str = "auto_tagging";
pub(super) const TITLE_LANGUAGE_DETECTION_BATCH_SIZE: i64 = 25;
pub(super) const MAX_AI_WORKERS: usize = 16;
pub(super) const TITLE_LANGUAGE_CONFIDENCE_THRESHOLD: f64 = 0.85;

/// In-process wakeups keep the durable SQLite queue as the source of truth while allowing an
/// idle worker pool to sleep without repeatedly querying the database.
pub(super) struct AiQueueSignal {
    pub(super) work: Notify,
    pub(super) scheduler: Notify,
}

pub(super) static AI_QUEUE_SIGNAL: OnceLock<Arc<AiQueueSignal>> = OnceLock::new();

pub(super) fn ai_queue_signal() -> &'static Arc<AiQueueSignal> {
    AI_QUEUE_SIGNAL.get_or_init(|| {
        Arc::new(AiQueueSignal {
            work: Notify::new(),
            scheduler: Notify::new(),
        })
    })
}

pub fn notify_ai_queue() {
    let signal = ai_queue_signal();
    signal.work.notify_waiters();
    signal.scheduler.notify_one();
}

pub(super) static TITLE_LANGUAGE_DETECTOR: LazyLock<LanguageDetector> = LazyLock::new(|| {
    LanguageDetectorBuilder::from_languages(&[
        Language::Chinese,
        Language::English,
        Language::Japanese,
        Language::Korean,
    ])
    .build()
});

#[derive(Debug, Clone, Copy, Default)]
pub struct BackfillResult {
    pub queued: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ClaimedJob {
    pub(crate) id: String,
    pub(crate) archive_id: String,
    pub(crate) source_hash: Option<String>,
    pub(crate) job_type: String,
    pub(crate) payload: Option<String>,
    pub(crate) profile_id: Option<String>,
}

#[derive(Debug)]
pub(crate) struct TitleTranslationJobError {
    pub(crate) message: String,
    pub(crate) retry_policy: RetryPolicy,
    pub(crate) retry_after_seconds: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetryPolicy {
    Permanent,
    Limited,
    Indefinite,
    ProviderCooldown,
}

/// A transport or provider-side outage that can be retried on another enabled profile.
/// Request validation and model-output errors deliberately do not use this type: sending the
/// same invalid task to every configured provider is not failover.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ProviderRequestError {
    #[error("{message}")]
    Unavailable {
        message: String,
        retry_after_seconds: Option<i64>,
    },
}

impl ProviderRequestError {
    pub(crate) fn unavailable(
        message: impl Into<String>,
        retry_after_seconds: Option<i64>,
    ) -> Self {
        Self::Unavailable {
            message: message.into(),
            retry_after_seconds,
        }
    }

    pub(crate) fn retry_after_seconds(&self) -> Option<i64> {
        match self {
            Self::Unavailable {
                retry_after_seconds,
                ..
            } => *retry_after_seconds,
        }
    }
}

#[derive(Debug)]
pub(super) enum TitleTranslationOutput {
    Translated(String),
    AlreadyInTargetLanguage,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ModelTitleTranslation {
    pub(super) title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TitleLanguageDecision {
    Target,
    NonTarget,
    Ambiguous,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TitleLanguageBatchItem {
    pub(super) archive_id: String,
    pub(super) source_hash: String,
    pub(super) title: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TitleLanguageBatchPayload {
    pub(super) target_language: String,
    pub(super) items: Vec<TitleLanguageBatchItem>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TitleTranslationPayload {
    pub(super) target_language: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ModelTitleLanguageDecision {
    pub(super) archive_id: String,
    pub(super) source_hash: String,
    pub(super) is_target_language: bool,
}

impl TitleTranslationJobError {
    pub(crate) fn retryable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retry_policy: RetryPolicy::Indefinite,
            retry_after_seconds: None,
        }
    }

    pub(crate) fn retryable_after(
        message: impl Into<String>,
        retry_after_seconds: Option<i64>,
    ) -> Self {
        Self {
            message: message.into(),
            retry_policy: RetryPolicy::Indefinite,
            retry_after_seconds,
        }
    }

    pub(crate) fn rate_limited(
        message: impl Into<String>,
        retry_after_seconds: Option<i64>,
    ) -> Self {
        Self {
            message: message.into(),
            retry_policy: RetryPolicy::ProviderCooldown,
            retry_after_seconds,
        }
    }

    pub(crate) fn provider_unavailable(
        message: impl Into<String>,
        retry_after_seconds: Option<i64>,
    ) -> Self {
        Self {
            message: message.into(),
            retry_policy: RetryPolicy::ProviderCooldown,
            retry_after_seconds,
        }
    }

    pub(crate) fn limited(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retry_policy: RetryPolicy::Limited,
            retry_after_seconds: None,
        }
    }

    pub(crate) fn permanent(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retry_policy: RetryPolicy::Permanent,
            retry_after_seconds: None,
        }
    }
}

impl std::fmt::Display for TitleTranslationJobError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TitleTranslationJobError {}
