use super::*;
use crate::models::{AITaskExecutionSettings, AIWorkflowTask, AI_SETTINGS_VERSION};

pub async fn load_ai_settings(pool: &Pool<Sqlite>) -> Result<AISettings> {
    let mut settings: AISettings =
        sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
            .bind(SETTINGS_KEY)
            .fetch_optional(pool)
            .await?
            .map(|raw| deserialize_stored_settings(&raw))
            .unwrap_or_default();

    settings.settings_version = AI_SETTINGS_VERSION;
    normalize_execution_settings(&mut settings);
    normalize_profiles(&mut settings)?;
    for profile in &mut settings.profiles {
        let stored_key =
            sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
                .bind(profile_api_key_settings_key(&profile.id))
                .fetch_optional(pool)
                .await?;
        profile.connection.api_key = stored_key;
        profile.connection.api_key_configured =
            configured_api_key_for_connection(&profile.connection).is_some();
    }
    sync_active_connection(&mut settings)?;
    Ok(settings)
}

pub(super) fn deserialize_stored_settings(raw: &str) -> AISettings {
    serde_json::from_str(raw).unwrap_or_default()
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
        settings.connection.base_url.trim_end_matches('/'),
        settings.connection.model
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

/// The quality retry variant is deliberately narrower than a task override. It changes only
/// sampling temperature for a failed durable task and leaves the selected profile, thinking
/// policy, budgets, timeouts, and prompt settings untouched.
pub const QUALITY_RETRY_TEMPERATURE_DELTA: f64 = 0.1;

pub fn settings_for_task_quality_retry(
    settings: &AISettings,
    task: AIWorkflowTask,
    enabled: bool,
) -> AISettings {
    if !enabled {
        return settings.clone();
    }
    let mut retry = settings.clone();
    let execution = match task {
        AIWorkflowTask::TitleLocalization => &mut retry.features.title_translation.execution,
        AIWorkflowTask::TagLocalization => &mut retry.features.tag_localization.execution,
        AIWorkflowTask::ContentUnderstanding => &mut retry.features.content_understanding.execution,
        AIWorkflowTask::TagGeneration => &mut retry.features.auto_tagging.execution,
    };
    execution.temperature = (execution.temperature + QUALITY_RETRY_TEMPERATURE_DELTA).min(2.0);
    retry
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
    let mut first_token_timeout =
        if task == AIWorkflowTask::ContentUnderstanding || task == AIWorkflowTask::TagGeneration {
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
