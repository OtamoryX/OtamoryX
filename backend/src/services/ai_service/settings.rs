use super::*;

pub async fn load_ai_settings(pool: &Pool<Sqlite>) -> Result<AISettings> {
    let mut settings: AISettings =
        sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
            .bind(SETTINGS_KEY)
            .fetch_optional(pool)
            .await?
            .map(|raw| deserialize_stored_settings(&raw))
            .unwrap_or_default();

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
        let mut settings = serde_json::from_value(value).unwrap_or_default();
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
        settings.execution.max_concurrent_tasks = limits
            .get("max_concurrent_tasks")
            .or_else(|| limits.get("maxConcurrentTasks"))
            .and_then(Value::as_u64)
            .filter(|count| (1..=MAX_AI_WORKERS as u64).contains(count))
            .map(|count| count as usize)
            .unwrap_or(settings.execution.max_concurrent_tasks);
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
    }
    settings
}

pub async fn save_ai_settings(pool: &Pool<Sqlite>, mut settings: AISettings) -> Result<()> {
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

pub(crate) fn active_enabled_profile_id(settings: &AISettings) -> Result<String> {
    let profile = settings
        .profiles
        .iter()
        .find(|profile| profile.id == settings.active_profile_id)
        .ok_or_else(|| anyhow!("Active AI profile does not exist"))?;
    if !profile.enabled {
        return Err(anyhow!("Active AI profile is disabled"));
    }
    Ok(profile.id.clone())
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
