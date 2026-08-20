use anyhow::{Context, Result};
use serde_json::Value;
use sqlx::{Pool, Row, Sqlite};

use crate::models::{
    Archive, CollectionDetail, CollectionMember, CollectionMemberReview, CollectionReviewItem,
    CollectionSummary,
};
use crate::services::archive::query::ArchiveDeleteTarget;

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
    let mut sql = String::from(
        "SELECT c.id, c.display_title, c.subtitle, c.cover_archive_id, c.status, c.is_manual_locked, COUNT(cm.archive_id) AS member_count, COUNT(DISTINCT COALESCE(cm.variant_group_key, cm.archive_id)) AS content_count, (SELECT COUNT(*) FROM (SELECT COALESCE(variant_group_key, archive_id) AS unit_key FROM collection_members WHERE collection_id = c.id GROUP BY unit_key HAVING COUNT(*) > 1)) AS variant_group_count, COUNT(cm.archive_id) - COUNT(DISTINCT COALESCE(cm.variant_group_key, cm.archive_id)) AS variant_count, (SELECT COUNT(*) FROM collection_review_items r WHERE r.collection_id = c.id AND r.status = 'pending') AS review_count, ",
    );
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
            "recognitionPriority" => {
                "CASE
                WHEN c.status = 'auto' AND c.is_manual_locked = FALSE THEN 0
                WHEN c.status = 'manual' OR c.is_manual_locked = TRUE THEN 1
                WHEN c.status = 'needs_review' THEN 2
                ELSE 1
              END"
            }
            "title" => "c.display_title COLLATE NOCASE",
            "contentCount" => "content_count",
            "memberCount" => "member_count",
            _ => "c.updated_at",
        };
        let sort_direction = if sort_by == "recognitionPriority"
            || sort_order.is_some_and(|value| value.eq_ignore_ascii_case("asc"))
        {
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
                cm.confidence, cm.membership_source, cm.is_manual_locked,
                r.id AS review_id, r.reason AS review_reason, r.evidence_json AS review_evidence
         FROM collection_members cm
         LEFT JOIN collection_review_items r
           ON r.archive_id = cm.archive_id AND r.collection_id = cm.collection_id AND r.status = 'pending'
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
        let review =
            member_row
                .get::<Option<String>, _>("review_id")
                .map(|id| CollectionMemberReview {
                    id,
                    reason: member_row.get("review_reason"),
                    evidence: member_row
                        .get::<Option<String>, _>("review_evidence")
                        .and_then(|value| serde_json::from_str(&value).ok())
                        .unwrap_or(Value::Null),
                });
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
            review,
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
pub(crate) async fn get_collection_summary(
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

pub(crate) fn summary_from_row(row: sqlx::sqlite::SqliteRow) -> CollectionSummary {
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
pub(crate) async fn load_archive(pool: &Pool<Sqlite>, id: &str) -> Result<Option<Archive>> {
    let row = sqlx::query("SELECT id, title, subtitle, subtitle_language, path, file_size, page_count, file_hash, created_at, updated_at FROM archives WHERE id = ?")
        .bind(id).fetch_optional(pool).await?;
    let Some(row) = row else { return Ok(None) };
    let tag_rows = sqlx::query("SELECT t.id, t.name, t.namespace, l.name AS localized_name FROM tags t JOIN archive_tags at ON at.tag_id = t.id LEFT JOIN tag_localizations l ON l.tag_id = t.id AND l.locale = 'zh-Hans' AND l.status = 'completed' WHERE at.archive_id = ? ORDER BY t.namespace, t.name")
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
                localized_name: tag.get("localized_name"),
            })
            .collect(),
    }))
}
