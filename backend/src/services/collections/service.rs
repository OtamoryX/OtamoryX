//! Collection domain facade.
//!
//! The implementation is split by responsibility so callers can keep using
//! `services::collections` while the parsing, querying, rebuild, and version
//! cleanup workflows evolve independently.

#[cfg(test)]
use sqlx::{Pool, Sqlite};

pub use super::query::{
    collection_member_delete_targets, collection_progress, delete_collection, get_collection,
    list_collections, list_review_items,
};
pub use super::rebuild::{
    add_member, apply_review, preview_collection_rebuild, rebuild_collections, remove_member,
    update_collection,
};
pub use super::versions::{
    cleanup_versions, keep_all_versions, list_version_groups, restore_version_group,
};

// Test and intra-domain helpers remain available through the facade without
// making implementation details part of the public API.
#[cfg(test)]
pub(crate) use super::identity::{
    content_unit_key, infer_missing_first_numbers, parse_identity, terminal_sequence_suffix,
    ArchiveRow,
};
#[cfg(test)]
pub(crate) use super::versions::{load_idempotent_version_cleanup, VersionCleanupRequestSnapshot};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
