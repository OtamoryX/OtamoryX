use super::quote_identifier;
use super::{ReadingProgressSnapshot, VersionProgressMigration, VersionRelationMigration};
use anyhow::{anyhow, Result};
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

pub(super) async fn migrate_version_relations(
    tx: &mut Transaction<'_, Sqlite>,
    archive_id: &str,
    keep_archive_id: &str,
    keeper_pages: i32,
) -> Result<VersionRelationMigration> {
    let before_tag_ids =
        keeper_relation_ids(tx, "archive_tags", "tag_id", "archive_id", keep_archive_id).await?;
    let source_tag_ids =
        keeper_relation_ids(tx, "archive_tags", "tag_id", "archive_id", archive_id).await?;
    for tag_id in source_tag_ids {
        sqlx::query("INSERT OR IGNORE INTO archive_tags (archive_id, tag_id) VALUES (?, ?)")
            .bind(keep_archive_id)
            .bind(tag_id)
            .execute(&mut **tx)
            .await?;
    }
    let after_tag_ids =
        keeper_relation_ids(tx, "archive_tags", "tag_id", "archive_id", keep_archive_id).await?;

    let before_category_ids = keeper_relation_ids(
        tx,
        "category_archives",
        "category_id",
        "archive_id",
        keep_archive_id,
    )
    .await?;
    let source_category_ids = keeper_relation_ids(
        tx,
        "category_archives",
        "category_id",
        "archive_id",
        archive_id,
    )
    .await?;
    for category_id in source_category_ids {
        sqlx::query(
            "INSERT OR IGNORE INTO category_archives (category_id, archive_id) VALUES (?, ?)",
        )
        .bind(category_id)
        .bind(keep_archive_id)
        .execute(&mut **tx)
        .await?;
    }
    let after_category_ids = keeper_relation_ids(
        tx,
        "category_archives",
        "category_id",
        "archive_id",
        keep_archive_id,
    )
    .await?;

    let progress_rows = sqlx::query_as::<_, ReadingProgressSnapshot>(
        "SELECT id, user_id, archive_id, current_page, total_pages, progress_percentage,
                last_read_at, created_at, updated_at
         FROM reading_progress WHERE archive_id = ?",
    )
    .bind(archive_id)
    .fetch_all(&mut **tx)
    .await?;
    let mut progress = Vec::with_capacity(progress_rows.len());
    for source_progress in progress_rows {
        let before = sqlx::query_as::<_, ReadingProgressSnapshot>(
            "SELECT id, user_id, archive_id, current_page, total_pages, progress_percentage,
                    last_read_at, created_at, updated_at
             FROM reading_progress WHERE user_id = ? AND archive_id = ?",
        )
        .bind(&source_progress.user_id)
        .bind(keep_archive_id)
        .fetch_optional(&mut **tx)
        .await?;
        let current_page =
            ((source_progress.progress_percentage * f64::from(keeper_pages)).ceil() as i32).max(1);
        sqlx::query(
            "INSERT INTO reading_progress
                (id, user_id, archive_id, current_page, total_pages, progress_percentage, last_read_at, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(user_id, archive_id) DO UPDATE SET
                current_page = CASE WHEN excluded.progress_percentage > reading_progress.progress_percentage THEN excluded.current_page ELSE reading_progress.current_page END,
                total_pages = CASE WHEN excluded.progress_percentage > reading_progress.progress_percentage THEN excluded.total_pages ELSE reading_progress.total_pages END,
                progress_percentage = MAX(reading_progress.progress_percentage, excluded.progress_percentage),
                last_read_at = CASE WHEN excluded.progress_percentage > reading_progress.progress_percentage THEN excluded.last_read_at ELSE reading_progress.last_read_at END,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&source_progress.user_id)
        .bind(keep_archive_id)
        .bind(current_page)
        .bind(keeper_pages)
        .bind(source_progress.progress_percentage)
        .execute(&mut **tx)
        .await?;
        let after = sqlx::query_as::<_, ReadingProgressSnapshot>(
            "SELECT id, user_id, archive_id, current_page, total_pages, progress_percentage,
                    last_read_at, created_at, updated_at
             FROM reading_progress WHERE user_id = ? AND archive_id = ?",
        )
        .bind(&source_progress.user_id)
        .bind(keep_archive_id)
        .fetch_one(&mut **tx)
        .await?;
        progress.push(VersionProgressMigration {
            user_id: source_progress.user_id,
            before,
            after,
        });
    }

    Ok(VersionRelationMigration {
        version: 1,
        keeper_archive_id: keep_archive_id.to_string(),
        before_tag_ids,
        after_tag_ids,
        before_category_ids,
        after_category_ids,
        progress,
    })
}

async fn keeper_relation_ids(
    tx: &mut Transaction<'_, Sqlite>,
    table: &str,
    relation_column: &str,
    archive_column: &str,
    archive_id: &str,
) -> Result<Vec<String>> {
    let table = quote_identifier(table);
    let relation_column = quote_identifier(relation_column);
    let archive_column = quote_identifier(archive_column);
    let mut ids = sqlx::query_scalar::<_, String>(&format!(
        "SELECT {relation_column} FROM {table} WHERE {archive_column} = ? ORDER BY {relation_column}"
    ))
    .bind(archive_id)
    .fetch_all(&mut **tx)
    .await?;
    ids.sort();
    Ok(ids)
}

pub(super) async fn revert_version_relations(
    tx: &mut Transaction<'_, Sqlite>,
    migration: &VersionRelationMigration,
) -> Result<()> {
    let current_tag_ids = keeper_relation_ids(
        tx,
        "archive_tags",
        "tag_id",
        "archive_id",
        &migration.keeper_archive_id,
    )
    .await?;
    if current_tag_ids != migration.after_tag_ids {
        return Err(anyhow!(
            "version cleanup relation state changed since cleanup (tags)"
        ));
    }
    for tag_id in migration
        .after_tag_ids
        .iter()
        .filter(|id| !migration.before_tag_ids.contains(id))
    {
        sqlx::query("DELETE FROM archive_tags WHERE archive_id = ? AND tag_id = ?")
            .bind(&migration.keeper_archive_id)
            .bind(tag_id)
            .execute(&mut **tx)
            .await?;
    }

    let current_category_ids = keeper_relation_ids(
        tx,
        "category_archives",
        "category_id",
        "archive_id",
        &migration.keeper_archive_id,
    )
    .await?;
    if current_category_ids != migration.after_category_ids {
        return Err(anyhow!(
            "version cleanup relation state changed since cleanup (categories)"
        ));
    }
    for category_id in migration
        .after_category_ids
        .iter()
        .filter(|id| !migration.before_category_ids.contains(id))
    {
        sqlx::query("DELETE FROM category_archives WHERE category_id = ? AND archive_id = ?")
            .bind(category_id)
            .bind(&migration.keeper_archive_id)
            .execute(&mut **tx)
            .await?;
    }

    for progress in &migration.progress {
        let current = sqlx::query_as::<_, ReadingProgressSnapshot>(
            "SELECT id, user_id, archive_id, current_page, total_pages, progress_percentage,
                    last_read_at, created_at, updated_at
             FROM reading_progress WHERE user_id = ? AND archive_id = ?",
        )
        .bind(&progress.user_id)
        .bind(&migration.keeper_archive_id)
        .fetch_optional(&mut **tx)
        .await?;
        if current.as_ref() != Some(&progress.after) {
            return Err(anyhow!(
                "version cleanup relation state changed since cleanup (reading progress)"
            ));
        }
        if let Some(before) = &progress.before {
            sqlx::query(
                "UPDATE reading_progress SET id = ?, current_page = ?, total_pages = ?,
                        progress_percentage = ?, last_read_at = ?, created_at = ?, updated_at = ?
                 WHERE user_id = ? AND archive_id = ?",
            )
            .bind(&before.id)
            .bind(before.current_page)
            .bind(before.total_pages)
            .bind(before.progress_percentage)
            .bind(&before.last_read_at)
            .bind(&before.created_at)
            .bind(&before.updated_at)
            .bind(&progress.user_id)
            .bind(&migration.keeper_archive_id)
            .execute(&mut **tx)
            .await?;
        } else {
            sqlx::query("DELETE FROM reading_progress WHERE user_id = ? AND archive_id = ?")
                .bind(&progress.user_id)
                .bind(&migration.keeper_archive_id)
                .execute(&mut **tx)
                .await?;
        }
    }

    Ok(())
}
