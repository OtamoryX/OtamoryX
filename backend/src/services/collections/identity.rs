use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct IdentityFact {
    pub(crate) archive_id: String,
    pub(crate) raw_filename: String,
    pub(crate) parent_path: String,
    pub(crate) normalized_key: String,
    pub(crate) display_title: String,
    pub(crate) creator: Option<String>,
    pub(crate) unit_type: String,
    pub(crate) volume_number: Option<String>,
    pub(crate) chapter_number: Option<String>,
    pub(crate) issue_number: Option<String>,
    pub(crate) raw_number: Option<String>,
    pub(crate) edition_marker: Option<String>,
    pub(crate) sort_key: f64,
    pub(crate) confidence: f64,
    pub(crate) evidence: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct ArchiveRow {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) path: String,
}
pub(crate) fn work_group_key(fact: &IdentityFact) -> String {
    let creator_key = fact
        .creator
        .as_deref()
        .map(normalize_text)
        .unwrap_or_default();
    if creator_key.is_empty() {
        fact.normalized_key.clone()
    } else {
        format!("{}::{creator_key}", fact.normalized_key)
    }
}

pub(crate) fn version_group_key(fact: &IdentityFact) -> String {
    content_unit_key(fact)
}

pub(crate) fn content_unit_key(fact: &IdentityFact) -> String {
    let unit_number = fact.raw_number.as_deref().unwrap_or("standalone");
    format!(
        "{}::{}::{unit_number}",
        work_group_key(fact),
        fact.unit_type
    )
}

