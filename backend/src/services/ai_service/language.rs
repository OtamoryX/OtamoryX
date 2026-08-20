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

    let target = target.to_ascii_lowercase();
    let has_letters = source.chars().any(char::is_alphabetic);
    if target.starts_with("zh") {
        if translated.chars().any(is_japanese_kana) {
            return Some("a Chinese result still contains Japanese kana".to_string());
        }
        if translated.chars().any(is_hangul) {
            return Some("a Chinese result still contains Hangul".to_string());
        }
        if translated.chars().any(is_cyrillic) {
            return Some("a Chinese result still contains Cyrillic letters".to_string());
        }
        if has_letters && !translated.chars().any(is_han) {
            return Some("a Chinese result contains no Chinese characters".to_string());
        }
    } else if target.starts_with("ja") {
        if translated.chars().any(is_hangul) {
            return Some("a Japanese result still contains Hangul".to_string());
        }
        if translated.chars().any(is_cyrillic) {
            return Some("a Japanese result still contains Cyrillic letters".to_string());
        }
        if has_letters && !translated.chars().any(|c| is_japanese_kana(c) || is_han(c)) {
            return Some("a Japanese result contains no Japanese writing".to_string());
        }
    } else if target.starts_with("ko") {
        if translated.chars().any(is_japanese_kana) {
            return Some("a Korean result still contains Japanese kana".to_string());
        }
        if translated.chars().any(is_cyrillic) {
            return Some("a Korean result still contains Cyrillic letters".to_string());
        }
        if has_letters && !translated.chars().any(is_hangul) {
            return Some("a Korean result contains no Hangul".to_string());
        }
    } else if is_latin_target(&target) {
        if translated
            .chars()
            .any(|c| is_han(c) || is_japanese_kana(c) || is_hangul(c) || is_cyrillic(c))
        {
            return Some("a Latin-script result still contains another writing system".to_string());
        }
        if has_letters && !translated.chars().any(is_latin) {
            return Some("a Latin-script result contains no Latin letters".to_string());
        }
    } else if is_cyrillic_target(&target) {
        if translated
            .chars()
            .any(|c| is_han(c) || is_japanese_kana(c) || is_hangul(c))
        {
            return Some(
                "a Cyrillic result still contains an East Asian writing system".to_string(),
            );
        }
        if has_letters && !translated.chars().any(is_cyrillic) {
            return Some("a Cyrillic result contains no Cyrillic letters".to_string());
        }
    }
    None
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

fn language_matches(language: &str, prefix: &str) -> bool {
    language == prefix
        || language
            .strip_prefix(prefix)
            .is_some_and(|suffix| suffix.starts_with('-'))
}

fn is_latin_target(language: &str) -> bool {
    [
        "en", "fr", "de", "es", "pt", "it", "nl", "pl", "cs", "sk", "hu", "ro", "tr", "vi", "id",
        "ms", "sv", "no", "da", "fi",
    ]
    .iter()
    .any(|prefix| language_matches(language, prefix))
}

fn is_cyrillic_target(language: &str) -> bool {
    ["ru", "uk", "be", "bg", "mk"]
        .iter()
        .any(|prefix| language_matches(language, prefix))
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
    )?;
    validate_task_execution_settings(
        settings,
        "tag localization",
        &settings.features.tag_localization.execution,
    )?;
    validate_task_execution_settings(
        settings,
        "content understanding",
        &settings.features.content_understanding.execution,
    )?;
    validate_task_execution_settings(
        settings,
        "tag generation",
        &settings.features.auto_tagging.execution,
    )?;
    Ok(())
}

fn validate_task_execution_settings(
    settings: &AISettings,
    task_name: &str,
    execution: &AITaskExecutionSettings,
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
    if let Some(matches_target) = title_script_matches_target_language(title, language) {
        return if matches_target {
            TitleLanguageDecision::Target
        } else {
            TitleLanguageDecision::NonTarget
        };
    }
    if language.to_ascii_lowercase().starts_with("zh") && title.chars().any(is_han) {
        return classify_han_title_as_chinese(title);
    }
    target_lingua_language(language).map_or(TitleLanguageDecision::Ambiguous, |target| {
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
        } else {
            TitleLanguageDecision::NonTarget
        }
    })
}

