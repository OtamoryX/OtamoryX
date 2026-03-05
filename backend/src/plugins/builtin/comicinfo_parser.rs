use crate::plugins::{
    BuiltinPlugin, BuiltinPluginKind, BuiltinPluginResult, PluginContext, PluginOutput,
    TagProposal, BUILTIN_COMICINFO_PARSER_ID, BUILTIN_METADATA_ORDER_COMICINFO,
};

#[derive(Debug, Default)]
pub struct ComicInfoParser;

impl BuiltinPlugin for ComicInfoParser {
    fn id(&self) -> &'static str {
        BUILTIN_COMICINFO_PARSER_ID
    }

    fn kind(&self) -> BuiltinPluginKind {
        BuiltinPluginKind::MetadataPipeline
    }

    fn order(&self) -> Option<u16> {
        Some(BUILTIN_METADATA_ORDER_COMICINFO)
    }

    fn run(&self, ctx: &PluginContext) -> BuiltinPluginResult {
        let has_comicinfo = ctx
            .embedded_files
            .iter()
            .any(|name| is_comicinfo_filename(name));
        let xml = ctx.comicinfo_xml.as_deref().map(str::trim);

        if !has_comicinfo && xml.is_none() {
            return Ok(PluginOutput::default());
        }

        let mut out = PluginOutput::default();
        out.tags.push(TagProposal::deterministic(
            "metadata_source",
            "comicinfo_xml",
            self.id(),
        ));

        let Some(xml) = xml.filter(|content| !content.is_empty()) else {
            out.notes
                .push("comicinfo-parser skipped: ComicInfo.xml content unavailable".to_string());
            return Ok(out);
        };

        out.metadata.title = extract_tag_text(xml, "Title");
        out.metadata.summary = extract_tag_text(xml, "Summary");
        out.metadata.source_url = extract_tag_text(xml, "Web");

        push_if_present(
            &mut out.tags,
            "series",
            extract_tag_text(xml, "Series"),
            self.id(),
        );
        push_if_present(
            &mut out.tags,
            "issue",
            extract_tag_text(xml, "Number"),
            self.id(),
        );
        push_if_present(
            &mut out.tags,
            "volume",
            extract_tag_text(xml, "Volume"),
            self.id(),
        );
        push_if_present(
            &mut out.tags,
            "year",
            extract_tag_text(xml, "Year"),
            self.id(),
        );
        push_if_present(
            &mut out.tags,
            "author",
            extract_tag_text(xml, "Writer"),
            self.id(),
        );
        push_if_present(
            &mut out.tags,
            "publisher",
            extract_tag_text(xml, "Publisher"),
            self.id(),
        );
        push_if_present(
            &mut out.tags,
            "language",
            extract_tag_text(xml, "LanguageISO"),
            self.id(),
        );

        for genre in split_multi_values(extract_tag_text(xml, "Genre").as_deref()) {
            push_if_present(&mut out.tags, "genre", Some(genre), self.id());
        }
        for character in split_multi_values(extract_tag_text(xml, "Characters").as_deref()) {
            push_if_present(&mut out.tags, "character", Some(character), self.id());
        }
        for team in split_multi_values(extract_tag_text(xml, "Teams").as_deref()) {
            push_if_present(&mut out.tags, "team", Some(team), self.id());
        }
        for location in split_multi_values(extract_tag_text(xml, "Locations").as_deref()) {
            push_if_present(&mut out.tags, "location", Some(location), self.id());
        }

        Ok(out)
    }
}

fn is_comicinfo_filename(name: &str) -> bool {
    name.rsplit(['/', '\\'])
        .next()
        .map(|value| value.eq_ignore_ascii_case("ComicInfo.xml"))
        .unwrap_or(false)
}

fn push_if_present(
    tags: &mut Vec<TagProposal>,
    namespace: &str,
    value: Option<String>,
    source: &'static str,
) {
    let Some(value) = value.filter(|v| !v.is_empty()) else {
        return;
    };

    if tags.iter().any(|tag| {
        tag.namespace.eq_ignore_ascii_case(namespace) && tag.value.eq_ignore_ascii_case(&value)
    }) {
        return;
    }

    tags.push(TagProposal::deterministic(namespace, value, source));
}

