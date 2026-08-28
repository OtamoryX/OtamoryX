//! AI settings, durable queue processing, and provider integrations.
//!
//! The public facade stays intentionally small.  Each implementation module owns one concern,
//! while `pub(super)` re-exports preserve the existing private collaboration between queue jobs.

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use chrono::{Duration as ChronoDuration, Utc};
use lingua::{Language, LanguageDetector, LanguageDetectorBuilder};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};
use std::{
    collections::HashSet,
    io::Cursor,
    sync::{Arc, LazyLock, OnceLock},
    time::Duration,
};
use tokio::sync::Notify;
use tracing::warn;
use uuid::Uuid;

use crate::models::{AIAuthMode, AIConnectionProfile, AISettings, AIWorkflowTask};

// New-archive work is ordered so inexpensive canonicalization happens before work that consumes
// the title. A larger value is claimed first by the shared durable queue.
pub(crate) const INTAKE_METADATA_PRIORITY: i32 = 5;
pub(crate) const INTAKE_TITLE_RESOLUTION_PRIORITY: i32 = 4;
pub(crate) const INTAKE_OCR_PRIORITY: i32 = 3;
pub(crate) const INTAKE_AUTO_TAGGING_PRIORITY: i32 = 2;
pub(crate) const INTAKE_SYNTHESIS_PRIORITY: i32 = 1;

mod language;
mod provider;
mod queue;
mod settings;
mod tag_jobs;
mod title_jobs;
mod types;

#[cfg(test)]
mod tests;

pub use language::title_hash;
pub(crate) use provider::effective_output_token_limit;
pub(crate) use provider::run_vision_chat_completion_with_prompt_builder;
#[allow(unused_imports)]
pub use provider::{run_chat_completion, run_vision_chat_completion, test_connection, VisionImage};
pub(crate) use queue::MODEL_AVAILABILITY_WAIT_ERROR;
#[allow(unused_imports)]
pub use queue::{process_next_job, spawn_job_worker};
pub use settings::{
    load_ai_settings, provider_state_model, save_ai_settings, select_enabled_profile_id_for_task,
    settings_for_connection_test, settings_for_profile, settings_for_response,
    settings_for_task_execution, settings_for_task_profile, settings_for_task_quality_retry,
    task_system_prompt,
};
pub use tag_jobs::{enqueue_tag_localization, enqueue_tag_localization_backfill};
pub use title_jobs::{
    enqueue_suspicious_title_translation_repairs, enqueue_title_translation,
    enqueue_title_translation_backfill, enqueue_title_translation_retry,
};
pub use types::notify_ai_queue;
pub use types::BackfillResult;

// Internal APIs are visible to sibling implementation modules and the colocated tests only.
pub(super) use language::*;
pub(super) use provider::*;
pub(crate) use provider::{preview_title_translation, TitleTranslationPreview};
#[cfg(test)]
pub(super) use queue::{claim_next_job, release_expired_leases};
pub(crate) use queue::{enqueue_pipeline_job, ActiveQueueConflict, FORCED_MODEL_RETRY_ATTEMPTS};
#[cfg(test)]
pub(super) use settings::*;
pub(super) use tag_jobs::*;
pub(super) use title_jobs::*;
pub(super) use types::*;
