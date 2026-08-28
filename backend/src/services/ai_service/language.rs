use super::*;
use crate::models::AITaskExecutionSettings;

pub(super) fn normalize_translated_title(title: &str) -> Result<String> {
    let title = title.trim();
    if title.is_empty() {
        return Err(anyhow!("AI provider returned an empty title"));
    }
    if title.lines().count() != 1 {
        return Err(anyhow!("AI provider returned a multi-line title"));
    }
    Ok(title.to_string())
}

pub(super) fn translation_quality_issue(
    source: &str,
    translated: &str,
    target: &str,
) -> Option<String> {
    let translated = translated.trim();
    if translated.is_empty() {
        return Some("the result is empty".to_string());
    }
    if is_title_translation_refusal(translated) {
        return Some("the model refused to translate the title".to_string());
    }
    if translated.len() > 1_000 {
        return Some("the result is longer than 1000 bytes".to_string());
    }
    let source_length = source.trim().chars().count();
    let translated_length = translated.chars().count();
    let maximum_title_length = source_length.saturating_mul(6).saturating_add(24).max(80);
    if translated_length > maximum_title_length {
        return Some("the result is implausibly long for a title".to_string());
    }
    if translated.contains(source.trim()) && translated_length > source_length.saturating_add(24) {
        return Some("the result embeds the source title in additional text".to_string());
    }
    if source.trim().eq_ignore_ascii_case(translated)
        && !title_looks_like_target_language(source, target)
    {
        return Some("the result repeats the source title unchanged".to_string());
    }

    translation_writing_system_issue(source, translated, target)
}

