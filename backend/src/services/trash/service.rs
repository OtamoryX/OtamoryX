//! Application facade for trash and recoverable-version operations.
//!
//! The implementation is split by responsibility into move, query, restore,
//! cleanup, snapshot, and relation modules.  This file owns the service state
//! and keeps the historical `services::trash::service` API stable.

use anyhow::Result;
use sqlx::{Pool, Sqlite, Transaction};
use std::time::Duration;

#[path = "cleanup.rs"]
mod cleanup;
#[path = "moves.rs"]
mod moves;
#[path = "query.rs"]
mod query;
#[path = "relations.rs"]
mod relations;
#[path = "restore.rs"]
mod restore;
#[path = "snapshot.rs"]
mod snapshot;
#[path = "types.rs"]
mod types;

pub use types::TrashCleanupReport;
use types::{
    ArchiveSnapshot, ReadingProgressSnapshot, TagSnapshot, TrashCleanupCandidate,
    TrashFileCleanupResult, VersionOperationMember, VersionProgressMigration,
    VersionRelationMigration,
};

pub struct TrashService {
    pool: Pool<Sqlite>,
}

const TRASH_CLEANUP_BATCH_SIZE: u32 = 100;
const TRASH_CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60);

impl TrashService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

pub fn spawn_trash_expiration_cleanup(pool: Pool<Sqlite>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(TRASH_CLEANUP_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            interval.tick().await;
            match TrashService::new(pool.clone())
                .cleanup_expired_entries(TRASH_CLEANUP_BATCH_SIZE)
                .await
            {
                Ok(report) if report.claimed > 0 || report.failed > 0 => {
                    tracing::info!(
                        claimed = report.claimed,
                        deleted_files = report.deleted_files,
                        missing_files = report.missing_files,
                        failed = report.failed,
                        "Finished trash expiration cleanup"
                    );
                }
                Ok(_) => tracing::debug!("Trash expiration cleanup found no pending entries"),
                Err(error) => tracing::warn!("Trash expiration cleanup failed: {error:#}"),
            }
        }
    });
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

/// Legacy theme relations predate the archive_tags boundary. Trash/version restore must be able
/// to put those historical rows back, but only inside the transaction that explicitly requested
/// the restore. The trigger is dropped and recreated transactionally while the writer lock is held.
pub(super) async fn suspend_legacy_theme_archive_tag_guard(
    tx: &mut Transaction<'_, Sqlite>,
) -> Result<bool> {
    let present: i64 = sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM sqlite_master
             WHERE type = 'trigger' AND name = 'prevent_theme_archive_tag_insert'
         )",
    )
    .fetch_one(&mut **tx)
    .await?;
    if present != 0 {
        sqlx::query("DROP TRIGGER prevent_theme_archive_tag_insert")
            .execute(&mut **tx)
            .await?;
    }
    Ok(present != 0)
}

pub(super) async fn restore_legacy_theme_archive_tag_guard(
    tx: &mut Transaction<'_, Sqlite>,
    was_present: bool,
) -> Result<()> {
    if !was_present {
        return Ok(());
    }
    sqlx::query(
        "CREATE TRIGGER prevent_theme_archive_tag_insert
         BEFORE INSERT ON archive_tags
         FOR EACH ROW
         WHEN EXISTS (
             SELECT 1
             FROM tags
             WHERE id = NEW.tag_id
               AND lower(trim(namespace)) = 'theme'
         )
         BEGIN
             SELECT RAISE(ABORT, 'system-managed theme tags cannot be stored in archive_tags');
         END",
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
