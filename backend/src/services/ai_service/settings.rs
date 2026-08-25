use super::*;
use crate::models::{
    AITaskExecutionSettings, AIWorkflowTask, AI_SETTINGS_VERSION, DEFAULT_OUTPUT_TOKEN_LIMIT,
    DEFAULT_THINKING_OUTPUT_TOKEN_LIMIT, LEGACY_DEFAULT_OUTPUT_TOKEN_LIMIT,
    LEGACY_DEFAULT_THINKING_OUTPUT_TOKEN_LIMIT,
};

const MODEL_THINKING_DEFAULT_MIGRATION_VERSION: u32 = 2;

pub async fn load_ai_settings(pool: &Pool<Sqlite>) -> Result<AISettings> {
    let mut settings: AISettings =
        sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
            .bind(SETTINGS_KEY)
            .fetch_optional(pool)
            .await?
            .map(|raw| deserialize_stored_settings(&raw))
            .unwrap_or_default();

    normalize_execution_settings(&mut settings);
    normalize_profiles(&mut settings)?;
    let legacy_key = sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(API_KEY_SETTINGS_KEY)
        .fetch_optional(pool)
        .await?;
    for profile in &mut settings.profiles {
        let stored_key =
            sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
                .bind(profile_api_key_settings_key(&profile.id))
                .fetch_optional(pool)
                .await?
                .or_else(|| {
                    (profile.id == "default")
                        .then(|| legacy_key.clone())
                        .flatten()
                });
        profile.connection.api_key = stored_key;
        profile.connection.api_key_configured =
            configured_api_key_for_connection(&profile.connection).is_some();
    }
    sync_active_connection(&mut settings)?;
    Ok(settings)
}

pub(super) fn deserialize_stored_settings(raw: &str) -> AISettings {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return AISettings::default();
    };
    if value.get("connection").is_some() {
        let stored_version = value
            .get("settingsVersion")
            .or_else(|| value.get("settings_version"))
            .and_then(Value::as_u64)
            .unwrap_or_default() as u32;
        let legacy_timeout = value
            .pointer("/execution/timeoutSeconds")
            .or_else(|| value.pointer("/execution/timeout_seconds"))
            .and_then(Value::as_u64)
            .filter(|timeout| (5..=3_600).contains(timeout));
        let active_connection_has_timeout = value
            .pointer("/connection/timeoutSeconds")
            .or_else(|| value.pointer("/connection/timeout_seconds"))
            .is_some();
        let mut settings: AISettings = serde_json::from_value(value.clone()).unwrap_or_default();
        migrate_legacy_profile_repetition_settings(&value, &mut settings);
        migrate_legacy_execution_budget_defaults(stored_version, &mut settings);
        migrate_task_defaults(&value, stored_version, &mut settings);
        settings.settings_version = AI_SETTINGS_VERSION;
        if let Some(timeout) = legacy_timeout {
            if !active_connection_has_timeout {
                settings.connection.timeout_seconds = timeout;
            }
            for profile in &mut settings.profiles {
                if profile.connection.timeout_seconds
                    == crate::models::AIConnectionSettings::default().timeout_seconds
                {
                    profile.connection.timeout_seconds = timeout;
                }
            }
        }
        let _ = normalize_profiles(&mut settings);
        return settings;
    }

    // Preserve the subset of the original flat settings schema that still has a destination in
    // the shared AI configuration. The old scheduler fields had no executing implementation.
    let mut settings = AISettings::default();
    settings.features.auto_tagging.enabled = value
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let limits = value
        .get("resource_limits")
        .or_else(|| value.get("resourceLimits"));
    if let Some(limits) = limits {
        let legacy_max_concurrent_tasks = limits
            .get("max_concurrent_tasks")
            .or_else(|| limits.get("maxConcurrentTasks"))
            .and_then(Value::as_u64)
            .filter(|count| (1..=MAX_AI_WORKERS_PER_LANE as u64).contains(count))
            .map(|count| count as usize)
            .unwrap_or(settings.execution.lanes.llm);
        settings
            .execution
            .lanes
            .apply_legacy_global_limit(legacy_max_concurrent_tasks);
        settings.execution.timeout_seconds = limits
            .get("timeout_seconds")
            .or_else(|| limits.get("timeoutSeconds"))
            .and_then(Value::as_u64)
            .filter(|timeout| (5..=1_800).contains(timeout))
            .unwrap_or(settings.execution.timeout_seconds);
        settings.execution.max_retries = limits
            .get("max_retries")
            .or_else(|| limits.get("maxRetries"))
            .and_then(Value::as_u64)
            .filter(|retries| *retries <= 10)
            .map(|retries| retries as u32)
            .unwrap_or(settings.execution.max_retries);
        settings.connection.timeout_seconds = settings.execution.timeout_seconds;
    }
    settings
}