fn extract_tag_text(xml: &str, tag_name: &str) -> Option<String> {
    let xml_lower = xml.to_ascii_lowercase();
    let tag_lower = tag_name.to_ascii_lowercase();
    let tag_prefix = format!("<{}", tag_lower);

    let mut search_from = 0usize;
    while let Some(found_rel) = xml_lower[search_from..].find(&tag_prefix) {
        let found = search_from + found_rel;
        let after_prefix = found + tag_prefix.len();
        let suffix = xml_lower[after_prefix..].chars().next();
        if matches!(suffix, Some(ch) if ch.is_ascii_alphanumeric()) {
            search_from = after_prefix;
            continue;
        }

        let open_end_rel = xml_lower[after_prefix..].find('>')?;
        let open_end = after_prefix + open_end_rel;
        if xml_lower.as_bytes().get(open_end.wrapping_sub(1)) == Some(&b'/') {
            search_from = open_end + 1;
            continue;
        }

        let close_tag = format!("</{}>", tag_lower);
        let close_start_rel = xml_lower[(open_end + 1)..].find(&close_tag)?;
        let close_start = open_end + 1 + close_start_rel;
        let raw = &xml[(open_end + 1)..close_start];
        let cleaned = normalize_xml_text(raw);
        return (!cleaned.is_empty()).then_some(cleaned);
    }

    None
}

fn normalize_xml_text(value: &str) -> String {
    let stripped_cdata = value
        .trim()
        .strip_prefix("<![CDATA[")
        .and_then(|inner| inner.strip_suffix("]]>"))
        .unwrap_or(value);

    let without_tags = strip_xml_tags(stripped_cdata);
    let unescaped = decode_xml_entities(&without_tags);
    unescaped.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_xml_tags(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn decode_xml_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn split_multi_values(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split([',', ';', '|', '/'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::{BuiltinPlugin, PluginContext};

    fn test_context(xml: Option<&str>, embedded_files: Vec<&str>) -> PluginContext {
        PluginContext {
            archive_id: "a1".to_string(),
            archive_path: "/library/test.cbz".to_string(),
            ingested_at_unix: 1,
            embedded_files: embedded_files.into_iter().map(ToOwned::to_owned).collect(),
            comicinfo_xml: xml.map(ToOwned::to_owned),
            tag_copier_request: None,
        }
    }

    #[test]
    fn returns_empty_when_comicinfo_is_absent() {
        let parser = ComicInfoParser;
        let out = parser
            .run(&test_context(None, vec!["page-1.jpg"]))
            .expect("comicinfo parser should succeed");

        assert_eq!(out, PluginOutput::default());
    }

    #[test]
    fn extracts_basic_fields_from_comicinfo_xml() {
        let parser = ComicInfoParser;
        let xml = r#"
            <ComicInfo>
              <Title>Demo &amp; Title</Title>
              <Summary><![CDATA[Line1<br/>Line2]]></Summary>
              <Series>My Series</Series>
              <Number>003</Number>
              <Volume>01</Volume>
              <Year>2025</Year>
              <Writer>Alice</Writer>
              <Publisher>Test House</Publisher>
              <LanguageISO>zh</LanguageISO>
              <Genre>Action, Comedy</Genre>
              <Characters>A;B</Characters>
              <Web>https://example.com/demo</Web>
            </ComicInfo>
        "#;

        let out = parser
            .run(&test_context(Some(xml), vec!["metadata/ComicInfo.xml"]))
            .expect("comicinfo parser should succeed");

        assert_eq!(out.metadata.title.as_deref(), Some("Demo & Title"));
        assert_eq!(out.metadata.summary.as_deref(), Some("Line1Line2"));
        assert_eq!(
            out.metadata.source_url.as_deref(),
            Some("https://example.com/demo")
        );
        assert!(out
            .tags
            .iter()
            .any(|t| t.namespace == "metadata_source" && t.value == "comicinfo_xml"));
        assert!(out
            .tags
            .iter()
            .any(|t| t.namespace == "series" && t.value == "My Series"));
        assert!(out
            .tags
            .iter()
            .any(|t| t.namespace == "issue" && t.value == "003"));
        assert!(out
            .tags
            .iter()
            .any(|t| t.namespace == "genre" && t.value == "Action"));
        assert!(out
            .tags
            .iter()
            .any(|t| t.namespace == "character" && t.value == "B"));
    }

    #[test]
    fn adds_note_when_content_is_missing() {
        let parser = ComicInfoParser;
        let out = parser
            .run(&test_context(None, vec!["ComicInfo.xml"]))
            .expect("comicinfo parser should succeed");

        assert!(out
            .notes
            .iter()
            .any(|note| note.contains("content unavailable")));
    }
}
