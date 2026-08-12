use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use std::sync::Arc;

use super::{delete_archive_file, ArchiveCacheService, ArchiveDeleteTarget};

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
        mut targets: Vec<ArchiveDeleteTarget>,
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

            let mut tx = self
                .pool
                .begin()
                .await
                .context("Failed to start archive deletion transaction")?;
            let result = match sqlx::query("DELETE FROM archives WHERE id = ?")
                .bind(&target.id)
                .execute(&mut *tx)
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    tracing::error!("Failed to delete archive {}: {}", target.id, error);
                    let _ = tx.rollback().await;
                    failed += 1;
                    continue;
                }
            };

            if result.rows_affected() == 0 {
                let _ = tx.rollback().await;
                failed += 1;
                continue;
            }

            if let Err(error) = delete_archive_file(&target.path).await {
                tracing::error!(
                    "Failed to delete archive file {} for {}: {}",
                    target.path,
                    target.id,
                    error
                );
                if let Err(rollback_error) = tx.rollback().await {
                    tracing::error!(
                        "Failed to roll back archive {} deletion: {}",
                        target.id,
                        rollback_error
                    );
                }
                failed += 1;
                continue;
            }

            tx.commit()
                .await
                .with_context(|| format!("Failed to commit archive {} deletion", target.id))?;

            deleted += result.rows_affected();
            self.archive_cache.clear_archive_cache(&target.id).await;
        }

        Ok(ArchiveDeletionSummary {
            matched,
            deleted,
            failed,
        })
    }
}