/// Version 3 raises cold-start output reservations. Settings whose values no longer match the
/// old defaults are treated as administrator choices and are left untouched. The exact old
/// default is the only ambiguous case, and is upgraded so existing installations benefit from
/// the safer baseline without adding another user-facing policy switch.
fn migrate_legacy_execution_budget_defaults(stored_version: u32, settings: &mut AISettings) {
    if stored_version >= AI_SETTINGS_VERSION {
        return;
    }

    if settings.execution.output_token_limit == LEGACY_DEFAULT_OUTPUT_TOKEN_LIMIT {
        settings.execution.output_token_limit = DEFAULT_OUTPUT_TOKEN_LIMIT;
    }
    if settings.execution.thinking_output_token_limit == LEGACY_DEFAULT_THINKING_OUTPUT_TOKEN_LIMIT
    {
        settings.execution.thinking_output_token_limit = DEFAULT_THINKING_OUTPUT_TOKEN_LIMIT;
    }
}

fn migrate_legacy_profile_repetition_settings(value: &Value, settings: &mut AISettings) {
    let legacy_penalty = settings.features.title_translation.ollama_repeat_penalty;
    let legacy_last_n = settings.features.title_translation.ollama_repeat_last_n;
    let raw_connection = value.get("connection");
    if !raw_connection.is_some_and(|connection| {
        connection.get("ollamaRepeatPenalty").is_some()
            || connection.get("ollama_repeat_penalty").is_some()
    }) {
        settings.connection.ollama_repeat_penalty = legacy_penalty;
    }
    if !raw_connection.is_some_and(|connection| {
        connection.get("ollamaRepeatLastN").is_some()
            || connection.get("ollama_repeat_last_n").is_some()
    }) {
        settings.connection.ollama_repeat_last_n = legacy_last_n;
    }

    let raw_profiles = value.get("profiles").and_then(Value::as_array);
    for (index, profile) in settings.profiles.iter_mut().enumerate() {
        let raw_profile_connection = raw_profiles
            .and_then(|profiles| profiles.get(index))
            .and_then(|profile| profile.get("connection"));
        if !raw_profile_connection.is_some_and(|connection| {
            connection.get("ollamaRepeatPenalty").is_some()
                || connection.get("ollama_repeat_penalty").is_some()
        }) {
            profile.connection.ollama_repeat_penalty = legacy_penalty;
        }
        if !raw_profile_connection.is_some_and(|connection| {
            connection.get("ollamaRepeatLastN").is_some()
                || connection.get("ollama_repeat_last_n").is_some()
        }) {
            profile.connection.ollama_repeat_last_n = legacy_last_n;
        }
    }
}

