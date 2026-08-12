use anyhow::{Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::models::{
    Archive, CollectionDetail, CollectionMember, CollectionRebuildResponse, CollectionReviewItem,
    CollectionSummary, VersionCandidate, VersionCleanupResponse, VersionGroup,
};
use crate::services::ArchiveDeleteTarget;

const PARSER_VERSION: &str = "collections-v3";

#[derive(Debug, Clone)]
struct IdentityFact {
    archive_id: String,
    raw_filename: String,
    parent_path: String,
    normalized_key: String,
    display_title: String,
    creator: Option<String>,
    unit_type: String,
    volume_number: Option<String>,
    chapter_number: Option<String>,
    issue_number: Option<String>,
    raw_number: Option<String>,
    edition_marker: Option<String>,
    sort_key: f64,
    confidence: f64,
    evidence: Value,
}

#[derive(Debug, Clone)]
struct ArchiveRow {
    id: String,
    title: String,
    path: String,
}

pub async fn list_collections(
    pool: &Pool<Sqlite>,
    matching_archive_ids: Option<&[String]>,
    sort_by: Option<&str>,
    sort_order: Option<&str>,
) -> Result<Vec<CollectionSummary>> {
    if matching_archive_ids.is_some_and(|ids| ids.is_empty()) {
        return Ok(Vec::new());
    }
    let match_expression = matching_archive_ids.map(|ids| {
        format!(
            "SUM(CASE WHEN cm.archive_id IN ({}) THEN 1 ELSE 0 END)",
            ids.iter().map(|_| "?").collect::<Vec<_>>().join(",")
        )
    });
    let mut sql = String::from("SELECT c.id, c.display_title, c.subtitle, c.cover_archive_id, c.status, c.is_manual_locked, COUNT(cm.archive_id) AS member_count, COUNT(DISTINCT COALESCE(cm.variant_group_key, cm.archive_id)) AS content_count, (SELECT COUNT(*) FROM (SELECT COALESCE(variant_group_key, archive_id) AS unit_key FROM collection_members WHERE collection_id = c.id GROUP BY unit_key HAVING COUNT(*) > 1)) AS variant_group_count, COUNT(cm.archive_id) - COUNT(DISTINCT COALESCE(cm.variant_group_key, cm.archive_id)) AS variant_count, (SELECT COUNT(*) FROM collection_review_items r WHERE r.collection_id = c.id AND r.status = 'pending') AS review_count, ");
    sql.push_str(
        match_expression
            .as_deref()
            .unwrap_or("COUNT(cm.archive_id)"),
    );
    sql.push_str(" AS matched_member_count FROM collections c LEFT JOIN collection_members cm ON cm.collection_id = c.id GROUP BY c.id HAVING (COUNT(cm.archive_id) > 1 OR c.is_manual_locked = TRUE)");
    if let Some(expression) = &match_expression {
        sql.push_str(" AND ");
        sql.push_str(expression);
        sql.push_str(" > 0");
    }
    if let Some(sort_by) = sort_by {
        let sort_column = match sort_by {
            "title" => "c.display_title COLLATE NOCASE",
            "contentCount" => "content_count",
            "memberCount" => "member_count",
            _ => "c.updated_at",
        };
        let sort_direction = if sort_order.is_some_and(|value| value.eq_ignore_ascii_case("asc")) {
            "ASC"
        } else {
            "DESC"
        };
        sql.push_str(&format!(
            " ORDER BY {sort_column} {sort_direction}, c.display_title COLLATE NOCASE, c.id"
        ));
    } else {
        sql.push_str(
            " ORDER BY CASE
                WHEN c.status = 'auto' AND c.is_manual_locked = FALSE THEN 0
                WHEN c.status = 'manual' OR c.is_manual_locked = TRUE THEN 1
                WHEN c.status = 'needs_review' THEN 2
                ELSE 1
              END, c.display_title COLLATE NOCASE, c.id",
        );
    }
    let mut request = sqlx::query(&sql);
    if let Some(ids) = matching_archive_ids {
        for id in ids {
            request = request.bind(id);
        }
        for id in ids {
            request = request.bind(id);
        }
    }
    let rows = request
        .fetch_all(pool)
        .await
        .context("Failed to list collections")?;
    Ok(rows.into_iter().map(summary_from_row).collect())
}

pub async fn get_collection(pool: &Pool<Sqlite>, id: &str) -> Result<Option<CollectionDetail>> {
    let row = sqlx::query(
        "SELECT c.id, c.display_title, c.subtitle, c.cover_archive_id, c.status, c.is_manual_locked,
                COUNT(cm.archive_id) AS member_count,
                COUNT(DISTINCT COALESCE(cm.variant_group_key, cm.archive_id)) AS content_count,
                (SELECT COUNT(*) FROM (SELECT COALESCE(variant_group_key, archive_id) AS unit_key FROM collection_members WHERE collection_id = c.id GROUP BY unit_key HAVING COUNT(*) > 1)) AS variant_group_count,
                COUNT(cm.archive_id) - COUNT(DISTINCT COALESCE(cm.variant_group_key, cm.archive_id)) AS variant_count,
                (SELECT COUNT(*) FROM collection_review_items r WHERE r.collection_id = c.id AND r.status = 'pending') AS review_count,
                COUNT(cm.archive_id) AS matched_member_count
         FROM collections c
         LEFT JOIN collection_members cm ON cm.collection_id = c.id
         WHERE c.id = ?
         GROUP BY c.id",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("Failed to fetch collection")?;
    let Some(row) = row else { return Ok(None) };

    let summary = summary_from_row(row);
    let member_rows = sqlx::query(
        "SELECT cm.archive_id, cm.unit_type, cm.volume_number, cm.chapter_number,
                cm.issue_number, cm.raw_number, cm.sort_key, cm.variant_group_key,
                cm.confidence, cm.membership_source, cm.is_manual_locked
         FROM collection_members cm
         WHERE cm.collection_id = ?
         ORDER BY cm.sort_key, cm.archive_id",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .context("Failed to fetch collection members")?;

    let mut members = Vec::with_capacity(member_rows.len());
    for member_row in member_rows {
        let archive_id: String = member_row.get("archive_id");
        let Some(archive) = load_archive(pool, &archive_id).await? else {
            continue;
        };
        members.push(CollectionMember {
            archive,
            matches_filter: false,
            unit_type: member_row.get("unit_type"),
            volume_number: member_row.get("volume_number"),
            chapter_number: member_row.get("chapter_number"),
            issue_number: member_row.get("issue_number"),
            raw_number: member_row.get("raw_number"),
            sort_key: member_row.get("sort_key"),
            variant_group_key: member_row.get("variant_group_key"),
            confidence: member_row.get("confidence"),
            membership_source: member_row.get("membership_source"),
            is_manual_locked: member_row.get("is_manual_locked"),
        });
    }

    Ok(Some(CollectionDetail {
        collection: summary,
        members,
    }))
}

pub async fn list_review_items(pool: &Pool<Sqlite>) -> Result<Vec<CollectionReviewItem>> {
    let rows = sqlx::query(
        "SELECT r.id, r.archive_id, r.collection_id, r.reason, r.evidence_json, r.status
         FROM collection_review_items r
         WHERE r.status = 'pending'
         ORDER BY r.updated_at DESC",
    )
    .fetch_all(pool)
    .await
    .context("Failed to list collection reviews")?;

    let mut items = Vec::new();
    for row in rows {
        let archive_id: String = row.get("archive_id");
        let collection_id: String = row.get("collection_id");
        let Some(archive) = load_archive(pool, &archive_id).await? else {
            continue;
        };
        let Some(collection) = get_collection_summary(pool, &collection_id).await? else {
            continue;
        };
        let evidence_json: String = row.get("evidence_json");
        items.push(CollectionReviewItem {
            id: row.get("id"),
            archive,
            collection,
            reason: row.get("reason"),
            evidence: serde_json::from_str(&evidence_json).unwrap_or(Value::Null),
            status: row.get("status"),
        });
    }
    Ok(items)
}

pub async fn collection_member_delete_targets(
    pool: &Pool<Sqlite>,
    collection_id: &str,
) -> Result<Vec<ArchiveDeleteTarget>> {
    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM collections WHERE id = ?")
        .bind(collection_id)
        .fetch_one(pool)
        .await
        .context("Failed to find collection")?;
    if exists == 0 {
        return Err(anyhow::anyhow!("collection not found"));
    }

    let rows = sqlx::query(
        "SELECT a.id, a.path
         FROM collection_members cm
         JOIN archives a ON a.id = cm.archive_id
         WHERE cm.collection_id = ?
         ORDER BY cm.sort_key, cm.archive_id",
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await
    .context("Failed to load collection members for deletion")?;

    Ok(rows
        .into_iter()
        .map(|row| ArchiveDeleteTarget {
            id: row.get("id"),
            path: row.get("path"),
        })
        .collect())
}

pub async fn delete_collection(pool: &Pool<Sqlite>, collection_id: &str) -> Result<()> {
    let deleted = sqlx::query("DELETE FROM collections WHERE id = ?")
        .bind(collection_id)
        .execute(pool)
        .await
        .context("Failed to delete collection")?
        .rows_affected();
    if deleted == 0 {
        return Err(anyhow::anyhow!("collection not found"));
    }
    Ok(())
}

pub async fn rebuild_collections(pool: &Pool<Sqlite>) -> Result<CollectionRebuildResponse> {
    let rows = sqlx::query("SELECT id, title, subtitle, path FROM archives ORDER BY id")
        .fetch_all(pool)
        .await
        .context("Failed to load archives for collection rebuild")?;

    let mut facts = Vec::with_capacity(rows.len());
    let mut archive_subtitles = HashMap::new();
    for row in rows {
        let archive = ArchiveRow {
            id: row.get("id"),
            title: row.get("title"),
            path: row.get("path"),
        };
        archive_subtitles.insert(archive.id.clone(), row.get::<Option<String>, _>("subtitle"));
        facts.push(parse_identity(&archive));
    }
    infer_missing_first_numbers(&mut facts);

    let mut tx = pool
        .begin()
        .await
        .context("Failed to start collection rebuild")?;
    for fact in &facts {
        sqlx::query(
            "INSERT INTO archive_identity_facts
                (archive_id, raw_filename, parent_path, normalized_key, display_title, creator,
                 unit_type, volume_number, chapter_number, issue_number, raw_number,
                 edition_marker, content_unit_key, confidence, evidence_json, parser_version, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(archive_id) DO UPDATE SET
                raw_filename = excluded.raw_filename,
                parent_path = excluded.parent_path,
                normalized_key = excluded.normalized_key,
                display_title = excluded.display_title,
                creator = excluded.creator,
                unit_type = excluded.unit_type,
                volume_number = excluded.volume_number,
                chapter_number = excluded.chapter_number,
                issue_number = excluded.issue_number,
                raw_number = excluded.raw_number,
                edition_marker = excluded.edition_marker,
                content_unit_key = excluded.content_unit_key,
                confidence = excluded.confidence,
                evidence_json = excluded.evidence_json,
                parser_version = excluded.parser_version,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(&fact.archive_id)
        .bind(&fact.raw_filename)
        .bind(&fact.parent_path)
        .bind(&fact.normalized_key)
        .bind(&fact.display_title)
        .bind(&fact.creator)
        .bind(&fact.unit_type)
        .bind(&fact.volume_number)
        .bind(&fact.chapter_number)
        .bind(&fact.issue_number)
        .bind(&fact.raw_number)
        .bind(&fact.edition_marker)
        .bind(content_unit_key(fact))
        .bind(fact.confidence)
        .bind(fact.evidence.to_string())
        .bind(PARSER_VERSION)
        .execute(&mut *tx)
        .await
        .context("Failed to save collection identity fact")?;
    }

    sqlx::query(
        "DELETE FROM collection_review_items
         WHERE status = 'pending' AND collection_id IN
             (SELECT id FROM collections WHERE is_manual_locked = FALSE)",
    )
    .execute(&mut *tx)
    .await
    .context("Failed to clear stale collection reviews")?;
    sqlx::query(
        "DELETE FROM collection_members
         WHERE membership_source = 'auto' AND is_manual_locked = FALSE
           AND collection_id IN (SELECT id FROM collections WHERE is_manual_locked = FALSE)",
    )
    .execute(&mut *tx)
    .await
    .context("Failed to clear auto collection members")?;
    sqlx::query(
        "DELETE FROM collections WHERE is_manual_locked = FALSE
         AND NOT EXISTS (SELECT 1 FROM collection_members cm WHERE cm.collection_id = collections.id)
         AND NOT EXISTS (SELECT 1 FROM collection_exclusions ce WHERE ce.collection_id = collections.id)",
    )
    .execute(&mut *tx)
    .await
    .context("Failed to remove empty collections")?;

    let mut grouped = HashMap::<String, Vec<IdentityFact>>::new();
    for fact in facts.iter().cloned() {
        grouped.entry(work_group_key(&fact)).or_default().push(fact);
    }

    let mut created_collections = 0i64;
    let mut grouped_archives = 0i64;
    let mut pending_reviews = 0i64;
    for (group_key, mut group) in grouped {
        // A collection represents a sequence of distinct content units. Several
        // files for the same unit are versions, not a one-item collection.
        let unit_keys = group.iter().map(version_group_key).collect::<HashSet<_>>();
        if unit_keys.len() < 2 {
            continue;
        }
        group.sort_by(|left, right| left.sort_key.total_cmp(&right.sort_key));
        let collection_id: String =
            sqlx::query_scalar("SELECT id FROM collections WHERE normalized_key = ? LIMIT 1")
                .bind(&group_key)
                .fetch_optional(&mut *tx)
                .await
                .context("Failed to find existing collection")?
                .unwrap_or_else(|| collection_id_for_key(&group_key));
        let title = group
            .first()
            .map(|fact| clean_title_for_key(&fact.display_title, &fact.unit_type))
            .unwrap_or_else(|| "未命名合集".to_string());
        let subtitle = common_collection_subtitle(&group, &archive_subtitles);
        let cover_archive_id = group.first().map(|fact| fact.archive_id.clone());
        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO collections
                (id, display_title, subtitle, normalized_key, cover_archive_id, status)
             VALUES (?, ?, ?, ?, ?, 'auto')",
        )
        .bind(&collection_id)
        .bind(&title)
        .bind(&subtitle)
        .bind(&group_key)
        .bind(&cover_archive_id)
        .execute(&mut *tx)
        .await
        .context("Failed to create collection")?
        .rows_affected();
        created_collections += i64::from(inserted > 0);

        if subtitle.is_some() {
            sqlx::query(
                "UPDATE collections SET subtitle = ?, updated_at = CURRENT_TIMESTAMP
                 WHERE id = ? AND subtitle IS NULL AND is_manual_locked = FALSE",
            )
            .bind(&subtitle)
            .bind(&collection_id)
            .execute(&mut *tx)
            .await?;
        }

        let needs_review = group.iter().any(|fact| fact.confidence < 0.75);
        if needs_review {
            sqlx::query("UPDATE collections SET status = 'needs_review', updated_at = CURRENT_TIMESTAMP WHERE id = ? AND is_manual_locked = FALSE")
                .bind(&collection_id)
                .execute(&mut *tx)
                .await?;
        }

        for fact in group {
            let variant_group_key = Some(content_unit_key(&fact));
            let inserted = sqlx::query(
                "INSERT OR IGNORE INTO collection_members
                    (collection_id, archive_id, unit_type, volume_number, chapter_number,
                     issue_number, raw_number, sort_key, variant_group_key, confidence, membership_source)
                 SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'auto'
                 WHERE NOT EXISTS (SELECT 1 FROM collection_exclusions WHERE archive_id = ? AND collection_id = ?)",
            )
            .bind(&collection_id)
            .bind(&fact.archive_id)
            .bind(&fact.unit_type)
            .bind(&fact.volume_number)
            .bind(&fact.chapter_number)
            .bind(&fact.issue_number)
            .bind(&fact.raw_number)
            .bind(fact.sort_key)
            .bind(&variant_group_key)
            .bind(fact.confidence)
            .bind(&fact.archive_id)
            .bind(&collection_id)
            .execute(&mut *tx)
            .await
            .context("Failed to add collection member")?
            .rows_affected();
            grouped_archives += i64::from(inserted > 0);

            if needs_review {
                let review_id = Uuid::new_v4().to_string();
                let reason = if fact.raw_number.is_none() {
                    "标题相同，但没有明确的卷号或话号；可能是不同版本"
                } else {
                    "编号来自启发式文件名解析，需要确认是否属于同一合集"
                };
                let result = sqlx::query(
                    "INSERT OR IGNORE INTO collection_review_items
                        (id, archive_id, collection_id, reason, evidence_json)
                     VALUES (?, ?, ?, ?, ?)",
                )
                .bind(review_id)
                .bind(&fact.archive_id)
                .bind(&collection_id)
                .bind(reason)
                .bind(fact.evidence.to_string())
                .execute(&mut *tx)
                .await
                .context("Failed to create collection review")?;
                pending_reviews += i64::from(result.rows_affected() > 0);
            }
        }
    }

    tx.commit()
        .await
        .context("Failed to commit collection rebuild")?;
    Ok(CollectionRebuildResponse {
        parsed_archives: facts.len() as i64,
        created_collections,
        grouped_archives,
        pending_reviews,
    })
}

pub async fn add_member(pool: &Pool<Sqlite>, collection_id: &str, archive_id: &str) -> Result<()> {
    let fact = sqlx::query("SELECT unit_type, volume_number, chapter_number, issue_number, confidence FROM archive_identity_facts WHERE archive_id = ?")
        .bind(archive_id).fetch_optional(pool).await?.ok_or_else(|| anyhow::anyhow!("archive identity has not been parsed"))?;
    let unit_type: String = fact.get("unit_type");
    let volume_number: Option<String> = fact.get("volume_number");
    let chapter_number: Option<String> = fact.get("chapter_number");
    let issue_number: Option<String> = fact.get("issue_number");
    let confidence: f64 = fact.get("confidence");
    let sort_key = calculate_sort_key(
        &unit_type,
        volume_number.as_deref(),
        chapter_number.as_deref(),
    );
    sqlx::query("INSERT INTO collection_members (collection_id, archive_id, unit_type, volume_number, chapter_number, issue_number, sort_key, confidence, membership_source, is_manual_locked) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'manual', TRUE) ON CONFLICT(archive_id) DO UPDATE SET collection_id = excluded.collection_id, sort_key = excluded.sort_key, confidence = 1, membership_source = 'manual', is_manual_locked = TRUE, updated_at = CURRENT_TIMESTAMP")
        .bind(collection_id).bind(archive_id).bind(unit_type).bind(volume_number).bind(chapter_number).bind(issue_number).bind(sort_key).bind(confidence)
        .execute(pool).await?;
    sqlx::query("UPDATE collections SET is_manual_locked = TRUE, status = 'manual', updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(collection_id).execute(pool).await?;
    Ok(())
}

pub async fn remove_member(pool: &Pool<Sqlite>, archive_id: &str) -> Result<()> {
    let collection_id = sqlx::query_scalar::<_, String>(
        "SELECT collection_id FROM collection_members WHERE archive_id = ?",
    )
    .bind(archive_id)
    .fetch_optional(pool)
    .await?;
    let Some(collection_id) = collection_id else {
        return Ok(());
    };
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM collection_members WHERE archive_id = ?")
        .bind(archive_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT OR IGNORE INTO collection_exclusions (archive_id, collection_id) VALUES (?, ?)",
    )
    .bind(archive_id)
    .bind(collection_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn update_collection(
    pool: &Pool<Sqlite>,
    id: &str,
    title: Option<&str>,
    subtitle: Option<&str>,
    locked: Option<bool>,
) -> Result<()> {
    if let Some(title) = title.filter(|value| !value.trim().is_empty()) {
        sqlx::query(
            "UPDATE collections SET display_title = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(title.trim())
        .bind(id)
        .execute(pool)
        .await?;
    }
    if let Some(subtitle) = subtitle.map(str::trim) {
        sqlx::query(
            "UPDATE collections SET subtitle = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(if subtitle.is_empty() {
            None
        } else {
            Some(subtitle)
        })
        .bind(id)
        .execute(pool)
        .await?;
    }
    if let Some(locked) = locked {
        sqlx::query("UPDATE collections SET is_manual_locked = ?, status = CASE WHEN ? THEN 'manual' ELSE status END, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(locked).bind(locked).bind(id).execute(pool).await?;
    }
    Ok(())
}

pub async fn apply_review(pool: &Pool<Sqlite>, review_id: &str, action: &str) -> Result<()> {
    let row = sqlx::query("SELECT archive_id, collection_id FROM collection_review_items WHERE id = ? AND status = 'pending'")
        .bind(review_id).fetch_optional(pool).await?.ok_or_else(|| anyhow::anyhow!("review item not found"))?;
    let archive_id: String = row.get("archive_id");
    let collection_id: String = row.get("collection_id");
    match action {
        "approve" => {
            sqlx::query("UPDATE collection_members SET membership_source = 'manual', is_manual_locked = TRUE, confidence = 1, updated_at = CURRENT_TIMESTAMP WHERE archive_id = ? AND collection_id = ?")
                .bind(&archive_id).bind(&collection_id).execute(pool).await?;
            sqlx::query("UPDATE collections SET status = 'manual', is_manual_locked = TRUE, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                .bind(&collection_id).execute(pool).await?;
        }
        "reject" => {
            sqlx::query("DELETE FROM collection_members WHERE archive_id = ? AND collection_id = ? AND membership_source = 'auto'")
                .bind(&archive_id).bind(&collection_id).execute(pool).await?;
            sqlx::query("INSERT OR IGNORE INTO collection_exclusions (archive_id, collection_id) VALUES (?, ?)")
                .bind(&archive_id).bind(&collection_id).execute(pool).await?;
        }
        _ => return Err(anyhow::anyhow!("unsupported review action")),
    }
    sqlx::query("UPDATE collection_review_items SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(if action == "approve" { "approved" } else { "rejected" })
        .bind(review_id).execute(pool).await?;
    Ok(())
}

pub async fn collection_progress(
    pool: &Pool<Sqlite>,
    collection_id: &str,
    user_id: &str,
) -> Result<Option<f64>> {
    let progress = sqlx::query_scalar::<_, Option<f64>>(
        "SELECT AVG(unit_progress) FROM (
             SELECT MAX(COALESCE(rp.progress_percentage, 0)) AS unit_progress
             FROM collection_members cm
             LEFT JOIN reading_progress rp ON rp.archive_id = cm.archive_id AND rp.user_id = ?
             WHERE cm.collection_id = ?
             GROUP BY COALESCE(cm.variant_group_key, cm.archive_id)
         )",
    )
    .bind(user_id)
    .bind(collection_id)
    .fetch_one(pool)
    .await?;
    Ok(progress)
}

pub async fn list_version_groups(
    pool: &Pool<Sqlite>,
    matching_archive_ids: Option<&HashSet<String>>,
    sort_by: Option<&str>,
    sort_order: Option<&str>,
) -> Result<Vec<VersionGroup>> {
    let rows = sqlx::query(
        "SELECT f.archive_id, f.raw_filename, f.parent_path, f.normalized_key, f.display_title,
                f.creator, f.unit_type, f.volume_number, f.chapter_number, f.issue_number,
                f.raw_number, f.edition_marker, f.content_unit_key, f.confidence, f.evidence_json,
                cm.collection_id, c.display_title AS collection_title
         FROM archive_identity_facts f
         JOIN archives a ON a.id = f.archive_id
         LEFT JOIN collection_members cm ON cm.archive_id = f.archive_id
         LEFT JOIN collections c ON c.id = cm.collection_id
         ORDER BY f.normalized_key, f.archive_id",
    )
    .fetch_all(pool)
    .await
    .context("Failed to load version candidates")?;

    let mut grouped =
        HashMap::<String, Vec<(IdentityFact, Archive, Option<String>, Option<String>)>>::new();
    for row in rows {
        let archive_id: String = row.get("archive_id");
        let Some(archive) = load_archive(pool, &archive_id).await? else {
            continue;
        };
        let evidence_json: String = row.get("evidence_json");
        let fact = IdentityFact {
            archive_id,
            raw_filename: row.get("raw_filename"),
            parent_path: row.get("parent_path"),
            normalized_key: row.get("normalized_key"),
            display_title: row.get("display_title"),
            creator: row.get("creator"),
            unit_type: row.get("unit_type"),
            volume_number: row.get("volume_number"),
            chapter_number: row.get("chapter_number"),
            issue_number: row.get("issue_number"),
            raw_number: row.get("raw_number"),
            edition_marker: row.get("edition_marker"),
            sort_key: 0.0,
            confidence: row.get("confidence"),
            evidence: serde_json::from_str(&evidence_json).unwrap_or(Value::Null),
        };
        let content_key: String = row
            .try_get("content_unit_key")
            .ok()
            .flatten()
            .unwrap_or_else(|| content_unit_key(&fact));
        grouped.entry(content_key).or_default().push((
            fact,
            archive,
            row.get("collection_id"),
            row.get("collection_title"),
        ));
    }

    let mut groups = Vec::new();
    for (group_key, mut entries) in grouped {
        if entries.len() < 2 {
            continue;
        }
        entries.sort_by(|left, right| {
            right
                .1
                .page_count
                .cmp(&left.1.page_count)
                .then_with(|| right.1.file_size.cmp(&left.1.file_size))
        });
        let display_title = entries[0].0.display_title.clone();
        let matched_member_count = matching_archive_ids
            .map(|ids| {
                entries
                    .iter()
                    .filter(|entry| ids.contains(&entry.1.id))
                    .count()
            })
            .unwrap_or(entries.len());
        if matching_archive_ids.is_some() && matched_member_count == 0 {
            continue;
        }
        let status = sqlx::query_scalar::<_, String>(
            "SELECT status FROM version_group_decisions WHERE group_key = ?",
        )
        .bind(&group_key)
        .fetch_optional(pool)
        .await?
        .unwrap_or_else(|| "active".to_string());
        let confidence = entries
            .iter()
            .map(|entry| entry.0.confidence)
            .fold(1.0, f64::min);
        let recommendation = recommend_version(&entries, confidence, &status);
        let recommended_archive_id = recommendation.as_ref().map(|value| value.0.clone());
        let reclaimable_size = recommended_archive_id
            .as_ref()
            .and_then(|id| entries.iter().find(|entry| entry.1.id == *id))
            .map(|keeper| {
                entries.iter().map(|entry| entry.1.file_size).sum::<i64>() - keeper.1.file_size
            })
            .unwrap_or(0);
        let collection_id = entries.iter().find_map(|entry| entry.2.clone());
        let collection_title = entries.iter().find_map(|entry| entry.3.clone());
        let subtitle = entries.iter().find_map(|entry| entry.1.subtitle.clone());
        let unit_label = unit_label(&entries[0].0);
        let members: Vec<VersionCandidate> = entries
            .into_iter()
            .map(|(fact, archive, _, _)| VersionCandidate {
                matches_filter: matching_archive_ids.is_some_and(|ids| ids.contains(&archive.id)),
                is_recommended: recommended_archive_id.as_deref() == Some(archive.id.as_str()),
                recommendation_reasons: recommendation
                    .as_ref()
                    .filter(|value| value.0 == archive.id)
                    .map(|value| value.1.clone())
                    .unwrap_or_default(),
                archive,
                confidence: fact.confidence,
            })
            .collect();
        groups.push(VersionGroup {
            id: version_group_id(&group_key),
            group_key,
            display_title,
            subtitle,
            collection_id,
            collection_title,
            unit_label,
            confidence,
            status,
            recommended_archive_id,
            reclaimable_size,
            matched_member_count: matched_member_count as i64,
            members,
        });
    }
    groups.sort_by(|left, right| {
        let ordering = match sort_by {
            Some("reclaimableSize") => left.reclaimable_size.cmp(&right.reclaimable_size),
            Some("memberCount") => left.members.len().cmp(&right.members.len()),
            Some("createdAt") => left
                .members
                .iter()
                .map(|member| member.archive.created_at)
                .max()
                .cmp(
                    &right
                        .members
                        .iter()
                        .map(|member| member.archive.created_at)
                        .max(),
                ),
            Some(_) => left.display_title.cmp(&right.display_title),
            None => version_priority(left).cmp(&version_priority(right)),
        };
        if sort_by
            .is_some_and(|_| sort_order.is_some_and(|value| value.eq_ignore_ascii_case("asc")))
        {
            ordering
        } else if sort_by.is_some() {
            ordering.reverse()
        } else {
            ordering
        }
        .then_with(|| left.display_title.cmp(&right.display_title))
    });
    Ok(groups)
}

pub async fn keep_all_versions(pool: &Pool<Sqlite>, id: &str) -> Result<()> {
    let group = list_version_groups(pool, None, None, None)
        .await?
        .into_iter()
        .find(|group| group.id == id)
        .ok_or_else(|| anyhow::anyhow!("version group not found"))?;
    sqlx::query(
        "INSERT INTO version_group_decisions (group_key, status, updated_at)
         VALUES (?, 'keep_all', CURRENT_TIMESTAMP)
         ON CONFLICT(group_key) DO UPDATE SET status = excluded.status, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(group.group_key)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn restore_version_group(pool: &Pool<Sqlite>, id: &str) -> Result<()> {
    let group = list_version_groups(pool, None, None, None)
        .await?
        .into_iter()
        .find(|group| group.id == id)
        .ok_or_else(|| anyhow::anyhow!("version group not found"))?;
    sqlx::query("DELETE FROM version_group_decisions WHERE group_key = ?")
        .bind(group.group_key)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn cleanup_versions(
    pool: &Pool<Sqlite>,
    archive_cache: &std::sync::Arc<crate::services::ArchiveCacheService>,
    group_id: &str,
    keep_archive_id: &str,
    delete_archive_ids: &[String],
) -> Result<VersionCleanupResponse> {
    let group = list_version_groups(pool, None, None, None)
        .await?
        .into_iter()
        .find(|group| group.id == group_id)
        .ok_or_else(|| anyhow::anyhow!("version group not found"))?;
    if group
        .members
        .iter()
        .all(|member| member.archive.id != keep_archive_id)
    {
        return Err(anyhow::anyhow!(
            "keeper does not belong to the version group"
        ));
    }
    let expected_deletions = group
        .members
        .iter()
        .filter(|member| member.archive.id != keep_archive_id)
        .map(|member| member.archive.id.clone())
        .collect::<HashSet<_>>();
    let requested_deletions = delete_archive_ids.iter().cloned().collect::<HashSet<_>>();
    if expected_deletions.is_empty() || expected_deletions != requested_deletions {
        return Err(anyhow::anyhow!(
            "cleanup request no longer matches the version group"
        ));
    }
    let keeper_pages = group
        .members
        .iter()
        .find(|member| member.archive.id == keep_archive_id)
        .map(|member| member.archive.page_count)
        .unwrap_or(0);
    let mut failed_archive_ids = Vec::new();
    let mut deleted = 0;
    for member in group
        .members
        .into_iter()
        .filter(|member| member.archive.id != keep_archive_id)
    {
        match migrate_and_delete_version(pool, &member.archive, keep_archive_id, keeper_pages).await
        {
            Ok(()) => {
                deleted += 1;
                archive_cache.clear_archive_cache(&member.archive.id).await;
            }
            Err(error) => {
                tracing::error!(archive_id = %member.archive.id, "Failed to clean up version: {error:#}");
                failed_archive_ids.push(member.archive.id);
            }
        }
    }
    Ok(VersionCleanupResponse {
        kept_archive_id: keep_archive_id.to_string(),
        deleted,
        failed_archive_ids,
    })
}

async fn migrate_and_delete_version(
    pool: &Pool<Sqlite>,
    archive: &Archive,
    keep_archive_id: &str,
    keeper_pages: i32,
) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT OR IGNORE INTO archive_tags (archive_id, tag_id)
         SELECT ?, tag_id FROM archive_tags WHERE archive_id = ?",
    )
    .bind(keep_archive_id)
    .bind(&archive.id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT OR IGNORE INTO category_archives (category_id, archive_id)
         SELECT category_id, ? FROM category_archives WHERE archive_id = ?",
    )
    .bind(keep_archive_id)
    .bind(&archive.id)
    .execute(&mut *tx)
    .await?;
    let progress_rows = sqlx::query(
        "SELECT user_id, progress_percentage FROM reading_progress WHERE archive_id = ?",
    )
    .bind(&archive.id)
    .fetch_all(&mut *tx)
    .await?;
    for row in progress_rows {
        let user_id: String = row.get("user_id");
        let progress: f64 = row.get("progress_percentage");
        let current_page = ((progress * f64::from(keeper_pages)).ceil() as i32).max(1);
        sqlx::query(
            "INSERT INTO reading_progress
                (id, user_id, archive_id, current_page, total_pages, progress_percentage, last_read_at, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(user_id, archive_id) DO UPDATE SET
                current_page = CASE WHEN excluded.progress_percentage > reading_progress.progress_percentage THEN excluded.current_page ELSE reading_progress.current_page END,
                total_pages = CASE WHEN excluded.progress_percentage > reading_progress.progress_percentage THEN excluded.total_pages ELSE reading_progress.total_pages END,
                progress_percentage = MAX(reading_progress.progress_percentage, excluded.progress_percentage),
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(keep_archive_id)
        .bind(current_page)
        .bind(keeper_pages)
        .bind(progress)
        .execute(&mut *tx)
        .await?;
    }
    let result = sqlx::query("DELETE FROM archives WHERE id = ?")
        .bind(&archive.id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() != 1 {
        return Err(anyhow::anyhow!("archive no longer exists"));
    }
    crate::services::delete_archive_file(&archive.path)
        .await
        .context("Failed to delete version file")?;
    tx.commit().await?;
    Ok(())
}

async fn get_collection_summary(
    pool: &Pool<Sqlite>,
    id: &str,
) -> Result<Option<CollectionSummary>> {
    let row = sqlx::query(
        "SELECT c.id, c.display_title, c.subtitle, c.cover_archive_id, c.status, c.is_manual_locked,
                COUNT(cm.archive_id) AS member_count,
                COUNT(DISTINCT COALESCE(cm.variant_group_key, cm.archive_id)) AS content_count,
                (SELECT COUNT(*) FROM (SELECT COALESCE(variant_group_key, archive_id) AS unit_key FROM collection_members WHERE collection_id = c.id GROUP BY unit_key HAVING COUNT(*) > 1)) AS variant_group_count,
                COUNT(cm.archive_id) - COUNT(DISTINCT COALESCE(cm.variant_group_key, cm.archive_id)) AS variant_count,
                (SELECT COUNT(*) FROM collection_review_items r WHERE r.collection_id = c.id AND r.status = 'pending') AS review_count,
                COUNT(cm.archive_id) AS matched_member_count
         FROM collections c LEFT JOIN collection_members cm ON cm.collection_id = c.id
         WHERE c.id = ? GROUP BY c.id",
    )
    .bind(id).fetch_optional(pool).await?;
    Ok(row.map(summary_from_row))
}

fn summary_from_row(row: sqlx::sqlite::SqliteRow) -> CollectionSummary {
    CollectionSummary {
        id: row.get("id"),
        display_title: row.get("display_title"),
        subtitle: row.get("subtitle"),
        cover_archive_id: row.get("cover_archive_id"),
        status: row.get("status"),
        is_manual_locked: row.get("is_manual_locked"),
        member_count: row.get("member_count"),
        content_count: row.get("content_count"),
        variant_group_count: row.get("variant_group_count"),
        variant_count: row.get("variant_count"),
        review_count: row.get("review_count"),
        matched_member_count: row.get("matched_member_count"),
        progress_percentage: None,
    }
}

fn collection_id_for_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let digest = hasher.finalize();
    let suffix = digest
        .iter()
        .take(12)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("auto-{suffix}")
}

fn work_group_key(fact: &IdentityFact) -> String {
    let creator_key = fact
        .creator
        .as_deref()
        .map(normalize_text)
        .unwrap_or_default();
    if creator_key.is_empty() {
        fact.normalized_key.clone()
    } else {
        format!("{}::{creator_key}", fact.normalized_key)
    }
}

fn version_group_key(fact: &IdentityFact) -> String {
    content_unit_key(fact)
}

fn content_unit_key(fact: &IdentityFact) -> String {
    let unit_number = fact.raw_number.as_deref().unwrap_or("standalone");
    format!(
        "{}::{}::{unit_number}",
        work_group_key(fact),
        fact.unit_type
    )
}

// A lone title without a number alongside a sequence starting at 2 is commonly
// the omitted first part. Keep this deliberately conservative and mark it for review.
fn infer_missing_first_numbers(facts: &mut [IdentityFact]) {
    let mut groups = HashMap::<String, Vec<usize>>::new();
    for (index, fact) in facts.iter().enumerate() {
        groups.entry(work_group_key(fact)).or_default().push(index);
    }

    for indexes in groups.values() {
        let unnumbered = indexes
            .iter()
            .copied()
            .filter(|index| {
                facts[*index].unit_type == "standalone" && facts[*index].raw_number.is_none()
            })
            .collect::<Vec<_>>();
        if unnumbered.len() != 1 {
            continue;
        }
        let numbered = indexes
            .iter()
            .filter_map(|index| {
                facts[*index]
                    .raw_number
                    .as_deref()
                    .and_then(|number| number.parse::<u32>().ok())
            })
            .collect::<Vec<_>>();
        if numbered.is_empty() || numbered.iter().copied().min() != Some(2) || numbered.contains(&1)
        {
            continue;
        }

        let fact = &mut facts[unnumbered[0]];
        fact.unit_type = "unknown".to_string();
        fact.raw_number = Some("1".to_string());
        fact.sort_key = calculate_sort_key("unknown", None, Some("1"));
        fact.confidence = fact.confidence.min(0.55);
        if let Some(evidence) = fact.evidence.as_object_mut() {
            evidence.insert("inferredNumber".to_string(), json!(1));
            evidence.insert("numberSource".to_string(), json!("inferred_missing_first"));
        }
    }
}

fn version_priority(group: &VersionGroup) -> u8 {
    if group.status == "keep_all" {
        2
    } else if group.recommended_archive_id.is_some() {
        0
    } else {
        1
    }
}

fn version_group_id(group_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(group_key.as_bytes());
    let digest = hasher.finalize();
    let suffix = digest
        .iter()
        .take(12)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("versions-{suffix}")
}

fn unit_label(fact: &IdentityFact) -> String {
    if let Some(number) = fact.volume_number.as_deref() {
        return format!("第 {number} 卷");
    }
    if let Some(number) = fact.chapter_number.as_deref() {
        return format!("第 {number} 话");
    }
    if let Some(number) = fact.issue_number.as_deref() {
        return format!("期号 {number}");
    }
    "未编号内容".to_string()
}

fn common_collection_subtitle(
    facts: &[IdentityFact],
    subtitles: &HashMap<String, Option<String>>,
) -> Option<String> {
    let mut counts = HashMap::<String, (String, usize)>::new();
    for fact in facts {
        let Some(subtitle) = subtitles
            .get(&fact.archive_id)
            .and_then(|subtitle| subtitle.as_deref())
            .map(str::trim)
            .filter(|subtitle| !subtitle.is_empty())
        else {
            continue;
        };
        let normalized = normalize_text(subtitle);
        if normalized.is_empty() {
            continue;
        }
        let entry = counts
            .entry(normalized)
            .or_insert_with(|| (subtitle.to_string(), 0));
        entry.1 += 1;
    }
    counts
        .into_values()
        .max_by_key(|(_, count)| *count)
        .and_then(|(subtitle, count)| (count >= 2).then_some(subtitle))
}

fn recommend_version(
    entries: &[(IdentityFact, Archive, Option<String>, Option<String>)],
    confidence: f64,
    status: &str,
) -> Option<(String, Vec<String>)> {
    if status == "keep_all" || confidence < 0.75 || entries.is_empty() {
        return None;
    }
    let min_pages = entries
        .iter()
        .map(|entry| entry.1.page_count)
        .min()
        .unwrap_or(0);
    let max_pages = entries
        .iter()
        .map(|entry| entry.1.page_count)
        .max()
        .unwrap_or(0);
    if max_pages - min_pages > std::cmp::max(4, max_pages / 10) {
        return None;
    }
    let keeper = entries.iter().max_by(|left, right| {
        let left_density = if left.1.page_count > 0 {
            left.1.file_size as f64 / f64::from(left.1.page_count)
        } else {
            0.0
        };
        let right_density = if right.1.page_count > 0 {
            right.1.file_size as f64 / f64::from(right.1.page_count)
        } else {
            0.0
        };
        left.1
            .page_count
            .cmp(&right.1.page_count)
            .then_with(|| left_density.total_cmp(&right_density))
    })?;
    let mut reasons = Vec::new();
    if max_pages > min_pages {
        reasons.push("页数更多，可能更完整".to_string());
    } else {
        reasons.push("页数一致，单位页文件大小更高".to_string());
    }
    reasons.push("文件名识别置信度较高".to_string());
    Some((keeper.1.id.clone(), reasons))
}

async fn load_archive(pool: &Pool<Sqlite>, id: &str) -> Result<Option<Archive>> {
    let row = sqlx::query("SELECT id, title, subtitle, subtitle_language, path, file_size, page_count, file_hash, created_at, updated_at FROM archives WHERE id = ?")
        .bind(id).fetch_optional(pool).await?;
    let Some(row) = row else { return Ok(None) };
    let tag_rows = sqlx::query("SELECT t.id, t.name, t.namespace FROM tags t JOIN archive_tags at ON at.tag_id = t.id WHERE at.archive_id = ? ORDER BY t.namespace, t.name")
        .bind(id).fetch_all(pool).await?;
    Ok(Some(Archive {
        id: row.get("id"),
        title: row.get("title"),
        subtitle: row.get("subtitle"),
        subtitle_language: row.get("subtitle_language"),
        path: row.get("path"),
        file_size: row.get("file_size"),
        page_count: row.get("page_count"),
        hash: row.get("file_hash"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        tags: tag_rows
            .into_iter()
            .map(|tag| crate::models::TagModel {
                id: tag.get("id"),
                name: tag.get("name"),
                namespace: tag.get("namespace"),
            })
            .collect(),
    }))
}

fn parse_identity(archive: &ArchiveRow) -> IdentityFact {
    let (raw_filename, parent_path) = split_path(&archive.path);
    let stem = strip_extension(&raw_filename);
    let (body, tokens) = extract_bracket_tokens(&stem);
    let lower_body = body.to_lowercase();
    let metadata_text = format!("{} {}", body, tokens.join(" "));
    let lower_metadata = metadata_text.to_lowercase();
    let release_tokens = tokens
        .iter()
        .filter(|token| is_release_token(token))
        .cloned()
        .collect::<Vec<_>>();
    let creator = tokens
        .iter()
        .find(|token| !is_release_token(token) && !is_context_token(token))
        .cloned();
    let edition_marker = release_tokens
        .iter()
        .find(|token| is_edition_token(token))
        .cloned();

    let magazine_issue = contains_any(
        &lower_metadata,
        &[
            "comic",
            "コミック",
            "x-eros",
            "ゼロス",
            "快楽天",
            "真激",
            "アンスリウム",
        ],
    );
    let hash_number = find_marker_number(&metadata_text, '#');
    let volume_number = find_word_number(&lower_body, &["volume", "vol", "卷", "巻"]);
    let chapter_number =
        find_word_number(&lower_body, &["chapter", "ch", "episode", "ep", "话", "話"]);
    let part_number = find_word_number(&lower_body, &["part"]);
    let bracket_number = tokens
        .iter()
        .find(|token| is_number(token))
        .and_then(|token| normalize_number(token));
    let trailing_number = trailing_number(&body);
    let terminal_sequence = terminal_sequence_suffix(&body);

    let (unit_type, volume_number, chapter_number, issue_number, raw_number, number_source) =
        if magazine_issue && hash_number.is_some() {
            (
                "issue".to_string(),
                volume_number,
                None,
                hash_number.clone(),
                hash_number,
                "magazine_issue",
            )
        } else if volume_number.is_some() {
            (
                "volume".to_string(),
                volume_number.clone(),
                chapter_number.or(part_number),
                None,
                volume_number,
                "volume_marker",
            )
        } else if chapter_number.is_some() || part_number.is_some() || hash_number.is_some() {
            let number = chapter_number.or(part_number).or(hash_number);
            (
                "chapter".to_string(),
                None,
                number.clone(),
                None,
                number,
                "chapter_marker",
            )
        } else if let Some(sequence) = terminal_sequence.as_ref() {
            (
                sequence.unit_type.to_string(),
                (sequence.unit_type == "volume").then(|| sequence.number.clone()),
                (sequence.unit_type == "chapter").then(|| sequence.number.clone()),
                None,
                Some(sequence.number.clone()),
                sequence.source,
            )
        } else if bracket_number.is_some() || trailing_number.is_some() {
            (
                "unknown".to_string(),
                None,
                None,
                None,
                bracket_number.or(trailing_number),
                "ambiguous_number",
            )
        } else {
            (
                "standalone".to_string(),
                None,
                None,
                None,
                None,
                "no_number",
            )
        };

    let title_body = terminal_sequence
        .as_ref()
        .map(|sequence| sequence.title)
        .unwrap_or(&body);
    let display_title = clean_display_title(title_body);
    let normalized_key = normalize_text(&clean_title_for_key(title_body, &unit_type));
    let creator_key = creator.as_deref().map(normalize_text).unwrap_or_default();
    let confidence = if unit_type == "unknown" {
        0.58
    } else if unit_type == "issue" {
        0.35
    } else if creator.is_some() {
        0.83
    } else {
        0.72
    };
    let sort_key = calculate_sort_key(
        &unit_type,
        volume_number.as_deref(),
        chapter_number.as_deref().or(raw_number.as_deref()),
    );
    let evidence = json!({
        "rawFilename": raw_filename,
        "parentPath": parent_path,
        "numberSource": number_source,
        "creator": creator,
        "releaseTokens": release_tokens,
        "magazineIssueContext": magazine_issue,
    });
    IdentityFact {
        archive_id: archive.id.clone(),
        raw_filename,
        parent_path,
        normalized_key,
        display_title: if display_title.is_empty() {
            archive.title.clone()
        } else {
            display_title
        },
        creator: if creator_key.is_empty() {
            None
        } else {
            Some(creator.unwrap_or_default())
        },
        unit_type,
        volume_number,
        chapter_number,
        issue_number,
        raw_number,
        edition_marker,
        sort_key,
        confidence,
        evidence,
    }
}

fn split_path(path: &str) -> (String, String) {
    let index = path.rfind(['/', '\\']).map(|value| value + 1).unwrap_or(0);
    (
        path[index..].to_string(),
        path[..index.saturating_sub(1)].to_string(),
    )
}

fn strip_extension(value: &str) -> String {
    value
        .rsplit_once('.')
        .map(|(stem, _)| stem.to_string())
        .unwrap_or_else(|| value.to_string())
}

fn extract_bracket_tokens(input: &str) -> (String, Vec<String>) {
    let mut stripped = String::with_capacity(input.len());
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let Some(close) = (match chars[index] {
            '[' => Some(']'),
            '(' => Some(')'),
            '【' => Some('】'),
            '（' => Some('）'),
            '{' => Some('}'),
            _ => None,
        }) else {
            stripped.push(chars[index]);
            index += 1;
            continue;
        };
        let mut end = index + 1;
        while end < chars.len() && chars[end] != close {
            end += 1;
        }
        if end == chars.len() {
            stripped.push(chars[index]);
            index += 1;
            continue;
        }
        let token: String = chars[index + 1..end].iter().collect();
        if !token.trim().is_empty() {
            tokens.push(token.trim().to_string());
        }
        stripped.push(' ');
        index = end + 1;
    }
    (stripped, tokens)
}

fn clean_display_title(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches([' ', '-', '_', '#'])
        .to_string()
}

fn clean_title_for_key(value: &str, unit_type: &str) -> String {
    let mut result = value.to_string();
    if let Some(index) = result.find('#') {
        result.truncate(index);
    }
    for marker in ["chapter", "episode", "volume", "vol", "part"] {
        if let Some(index) = result.to_lowercase().find(marker) {
            result.truncate(index);
            break;
        }
    }
    if unit_type != "standalone" {
        let words: Vec<&str> = result.split_whitespace().collect();
        if words
            .last()
            .is_some_and(|word| word.chars().all(|ch| ch.is_ascii_digit()))
        {
            result = words[..words.len() - 1].join(" ");
        }
    }
    clean_display_title(&result)
}

fn normalize_text(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| ch.to_lowercase())
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn contains_any(value: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| value.contains(marker))
}

fn is_release_token(token: &str) -> bool {
    let lower = normalize_text(token);
    lower.is_empty()
        || contains_any(
            &lower,
            &[
                "chinese",
                "中文",
                "翻訳",
                "汉化",
                "digital",
                "dl版",
                "無修正",
                "ai generated",
                "ai生成",
                "v2",
                "v3",
                "自用",
                "全彩",
                "新刊進捗",
                "无毒",
                "多语言",
                "page",
                "分辨率",
                "禁漫水印",
                "買動漫",
                "pubu",
            ],
        )
}

fn is_context_token(token: &str) -> bool {
    let lower = normalize_text(token);
    lower.starts_with('c') && lower[1..].chars().all(|ch| ch.is_ascii_digit())
        || lower.chars().all(|ch| ch.is_ascii_digit())
        || contains_any(
            &lower,
            &[
                "original",
                "オリジナル",
                "ブルーアーカイブ",
                "fate grand order",
                "原神",
                "艦隊これくしょん",
            ],
        )
}

fn is_edition_token(token: &str) -> bool {
    let lower = normalize_text(token);
    lower == "v2" || lower == "v3" || lower.contains("分辨率") || lower.contains("digital")
}

fn is_number(value: &str) -> bool {
    value
        .trim()
        .chars()
        .all(|ch| ch.is_ascii_digit() || ch == '.' || ch == '-')
}

fn normalize_number(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let value = value.trim_start_matches('0');
    let value = if value.is_empty() { "0" } else { value };
    value.split('-').next()?.parse::<f64>().ok().map(|number| {
        if number.fract() == 0.0 {
            format!("{number:.0}")
        } else {
            number.to_string()
        }
    })
}

fn find_marker_number(value: &str, marker: char) -> Option<String> {
    let index = value.find(marker)?;
    let digits: String = value[index + marker.len_utf8()..]
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
        .collect();
    normalize_number(&digits)
}

fn find_word_number(value: &str, markers: &[&str]) -> Option<String> {
    for marker in markers {
        if let Some(index) = value.find(marker) {
            let tail = &value[index + marker.len()..];
            let digits: String = tail
                .chars()
                .skip_while(|ch| !ch.is_ascii_digit())
                .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
                .collect();
            if let Some(number) = normalize_number(&digits) {
                return Some(number);
            }
        }
    }
    None
}

fn trailing_number(value: &str) -> Option<String> {
    let word = value.split_whitespace().last()?;
    normalize_number(word.trim_matches([')', '）', ']', '】']))
}

struct TerminalSequenceSuffix<'a> {
    title: &'a str,
    unit_type: &'static str,
    number: String,
    source: &'static str,
}

// Treat only a terminal, explicit content unit as a sequence suffix. This
// covers Japanese anthology parts and Japanese/Chinese ordinal units while
// leaving a phrase containing e.g. "中編" in the middle of a title untouched.
fn terminal_sequence_suffix(value: &str) -> Option<TerminalSequenceSuffix<'_>> {
    let value = value.trim_matches([' ', '\u{3000}', '-', '_', '~', '～']);
    for (marker, number) in [
        ("前編", "1"),
        ("上編", "1"),
        ("中編", "2"),
        ("後編", "3"),
        ("下編", "3"),
    ] {
        let Some(title) = value.strip_suffix(marker) else {
            continue;
        };
        let title = title.trim_matches([' ', '\u{3000}', '-', '_', '~', '～']);
        if !title.is_empty() {
            return Some(TerminalSequenceSuffix {
                title,
                unit_type: "chapter",
                number: number.to_string(),
                source: "japanese_part_suffix",
            });
        }
    }

    for (ordinal, marker, unit_type) in [
        ("第", "話", "chapter"),
        ("第", "话", "chapter"),
        ("第", "章", "chapter"),
        ("第", "巻", "volume"),
        ("第", "卷", "volume"),
        ("第", "部", "volume"),
        ("제", "화", "chapter"),
        ("제", "권", "volume"),
    ] {
        let Some(before_marker) = value.strip_suffix(marker) else {
            continue;
        };
        let Some(number_start) = before_marker.rfind(ordinal) else {
            continue;
        };
        let number = sequence_number(&before_marker[number_start + ordinal.len()..])?;
        let title =
            before_marker[..number_start].trim_matches([' ', '\u{3000}', '-', '_', '~', '～']);
        if !title.is_empty() {
            return Some(TerminalSequenceSuffix {
                title,
                unit_type,
                number,
                source: "east_asian_ordinal_suffix",
            });
        }
    }

    let Some(number_start) = value.rfind("その") else {
        return None;
    };
    let number = sequence_number(&value[number_start + "その".len()..])?;
    let title = value[..number_start].trim_matches([' ', '\u{3000}', '-', '_', '~', '～']);
    (!title.is_empty()).then(|| TerminalSequenceSuffix {
        title,
        unit_type: "chapter",
        number,
        source: "japanese_sono_suffix",
    })
}

fn sequence_number(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit() || ch == '.') {
        return None;
    }
    normalize_number(value)
}

fn calculate_sort_key(unit_type: &str, volume: Option<&str>, chapter: Option<&str>) -> f64 {
    let volume = volume
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);
    let chapter = chapter
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);
    match unit_type {
        "volume" => volume * 10000.0 + chapter,
        "chapter" => chapter,
        _ => 999999.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    fn archive(path: &str) -> ArchiveRow {
        ArchiveRow {
            id: "a".into(),
            title: "title".into(),
            path: path.into(),
        }
    }

    #[test]
    fn distinguishes_revision_and_magazine_numbers() {
        let revision = parse_identity(&archive("/x/[artist] Work [v2].zip"));
        assert_eq!(revision.edition_marker.as_deref(), Some("v2"));
        assert_eq!(revision.volume_number, None);
        let issue = parse_identity(&archive("/x/[artist] Work (COMIC X-EROS #107).zip"));
        assert_eq!(issue.issue_number.as_deref(), Some("107"));
        assert_eq!(issue.unit_type, "issue");
    }

    #[test]
    fn parses_chapter_and_volume_markers() {
        let chapter = parse_identity(&archive("/x/[artist] Work Chapter 2 [Chinese].zip"));
        assert_eq!(chapter.chapter_number.as_deref(), Some("2"));
        let volume = parse_identity(&archive("/x/[artist] Work Vol.03 [DL版].zip"));
        assert_eq!(volume.volume_number.as_deref(), Some("3"));
    }

    #[tokio::test]
    async fn lists_confirmed_collections_before_manual_and_pending_collections() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(
            "CREATE TABLE collections (
                id TEXT PRIMARY KEY, display_title TEXT NOT NULL, subtitle TEXT,
                cover_archive_id TEXT, status TEXT NOT NULL, is_manual_locked BOOLEAN NOT NULL,
                normalized_key TEXT NOT NULL, updated_at DATETIME NOT NULL
            );
            CREATE TABLE collection_members (
                collection_id TEXT NOT NULL, archive_id TEXT NOT NULL, variant_group_key TEXT
            );
            CREATE TABLE collection_review_items (collection_id TEXT NOT NULL, status TEXT NOT NULL);",
        )
        .execute(&pool)
        .await
        .unwrap();

        for (id, title, status, locked) in [
            ("pending", "A pending", "needs_review", false),
            ("manual", "B manual", "manual", true),
            ("auto", "C confirmed", "auto", false),
        ] {
            sqlx::query("INSERT INTO collections (id, display_title, status, is_manual_locked, normalized_key, updated_at) VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)")
                .bind(id)
                .bind(title)
                .bind(status)
                .bind(locked)
                .bind(id)
                .execute(&pool)
                .await
                .unwrap();
            for suffix in ["one", "two"] {
                sqlx::query(
                    "INSERT INTO collection_members (collection_id, archive_id) VALUES (?, ?)",
                )
                .bind(id)
                .bind(format!("{id}-{suffix}"))
                .execute(&pool)
                .await
                .unwrap();
            }
        }

        let ids: Vec<String> = list_collections(&pool, None, None, None)
            .await
            .unwrap()
            .into_iter()
            .map(|collection| collection.id)
            .collect();
        assert_eq!(ids, ["auto", "manual", "pending"]);
    }

    #[tokio::test]
    async fn rebuild_groups_numbered_siblings_idempotently() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(
            "CREATE TABLE archives (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT,
                path TEXT NOT NULL, file_hash TEXT NOT NULL, file_size INTEGER NOT NULL,
                page_count INTEGER NOT NULL, created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL
            );
            CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL);
            CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL);",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::raw_sql(include_str!("../../migrations/sqlite/0004_collections.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!(
            "../../migrations/sqlite/0005_collection_versions.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        sqlx::raw_sql(include_str!(
            "../../migrations/sqlite/0006_collection_identity_keys.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        for (id, path) in [
            ("one", "/library/[Artist] Demo Story (2) [Chinese].cbz"),
            ("two", "/library/[Artist] Demo Story (3) [Chinese].cbz"),
            (
                "issue",
                "/library/[Artist] Other Story (COMIC X-EROS #107) [v2].cbz",
            ),
        ] {
            sqlx::query("INSERT INTO archives (id, title, path, file_hash, file_size, page_count, created_at, updated_at) VALUES (?, ?, ?, ?, 1, 20, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
                .bind(id).bind(id).bind(path).bind(format!("hash-{id}")).execute(&pool).await.unwrap();
        }

        let first = rebuild_collections(&pool).await.unwrap();
        assert_eq!(first.parsed_archives, 3);
        assert_eq!(first.created_collections, 1);
        let collections = list_collections(&pool, None, None, None).await.unwrap();
        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0].member_count, 2);
        let detail = get_collection(&pool, &collections[0].id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(detail.members.len(), 2);
        assert_eq!(detail.members[0].raw_number.as_deref(), Some("2"));
        assert_eq!(detail.members[1].raw_number.as_deref(), Some("3"));

        rebuild_collections(&pool).await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_members")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn same_content_versions_are_not_created_as_a_collection() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(
            "CREATE TABLE archives (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT,
                path TEXT NOT NULL, file_hash TEXT NOT NULL, file_size INTEGER NOT NULL,
                page_count INTEGER NOT NULL, created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL
            );
            CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL);
            CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL);",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::raw_sql(include_str!("../../migrations/sqlite/0004_collections.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!(
            "../../migrations/sqlite/0005_collection_versions.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        sqlx::raw_sql(include_str!(
            "../../migrations/sqlite/0006_collection_identity_keys.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        for (id, path, size) in [
            (
                "one",
                "/library/[Artist] Demo Story (2) [Chinese].cbz",
                100_i64,
            ),
            (
                "two",
                "/library/[Artist] Demo Story (2) [English].cbz",
                200_i64,
            ),
        ] {
            sqlx::query("INSERT INTO archives (id, title, path, file_hash, file_size, page_count, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 20, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
                .bind(id).bind(id).bind(path).bind(format!("hash-{id}")).bind(size).execute(&pool).await.unwrap();
        }

        rebuild_collections(&pool).await.unwrap();
        assert!(list_collections(&pool, None, None, None)
            .await
            .unwrap()
            .is_empty());
        let versions = list_version_groups(&pool, None, None, None).await.unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].members.len(), 2);
    }

    #[tokio::test]
    async fn stage_numbers_are_collection_members_not_versions() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(
            "CREATE TABLE archives (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT,
                path TEXT NOT NULL, file_hash TEXT NOT NULL, file_size INTEGER NOT NULL,
                page_count INTEGER NOT NULL, created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL
            );
            CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL);
            CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL);",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::raw_sql(include_str!("../../migrations/sqlite/0004_collections.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!(
            "../../migrations/sqlite/0005_collection_versions.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        sqlx::raw_sql(include_str!(
            "../../migrations/sqlite/0006_collection_identity_keys.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        for (id, stage) in [("three", 3), ("four", 4)] {
            let path = format!("/library/[NR] BLANC Stage {stage} (Comic Exe) [DL].cbz");
            sqlx::query("INSERT INTO archives (id, title, path, file_hash, file_size, page_count, created_at, updated_at) VALUES (?, ?, ?, ?, 1, 20, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
                .bind(id).bind(format!("BLANC Stage {stage}")).bind(path).bind(format!("hash-{id}")).execute(&pool).await.unwrap();
        }

        rebuild_collections(&pool).await.unwrap();
        let collections = list_collections(&pool, None, None, None).await.unwrap();
        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0].display_title, "BLANC Stage");
        assert_eq!(collections[0].content_count, 2);
        assert!(list_version_groups(&pool, None, None, None)
            .await
            .unwrap()
            .is_empty());
    }

    #[test]
    fn infers_a_missing_first_number_but_keeps_multiple_unnumbered_files_as_versions() {
        let mut sequence = vec![
            parse_identity(&ArchiveRow {
                id: "one".into(),
                title: "Demo".into(),
                path: "/library/[Artist] Demo [DL].cbz".into(),
            }),
            parse_identity(&ArchiveRow {
                id: "two".into(),
                title: "Demo 2".into(),
                path: "/library/[Artist] Demo 2 [DL].cbz".into(),
            }),
        ];
        infer_missing_first_numbers(&mut sequence);
        assert_eq!(sequence[0].raw_number.as_deref(), Some("1"));
        assert_eq!(sequence[0].unit_type, "unknown");

        let mut duplicates = vec![
            parse_identity(&ArchiveRow {
                id: "a".into(),
                title: "Same".into(),
                path: "/library/[Artist] Same [Chinese].cbz".into(),
            }),
            parse_identity(&ArchiveRow {
                id: "b".into(),
                title: "Same".into(),
                path: "/library/[Artist] Same [English].cbz".into(),
            }),
        ];
        infer_missing_first_numbers(&mut duplicates);
        assert!(duplicates.iter().all(|fact| fact.raw_number.is_none()));
        assert_eq!(
            content_unit_key(&duplicates[0]),
            content_unit_key(&duplicates[1])
        );
    }

    #[tokio::test]
    async fn groups_terminal_sequence_suffixes_across_languages_without_grouping_a_lone_part() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::raw_sql(
            "CREATE TABLE archives (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT,
                path TEXT NOT NULL, file_hash TEXT NOT NULL, file_size INTEGER NOT NULL,
                page_count INTEGER NOT NULL, created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL
            );
            CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL);
            CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL);",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::raw_sql(include_str!("../../migrations/sqlite/0004_collections.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!(
            "../../migrations/sqlite/0005_collection_versions.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        sqlx::raw_sql(include_str!(
            "../../migrations/sqlite/0006_collection_identity_keys.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();
        for (id, title, path) in [
            ("base", "星降る図書館", "/library/[ロケットモンキー] 星降る図書館 (コミックメガストア Vol.2) [中国翻訳] [DL版].cbz"),
            ("middle", "星降る図書館 中編", "/library/[ロケットモンキー] 星降る図書館 中編 (コミックメガストア Vol.3) [中国翻訳] [DL版].cbz"),
            ("lone", "雨宿りの午後 中編", "/library/[青空堂] 雨宿りの午後 中編 [中国翻訳] [DL版].cbz"),
            ("chapter24", "放課後の図書室 第24話", "/library/[あずせ] 放課後の図書室 第24話 (アナンガ・ランガ Vol.104) [中国翻訳].cbz"),
            ("chapter25", "放課後の図書室 第25話", "/library/[あずせ] 放課後の図書室 第25話 (アナンガ・ランガ Vol.106) [中国翻訳].cbz"),
            ("base_story", "コダマちゃんの冒険", "/library/[ワクセイブロ] コダマちゃんの冒険 [中国翻訳] [DL版].cbz"),
            ("story_two", "コダマちゃんの冒険その2", "/library/[ワクセイブロ] コダマちゃんの冒険その2 [中国翻訳] [DL版].cbz"),
            ("story_three", "コダマちゃんの冒険その3", "/library/[ワクセイブロ] コダマちゃんの冒険その3 [English] [Digital].cbz"),
            ("cn24", "星空补习班 第24话", "/library/[林墨] 星空补习班 第24话 [中文] [全彩].cbz"),
            ("cn25", "星空补习班 第25话", "/library/[林墨] 星空补习班 第25话 [Chinese] [Full Color].cbz"),
            ("en4", "City Library Chronicles Chapter 4", "/library/[North Star Studio] City Library Chronicles Chapter 4 [English] [Digital].cbz"),
            ("en5", "City Library Chronicles Chapter 5", "/library/[North Star Studio] City Library Chronicles Chapter 5 [Chinese] [Digital].cbz"),
            ("kr2", "봄날의 도서관 제2화", "/library/[달빛작가] 봄날의 도서관 제2화 [한국어] [Digital].cbz"),
            ("kr3", "봄날의 도서관 제3화", "/library/[달빛작가] 봄날의 도서관 제3화 [Chinese] [Digital].cbz"),
        ] {
            sqlx::query("INSERT INTO archives (id, title, path, file_hash, file_size, page_count, created_at, updated_at) VALUES (?, ?, ?, ?, 1, 20, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
                .bind(id).bind(title).bind(path).bind(format!("hash-{id}")).execute(&pool).await.unwrap();
        }

        let japanese_chapter = parse_identity(&archive(
            "/library/[あずせ] 放課後の図書室 第25話 [中国翻訳].cbz",
        ));
        assert_eq!(japanese_chapter.chapter_number.as_deref(), Some("25"));
        assert_eq!(japanese_chapter.normalized_key, "放課後の図書室");
        let chinese_chapter =
            parse_identity(&archive("/library/[林墨] 星空补习班 第25话 [中文].cbz"));
        assert_eq!(chinese_chapter.chapter_number.as_deref(), Some("25"));
        assert_eq!(chinese_chapter.normalized_key, "星空补习班");

        rebuild_collections(&pool).await.unwrap();
        let collections = list_collections(&pool, None, None, None).await.unwrap();
        assert_eq!(
            collections.len(),
            6,
            "formed collections: {:?}",
            collections
                .iter()
                .map(|collection| (&collection.display_title, collection.content_count))
                .collect::<Vec<_>>()
        );
        assert!(collections
            .iter()
            .any(|collection| collection.display_title == "星降る図書館"
                && collection.content_count == 2));
        assert!(collections
            .iter()
            .any(|collection| collection.display_title == "放課後の図書室"
                && collection.content_count == 2));
        assert!(collections.iter().any(|collection| collection.display_title
            == "コダマちゃんの冒険"
            && collection.content_count == 3));
        assert!(collections
            .iter()
            .any(|collection| collection.display_title == "星空补习班"
                && collection.content_count == 2));
        assert!(collections.iter().any(|collection| collection.display_title
            == "City Library Chronicles"
            && collection.content_count == 2));
        assert!(collections
            .iter()
            .any(|collection| collection.display_title == "봄날의 도서관"
                && collection.content_count == 2));

        let middle = parse_identity(&archive("/library/[Artist] 星降る図書館 中編 [DL].cbz"));
        assert_eq!(middle.unit_type, "chapter");
        assert_eq!(middle.chapter_number.as_deref(), Some("2"));
        assert_eq!(middle.normalized_key, "星降る図書館");

        let story_part = parse_identity(&archive(
            "/library/[Artist] コダマちゃんの冒険その3 [DL].cbz",
        ));
        assert_eq!(story_part.chapter_number.as_deref(), Some("3"));
        assert_eq!(story_part.normalized_key, "コダマちゃんの冒険");

        let korean_chapter = parse_identity(&archive(
            "/library/[달빛작가] 봄날의 도서관 제3화 [Chinese].cbz",
        ));
        assert_eq!(korean_chapter.chapter_number.as_deref(), Some("3"));
        assert_eq!(korean_chapter.normalized_key, "봄날의 도서관");

        assert!(terminal_sequence_suffix("連載 第01-06話").is_none());
        assert!(terminal_sequence_suffix("短編集 その1-2").is_none());
    }
}