// A lone title without a number alongside a sequence starting at 2 is commonly
// the omitted first part. Keep this deliberately conservative and mark it for review.
pub(crate) fn infer_missing_first_numbers(facts: &mut [IdentityFact]) {
    let mut groups = HashMap::<String, Vec<usize>>::new();
    for (index, fact) in facts.iter().enumerate() {
        groups.entry(work_group_key(fact)).or_default().push(index);
    }

    for indexes in groups.values() {
        let unnumbered = indexes
            .iter()
            .copied()
            .filter(|index| {
                facts[*index].unit_type == "standalone" && facts[*index].raw_number.is_none()
            })
            .collect::<Vec<_>>();
        if unnumbered.len() != 1 {
            continue;
        }
        let numbered = indexes
            .iter()
            .filter_map(|index| {
                facts[*index]
                    .raw_number
                    .as_deref()
                    .and_then(|number| number.parse::<u32>().ok())
            })
            .collect::<Vec<_>>();
        if numbered.is_empty() || numbered.iter().copied().min() != Some(2) || numbered.contains(&1)
        {
            continue;
        }

        let fact = &mut facts[unnumbered[0]];
        fact.unit_type = "unknown".to_string();
        fact.raw_number = Some("1".to_string());
        fact.sort_key = calculate_sort_key("unknown", None, Some("1"));
        fact.confidence = fact.confidence.min(0.55);
        if let Some(evidence) = fact.evidence.as_object_mut() {
            evidence.insert("inferredNumber".to_string(), json!(1));
            evidence.insert("numberSource".to_string(), json!("inferred_missing_first"));
        }
    }
}
pub(crate) fn parse_identity(archive: &ArchiveRow) -> IdentityFact {
    let (raw_filename, parent_path) = split_path(&archive.path);
    let stem = strip_extension(&raw_filename);
    let (body, tokens) = extract_bracket_tokens(&stem);
    let lower_body = body.to_lowercase();
    let metadata_text = format!("{} {}", body, tokens.join(" "));
    let lower_metadata = metadata_text.to_lowercase();
    let release_tokens = tokens
        .iter()
        .filter(|token| is_release_token(token))
        .cloned()
        .collect::<Vec<_>>();
    let creator = tokens
        .iter()
        .find(|token| !is_release_token(token) && !is_context_token(token))
        .cloned();
    let edition_marker = release_tokens
        .iter()
        .find(|token| is_edition_token(token))
        .cloned();

    let magazine_issue = contains_any(
        &lower_metadata,
        &[
            "comic",
            "コミック",
            "x-eros",
            "ゼロス",
            "快楽天",
            "真激",
            "アンスリウム",
        ],
    );
    let hash_number = find_marker_number(&metadata_text, '#');
    let volume_number = find_word_number(&lower_body, &["volume", "vol", "卷", "巻"]);
    let chapter_number =
        find_word_number(&lower_body, &["chapter", "ch", "episode", "ep", "话", "話"]);
    let part_number = find_word_number(&lower_body, &["part"]);
    let bracket_number = tokens
        .iter()
        .find(|token| is_number(token))
        .and_then(|token| normalize_number(token));
    let trailing_number = trailing_number(&body);
    let terminal_sequence = terminal_sequence_suffix(&body);

    let (unit_type, volume_number, chapter_number, issue_number, raw_number, number_source) =
        if magazine_issue && hash_number.is_some() {
            (
                "issue".to_string(),
                volume_number,
                None,
                hash_number.clone(),
                hash_number,
                "magazine_issue",
            )
        } else if volume_number.is_some() {
            (
                "volume".to_string(),
                volume_number.clone(),
                chapter_number.or(part_number),
                None,
                volume_number,
                "volume_marker",
            )
        } else if chapter_number.is_some() || part_number.is_some() || hash_number.is_some() {
            let number = chapter_number.or(part_number).or(hash_number);
            (
                "chapter".to_string(),
                None,
                number.clone(),
                None,
                number,
                "chapter_marker",
            )
        } else if let Some(sequence) = terminal_sequence.as_ref() {
            (
                sequence.unit_type.to_string(),
                (sequence.unit_type == "volume").then(|| sequence.number.clone()),
                (sequence.unit_type == "chapter").then(|| sequence.number.clone()),
                None,
                Some(sequence.number.clone()),
                sequence.source,
            )
        } else if bracket_number.is_some() || trailing_number.is_some() {
            (
                "unknown".to_string(),
                None,
                None,
                None,
                bracket_number.or(trailing_number),
                "ambiguous_number",
            )
        } else {
            (
                "standalone".to_string(),
                None,
                None,
                None,
                None,
                "no_number",
            )
        };

    let title_body = terminal_sequence
        .as_ref()
        .map(|sequence| sequence.title)
        .unwrap_or(&body);
    let display_title = clean_display_title(title_body);
    let normalized_key = normalize_text(&clean_title_for_key(title_body, &unit_type));
    let creator_key = creator.as_deref().map(normalize_text).unwrap_or_default();
    let confidence = if unit_type == "unknown" {
        0.58
    } else if unit_type == "issue" {
        0.35
    } else if creator.is_some() {
        0.83
    } else {
        0.72
    };
    let sort_key = calculate_sort_key(
        &unit_type,
        volume_number.as_deref(),
        chapter_number.as_deref().or(raw_number.as_deref()),
    );
    let evidence = json!({
        "rawFilename": raw_filename,
        "parentPath": parent_path,
        "numberSource": number_source,
        "creator": creator,
        "releaseTokens": release_tokens,
        "magazineIssueContext": magazine_issue,
    });
    IdentityFact {
        archive_id: archive.id.clone(),
        raw_filename,
        parent_path,
        normalized_key,
        display_title: if display_title.is_empty() {
            archive.title.clone()
        } else {
            display_title
        },
        creator: if creator_key.is_empty() {
            None
        } else {
            Some(creator.unwrap_or_default())
        },
        unit_type,
        volume_number,
        chapter_number,
        issue_number,
        raw_number,
        edition_marker,
        sort_key,
        confidence,
        evidence,
    }
}

fn split_path(path: &str) -> (String, String) {
    let index = path.rfind(['/', '\\']).map(|value| value + 1).unwrap_or(0);
    (
        path[index..].to_string(),
        path[..index.saturating_sub(1)].to_string(),
    )
}