fn migrate_task_defaults(value: &Value, stored_version: u32, settings: &mut AISettings) {
    let title_temperature = settings.features.title_translation.temperature;
    let title_structured_output_mode = settings
        .features
        .title_translation
        .structured_output_mode
        .clone();
    let default_content_images = Some(settings.execution.max_images_per_task.min(4).max(1));
    let default_tagging_images = Some(settings.execution.max_images_per_task.min(6).max(1));
    // Tag-localization administrator guidance was removed. Ignore it before matching the legacy
    // default fingerprint so a hidden obsolete value cannot keep thinking disabled.
    settings
        .features
        .tag_localization
        .execution
        .additional_instructions
        .clear();
    migrate_task_execution(
        task_execution_value(value, "titleTranslation", "title_translation"),
        stored_version,
        &mut settings.features.title_translation.execution,
        title_temperature,
        &title_structured_output_mode,
        None,
    );
    migrate_task_execution(
        task_execution_value(value, "tagLocalization", "tag_localization"),
        stored_version,
        &mut settings.features.tag_localization.execution,
        0.0,
        "jsonObject",
        None,
    );
    migrate_task_execution(
        task_execution_value(value, "contentUnderstanding", "content_understanding"),
        stored_version,
        &mut settings.features.content_understanding.execution,
        0.0,
        "jsonObject",
        default_content_images,
    );
    migrate_task_execution(
        task_execution_value(value, "autoTagging", "auto_tagging"),
        stored_version,
        &mut settings.features.auto_tagging.execution,
        0.0,
        "jsonObject",
        default_tagging_images,
    );
    if stored_version < AI_SETTINGS_VERSION
        && settings
            .features
            .auto_tagging
            .execution
            .max_images_per_request
            == Some(4)
    {
        settings
            .features
            .auto_tagging
            .execution
            .max_images_per_request =
            Some(settings.execution.max_images_per_task.min(6).max(1));
    }
    settings.features.title_translation.temperature =
        settings.features.title_translation.execution.temperature;
    settings.features.title_translation.structured_output_mode = settings
        .features
        .title_translation
        .execution
        .structured_output_mode
        .clone();
}

fn task_execution_value<'a>(
    value: &'a Value,
    camel_case: &str,
    snake_case: &str,
) -> Option<&'a Value> {
    value
        .get("features")
        .and_then(|features| {
            features
                .get(camel_case)
                .or_else(|| features.get(snake_case))
        })
        .and_then(|feature| feature.get("execution"))
}

fn migrate_task_execution(
    raw: Option<&Value>,
    stored_version: u32,
    execution: &mut AITaskExecutionSettings,
    default_temperature: f64,
    default_structured_output_mode: &str,
    default_max_images: Option<usize>,
) {
    let has_temperature = raw.is_some_and(|raw| raw.get("temperature").is_some());
    let has_max_images = raw.is_some_and(|raw| {
        raw.get("maxImagesPerRequest").is_some() || raw.get("max_images_per_request").is_some()
    });
    let has_structured_output_mode = raw.is_some_and(|raw| {
        raw.get("structuredOutputMode").is_some() || raw.get("structured_output_mode").is_some()
    });
    if !has_temperature {
        execution.temperature = default_temperature;
    }
    if !has_max_images {
        execution.max_images_per_request = default_max_images;
    }
    if !has_structured_output_mode {
        execution.structured_output_mode = default_structured_output_mode.to_string();
    }
    if stored_version < MODEL_THINKING_DEFAULT_MIGRATION_VERSION
        && is_legacy_disabled_task_default(execution)
    {
        execution.thinking_mode = "inherit".to_string();
    }
}

fn is_legacy_disabled_task_default(execution: &AITaskExecutionSettings) -> bool {
    execution.profile_id.trim() == "auto"
        && execution.thinking_mode == "disabled"
        && execution.output_token_limit.is_none()
        && execution.thinking_output_token_limit.is_none()
        && execution.thinking_context_window_tokens == Some(32_768)
        && execution.timeout_seconds.is_none()
        && execution.additional_instructions.trim().is_empty()
}

