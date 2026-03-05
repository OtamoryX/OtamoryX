use std::collections::HashSet;
use std::path::Path;

use crate::plugins::{
    BuiltinPlugin, BuiltinPluginKind, BuiltinPluginResult, PluginContext, PluginOutput,
    TagProposal, BUILTIN_FILENAME_PARSER_ID, BUILTIN_METADATA_ORDER_FILENAME,
};

#[derive(Debug, Default)]
pub struct FilenameParser;

impl BuiltinPlugin for FilenameParser {
    fn id(&self) -> &'static str {
        BUILTIN_FILENAME_PARSER_ID
    }

    fn kind(&self) -> BuiltinPluginKind {
        BuiltinPluginKind::MetadataPipeline
    }

    fn order(&self) -> Option<u16> {
        Some(BUILTIN_METADATA_ORDER_FILENAME)
    }

    fn run(&self, ctx: &PluginContext) -> BuiltinPluginResult {
        let mut out = PluginOutput::default();
        let raw_stem = Path::new(&ctx.archive_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if raw_stem.trim().is_empty() {
            out.notes
                .push("filename-parser skipped: empty filename stem".to_string());
            return Ok(out);
        }

        let parsed = parse_filename(raw_stem);
        if !parsed.title.is_empty() {
            out.metadata.title = Some(parsed.title);
        }

        let mut dedupe = HashSet::new();
        for tag in parsed.tags {
            let key = format!(
                "{}\u{0}{}",
                tag.namespace.to_ascii_lowercase(),
                tag.value.to_ascii_lowercase()
            );
            if dedupe.insert(key) {
                out.tags.push(tag);
            }
        }

        Ok(out)
    }
}

#[derive(Debug, Default)]
struct ParsedFilename {
    title: String,
    tags: Vec<TagProposal>,
}

fn parse_filename(stem: &str) -> ParsedFilename {
    let normalized_stem = normalize_filename_component(stem);
    let (title_without_brackets, tokens) = extract_bracket_tokens(stem);
    let normalized_title = normalize_filename_component(&title_without_brackets);
    let title = if normalized_title.is_empty() {
        if tokens.is_empty() {
            normalized_stem.clone()
        } else {
            tokens.join(" ")
        }
    } else {
        normalized_title
    };

    let mut tags = tokens
        .into_iter()
        .map(|token| TagProposal::heuristic("filename_token", token, BUILTIN_FILENAME_PARSER_ID))
        .collect::<Vec<_>>();

    if let Some(language) = detect_language(&normalized_stem, &tags) {
        tags.push(TagProposal::deterministic(
            "language",
            language,
            BUILTIN_FILENAME_PARSER_ID,
        ));
    }
    if let Some(volume) = detect_volume(&normalized_stem) {
        tags.push(TagProposal::heuristic(
            "volume",
            volume,
            BUILTIN_FILENAME_PARSER_ID,
        ));
    }
    if let Some(chapter) = detect_chapter(&normalized_stem) {
        tags.push(TagProposal::heuristic(
            "chapter",
            chapter,
            BUILTIN_FILENAME_PARSER_ID,
        ));
    }
    if let Some(year) = detect_year(&normalized_stem) {
        tags.push(TagProposal::heuristic(
            "year",
            year,
            BUILTIN_FILENAME_PARSER_ID,
        ));
    }

    ParsedFilename { title, tags }
}

fn extract_bracket_tokens(input: &str) -> (String, Vec<String>) {
    let mut stripped = String::with_capacity(input.len());
    let mut tokens = Vec::new();
    let chars = input.chars().collect::<Vec<_>>();
    let mut idx = 0usize;

    while idx < chars.len() {
        let ch = chars[idx];
        let Some(closer) = bracket_closer(ch) else {
            stripped.push(ch);
            idx += 1;
            continue;
        };

        let mut end = idx + 1;
        while end < chars.len() && chars[end] != closer {
            end += 1;
        }

        if end >= chars.len() {
            stripped.push(ch);
            idx += 1;
            continue;
        }

        let token = chars[(idx + 1)..end].iter().collect::<String>();
        let token = normalize_filename_component(&token);
        if !token.is_empty() {
            tokens.push(token);
        }
        stripped.push(' ');
        idx = end + 1;
    }

    (stripped, dedupe_strings_case_insensitive(tokens))
}

fn bracket_closer(ch: char) -> Option<char> {
    match ch {
        '[' => Some(']'),
        '(' => Some(')'),
        '【' => Some('】'),
        '（' => Some('）'),
        '{' => Some('}'),
        _ => None,
    }
}

fn normalize_filename_component(input: &str) -> String {
    let normalized = input
        .chars()
        .map(|ch| if matches!(ch, '_' | '.') { ' ' } else { ch })
        .collect::<String>();

    normalized
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|ch: char| ch == '-' || ch == '#' || ch.is_whitespace())
        .to_string()
}

fn dedupe_strings_case_insensitive(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for value in values {
        let key = value.to_ascii_lowercase();
        if seen.insert(key) {
            deduped.push(value);
        }
    }
    deduped
}