fn strip_extension(value: &str) -> String {
    value
        .rsplit_once('.')
        .map(|(stem, _)| stem.to_string())
        .unwrap_or_else(|| value.to_string())
}

fn extract_bracket_tokens(input: &str) -> (String, Vec<String>) {
    let mut stripped = String::with_capacity(input.len());
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let Some(close) = (match chars[index] {
            '[' => Some(']'),
            '(' => Some(')'),
            '【' => Some('】'),
            '（' => Some('）'),
            '{' => Some('}'),
            _ => None,
        }) else {
            stripped.push(chars[index]);
            index += 1;
            continue;
        };
        let mut end = index + 1;
        while end < chars.len() && chars[end] != close {
            end += 1;
        }
        if end == chars.len() {
            stripped.push(chars[index]);
            index += 1;
            continue;
        }
        let token: String = chars[index + 1..end].iter().collect();
        if !token.trim().is_empty() {
            tokens.push(token.trim().to_string());
        }
        stripped.push(' ');
        index = end + 1;
    }
    (stripped, tokens)
}

fn clean_display_title(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches([' ', '-', '_', '#'])
        .to_string()
}

pub(crate) fn clean_title_for_key(value: &str, unit_type: &str) -> String {
    let mut result = value.to_string();
    if let Some(index) = result.find('#') {
        result.truncate(index);
    }
    for marker in ["chapter", "episode", "volume", "vol", "part"] {
        if let Some(index) = result.to_lowercase().find(marker) {
            result.truncate(index);
            break;
        }
    }
    if unit_type != "standalone" {
        let words: Vec<&str> = result.split_whitespace().collect();
        if words
            .last()
            .is_some_and(|word| word.chars().all(|ch| ch.is_ascii_digit()))
        {
            result = words[..words.len() - 1].join(" ");
        }
    }
    clean_display_title(&result)
}

pub(crate) fn normalize_text(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| ch.to_lowercase())
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn contains_any(value: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| value.contains(marker))
}

fn is_release_token(token: &str) -> bool {
    let lower = normalize_text(token);
    lower.is_empty()
        || contains_any(
            &lower,
            &[
                "chinese",
                "中文",
                "翻訳",
                "汉化",
                "digital",
                "dl版",
                "無修正",
                "ai generated",
                "ai生成",
                "v2",
                "v3",
                "自用",
                "全彩",
                "新刊進捗",
                "无毒",
                "多语言",
                "page",
                "分辨率",
                "禁漫水印",
                "買動漫",
                "pubu",
            ],
        )
}

fn is_context_token(token: &str) -> bool {
    let lower = normalize_text(token);
    lower.starts_with('c') && lower[1..].chars().all(|ch| ch.is_ascii_digit())
        || lower.chars().all(|ch| ch.is_ascii_digit())
        || contains_any(
            &lower,
            &[
                "original",
                "オリジナル",
                "ブルーアーカイブ",
                "fate grand order",
                "原神",
                "艦隊これくしょん",
            ],
        )
}

fn is_edition_token(token: &str) -> bool {
    let lower = normalize_text(token);
    lower == "v2" || lower == "v3" || lower.contains("分辨率") || lower.contains("digital")
}

fn is_number(value: &str) -> bool {
    value
        .trim()
        .chars()
        .all(|ch| ch.is_ascii_digit() || ch == '.' || ch == '-')
}

fn normalize_number(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let value = value.trim_start_matches('0');
    let value = if value.is_empty() { "0" } else { value };
    value.split('-').next()?.parse::<f64>().ok().map(|number| {
        if number.fract() == 0.0 {
            format!("{number:.0}")
        } else {
            number.to_string()
        }
    })
}