pub async fn save_ai_settings(pool: &Pool<Sqlite>, mut settings: AISettings) -> Result<()> {
    settings.settings_version = AI_SETTINGS_VERSION;
    normalize_execution_settings(&mut settings);
    normalize_profiles(&mut settings)?;
    validate_settings(&settings)?;
    let submitted_keys: Vec<(String, String)> = settings
        .profiles
        .iter_mut()
        .filter_map(|profile| {
            profile
                .connection
                .api_key
                .take()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .map(|key| (profile.id.clone(), key))
        })
        .collect();
    sync_active_connection(&mut settings)?;

    let stored_json = serde_json::to_string(&settings)?;
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(SETTINGS_KEY)
    .bind(stored_json)
    .execute(pool)
    .await?;

    // Persist API keys independently so ordinary settings reads and responses cannot expose them.
    for (profile_id, api_key) in submitted_keys {
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(profile_api_key_settings_key(&profile_id))
        .bind(api_key)
        .execute(pool)
        .await?;
    }

    let active_profile_ids: HashSet<&str> = settings
        .profiles
        .iter()
        .map(|profile| profile.id.as_str())
        .collect();
    let stored_key_names = sqlx::query_scalar::<_, String>(
        "SELECT key FROM settings WHERE key LIKE 'ai_connection_api_key:%'",
    )
    .fetch_all(pool)
    .await?;
    for key_name in stored_key_names {
        let profile_id = key_name
            .strip_prefix(PROFILE_API_KEY_PREFIX)
            .unwrap_or_default();
        if !active_profile_ids.contains(profile_id) {
            sqlx::query("DELETE FROM settings WHERE key = ?")
                .bind(key_name)
                .execute(pool)
                .await?;
        }
    }
    // Wake the in-process scheduler and workers after the durable settings update. This also
    // allows a queue that was waiting on a disabled profile to resume immediately.
    notify_ai_queue();
    Ok(())
}

pub(super) fn normalize_execution_settings(settings: &mut AISettings) {
    if let Some(limit) = settings.execution.max_concurrent_tasks.take() {
        settings.execution.lanes.apply_legacy_global_limit(limit);
    }
    settings.features.title_translation.temperature =
        settings.features.title_translation.execution.temperature;
    settings.features.title_translation.structured_output_mode = settings
        .features
        .title_translation
        .execution
        .structured_output_mode
        .clone();
    // Tag localization has no administrator prompt extension. Clear legacy hidden values so
    // they cannot continue changing model behavior after the control was removed from the UI.
    settings
        .features
        .tag_localization
        .execution
        .additional_instructions
        .clear();
    let global_image_limit = settings.execution.max_images_per_task.max(1);
    for execution in [
        &mut settings.features.content_understanding.execution,
        &mut settings.features.auto_tagging.execution,
    ] {
        if let Some(limit) = execution.max_images_per_request.as_mut() {
            *limit = (*limit).min(global_image_limit);
        }
    }
}

pub fn settings_for_response(mut settings: AISettings) -> AISettings {
    for profile in &mut settings.profiles {
        profile.connection.api_key_configured =
            configured_api_key_for_connection(&profile.connection).is_some();
        profile.connection.api_key = None;
    }
    let _ = sync_active_connection(&mut settings);
    settings.connection.api_key = None;
    settings
}

pub fn settings_for_connection_test(
    stored: &AISettings,
    mut provided: AISettings,
) -> Result<AISettings> {
    normalize_execution_settings(&mut provided);
    normalize_profiles(&mut provided)?;
    for profile in &mut provided.profiles {
        if profile.connection.api_key.is_none() {
            if let Some(stored_profile) = stored.profiles.iter().find(|item| item.id == profile.id)
            {
                profile.connection.api_key = stored_profile.connection.api_key.clone();
                profile.connection.api_key_configured =
                    stored_profile.connection.api_key_configured;
            }
        }
    }
    sync_active_connection(&mut provided)?;
    validate_settings(&provided)?;
    Ok(provided)
}

pub fn provider_state_model(settings: &AISettings) -> String {
    format!(
        "{}:{}",
        settings.connection.model, settings.active_profile_id
    )
}

fn profile_api_key_settings_key(profile_id: &str) -> String {
    format!("{PROFILE_API_KEY_PREFIX}{profile_id}")
}

