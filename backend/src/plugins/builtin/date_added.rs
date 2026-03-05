use crate::plugins::{
    BuiltinPlugin, BuiltinPluginKind, BuiltinPluginResult, PluginContext, PluginOutput,
    TagProposal, BUILTIN_DATE_ADDED_ID, BUILTIN_METADATA_ORDER_DATE_ADDED,
};

#[derive(Debug, Default)]
pub struct DateAdded;

impl BuiltinPlugin for DateAdded {
    fn id(&self) -> &'static str {
        BUILTIN_DATE_ADDED_ID
    }

    fn kind(&self) -> BuiltinPluginKind {
        BuiltinPluginKind::MetadataPipeline
    }

    fn order(&self) -> Option<u16> {
        Some(BUILTIN_METADATA_ORDER_DATE_ADDED)
    }

    fn run(&self, ctx: &PluginContext) -> BuiltinPluginResult {
        let mut out = PluginOutput::default();
        if ctx.ingested_at_unix <= 0 {
            out.notes.push(format!(
                "date-added skipped: invalid ingested_at_unix={}",
                ctx.ingested_at_unix
            ));
            return Ok(out);
        }

        out.tags.push(TagProposal::deterministic(
            "date_added",
            ctx.ingested_at_unix.to_string(),
            self.id(),
        ));

        if let Some(date_time) =
            chrono::DateTime::<chrono::Utc>::from_timestamp(ctx.ingested_at_unix, 0)
        {
            out.tags.push(TagProposal::deterministic(
                "date_added_iso8601",
                date_time.to_rfc3339(),
                self.id(),
            ));
        } else {
            out.notes.push(format!(
                "date-added warning: timestamp out of ISO-8601 range ({})",
                ctx.ingested_at_unix
            ));
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::{BuiltinPlugin, PluginContext};

    fn test_context(ts: i64) -> PluginContext {
        PluginContext {
            archive_id: "a1".to_string(),
            archive_path: "/library/test.cbz".to_string(),
            ingested_at_unix: ts,
            embedded_files: Vec::new(),
            comicinfo_xml: None,
            tag_copier_request: None,
        }
    }

    #[test]
    fn emits_unix_and_iso_tags_for_valid_timestamp() {
        let plugin = DateAdded;
        let out = plugin
            .run(&test_context(1_700_000_000))
            .expect("date-added should succeed");

        assert!(out
            .tags
            .iter()
            .any(|tag| tag.namespace == "date_added" && tag.value == "1700000000"));
        assert!(out
            .tags
            .iter()
            .any(|tag| tag.namespace == "date_added_iso8601"));
    }

    #[test]
    fn skips_invalid_non_positive_timestamp() {
        let plugin = DateAdded;
        let out = plugin
            .run(&test_context(0))
            .expect("date-added should succeed");

        assert!(out.tags.is_empty());
        assert!(out
            .notes
            .iter()
            .any(|note| note.contains("invalid ingested_at_unix")));
    }

    #[test]
    fn keeps_unix_tag_and_warns_when_iso_conversion_fails() {
        let plugin = DateAdded;
        let out = plugin
            .run(&test_context(i64::MAX))
            .expect("date-added should succeed");

        assert!(out
            .tags
            .iter()
            .any(|tag| tag.namespace == "date_added" && tag.value == i64::MAX.to_string()));
        assert!(!out
            .tags
            .iter()
            .any(|tag| tag.namespace == "date_added_iso8601"));
        assert!(out
            .notes
            .iter()
            .any(|note| note.contains("out of ISO-8601")));
    }
}
