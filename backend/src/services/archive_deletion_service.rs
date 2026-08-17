use anyhow::Result;
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use std::sync::Arc;

use super::{ArchiveCacheService, ArchiveDeleteTarget, CurationService, TrashService};

#[derive(Debug, Clone, Copy)]
pub struct ArchiveDeletionSummary {
    pub matched: u64,
    pub deleted: u64,
    pub failed: u64,
}

pub struct ArchiveDeletionService {
    pool: Pool<Sqlite>,
    archive_cache: Arc<ArchiveCacheService>,
}

impl ArchiveDeletionService {
    pub fn new(pool: Pool<Sqlite>, archive_cache: Arc<ArchiveCacheService>) -> Self {
        Self {
            pool,
            archive_cache,
        }
    }

    pub async fn delete_targets(
        &self,
        user_id: &str,
        mut targets: Vec<ArchiveDeleteTarget>,
        reason: &str,
        source: &str,
    ) -> Result<ArchiveDeletionSummary> {
        let mut seen_ids = HashSet::new();
        targets.retain(|target| seen_ids.insert(target.id.clone()));
        let matched = targets.len() as u64;
        let mut deleted = 0;
        let mut failed = 0;

        for target in targets {
            if target.id.is_empty() {
                failed += 1;
                continue;
            }

            let entry = match TrashService::new(self.pool.clone())
                .move_archive_to_trash(user_id, &target.id, Some(reason), source)
                .await
            {
                Ok(entry) => entry,
                Err(error) => {
                    tracing::error!(
                        "Failed to move archive file {} for {} to trash: {}",
                        target.path,
                        target.id,
                        error
                    );
                    failed += 1;
                    continue;
                }
            };

            deleted += 1;
            self.archive_cache.clear_archive_cache(&target.id).await;
            self.record_feedback(user_id, &target.id, &entry.id, reason, source)
                .await;
        }

        Ok(ArchiveDeletionSummary {
            matched,
            deleted,
            failed,
        })
    }

    async fn record_feedback(
        &self,
        user_id: &str,
        archive_id: &str,
        trash_entry_id: &str,
        reason: &str,
        source: &str,
    ) {
        let curation = CurationService::new(self.pool.clone());
        if let Err(error) = curation
            .record_manual_delete_feedback(
                user_id,
                archive_id,
                trash_entry_id,
                Some(reason),
                source,
            )
            .await
        {
            tracing::warn!(
                "Failed to record batch delete disposition for archive {}: {}",
                archive_id,
                error
            );
        }
    }
}