fn normalize_profiles(settings: &mut AISettings) -> Result<()> {
    if settings.profiles.is_empty() {
        settings.profiles.push(AIConnectionProfile {
            id: "default".to_string(),
            name: "Default".to_string(),
            enabled: true,
            connection: settings.connection.clone(),
        });
    }
    for profile in &mut settings.profiles {
        if profile.connection.provider == "ollama" && profile.connection.ollama_max_num_ctx == 0 {
            // Older profiles treated zero as "do not send num_ctx". Native Ollama now always
            // receives the configured context window, so migrate those profiles to the visual
            // model recommendation instead of making existing installations unsaveable.
            profile.connection.ollama_max_num_ctx = 16_384;
        }
        if profile.connection.context_window_tokens == 0 {
            profile.connection.context_window_tokens =
                profile.connection.ollama_max_num_ctx.max(16_384);
        }
        if profile.connection.vision_capable {
            profile.connection.context_window_tokens =
                profile.connection.context_window_tokens.max(16_384);
        }
    }
    if settings.active_profile_id.trim().is_empty()
        || !settings
            .profiles
            .iter()
            .any(|profile| profile.id == settings.active_profile_id)
    {
        settings.active_profile_id = settings.profiles[0].id.clone();
    }
    sync_active_connection(settings)
}

fn sync_active_connection(settings: &mut AISettings) -> Result<()> {
    let profile = settings
        .profiles
        .iter()
        .find(|profile| profile.id == settings.active_profile_id)
        .ok_or_else(|| anyhow!("Active AI profile does not exist"))?;
    settings.connection = profile.connection.clone();
    settings.features.title_translation.ollama_repeat_penalty =
        settings.connection.ollama_repeat_penalty;
    settings.features.title_translation.ollama_repeat_last_n =
        settings.connection.ollama_repeat_last_n;
    Ok(())
}

pub fn settings_for_profile(settings: &AISettings, profile_id: Option<&str>) -> Result<AISettings> {
    let mut selected = settings.clone();
    if let Some(profile_id) = profile_id {
        selected.active_profile_id = profile_id.to_string();
    }
    sync_active_connection(&mut selected)?;
    Ok(selected)
}

pub fn settings_for_task_profile(
    settings: &AISettings,
    task: AIWorkflowTask,
    require_vision: bool,
) -> Result<AISettings> {
    if settings.profiles.is_empty() {
        return Ok(settings.clone());
    }
    let profile_id = select_enabled_profile_id_for_task(settings, task, require_vision)
        .ok_or_else(|| anyhow!("No enabled AI profile is available for this task"))?;
    settings_for_profile(settings, Some(&profile_id))
}

/// Prefer the active profile when it satisfies the requested capability, then fall back to an
/// enabled profile. Vision work can deliberately fall back to a text profile when no vision
/// model is configured; callers then omit image input and use metadata/OCR context only.
pub fn select_enabled_profile_id(settings: &AISettings, require_vision: bool) -> Option<String> {
    let matches = |profile: &AIConnectionProfile| {
        profile.enabled && (!require_vision || profile.connection.vision_capable)
    };
    settings
        .profiles
        .iter()
        .find(|profile| profile.id == settings.active_profile_id && matches(profile))
        .or_else(|| settings.profiles.iter().find(|profile| matches(*profile)))
        .map(|profile| profile.id.clone())
}

pub fn task_execution_settings(
    settings: &AISettings,
    task: AIWorkflowTask,
) -> &AITaskExecutionSettings {
    match task {
        AIWorkflowTask::TitleLocalization => &settings.features.title_translation.execution,
        AIWorkflowTask::TagLocalization => &settings.features.tag_localization.execution,
        AIWorkflowTask::ContentUnderstanding => &settings.features.content_understanding.execution,
        AIWorkflowTask::TagGeneration => &settings.features.auto_tagging.execution,
    }
}