fn find_marker_number(value: &str, marker: char) -> Option<String> {
    let index = value.find(marker)?;
    let digits: String = value[index + marker.len_utf8()..]
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
        .collect();
    normalize_number(&digits)
}

fn find_word_number(value: &str, markers: &[&str]) -> Option<String> {
    for marker in markers {
        if let Some(index) = value.find(marker) {
            let tail = &value[index + marker.len()..];
            let digits: String = tail
                .chars()
                .skip_while(|ch| !ch.is_ascii_digit())
                .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
                .collect();
            if let Some(number) = normalize_number(&digits) {
                return Some(number);
            }
        }
    }
    None
}

fn trailing_number(value: &str) -> Option<String> {
    let word = value.split_whitespace().last()?;
    normalize_number(word.trim_matches([')', '）', ']', '】']))
}

pub(crate) struct TerminalSequenceSuffix<'a> {
    title: &'a str,
    unit_type: &'static str,
    number: String,
    source: &'static str,
}

// Treat only a terminal, explicit content unit as a sequence suffix. This
// covers Japanese anthology parts and Japanese/Chinese ordinal units while
// leaving a phrase containing e.g. "中編" in the middle of a title untouched.
pub(crate) fn terminal_sequence_suffix(value: &str) -> Option<TerminalSequenceSuffix<'_>> {
    let value = value.trim_matches([' ', '\u{3000}', '-', '_', '~', '～']);
    for (marker, number) in [
        ("前編", "1"),
        ("上編", "1"),
        ("中編", "2"),
        ("後編", "3"),
        ("下編", "3"),
    ] {
        let Some(title) = value.strip_suffix(marker) else {
            continue;
        };
        let title = title.trim_matches([' ', '\u{3000}', '-', '_', '~', '～']);
        if !title.is_empty() {
            return Some(TerminalSequenceSuffix {
                title,
                unit_type: "chapter",
                number: number.to_string(),
                source: "japanese_part_suffix",
            });
        }
    }

    for (ordinal, marker, unit_type) in [
        ("第", "話", "chapter"),
        ("第", "话", "chapter"),
        ("第", "章", "chapter"),
        ("第", "巻", "volume"),
        ("第", "卷", "volume"),
        ("第", "部", "volume"),
        ("제", "화", "chapter"),
        ("제", "권", "volume"),
    ] {
        let Some(before_marker) = value.strip_suffix(marker) else {
            continue;
        };
        let Some(number_start) = before_marker.rfind(ordinal) else {
            continue;
        };
        let number = sequence_number(&before_marker[number_start + ordinal.len()..])?;
        let title =
            before_marker[..number_start].trim_matches([' ', '\u{3000}', '-', '_', '~', '～']);
        if !title.is_empty() {
            return Some(TerminalSequenceSuffix {
                title,
                unit_type,
                number,
                source: "east_asian_ordinal_suffix",
            });
        }
    }

    let Some(number_start) = value.rfind("その") else {
        return None;
    };
    let number = sequence_number(&value[number_start + "その".len()..])?;
    let title = value[..number_start].trim_matches([' ', '\u{3000}', '-', '_', '~', '～']);
    (!title.is_empty()).then(|| TerminalSequenceSuffix {
        title,
        unit_type: "chapter",
        number,
        source: "japanese_sono_suffix",
    })
}

fn sequence_number(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit() || ch == '.') {
        return None;
    }
    normalize_number(value)
}

pub(crate) fn calculate_sort_key(
    unit_type: &str,
    volume: Option<&str>,
    chapter: Option<&str>,
) -> f64 {
    let volume = volume
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);
    let chapter = chapter
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);
    match unit_type {
        "volume" => volume * 10000.0 + chapter,
        // Units without an explicit marker (e.g. "Title 12") still carry a
        // parsed number; sort by it so collections follow number order instead
        // of recognition order. Only truly unnumbered works stay at the end.
        _ => {
            if chapter > 0.0 {
                chapter
            } else {
                999999.0
            }
        }
    }
}
