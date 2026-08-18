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

use crate::models::{AIAuthMode, AIConnectionProfile, AISettings};

mod language;
mod provider;
mod queue;
mod settings;
mod title_jobs;
mod types;

#[cfg(test)]
mod tests;

pub use language::title_hash;
#[allow(unused_imports)]
pub use provider::{run_chat_completion, run_vision_chat_completion, test_connection, VisionImage};
#[allow(unused_imports)]
pub use queue::{process_next_job, spawn_ai_worker};
pub use settings::{
    load_ai_settings, provider_state_model, save_ai_settings, settings_for_connection_test,
    settings_for_response,
};
pub use title_jobs::{
    enqueue_suspicious_title_translation_repairs, enqueue_title_translation,
    enqueue_title_translation_backfill, enqueue_title_translation_retry,
};
pub use types::BackfillResult;

// Internal APIs are visible to sibling implementation modules and the colocated tests only.
pub(super) use language::*;
pub(super) use provider::*;
#[cfg(test)]
pub(super) use queue::{claim_next_job, release_expired_leases};
pub(super) use settings::*;
pub(super) use title_jobs::*;
pub(super) use types::*;