/// Resolves the preferred profile for a business workflow. `auto` preserves the existing active
/// profile and capability fallback behavior, while a selected profile pins the initial queue
/// attempt without disabling the queue's provider-failure fallback.
pub fn select_enabled_profile_id_for_task(
    settings: &AISettings,
    task: AIWorkflowTask,
    require_vision: bool,
) -> Option<String> {
    let preferred = task_execution_settings(settings, task).profile_id.trim();
    if !preferred.is_empty() && preferred != "auto" {
        return settings
            .profiles
            .iter()
            .find(|profile| {
                profile.id == preferred
                    && profile.enabled
                    && (!require_vision || profile.connection.vision_capable)
            })
            .map(|profile| profile.id.clone());
    }
    select_enabled_profile_id(settings, require_vision)
}

/// Applies a task's execution overrides after the queue has chosen its active profile. Keeping
/// this separate from profile selection lets task configuration remain stable while provider
/// failover still works normally.
pub fn settings_for_task_execution(settings: &AISettings, task: AIWorkflowTask) -> AISettings {
    let execution = task_execution_settings(settings, task);
    let mut effective = settings.clone();
    let timeout = execution
        .timeout_seconds
        .unwrap_or(settings.execution.timeout_seconds);
    effective.connection.timeout_seconds = timeout;
    let mut first_token_timeout = if task == AIWorkflowTask::ContentUnderstanding
        || task == AIWorkflowTask::TagGeneration
    {
        90
    } else {
        effective.connection.first_token_timeout_seconds
    };
    if let Some(task_first_token) = execution.first_token_timeout_seconds {
        first_token_timeout = task_first_token;
    }
    effective.connection.first_token_timeout_seconds = first_token_timeout.min(timeout).max(1);
    match execution.thinking_mode.as_str() {
        "enabled" => effective.connection.ollama_thinking = true,
        "disabled" => effective.connection.ollama_thinking = false,
        _ => {}
    }
    let native_ollama_thinking =
        effective.connection.provider == "ollama" && effective.connection.ollama_thinking;
    if native_ollama_thinking {
        if let Some(context_window_tokens) = execution.thinking_context_window_tokens {
            effective.connection.ollama_max_num_ctx = context_window_tokens;
            effective.connection.context_window_tokens = context_window_tokens;
        }
    }
    effective.execution.resolved_output_token_limit = Some(if native_ollama_thinking {
        execution
            .thinking_output_token_limit
            // Before dual budgets existed, a large task override also applied to thinking. Keep
            // that behavior for existing settings while preserving the safe 4096-token floor.
            .or_else(|| {
                execution
                    .output_token_limit
                    .filter(|limit| *limit > settings.execution.thinking_output_token_limit)
            })
            .unwrap_or(settings.execution.thinking_output_token_limit)
    } else {
        execution
            .output_token_limit
            .unwrap_or(settings.execution.output_token_limit)
    });
    effective.execution.resolved_temperature = Some(execution.temperature);
    if let Some(max_images) = execution.max_images_per_request {
        effective.execution.max_images_per_task =
            effective.execution.max_images_per_task.min(max_images);
    }
    if task == AIWorkflowTask::TitleLocalization {
        // Keep the legacy field synchronized until every provider caller consumes the resolved
        // task value directly.
        effective.features.title_translation.temperature = execution.temperature;
        effective.features.title_translation.structured_output_mode =
            execution.structured_output_mode.clone();
    }
    effective
}

pub fn task_system_prompt(settings: &AISettings, task: AIWorkflowTask, base: &str) -> String {
    if task == AIWorkflowTask::TagLocalization {
        return base.to_string();
    }
    let instructions = task_execution_settings(settings, task)
        .additional_instructions
        .trim();
    if instructions.is_empty() {
        return base.to_string();
    }
    format!(
        "{base}\n\nAdditional administrator guidance: {instructions}\nKeep the required JSON output, application-owned schema, and input-data boundary unchanged."
    )
}
