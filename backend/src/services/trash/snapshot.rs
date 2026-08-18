use super::quote_identifier;
use super::{ArchiveSnapshot, TagSnapshot, TrashService};
use anyhow::{anyhow, Context, Result};
use sqlx::{Row, Sqlite, Transaction};
use std::path::{Path, PathBuf};

impl TrashService {
    pub(super) async fn load_snapshot(&self, archive_id: &str) -> Result<ArchiveSnapshot> {
        let row = sqlx::query(
            "SELECT id, title, subtitle, subtitle_language, path, file_hash, file_size, page_count, created_at, updated_at
             FROM archives WHERE id = ?",
        )
        .bind(archive_id)
        .fetch_optional(&self.pool)
        .await
        .context("failed to load archive for trash")?
        .ok_or_else(|| anyhow!("archive not found: {archive_id}"))?;

        let tag_rows = sqlx::query(
            "SELECT t.id, t.name, t.namespace FROM tags t
             INNER JOIN archive_tags at ON at.tag_id = t.id WHERE at.archive_id = ?",
        )
        .bind(archive_id)
        .fetch_all(&self.pool)
        .await
        .context("failed to load archive tags for trash")?;

        let (related_inserts, related_updates) = self.load_related_snapshots(archive_id).await?;

        Ok(ArchiveSnapshot {
            id: row.try_get("id")?,
            title: row.try_get("title")?,
            subtitle: row.try_get("subtitle")?,
            subtitle_language: row.try_get("subtitle_language")?,
            path: row.try_get("path")?,
            file_hash: row.try_get("file_hash")?,
            file_size: row.try_get("file_size")?,
            page_count: row.try_get("page_count")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            tags: tag_rows
                .into_iter()
                .map(|tag| {
                    Ok(TagSnapshot {
                        id: tag.try_get("id")?,
                        name: tag.try_get("name")?,
                        namespace: tag.try_get("namespace")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            related_inserts,
            related_updates,
            source: None,
            evidence_pages: Vec::new(),
            decision_key: None,
        })
    }

    pub(super) async fn load_related_snapshots(
        &self,
        archive_id: &str,
    ) -> Result<(Vec<String>, Vec<String>)> {
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list archive relation tables")?;

        let mut inserts = Vec::new();
        let mut updates = Vec::new();
        for table in tables {
            // Tags are captured separately in ArchiveSnapshot so restoring an
            // archive cannot attempt to insert the same archive_tags rows twice.
            if matches!(table.as_str(), "archives" | "archive_tags") {
                continue;
            }

            let table_sql = quote_identifier(&table);
            let foreign_keys = sqlx::query(&format!("PRAGMA foreign_key_list({table_sql})"))
                .fetch_all(&self.pool)
                .await
                .with_context(|| format!("failed to inspect foreign keys for {table}"))?;
            let archive_keys = foreign_keys
                .iter()
                .filter_map(|row| {
                    let referenced_table: String = row.try_get("table").ok()?;
                    if referenced_table != "archives" {
                        return None;
                    }
                    Some((
                        row.try_get::<String, _>("from").ok()?,
                        row.try_get::<String, _>("on_delete").ok()?,
                    ))
                })
                .collect::<Vec<_>>();
            if archive_keys.is_empty() {
                continue;
            }

            let column_rows = sqlx::query(&format!("PRAGMA table_info({table_sql})"))
                .fetch_all(&self.pool)
                .await
                .with_context(|| format!("failed to inspect columns for {table}"))?;
            let columns = column_rows
                .iter()
                .map(|row| row.try_get::<String, _>("name"))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let primary_key_columns = column_rows
                .iter()
                .filter_map(|row| {
                    let position: i64 = row.try_get("pk").ok()?;
                    if position == 0 {
                        return None;
                    }
                    Some((position, row.try_get::<String, _>("name").ok()?))
                })
                .collect::<Vec<_>>();
            if columns.is_empty() {
                continue;
            }

            let select_values = columns
                .iter()
                .map(|column| format!("quote({})", quote_identifier(column)))
                .collect::<Vec<_>>()
                .join(", ");
            let where_clause = archive_keys
                .iter()
                .map(|(column, _)| format!("{} = ?", quote_identifier(column)))
                .collect::<Vec<_>>()
                .join(" OR ");
            let query = format!("SELECT {select_values} FROM {table_sql} WHERE {where_clause}");
            let mut request = sqlx::query(&query);
            for _ in &archive_keys {
                request = request.bind(archive_id);
            }
            for row in request
                .fetch_all(&self.pool)
                .await
                .with_context(|| format!("failed to snapshot rows from {table}"))?
            {
                let values = (0..columns.len())
                    .map(|index| row.try_get::<String, _>(index))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                let cascade = archive_keys
                    .iter()
                    .any(|(_, action)| action.eq_ignore_ascii_case("CASCADE"));
                if cascade {
                    let columns_sql = columns
                        .iter()
                        .map(|column| quote_identifier(column))
                        .collect::<Vec<_>>()
                        .join(", ");
                    inserts.push(format!(
                        "INSERT INTO {table_sql} ({columns_sql}) VALUES ({})",
                        values.join(", ")
                    ));
                } else {
                    let where_columns = if primary_key_columns.is_empty() {
                        columns
                            .iter()
                            .enumerate()
                            .map(|(index, column)| (column.clone(), values[index].clone()))
                            .collect::<Vec<_>>()
                    } else {
                        let mut key_columns = primary_key_columns.clone();
                        key_columns.sort_by_key(|(position, _)| *position);
                        key_columns
                            .into_iter()
                            .filter_map(|(_, column)| {
                                let index = columns.iter().position(|value| value == &column)?;
                                Some((column, values[index].clone()))
                            })
                            .collect::<Vec<_>>()
                    };
                    let set_clause = archive_keys
                        .iter()
                        .map(|(column, _)| {
                            let index = columns.iter().position(|value| value == column).unwrap();
                            format!("{} = {}", quote_identifier(column), values[index])
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    let where_clause = where_columns
                        .iter()
                        .map(|(column, value)| {
                            if value == "NULL" {
                                format!("{} IS NULL", quote_identifier(column))
                            } else {
                                format!("{} = {value}", quote_identifier(column))
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" AND ");
                    updates.push(format!(
                        "UPDATE {table_sql} SET {set_clause} WHERE {where_clause}"
                    ));
                }
            }
        }

        Ok((inserts, updates))
    }
}

pub(super) async fn restore_archive_snapshot(
    tx: &mut Transaction<'_, Sqlite>,
    snapshot: &ArchiveSnapshot,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO archives
         (id, title, subtitle, subtitle_language, path, file_hash, file_size, page_count, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&snapshot.id)
    .bind(&snapshot.title)
    .bind(&snapshot.subtitle)
    .bind(&snapshot.subtitle_language)
    .bind(&snapshot.path)
    .bind(&snapshot.file_hash)
    .bind(snapshot.file_size)
    .bind(snapshot.page_count)
    .bind(&snapshot.created_at)
    .bind(&snapshot.updated_at)
    .execute(&mut **tx)
    .await
    .context("failed to restore archive record")?;

    for tag in &snapshot.tags {
        sqlx::query("INSERT OR IGNORE INTO tags (id, name, namespace) VALUES (?, ?, ?)")
            .bind(&tag.id)
            .bind(&tag.name)
            .bind(&tag.namespace)
            .execute(&mut **tx)
            .await
            .context("failed to restore archive tag")?;
        sqlx::query("INSERT OR IGNORE INTO archive_tags (archive_id, tag_id) VALUES (?, ?)")
            .bind(&snapshot.id)
            .bind(&tag.id)
            .execute(&mut **tx)
            .await
            .context("failed to restore archive tag relation")?;
    }

    // Archive deletion cascades through several optional feature tables
    // (reading progress, categories, collections, AI data, ...). These
    // statements restore the rows and references captured before deletion.
    for statement in &snapshot.related_inserts {
        sqlx::query(statement)
            .execute(&mut **tx)
            .await
            .context("failed to restore archive relations")?;
    }
    for statement in &snapshot.related_updates {
        sqlx::query(statement)
            .execute(&mut **tx)
            .await
            .context("failed to restore archive references")?;
    }

    Ok(())
}

pub(super) fn trash_path_for(original_path: &Path, entry_id: &str) -> Result<PathBuf> {
    let parent = original_path
        .parent()
        .ok_or_else(|| anyhow!("archive path has no parent directory"))?;
    let file_name = original_path
        .file_name()
        .ok_or_else(|| anyhow!("archive path has no file name"))?
        .to_string_lossy();
    Ok(parent
        .join(".otamoryx-trash")
        .join(format!("{entry_id}-{file_name}")))
}
