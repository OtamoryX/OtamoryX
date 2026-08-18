pub mod application;
pub mod builtin;
pub mod providers;
pub mod runtime;

pub const BUILTIN_FILENAME_PARSER_ID: &str = "filename-parser";
pub const BUILTIN_COMICINFO_PARSER_ID: &str = "comicinfo-parser";
pub const BUILTIN_DATE_ADDED_ID: &str = "date-added";
pub const BUILTIN_TAG_COPIER_ID: &str = "tag-copier";
pub const BUILTIN_EHENTAI_METADATA_ID: &str = "ehentai-metadata";
pub const BUILTIN_NHENTAI_METADATA_ID: &str = "nhentai-metadata";

pub const BUILTIN_METADATA_ORDER_FILENAME: u16 = 100;
pub const BUILTIN_METADATA_ORDER_COMICINFO: u16 = 200;
pub const BUILTIN_METADATA_ORDER_DATE_ADDED: u16 = 300;

pub const BUILTIN_METADATA_EXECUTION_ORDER: [(&str, u16); 3] = [
    (BUILTIN_FILENAME_PARSER_ID, BUILTIN_METADATA_ORDER_FILENAME),
    (
        BUILTIN_COMICINFO_PARSER_ID,
        BUILTIN_METADATA_ORDER_COMICINFO,
    ),
    (BUILTIN_DATE_ADDED_ID, BUILTIN_METADATA_ORDER_DATE_ADDED),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinPluginKind {
    MetadataPipeline,
    Utility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagProvenance {
    UserManual,
    PluginDeterministic,
    PluginHeuristic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagConflictDecision {
    KeepExisting,
    ReplaceWithIncoming,
    RequireManualReview,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagProposal {
    pub namespace: String,
    pub value: String,
    pub source_plugin: &'static str,
    pub provenance: TagProvenance,
}

impl TagProposal {
    pub fn manual(namespace: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            value: value.into(),
            source_plugin: "user-manual",
            provenance: TagProvenance::UserManual,
        }
    }

    pub fn deterministic(
        namespace: impl Into<String>,
        value: impl Into<String>,
        source_plugin: &'static str,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            value: value.into(),
            source_plugin,
            provenance: TagProvenance::PluginDeterministic,
        }
    }

    pub fn heuristic(
        namespace: impl Into<String>,
        value: impl Into<String>,
        source_plugin: &'static str,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            value: value.into(),
            source_plugin,
            provenance: TagProvenance::PluginHeuristic,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MetadataPatch {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub source_url: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginOutput {
    pub tags: Vec<TagProposal>,
    pub metadata: MetadataPatch,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagCopierRequest {
    pub tags: Vec<TagProposal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginContext {
    pub archive_id: String,
    pub archive_path: String,
    pub ingested_at_unix: i64,
    pub embedded_files: Vec<String>,
    pub comicinfo_xml: Option<String>,
    pub tag_copier_request: Option<TagCopierRequest>,
}

pub trait TagConflictResolver {
    fn resolve(&self, existing: &TagProposal, incoming: &TagProposal) -> TagConflictDecision;
}

#[derive(Debug, Default)]
pub struct DefaultTagConflictResolver;

impl TagConflictResolver for DefaultTagConflictResolver {
    fn resolve(&self, existing: &TagProposal, incoming: &TagProposal) -> TagConflictDecision {
        // Conflict rule 1: user-manual tags always win over plugin tags.
        if matches!(existing.provenance, TagProvenance::UserManual)
            && !matches!(incoming.provenance, TagProvenance::UserManual)
        {
            return TagConflictDecision::KeepExisting;
        }
        if !matches!(existing.provenance, TagProvenance::UserManual)
            && matches!(incoming.provenance, TagProvenance::UserManual)
        {
            return TagConflictDecision::ReplaceWithIncoming;
        }

        // Conflict rule 2: deterministic plugin tags win over heuristic plugin tags.
        if matches!(existing.provenance, TagProvenance::PluginDeterministic)
            && matches!(incoming.provenance, TagProvenance::PluginHeuristic)
        {
            return TagConflictDecision::KeepExisting;
        }
        if matches!(existing.provenance, TagProvenance::PluginHeuristic)
            && matches!(incoming.provenance, TagProvenance::PluginDeterministic)
        {
            return TagConflictDecision::ReplaceWithIncoming;
        }

        // Two deterministic values that disagree are escalated for manual review.
        if matches!(existing.provenance, TagProvenance::PluginDeterministic)
            && matches!(incoming.provenance, TagProvenance::PluginDeterministic)
            && existing.value != incoming.value
        {
            return TagConflictDecision::RequireManualReview;
        }

        TagConflictDecision::KeepExisting
    }
}

pub static DEFAULT_TAG_CONFLICT_RESOLVER: DefaultTagConflictResolver = DefaultTagConflictResolver;

pub fn merge_plugin_output(
    accumulator: &mut PluginOutput,
    incoming: PluginOutput,
    resolver: &dyn TagConflictResolver,
) {
    merge_optional_metadata_field(
        "title",
        &mut accumulator.metadata.title,
        incoming.metadata.title,
        &mut accumulator.notes,
    );
    merge_optional_metadata_field(
        "summary",
        &mut accumulator.metadata.summary,
        incoming.metadata.summary,
        &mut accumulator.notes,
    );
    merge_optional_metadata_field(
        "source_url",
        &mut accumulator.metadata.source_url,
        incoming.metadata.source_url,
        &mut accumulator.notes,
    );

    for incoming_tag in incoming.tags {
        if has_same_tag_value(&accumulator.tags, &incoming_tag) {
            continue;
        }

        if is_multi_value_namespace(&incoming_tag.namespace) {
            accumulator.tags.push(incoming_tag);
            continue;
        }

        let maybe_conflict_index = accumulator.tags.iter().position(|existing| {
            existing
                .namespace
                .eq_ignore_ascii_case(&incoming_tag.namespace)
        });

        if let Some(index) = maybe_conflict_index {
            let decision = resolver.resolve(&accumulator.tags[index], &incoming_tag);
            match decision {
                TagConflictDecision::KeepExisting => {}
                TagConflictDecision::ReplaceWithIncoming => {
                    accumulator.tags[index] = incoming_tag;
                }
                TagConflictDecision::RequireManualReview => {
                    accumulator.notes.push(format!(
                        "Tag conflict requires manual review: namespace='{}', existing='{}', incoming='{}'",
                        accumulator.tags[index].namespace,
                        accumulator.tags[index].value,
                        incoming_tag.value
                    ));
                }
            }
            continue;
        }

        accumulator.tags.push(incoming_tag);
    }

    accumulator.notes.extend(incoming.notes);
}

fn merge_optional_metadata_field(
    field_name: &str,
    existing: &mut Option<String>,
    incoming: Option<String>,
    notes: &mut Vec<String>,
) {
    let Some(incoming_value) = incoming
        .as_deref()
        .map(normalize_whitespace)
        .filter(|v| !v.is_empty())
    else {
        return;
    };

    match existing {
        Some(existing_value) => {
            if normalize_whitespace(existing_value) != incoming_value {
                notes.push(format!(
                    "Metadata conflict kept existing value: field='{}'",
                    field_name
                ));
            }
        }
        None => {
            *existing = Some(incoming_value);
        }
    }
}

fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn has_same_tag_value(existing: &[TagProposal], incoming: &TagProposal) -> bool {
    existing.iter().any(|tag| {
        tag.namespace.eq_ignore_ascii_case(&incoming.namespace)
            && tag.value.eq_ignore_ascii_case(&incoming.value)
    })
}

fn is_multi_value_namespace(namespace: &str) -> bool {
    matches!(
        namespace,
        "filename_token"
            | "genre"
            | "character"
            | "team"
            | "location"
            | "artist"
            | "group"
            | "parody"
            | "female"
            | "male"
            | "mixed"
            | "other"
            | "cosplayer"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinPluginError {
    message: String,
}

impl BuiltinPluginError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for BuiltinPluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for BuiltinPluginError {}

pub type BuiltinPluginResult = Result<PluginOutput, BuiltinPluginError>;

pub trait BuiltinPlugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn kind(&self) -> BuiltinPluginKind;
    fn order(&self) -> Option<u16>;
    fn run(&self, ctx: &PluginContext) -> BuiltinPluginResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_resolver_enforces_manual_priority() {
        let resolver = DefaultTagConflictResolver;
        let existing = TagProposal::manual("artist", "alice");
        let incoming = TagProposal::deterministic("artist", "bob", "filename-parser");

        assert_eq!(
            resolver.resolve(&existing, &incoming),
            TagConflictDecision::KeepExisting
        );
        assert_eq!(
            resolver.resolve(&incoming, &existing),
            TagConflictDecision::ReplaceWithIncoming
        );
    }

    #[test]
    fn merge_output_preserves_multi_value_metadata_namespaces() {
        let resolver = DefaultTagConflictResolver;
        let mut accumulator = PluginOutput {
            tags: vec![
                TagProposal::deterministic("artist", "alice", "comicinfo-parser"),
                TagProposal::heuristic("filename_token", "digital", "filename-parser"),
            ],
            metadata: MetadataPatch {
                title: Some("Existing Title".to_string()),
                summary: None,
                source_url: None,
            },
            notes: Vec::new(),
        };

        let incoming = PluginOutput {
            tags: vec![
                TagProposal::deterministic("artist", "bob", "tag-copier"),
                TagProposal::heuristic("filename_token", "digital", "filename-parser"),
                TagProposal::heuristic("filename_token", "scan", "filename-parser"),
            ],
            metadata: MetadataPatch {
                title: Some("Incoming Title".to_string()),
                summary: Some("summary".to_string()),
                source_url: None,
            },
            notes: vec!["incoming note".to_string()],
        };

        merge_plugin_output(&mut accumulator, incoming, &resolver);

        assert_eq!(
            accumulator.metadata.title.as_deref(),
            Some("Existing Title")
        );
        assert_eq!(accumulator.metadata.summary.as_deref(), Some("summary"));
        assert_eq!(accumulator.tags.len(), 4);
        assert_eq!(
            accumulator
                .tags
                .iter()
                .filter(|tag| tag.namespace == "artist")
                .count(),
            2,
            "artist is a multi-value E-Hentai namespace"
        );
        assert_eq!(
            accumulator
                .tags
                .iter()
                .filter(|tag| tag.namespace == "filename_token")
                .count(),
            2
        );
        assert!(accumulator.notes.iter().any(|note| note == "incoming note"));
    }
}
