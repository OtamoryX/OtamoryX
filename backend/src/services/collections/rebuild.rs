use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::models::{
    CollectionRebuildPreview, CollectionRebuildPreviewItem, CollectionRebuildResponse,
};

use super::identity::{
    calculate_sort_key, clean_title_for_key, content_unit_key, infer_missing_first_numbers,
    normalize_text, parse_identity, version_group_key, work_group_key, ArchiveRow, IdentityFact,
};

const PARSER_VERSION: &str = "collections-v3";

pub async fn preview_collection_rebuild(pool: &Pool<Sqlite>) -> Result<CollectionRebuildPreview> {
    let rows = sqlx::query("SELECT id, title, path FROM archives ORDER BY id")
        .fetch_all(pool)
        .await
        .context("Failed to load archives for collection preview")?;
    let mut facts = rows
        .into_iter()
        .map(|row| {
            parse_identity(&ArchiveRow {
                id: row.get("id"),
                title: row.get("title"),
                path: row.get("path"),
            })
        })
        .collect::<Vec<_>>();
    infer_missing_first_numbers(&mut facts);

    let mut groups = HashMap::<String, Vec<IdentityFact>>::new();
    for fact in facts.iter().cloned() {
        groups.entry(work_group_key(&fact)).or_default().push(fact);
    }

    let mut collection_candidates = Vec::new();
    let mut version_candidates = Vec::new();
    let mut pending_review_count = 0;
    for (_, mut group) in groups {
        group.sort_by(|left, right| left.sort_key.total_cmp(&right.sort_key));
        let title = group
            .first()
            .map(|fact| clean_title_for_key(&fact.display_title, &fact.unit_type))
            .unwrap_or_else(|| "未命名内容".to_string());
        let unit_keys = group.iter().map(version_group_key).collect::<HashSet<_>>();
        if unit_keys.len() >= 2 {
            let needs_review = group.iter().filter(|fact| fact.confidence < 0.75).count() as i64;
            pending_review_count += needs_review;
            collection_candidates.push(CollectionRebuildPreviewItem {
                display_title: title,
                member_count: group.len() as i64,
                status: if needs_review > 0 {
                    "needs_review"
                } else {
                    "auto"
                }
                .to_string(),
                reason: if needs_review > 0 {
                    format!("含 {needs_review} 个需要确认的编号或成员")
                } else {
                    "检测到多个明确的卷、话或篇章".to_string()
                },
            });
        } else if group.len() >= 2 {
            version_candidates.push(CollectionRebuildPreviewItem {
                display_title: title,
                member_count: group.len() as i64,
                status: "versions".to_string(),
                reason: "标题相同且属于同一内容单元，建议作为多版本比较".to_string(),
            });
        }
    }
    collection_candidates.sort_by(|left, right| left.display_title.cmp(&right.display_title));
    version_candidates.sort_by(|left, right| left.display_title.cmp(&right.display_title));
    Ok(CollectionRebuildPreview {
        parsed_archives: facts.len() as i64,
        collection_candidates,
        version_candidates,
        pending_review_count,
    })
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

            if fact.confidence < 0.75 {
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

        // A collection's state is derived from its remaining pending members. Confirming one
        // member must not freeze future automatic additions to the rest of the collection.
        sqlx::query(
            "UPDATE collections
             SET status = CASE WHEN EXISTS (
                 SELECT 1 FROM collection_review_items r
                 WHERE r.collection_id = collections.id AND r.status = 'pending'
             ) THEN 'needs_review' ELSE 'auto' END,
             updated_at = CURRENT_TIMESTAMP
             WHERE id = ? AND is_manual_locked = FALSE",
        )
        .bind(&collection_id)
        .execute(&mut *tx)
        .await?;
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
    sqlx::query(
        "UPDATE collections
         SET status = CASE WHEN EXISTS (
             SELECT 1 FROM collection_review_items r
             WHERE r.collection_id = collections.id AND r.status = 'pending'
         ) THEN 'needs_review' ELSE 'auto' END,
         updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND is_manual_locked = FALSE",
    )
    .bind(&collection_id)
    .execute(pool)
    .await?;
    Ok(())
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
