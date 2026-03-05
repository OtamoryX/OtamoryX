use crate::plugins::{
    BuiltinPlugin, BuiltinPluginError, BuiltinPluginKind, BuiltinPluginResult, PluginContext,
    PluginOutput, TagProposal, TagProvenance, BUILTIN_TAG_COPIER_ID,
};
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct TagCopier;

impl BuiltinPlugin for TagCopier {
    fn id(&self) -> &'static str {
        BUILTIN_TAG_COPIER_ID
    }

    fn kind(&self) -> BuiltinPluginKind {
        BuiltinPluginKind::Utility
    }

    fn order(&self) -> Option<u16> {
        None
    }

    fn run(&self, ctx: &PluginContext) -> BuiltinPluginResult {
        let request = ctx.tag_copier_request.as_ref().ok_or_else(|| {
            BuiltinPluginError::new("tag-copier requires tag_copier_request in PluginContext")
        })?;

        let mut out = PluginOutput::default();
        let mut seen = HashSet::new();
        for tag in &request.tags {
            let namespace = tag.namespace.trim();
            let value = tag.value.trim();

            if namespace.is_empty() || value.is_empty() {
                out.notes.push(
                    "tag-copier skipped a tag with empty namespace/value after trimming"
                        .to_string(),
                );
                continue;
            }

            let dedupe_key = format!(
                "{}\u{0}{}",
                namespace.to_ascii_lowercase(),
                value.to_ascii_lowercase()
            );
            if !seen.insert(dedupe_key) {
                continue;
            }

            out.tags.push(TagProposal {
                namespace: namespace.to_string(),
                value: value.to_string(),
                source_plugin: self.id(),
                // Tool invocation is user-driven and should keep manual priority semantics.
                provenance: TagProvenance::UserManual,
            });
        }

        if out.tags.is_empty() {
            out.notes
                .push("tag-copier produced no valid tags from request".to_string());
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::{BuiltinPlugin, PluginContext, TagCopierRequest};

    fn test_context(request: Option<TagCopierRequest>) -> PluginContext {
        PluginContext {
            archive_id: "a1".to_string(),
            archive_path: "/library/test.cbz".to_string(),
            ingested_at_unix: 1,
            embedded_files: Vec::new(),
            comicinfo_xml: None,
            tag_copier_request: request,
        }
    }

    #[test]
    fn errors_when_request_is_missing() {
        let plugin = TagCopier;
        let err = plugin
            .run(&test_context(None))
            .expect_err("missing request should fail");

        assert!(err
            .to_string()
            .contains("requires tag_copier_request in PluginContext"));
    }

    #[test]
    fn trims_filters_and_deduplicates_input_tags() {
        let plugin = TagCopier;
        let out = plugin
            .run(&test_context(Some(TagCopierRequest {
                tags: vec![
                    TagProposal::heuristic(" artist ", " Alice ", "x"),
                    TagProposal::deterministic("artist", "alice", "y"),
                    TagProposal::heuristic(" ", "value", "x"),
                    TagProposal::heuristic("group", " ", "x"),
                    TagProposal::heuristic("genre", "comedy", "x"),
                ],
            })))
            .expect("tag-copier should succeed");

        assert_eq!(out.tags.len(), 2);
        assert_eq!(out.tags[0].namespace, "artist");
        assert_eq!(out.tags[0].value, "Alice");
        assert_eq!(out.tags[0].provenance, TagProvenance::UserManual);
        assert_eq!(out.tags[0].source_plugin, BUILTIN_TAG_COPIER_ID);
        assert!(out
            .notes
            .iter()
            .any(|note| note.contains("empty namespace/value")));
    }

    #[test]
    fn emits_note_when_no_valid_tags_remain() {
        let plugin = TagCopier;
        let out = plugin
            .run(&test_context(Some(TagCopierRequest {
                tags: vec![TagProposal::heuristic(" ", " ", "x")],
            })))
            .expect("tag-copier should succeed");

        assert!(out.tags.is_empty());
        assert!(out
            .notes
            .iter()
            .any(|note| note.contains("produced no valid tags")));
    }
}