fn is_title_translation_refusal(value: &str) -> bool {
    let normalized = value
        .trim()
        .to_ascii_lowercase()
        .replace(['\u{2018}', '\u{2019}'], "'");
    if normalized == "[[refused]]"
        || [
            "as an ai",
            "i'm sorry",
            "i cannot",
            "i can't",
            "i'm unable",
            "cannot assist",
            "can't assist",
            "unable to translate",
            "content policy",
            "safety policy",
            "policy violation",
        ]
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return true;
    }

    (value.contains("抱歉")
        && ["不能", "无法", "不可以", "拒绝"]
            .iter()
            .any(|marker| value.contains(marker)))
        || (value.contains("无法")
            && ["翻译", "提供", "协助", "处理"]
                .iter()
                .any(|marker| value.contains(marker)))
        || (value.contains("作为 AI")
            && ["不能", "无法"].iter().any(|marker| value.contains(marker)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ScriptKind {
    Arabic,
    Armenian,
    Bengali,
    Cyrillic,
    Devanagari,
    Ethiopic,
    Georgian,
    Greek,
    Gujarati,
    Gurmukhi,
    Han,
    Hangul,
    Hebrew,
    Kana,
    Khmer,
    Lao,
    Latin,
    Myanmar,
    Sinhala,
    Tamil,
    Telugu,
    Thai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HanVariant {
    Any,
    Simplified,
    Traditional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetScript {
    Any,
    Han(HanVariant),
    Japanese,
    Primary(ScriptKind),
}

#[derive(Debug, Clone, Copy)]
struct TargetLocaleProfile {
    script: TargetScript,
    lingua: Option<Language>,
}

fn is_han(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF)
}

fn is_japanese_kana(c: char) -> bool {
    matches!(c as u32, 0x3040..=0x30FF | 0x31F0..=0x31FF | 0xFF66..=0xFF9D)
}

fn is_hangul(c: char) -> bool {
    matches!(c as u32, 0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xAC00..=0xD7AF | 0xD7B0..=0xD7FF)
}

fn is_latin(c: char) -> bool {
    c.is_ascii_alphabetic()
        || matches!(c as u32, 0x00C0..=0x02AF | 0x1D00..=0x1D7F | 0x1E00..=0x1EFF)
}

fn is_cyrillic(c: char) -> bool {
    matches!(c as u32, 0x0400..=0x052F | 0x2DE0..=0x2DFF | 0xA640..=0xA69F)
}

fn is_arabic(c: char) -> bool {
    matches!(
        c as u32,
        0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF
    )
}

fn is_armenian(c: char) -> bool {
    matches!(c as u32, 0x0530..=0x058F | 0xFB13..=0xFB17)
}

fn is_bengali(c: char) -> bool {
    matches!(c as u32, 0x0980..=0x09FF)
}

fn is_devanagari(c: char) -> bool {
    matches!(c as u32, 0x0900..=0x097F)
}

fn is_ethiopic(c: char) -> bool {
    matches!(c as u32, 0x1200..=0x137F | 0x1380..=0x139F | 0x2D80..=0x2DDF)
}

fn is_georgian(c: char) -> bool {
    matches!(c as u32, 0x10A0..=0x10FF | 0x2D00..=0x2D2F | 0x1C90..=0x1CBF)
}

fn is_greek(c: char) -> bool {
    matches!(c as u32, 0x0370..=0x03FF | 0x1F00..=0x1FFF)
}

fn is_gujarati(c: char) -> bool {
    matches!(c as u32, 0x0A80..=0x0AFF)
}

fn is_gurmukhi(c: char) -> bool {
    matches!(c as u32, 0x0A00..=0x0A7F)
}

fn is_hebrew(c: char) -> bool {
    matches!(c as u32, 0x0590..=0x05FF | 0xFB1D..=0xFB4F)
}

fn is_khmer(c: char) -> bool {
    matches!(c as u32, 0x1780..=0x17FF | 0x19E0..=0x19FF)
}

fn is_lao(c: char) -> bool {
    matches!(c as u32, 0x0E80..=0x0EFF)
}

fn is_myanmar(c: char) -> bool {
    matches!(c as u32, 0x1000..=0x109F | 0xAA60..=0xAA7F | 0xA9E0..=0xA9FF)
}

fn is_sinhala(c: char) -> bool {
    matches!(c as u32, 0x0D80..=0x0DFF)
}

fn is_tamil(c: char) -> bool {
    matches!(c as u32, 0x0B80..=0x0BFF)
}

fn is_telugu(c: char) -> bool {
    matches!(c as u32, 0x0C00..=0x0C7F)
}

fn is_thai(c: char) -> bool {
    matches!(c as u32, 0x0E00..=0x0E7F)
}

fn script_kind(c: char) -> Option<ScriptKind> {
    if is_han(c) {
        Some(ScriptKind::Han)
    } else if is_japanese_kana(c) {
        Some(ScriptKind::Kana)
    } else if is_hangul(c) {
        Some(ScriptKind::Hangul)
    } else if is_latin(c) {
        Some(ScriptKind::Latin)
    } else if is_cyrillic(c) {
        Some(ScriptKind::Cyrillic)
    } else if is_arabic(c) {
        Some(ScriptKind::Arabic)
    } else if is_armenian(c) {
        Some(ScriptKind::Armenian)
    } else if is_bengali(c) {
        Some(ScriptKind::Bengali)
    } else if is_devanagari(c) {
        Some(ScriptKind::Devanagari)
    } else if is_ethiopic(c) {
        Some(ScriptKind::Ethiopic)
    } else if is_georgian(c) {
        Some(ScriptKind::Georgian)
    } else if is_greek(c) {
        Some(ScriptKind::Greek)
    } else if is_gujarati(c) {
        Some(ScriptKind::Gujarati)
    } else if is_gurmukhi(c) {
        Some(ScriptKind::Gurmukhi)
    } else if is_hebrew(c) {
        Some(ScriptKind::Hebrew)
    } else if is_khmer(c) {
        Some(ScriptKind::Khmer)
    } else if is_lao(c) {
        Some(ScriptKind::Lao)
    } else if is_myanmar(c) {
        Some(ScriptKind::Myanmar)
    } else if is_sinhala(c) {
        Some(ScriptKind::Sinhala)
    } else if is_tamil(c) {
        Some(ScriptKind::Tamil)
    } else if is_telugu(c) {
        Some(ScriptKind::Telugu)
    } else if is_thai(c) {
        Some(ScriptKind::Thai)
    } else {
        None
    }
}

fn title_scripts(title: &str) -> HashSet<ScriptKind> {
    title.chars().filter_map(script_kind).collect()
}

fn locale_language_code(language: &str) -> &str {
    language
        .trim()
        .split(['-', '_'])
        .next()
        .unwrap_or_default()
        .trim()
}

fn locale_script_subtag(language: &str) -> Option<String> {
    language
        .trim()
        .split(['-', '_'])
        .find(|subtag| subtag.len() == 4 && subtag.chars().all(|c| c.is_ascii_alphabetic()))
        .map(|subtag| subtag.to_ascii_lowercase())
}

fn locale_region_subtag(language: &str) -> Option<String> {
    language
        .trim()
        .split(['-', '_'])
        .skip(1)
        .find(|subtag| {
            (subtag.len() == 2 && subtag.chars().all(|c| c.is_ascii_alphabetic()))
                || (subtag.len() == 3 && subtag.chars().all(|c| c.is_ascii_digit()))
        })
        .map(|subtag| subtag.to_ascii_lowercase())
}

fn target_script_from_locale(language: &str) -> TargetScript {
    if let Some(script) = locale_script_subtag(language) {
        return match script.as_str() {
            "hans" => TargetScript::Han(HanVariant::Simplified),
            "hant" => TargetScript::Han(HanVariant::Traditional),
            "hani" => TargetScript::Han(HanVariant::Any),
            "jpan" | "kana" => TargetScript::Japanese,
            "hang" => TargetScript::Primary(ScriptKind::Hangul),
            "latn" => TargetScript::Primary(ScriptKind::Latin),
            "cyrl" => TargetScript::Primary(ScriptKind::Cyrillic),
            "arab" => TargetScript::Primary(ScriptKind::Arabic),
            "armn" => TargetScript::Primary(ScriptKind::Armenian),
            "beng" => TargetScript::Primary(ScriptKind::Bengali),
            "deva" => TargetScript::Primary(ScriptKind::Devanagari),
            "ethi" => TargetScript::Primary(ScriptKind::Ethiopic),
            "geor" => TargetScript::Primary(ScriptKind::Georgian),
            "grek" => TargetScript::Primary(ScriptKind::Greek),
            "gujr" => TargetScript::Primary(ScriptKind::Gujarati),
            "guru" => TargetScript::Primary(ScriptKind::Gurmukhi),
            "hebr" => TargetScript::Primary(ScriptKind::Hebrew),
            "khmr" => TargetScript::Primary(ScriptKind::Khmer),
            "laoo" => TargetScript::Primary(ScriptKind::Lao),
            "mymr" => TargetScript::Primary(ScriptKind::Myanmar),
            "sinh" => TargetScript::Primary(ScriptKind::Sinhala),
            "taml" => TargetScript::Primary(ScriptKind::Tamil),
            "telu" => TargetScript::Primary(ScriptKind::Telugu),
            "thai" => TargetScript::Primary(ScriptKind::Thai),
            _ => TargetScript::Any,
        };
    }

    let code = locale_language_code(language).to_ascii_lowercase();
    match code.as_str() {
        "zh" => match locale_region_subtag(language).as_deref() {
            Some("cn" | "sg" | "my") => TargetScript::Han(HanVariant::Simplified),
            Some("tw" | "hk" | "mo") => TargetScript::Han(HanVariant::Traditional),
            _ => TargetScript::Han(HanVariant::Any),
        },
        "ja" => TargetScript::Japanese,
        "ko" => TargetScript::Primary(ScriptKind::Hangul),
        "ru" | "uk" | "be" | "bg" | "kk" | "mk" | "mn" | "sr" => {
            TargetScript::Primary(ScriptKind::Cyrillic)
        }
        "ar" | "fa" | "ur" | "ps" => TargetScript::Primary(ScriptKind::Arabic),
        "hy" => TargetScript::Primary(ScriptKind::Armenian),
        "bn" => TargetScript::Primary(ScriptKind::Bengali),
        "hi" | "mr" | "ne" => TargetScript::Primary(ScriptKind::Devanagari),
        "am" => TargetScript::Primary(ScriptKind::Ethiopic),
        "ka" => TargetScript::Primary(ScriptKind::Georgian),
        "el" => TargetScript::Primary(ScriptKind::Greek),
        "gu" => TargetScript::Primary(ScriptKind::Gujarati),
        "pa" => TargetScript::Primary(ScriptKind::Gurmukhi),
        "he" | "yi" => TargetScript::Primary(ScriptKind::Hebrew),
        "km" => TargetScript::Primary(ScriptKind::Khmer),
        "lo" => TargetScript::Primary(ScriptKind::Lao),
        "my" => TargetScript::Primary(ScriptKind::Myanmar),
        "si" => TargetScript::Primary(ScriptKind::Sinhala),
        "ta" => TargetScript::Primary(ScriptKind::Tamil),
        "te" => TargetScript::Primary(ScriptKind::Telugu),
        "th" => TargetScript::Primary(ScriptKind::Thai),
        _ if is_known_latin_language(&code) => TargetScript::Primary(ScriptKind::Latin),
        _ => TargetScript::Any,
    }
}

fn is_known_latin_language(code: &str) -> bool {
    matches!(
        code,
        "af" | "sq"
            | "az"
            | "eu"
            | "bs"
            | "ca"
            | "hr"
            | "cs"
            | "da"
            | "nl"
            | "en"
            | "et"
            | "fi"
            | "fr"
            | "de"
            | "hu"
            | "is"
            | "id"
            | "ga"
            | "it"
            | "lv"
            | "lt"
            | "ms"
            | "nb"
            | "nn"
            | "pl"
            | "pt"
            | "ro"
            | "sk"
            | "sl"
            | "es"
            | "sv"
            | "sw"
            | "tl"
            | "tr"
            | "vi"
            | "cy"
            | "xh"
            | "yo"
            | "zu"
            | "eo"
            | "la"
            | "mi"
            | "so"
            | "st"
            | "tn"
            | "ts"
            | "sn"
            | "lg"
    )
}

fn target_locale_profile(language: &str) -> TargetLocaleProfile {
    TargetLocaleProfile {
        script: target_script_from_locale(language),
        lingua: target_lingua_language(language),
    }
}

pub(super) fn title_translation_writing_system_guidance(language: &str) -> String {
    match target_locale_profile(language).script {
        TargetScript::Han(HanVariant::Simplified) => "Use Simplified Chinese Han characters for translated wording and names; do not output Japanese kana, Hangul, Cyrillic, Arabic, or other incompatible script letters.".to_string(),
        TargetScript::Han(HanVariant::Traditional) => "Use Traditional Chinese Han characters for translated wording and names; do not output Japanese kana, Hangul, Cyrillic, Arabic, or other incompatible script letters.".to_string(),
        TargetScript::Han(HanVariant::Any) => "Use the requested Chinese Han variant for translated wording and names; convert source-script names when needed and do not retain incompatible source letters merely because they belong to a name.".to_string(),
        TargetScript::Japanese => "Use Japanese Han and Kana writing for translated wording and names; convert incompatible source-script text when needed and preserve only conventional or opaque source text.".to_string(),
        TargetScript::Primary(script) => format!(
            "Use the target locale's normal {} writing system for translated wording and names; convert source-script names when needed and preserve identity rather than source spelling. Keep other scripts only for opaque identifiers or conventional target-locale names.",
            script_name(script)
        ),
        TargetScript::Any => "Use the target locale's normal writing system for translated wording and names; when source and target scripts differ, convert names while preserving identity rather than source spelling.".to_string(),
    }
}

fn script_name(script: ScriptKind) -> &'static str {
    match script {
        ScriptKind::Arabic => "Arabic",
        ScriptKind::Armenian => "Armenian",
        ScriptKind::Bengali => "Bengali",
        ScriptKind::Cyrillic => "Cyrillic",
        ScriptKind::Devanagari => "Devanagari",
        ScriptKind::Ethiopic => "Ethiopic",
        ScriptKind::Georgian => "Georgian",
        ScriptKind::Greek => "Greek",
        ScriptKind::Gujarati => "Gujarati",
        ScriptKind::Gurmukhi => "Gurmukhi",
        ScriptKind::Han => "Han",
        ScriptKind::Hangul => "Hangul",
        ScriptKind::Hebrew => "Hebrew",
        ScriptKind::Kana => "Kana",
        ScriptKind::Khmer => "Khmer",
        ScriptKind::Lao => "Lao",
        ScriptKind::Latin => "Latin",
        ScriptKind::Myanmar => "Myanmar",
        ScriptKind::Sinhala => "Sinhala",
        ScriptKind::Tamil => "Tamil",
        ScriptKind::Telugu => "Telugu",
        ScriptKind::Thai => "Thai",
    }
}

pub(super) fn validate_settings(settings: &AISettings) -> Result<()> {
    if settings.profiles.is_empty() {
        return Err(anyhow!("At least one AI profile is required"));
    }
    let mut ids = HashSet::new();
    for profile in &settings.profiles {
        if profile.id.trim().is_empty() || !ids.insert(profile.id.as_str()) {
            return Err(anyhow!("AI profile IDs must be non-empty and unique"));
        }
        if profile.name.trim().is_empty() {
            return Err(anyhow!("AI profile name must not be empty"));
        }
        if !matches!(
            profile.connection.provider.as_str(),
            "openaiCompatible" | "ollama"
        ) {
            return Err(anyhow!("AI provider must be openaiCompatible or ollama"));
        }
        chat_endpoint_for_connection(&profile.connection)?;
        if profile.connection.model.trim().is_empty() {
            return Err(anyhow!("AI model must not be empty"));
        }
        if !(1..=profile.connection.timeout_seconds)
            .contains(&profile.connection.first_token_timeout_seconds)
        {
            return Err(anyhow!(
                "AI profile `{}` firstTokenTimeoutSeconds must be between 1 and its timeoutSeconds",
                profile.name
            ));
        }
        if !(5..=3_600).contains(&profile.connection.timeout_seconds) {
            return Err(anyhow!(
                "AI profile `{}` timeoutSeconds must be between 5 and 3600",
                profile.name
            ));
        }
        if profile.connection.request_interval_seconds > 3_600 {
            return Err(anyhow!(
                "AI profile `{}` requestIntervalSeconds must be between 0 and 3600",
                profile.name
            ));
        }
        if profile.connection.provider == "ollama"
            && !(256..=1_048_576).contains(&profile.connection.ollama_max_num_ctx)
        {
            return Err(anyhow!(
                "AI profile `{}` ollamaMaxNumCtx must be between 256 and 1048576",
                profile.name
            ));
        }
        if profile.connection.provider == "ollama"
            && profile.connection.vision_capable
            && profile.connection.ollama_max_num_ctx < 16_384
        {
            return Err(anyhow!(
                "AI vision profile `{}` requires ollamaMaxNumCtx of at least 16384",
                profile.name
            ));
        }
        if !(1_024..=1_048_576).contains(&profile.connection.context_window_tokens) {
            return Err(anyhow!(
                "AI profile `{}` contextWindowTokens must be between 1024 and 1048576",
                profile.name
            ));
        }
        if profile.connection.vision_capable && profile.connection.context_window_tokens < 16_384 {
            return Err(anyhow!(
                "AI vision profile `{}` requires contextWindowTokens of at least 16384",
                profile.name
            ));
        }
        if !profile.connection.ollama_repeat_penalty.is_finite()
            || !(0.0..=2.0).contains(&profile.connection.ollama_repeat_penalty)
        {
            return Err(anyhow!(
                "AI profile `{}` ollamaRepeatPenalty must be between 0 and 2",
                profile.name
            ));
        }
        if profile.connection.ollama_repeat_last_n > 32_768 {
            return Err(anyhow!(
                "AI profile `{}` ollamaRepeatLastN must be between 0 and 32768",
                profile.name
            ));
        }
    }
    if !settings
        .profiles
        .iter()
        .any(|profile| profile.id == settings.active_profile_id && profile.enabled)
    {
        return Err(anyhow!("Active AI profile must exist and be enabled"));
    }
    if settings
        .features
        .title_translation
        .target_language
        .trim()
        .is_empty()
    {
        return Err(anyhow!(
            "Title translation target language must not be empty"
        ));
    }
    let title_translation = &settings.features.title_translation;
    if !title_translation.temperature.is_finite()
        || !(0.0..=2.0).contains(&title_translation.temperature)
    {
        return Err(anyhow!(
            "Title translation temperature must be between 0 and 2"
        ));
    }
    if !title_translation.ollama_repeat_penalty.is_finite()
        || !(0.0..=2.0).contains(&title_translation.ollama_repeat_penalty)
    {
        return Err(anyhow!(
            "Title translation ollamaRepeatPenalty must be between 0 and 2"
        ));
    }
    if title_translation.ollama_repeat_last_n > 32_768 {
        return Err(anyhow!(
            "Title translation ollamaRepeatLastN must be between 0 and 32768"
        ));
    }
    if !matches!(
        title_translation.structured_output_mode.as_str(),
        "jsonSchema" | "jsonObject" | "promptOnly"
    ) {
        return Err(anyhow!(
            "Title translation structuredOutputMode must be jsonSchema, jsonObject, or promptOnly"
        ));
    }
    for lane in crate::models::AI_EXECUTOR_LANES {
        let limit = settings
            .execution
            .lanes
            .limit_for_lane(lane)
            .expect("known executor lane has a configured limit");
        if limit == 0 || limit > MAX_AI_WORKERS_PER_LANE {
            return Err(anyhow!(
                "AI execution lane `{lane}` maxConcurrentJobs must be between 1 and {MAX_AI_WORKERS_PER_LANE}"
            ));
        }
    }
    if !(5..=1_800).contains(&settings.execution.timeout_seconds) {
        return Err(anyhow!("AI timeoutSeconds must be between 5 and 1800"));
    }
    let execution = &settings.execution;
    if !(1..=64).contains(&execution.max_images_per_task)
        || !(256..=32_768).contains(&execution.image_token_budget)
        || !(128..=32_768).contains(&execution.output_token_limit)
        || !(128..=32_768).contains(&execution.thinking_output_token_limit)
        || !(0..=16_384).contains(&execution.prompt_safety_margin)
        || execution.adaptive_context_retries > 5
        || !(1..=64).contains(&execution.ocr_max_pages)
        || !(100..=20_000).contains(&execution.ocr_chars_per_page)
    {
        return Err(anyhow!(
            "AI task prompt budget settings are outside their supported ranges"
        ));
    }
    if !matches!(
        settings.features.auto_tagging.mode.as_str(),
        "suggestions" | "autoApplyReliable"
    ) {
        return Err(anyhow!(
            "AI auto-tagging mode must be suggestions or autoApplyReliable"
        ));
    }
    if !(30..=730).contains(
        &settings
            .features
            .recommendations
            .analysis_refresh_after_days,
    ) {
        return Err(anyhow!(
            "Recommendation analysisRefreshAfterDays must be between 30 and 730"
        ));
    }
    validate_task_execution_settings(
        settings,
        "title localization",
        &settings.features.title_translation.execution,
        false,
        true,
    )?;
    validate_task_execution_settings(
        settings,
        "tag localization",
        &settings.features.tag_localization.execution,
        false,
        false,
    )?;
    validate_task_execution_settings(
        settings,
        "content understanding",
        &settings.features.content_understanding.execution,
        true,
        false,
    )?;
    validate_task_execution_settings(
        settings,
        "tag generation",
        &settings.features.auto_tagging.execution,
        true,
        false,
    )?;
    Ok(())
}

fn validate_task_execution_settings(
    settings: &AISettings,
    task_name: &str,
    execution: &AITaskExecutionSettings,
    supports_images: bool,
    supports_json_schema: bool,
) -> Result<()> {
    let profile_id = execution.profile_id.trim();
    if profile_id.is_empty() || profile_id == "auto" {
        // `auto` uses the active compatible profile and remains the zero-configuration default.
    } else if !settings
        .profiles
        .iter()
        .any(|profile| profile.id == profile_id && profile.enabled)
    {
        return Err(anyhow!(
            "AI task `{task_name}` profileId must be `auto` or an enabled profile"
        ));
    }
    if !matches!(
        execution.thinking_mode.as_str(),
        "inherit" | "disabled" | "enabled"
    ) {
        return Err(anyhow!(
            "AI task `{task_name}` thinkingMode must be inherit, disabled, or enabled"
        ));
    }
    if execution
        .output_token_limit
        .is_some_and(|limit| !(32..=32_768).contains(&limit))
    {
        return Err(anyhow!(
            "AI task `{task_name}` outputTokenLimit must be between 32 and 32768"
        ));
    }
    if execution
        .thinking_output_token_limit
        .is_some_and(|limit| !(32..=32_768).contains(&limit))
    {
        return Err(anyhow!(
            "AI task `{task_name}` thinkingOutputTokenLimit must be between 32 and 32768"
        ));
    }
    if execution
        .thinking_context_window_tokens
        .is_some_and(|limit| !(16_384..=1_048_576).contains(&limit))
    {
        return Err(anyhow!(
            "AI task `{task_name}` thinkingContextWindowTokens must be between 16384 and 1048576"
        ));
    }
    if !execution.temperature.is_finite() || !(0.0..=2.0).contains(&execution.temperature) {
        return Err(anyhow!(
            "AI task `{task_name}` temperature must be between 0 and 2"
        ));
    }
    if !matches!(
        execution.structured_output_mode.as_str(),
        "jsonObject" | "promptOnly"
    ) && !(supports_json_schema && execution.structured_output_mode == "jsonSchema")
    {
        return Err(anyhow!(
            "AI task `{task_name}` structuredOutputMode is not supported"
        ));
    }
    if execution
        .max_images_per_request
        .is_some_and(|limit| !(1..=64).contains(&limit))
    {
        return Err(anyhow!(
            "AI task `{task_name}` maxImagesPerRequest must be between 1 and 64"
        ));
    }
    if execution
        .max_images_per_request
        .is_some_and(|limit| limit > settings.execution.max_images_per_task)
    {
        return Err(anyhow!(
            "AI task `{task_name}` maxImagesPerRequest cannot exceed the global maxImagesPerTask"
        ));
    }
    if !supports_images && execution.max_images_per_request.is_some() {
        return Err(anyhow!(
            "AI task `{task_name}` does not accept maxImagesPerRequest"
        ));
    }
    if execution
        .timeout_seconds
        .is_some_and(|timeout| !(5..=3_600).contains(&timeout))
    {
        return Err(anyhow!(
            "AI task `{task_name}` timeoutSeconds must be between 5 and 3600"
        ));
    }
    if execution.additional_instructions.chars().count() > 2_000 {
        return Err(anyhow!(
            "AI task `{task_name}` additionalInstructions must be at most 2000 characters"
        ));
    }
    Ok(())
}

pub fn title_hash(title: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(title.trim().as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(super) fn classify_title_language_locally(
    title: &str,
    language: &str,
) -> TitleLanguageDecision {
    let profile = target_locale_profile(language);
    if let Some(matches_target) = title_script_matches_target_language(title, language) {
        return if matches_target {
            TitleLanguageDecision::Target
        } else {
            TitleLanguageDecision::NonTarget
        };
    }
    if title.chars().any(is_han)
        && matches!(
            profile.script,
            TargetScript::Han(_) | TargetScript::Japanese
        )
    {
        let han_decision = classify_han_title_by_locale(title, profile.script);
        if han_decision != TitleLanguageDecision::Ambiguous {
            return han_decision;
        }
        // Never let a broad statistical model turn shared Han characters into a deterministic
        // language decision. A Chinese/Japanese Han-only title must use the LLM fallback.
        return TitleLanguageDecision::Ambiguous;
    }

    let Some(target) = profile.lingua else {
        // A locale without a bundled local model must be confirmed by the LLM detector. A
        // missing model is not evidence that the title is or is not in the target language.
        return TitleLanguageDecision::Ambiguous;
    };
    let scores = TITLE_LANGUAGE_DETECTOR.compute_language_confidence_values(title);
    let Some((top_language, top_confidence)) = scores.first() else {
        return TitleLanguageDecision::Ambiguous;
    };
    let second_confidence = scores
        .get(1)
        .map(|(_, confidence)| *confidence)
        .unwrap_or_default();
    if *top_language == target
        && *top_confidence >= TITLE_LANGUAGE_CONFIDENCE_THRESHOLD
        && *top_confidence - second_confidence >= 0.20
    {
        TitleLanguageDecision::Target
    } else if *top_language != target
        && *top_confidence >= TITLE_LANGUAGE_NON_TARGET_CONFIDENCE_THRESHOLD
        && *top_confidence - second_confidence >= 0.20
    {
        TitleLanguageDecision::NonTarget
    } else {
        TitleLanguageDecision::Ambiguous
    }
}

pub(super) fn title_looks_like_target_language(title: &str, language: &str) -> bool {
    classify_title_language_locally(title, language) == TitleLanguageDecision::Target
}

fn classify_han_title_by_locale(title: &str, target_script: TargetScript) -> TitleLanguageDecision {
    // These are common mainland simplified forms whose traditional/Japanese counterparts are
    // different. The list is character-based, so it applies to unseen title vocabulary too.
    const SIMPLIFIED_CHINESE: &str = "爱碍坝办帮备贝笔毕边编变标仓层彻尘衬惩迟齿冲处传创词从达带单导敌递电调顶东冻队对吨夺儿尔饭贩飞废费纷风复该赶个归轨过还汉华欢环换汇动发间问闻积极际继夹价坚检简见讲奖节结经惊竞剧开宽矿类离丽历厉励连联炼练两辆疗辽猎临邻灵龄刘龙楼录陆乱论罗马买卖麦满门梦灭难鸟农气弃启签迁钱桥乔亲轻庆权让热认荣赛伤设胜师实识试视树谁说丝岁孙缩锁谈讨铁听图团为卫稳无戏县线显现乡响项协谢兴续选压盐阳药译艺阴隐应营语预圆员远愿阅云杂赞张这证织纸质专转庄准总纵组蓝游舰岛计划辑档册页鉴话绝长场务种广厂术园后书车级";
    // These forms distinguish Traditional Chinese from the Japanese shinjitai forms. Shared
    // Han characters are intentionally omitted because they cannot identify a locale safely.
    const TRADITIONAL_CHINESE_ONLY: &str = "體變參稱齒傳從單奪墮兒廢趕歸號歡匯發繼價檢獎經據礦歷厲勵聯兩獵鄰靈齡劉樓錄亂買賣麥滿氣棄啟錢輕區權讓榮聲實說絲歲鐵聽圖團穩縣顯鄉續壓鹽藥譯藝隱應營圓閱雜贊這證專轉莊狀總縱廣廠";
    // Traditional and Japanese forms are useful as non-target evidence for a zh-CN target. This
    // is also character-based; it does not encode title content or sensitive vocabulary.
    const NON_SIMPLIFIED_CJK: &str = "亞惡壓壞邊標變參層徹塵稱懲遲齒衝處傳創詞從達帶單導燈敵遞點電調頂東凍隊對奪墮兒飯販飛廢費風復該趕個歸軌過還漢號華歡環換匯動發間問聞積極際繼夾價堅檢簡見將講獎節結經驚競劇據開寬礦類離麗歷厲勵連聯煉練兩輛療遼獵臨鄰靈齡劉龍樓錄陸亂論羅馬買賣麥滿門夢滅難鳥農氣棄啟簽遷錢橋喬親輕慶區權讓熱認榮賽傷設聲勝師實識試視樹誰說絲歲孫縮鎖談討鐵聽圖團為衛穩無戲縣線顯現鄉響項協謝興續選壓鹽陽藥譯藝陰隱應營語預圓員遠願閱雲雜贊張這證織紙質專轉莊狀準總縱組場務種廣廠術園後書車級絶変伝図読売発薬訳覧竜両辺帰気広駅県帯単戦長話語巻";
    let simplified_characters: HashSet<char> = title
        .chars()
        .filter(|character| SIMPLIFIED_CHINESE.contains(*character))
        .collect();
    let simplified_score = simplified_characters.len();
    let non_simplified_characters: HashSet<char> = title
        .chars()
        .filter(|character| NON_SIMPLIFIED_CJK.contains(*character))
        .collect();
    let non_simplified_score = non_simplified_characters.len();
    let traditional_characters: HashSet<char> = title
        .chars()
        .filter(|character| TRADITIONAL_CHINESE_ONLY.contains(*character))
        .collect();
    match target_script {
        TargetScript::Han(HanVariant::Simplified) => {
            if simplified_score >= 2 && non_simplified_score == 0 {
                TitleLanguageDecision::Target
            } else if non_simplified_score >= 2 && simplified_score == 0 {
                TitleLanguageDecision::NonTarget
            } else {
                TitleLanguageDecision::Ambiguous
            }
        }
        TargetScript::Han(HanVariant::Any) => {
            if simplified_score >= 2 && non_simplified_score == 0 {
                TitleLanguageDecision::Target
            } else if traditional_characters.len() >= 2 && simplified_score == 0 {
                TitleLanguageDecision::Target
            } else if non_simplified_score >= 2
                && simplified_score == 0
                && traditional_characters.is_empty()
            {
                TitleLanguageDecision::NonTarget
            } else {
                TitleLanguageDecision::Ambiguous
            }
        }
        TargetScript::Han(HanVariant::Traditional) => {
            if traditional_characters.len() >= 2 && simplified_score == 0 {
                TitleLanguageDecision::Target
            } else if simplified_score >= 2 && traditional_characters.is_empty() {
                TitleLanguageDecision::NonTarget
            } else {
                TitleLanguageDecision::Ambiguous
            }
        }
        _ => TitleLanguageDecision::Ambiguous,
    }
}

pub(super) fn local_title_language_decision_source(title: &str, language: &str) -> &'static str {
    let profile = target_locale_profile(language);
    if title_script_matches_target_language_for_profile(title, profile).is_some() {
        "unicode_script"
    } else if title.chars().any(is_han)
        && matches!(
            profile.script,
            TargetScript::Han(_) | TargetScript::Japanese
        )
    {
        "han_orthography"
    } else if profile.lingua.is_some() {
        "lingua"
    } else {
        "unsupported_locale"
    }
}

fn translation_writing_system_issue(
    source: &str,
    translated: &str,
    target: &str,
) -> Option<String> {
    let profile = target_locale_profile(target);
    let scripts = title_scripts(translated);
    let has_letters = source.chars().any(char::is_alphabetic);
    let (allowed, required) = match profile.script {
        TargetScript::Any => return None,
        TargetScript::Han(_) => (
            &[ScriptKind::Han, ScriptKind::Latin][..],
            Some(ScriptKind::Han),
        ),
        TargetScript::Japanese => (
            &[ScriptKind::Han, ScriptKind::Kana, ScriptKind::Latin][..],
            None,
        ),
        TargetScript::Primary(script) => {
            if script == ScriptKind::Latin {
                (&[ScriptKind::Latin][..], Some(script))
            } else {
                (&[script, ScriptKind::Latin][..], Some(script))
            }
        }
    };
    if scripts.iter().any(|script| !allowed.contains(script)) {
        return Some("the result contains an incompatible writing system".to_string());
    }
    let has_required_script = match profile.script {
        TargetScript::Japanese => {
            scripts.contains(&ScriptKind::Han) || scripts.contains(&ScriptKind::Kana)
        }
        _ => required.is_some_and(|script| scripts.contains(&script)),
    };
    if has_letters && !has_required_script {
        return Some("the result contains no target-script letters".to_string());
    }
    None
}

// Script detection provides deterministic answers for writing systems that are incompatible or
// uniquely identify a language. Shared scripts remain undecided and go through Lingua or LLM.
pub(super) fn title_script_matches_target_language(title: &str, language: &str) -> Option<bool> {
    title_script_matches_target_language_for_profile(title, target_locale_profile(language))
}

fn title_script_matches_target_language_for_profile(
    title: &str,
    profile: TargetLocaleProfile,
) -> Option<bool> {
    let has_letters = title.chars().any(char::is_alphabetic);
    if !has_letters {
        return None;
    }

    let scripts = title_scripts(title);
    if scripts.is_empty() {
        return None;
    }
    match profile.script {
        TargetScript::Any => None,
        TargetScript::Han(_) => {
            if scripts
                .iter()
                .any(|script| !matches!(script, ScriptKind::Han | ScriptKind::Latin))
            {
                Some(false)
            } else {
                // Han-only and Han/Latin titles are ambiguous. Chinese and Japanese share Han,
                // and Latin can be a title abbreviation or a transliterated proper name.
                None
            }
        }
        TargetScript::Japanese => {
            if scripts.iter().any(|script| {
                !matches!(
                    script,
                    ScriptKind::Han | ScriptKind::Kana | ScriptKind::Latin
                )
            }) {
                Some(false)
            } else if scripts.contains(&ScriptKind::Kana) {
                Some(true)
            } else if scripts.contains(&ScriptKind::Han) {
                None
            } else {
                Some(false)
            }
        }
        TargetScript::Primary(primary) => {
            let has_primary = scripts.contains(&primary);
            let has_incompatible = scripts.iter().any(|script| {
                *script != primary && (*script != ScriptKind::Latin || primary == ScriptKind::Latin)
            });
            if has_incompatible {
                return Some(false);
            }
            if has_primary && is_language_identifying_script(primary, profile.lingua) {
                Some(true)
            } else if !has_primary {
                Some(false)
            } else {
                None
            }
        }
    }
}

fn target_lingua_language(language: &str) -> Option<Language> {
    match locale_language_code(language).to_ascii_lowercase().as_str() {
        "zh" => Some(Language::Chinese),
        "ja" => Some(Language::Japanese),
        "ko" => Some(Language::Korean),
        "en" => Some(Language::English),
        "fr" => Some(Language::French),
        "de" => Some(Language::German),
        "es" => Some(Language::Spanish),
        "pt" => Some(Language::Portuguese),
        "it" => Some(Language::Italian),
        "ru" => Some(Language::Russian),
        "uk" => Some(Language::Ukrainian),
        _ => None,
    }
}

fn is_language_identifying_script(script: ScriptKind, lingua: Option<Language>) -> bool {
    // A writing system can be unique while still being shared by multiple languages. Only
    // Korean Hangul and Japanese Kana are deterministic here; all other shared-script targets
    // need lexical evidence from Lingua or the LLM detector.
    matches!(
        (script, lingua),
        (ScriptKind::Hangul, Some(Language::Korean))
    )
}