pub(super) fn title_looks_like_target_language(title: &str, language: &str) -> bool {
    classify_title_language_locally(title, language) == TitleLanguageDecision::Target
}

// High-precision lexical signals for short Han-only titles. These are intentionally narrow: a
// weak or conflicting score is not a classification and is sent to the batch model instead.
fn classify_han_title_as_chinese(title: &str) -> TitleLanguageDecision {
    const JAPANESE: &[&str] = &[
        "絶頂", "妊娠", "監督", "従", "牝", "闘", "姦", "壱", "弐", "話", "巻", "編", "劇場",
        "電車", "悪堕", "無限",
    ];
    const CHINESE: &[&str] = &[
        "老婆",
        "女友",
        "女朋友",
        "原神",
        "记录",
        "合集",
        "小剧场",
        "温泉",
        "罗德岛",
        "舰长",
        "宝可梦",
        "剧情",
        "完整版",
        "福利视频",
        "魔法少女",
        "女教师",
        "老师",
        "因为",
        "所以",
        "为了",
        "与你",
        "我们",
        "没有",
        "成为",
        "不小心",
        "专属",
    ];
    let japanese_score = JAPANESE
        .iter()
        .filter(|marker| title.contains(**marker))
        .count();
    let chinese_score = CHINESE
        .iter()
        .filter(|marker| title.contains(**marker))
        .count();
    if japanese_score >= 1 && chinese_score == 0 {
        TitleLanguageDecision::NonTarget
    } else if chinese_score >= 1 && japanese_score == 0 {
        TitleLanguageDecision::Target
    } else {
        TitleLanguageDecision::Ambiguous
    }
}

pub(super) fn local_title_language_decision_source(title: &str, language: &str) -> &'static str {
    if title_script_matches_target_language(title, language).is_some() {
        "unicode_script"
    } else if language.to_ascii_lowercase().starts_with("zh") && title.chars().any(is_han) {
        "han_lexical"
    } else {
        "lingua"
    }
}

// Script detection provides deterministic answers for writing systems that do not overlap.
// Han-only text deliberately remains undecided because Chinese and Japanese share those chars.
pub(super) fn title_script_matches_target_language(title: &str, language: &str) -> Option<bool> {
    let has_letters = title.chars().any(char::is_alphabetic);
    if !has_letters {
        return None;
    }

    let normalized = language.to_ascii_lowercase();
    let has_kana = title.chars().any(is_japanese_kana);
    let has_hangul = title.chars().any(is_hangul);
    let has_han = title.chars().any(is_han);
    let has_cyrillic = title.chars().any(is_cyrillic);
    let has_latin = title.chars().any(is_latin);

    if normalized.starts_with("zh") {
        return if has_kana || has_hangul || has_cyrillic {
            Some(false)
        } else {
            // Han-only and Han/Latin titles are ambiguous. For example, a Chinese title may
            // include "Vol. 1", while a Japanese title may consist entirely of kanji.
            None
        };
    }
    if normalized.starts_with("ja") {
        return if has_kana {
            Some(true)
        } else if has_hangul || has_cyrillic || has_latin {
            Some(false)
        } else if has_han {
            None
        } else {
            Some(false)
        };
    }
    if normalized.starts_with("ko") {
        return if has_hangul {
            Some(true)
        } else if has_kana || has_han || has_cyrillic || has_latin {
            Some(false)
        } else {
            Some(false)
        };
    }
    if normalized.starts_with("en") {
        return if has_han || has_kana || has_hangul || has_cyrillic {
            Some(false)
        } else if has_latin {
            None
        } else {
            Some(false)
        };
    }
    None
}

fn target_lingua_language(language: &str) -> Option<Language> {
    let normalized = language.to_ascii_lowercase();
    if normalized.starts_with("zh") {
        Some(Language::Chinese)
    } else if normalized.starts_with("ja") {
        Some(Language::Japanese)
    } else if normalized.starts_with("ko") {
        Some(Language::Korean)
    } else if normalized.starts_with("en") {
        Some(Language::English)
    } else {
        None
    }
}