fn detect_language(normalized_stem: &str, token_tags: &[TagProposal]) -> Option<&'static str> {
    let token_values = token_tags
        .iter()
        .map(|tag| tag.value.to_ascii_lowercase())
        .collect::<Vec<_>>();

    let stem_lower = normalized_stem.to_ascii_lowercase();
    if has_language_marker(
        &stem_lower,
        &token_values,
        &["zh", "chs", "cht", "中文", "汉化"],
    ) {
        return Some("zh");
    }
    if has_language_marker(
        &stem_lower,
        &token_values,
        &["en", "eng", "english", "英文"],
    ) {
        return Some("en");
    }
    if has_language_marker(
        &stem_lower,
        &token_values,
        &["jp", "jpn", "japanese", "日文", "日本語"],
    ) {
        return Some("ja");
    }

    None
}

fn has_language_marker(stem_lower: &str, token_values: &[String], markers: &[&str]) -> bool {
    markers.iter().any(|marker| {
        token_values.iter().any(|token| token == marker) || stem_lower.contains(marker)
    })
}

fn detect_volume(normalized_stem: &str) -> Option<String> {
    let words = split_words(normalized_stem);
    for (idx, word) in words.iter().enumerate() {
        if let Some(number) = word
            .strip_prefix('v')
            .filter(|rest| rest.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty())
        {
            return Some(trim_leading_zeros(number));
        }
        if matches!(word.as_str(), "vol" | "volume" | "v") {
            if let Some(next) = words.get(idx + 1) {
                if next.chars().all(|c| c.is_ascii_digit()) {
                    return Some(trim_leading_zeros(next));
                }
            }
        }
    }
    None
}

fn detect_chapter(normalized_stem: &str) -> Option<String> {
    let words = split_words(normalized_stem);
    for (idx, word) in words.iter().enumerate() {
        if let Some(number) = word
            .strip_prefix("ch")
            .filter(|rest| rest.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty())
        {
            return Some(trim_leading_zeros(number));
        }
        if matches!(word.as_str(), "chapter" | "ch" | "c") {
            if let Some(next) = words.get(idx + 1) {
                if next.chars().all(|c| c.is_ascii_digit()) {
                    return Some(trim_leading_zeros(next));
                }
            }
        }
    }

    if let Some(idx) = normalized_stem.find('#') {
        let digits = normalized_stem[(idx + 1)..]
            .chars()
            .skip_while(|ch| !ch.is_ascii_digit())
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>();
        if !digits.is_empty() {
            return Some(trim_leading_zeros(&digits));
        }
    }
    None
}

fn detect_year(normalized_stem: &str) -> Option<String> {
    let digits_only_words = split_words(normalized_stem)
        .into_iter()
        .filter(|word| word.len() == 4 && word.chars().all(|ch| ch.is_ascii_digit()))
        .collect::<Vec<_>>();

    for year_text in digits_only_words {
        if let Ok(year) = year_text.parse::<u16>() {
            if (1900..=2099).contains(&year) {
                return Some(year.to_string());
            }
        }
    }

    None
}

fn split_words(input: &str) -> Vec<String> {
    input
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect()
}

fn trim_leading_zeros(number: &str) -> String {
    let trimmed = number.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::{BuiltinPlugin, PluginContext};

    fn test_context(path: &str) -> PluginContext {
        PluginContext {
            archive_id: "a1".to_string(),
            archive_path: path.to_string(),
            ingested_at_unix: 1,
            embedded_files: Vec::new(),
            comicinfo_xml: None,
            tag_copier_request: None,
        }
    }

    #[test]
    fn extracts_title_and_tags_from_filename() {
        let parser = FilenameParser;
        let out = parser
            .run(&test_context(
                "/library/[SomeGroup] My.Cool_Title v02 [CHS][Digital].cbz",
            ))
            .expect("filename parser should succeed");

        assert_eq!(out.metadata.title.as_deref(), Some("My Cool Title v02"));
        assert!(out
            .tags
            .iter()
            .any(|t| t.namespace == "filename_token" && t.value == "SomeGroup"));
        assert!(out
            .tags
            .iter()
            .any(|t| t.namespace == "filename_token" && t.value == "CHS"));
        assert!(out
            .tags
            .iter()
            .any(|t| t.namespace == "language" && t.value == "zh"));
        assert!(out
            .tags
            .iter()
            .any(|t| t.namespace == "volume" && t.value == "2"));
    }

    #[test]
    fn falls_back_to_normalized_stem_when_title_body_is_empty() {
        let parser = FilenameParser;
        let out = parser
            .run(&test_context("/library/[CHS][Digital].cbz"))
            .expect("filename parser should succeed");

        assert_eq!(out.metadata.title.as_deref(), Some("CHS Digital"));
    }

    #[test]
    fn deduplicates_repeated_tokens_case_insensitively() {
        let parser = FilenameParser;
        let out = parser
            .run(&test_context("/library/[Digital][digital] Demo.cbz"))
            .expect("filename parser should succeed");

        assert_eq!(
            out.tags
                .iter()
                .filter(|tag| tag.namespace == "filename_token")
                .count(),
            1
        );
    }
}
