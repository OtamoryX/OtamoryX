use anyhow::{Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::{
    Archive, CollectionDetail, CollectionMember, CollectionRebuildResponse, CollectionReviewItem,
    CollectionSummary,
};

const PARSER_VERSION: &str = "collections-v1";

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
    query: Option<&str>,
) -> Result<Vec<CollectionSummary>> {
    let mut sql = String::from(
        "SELECT c.id, c.display_title, c.cover_archive_id, c.status, c.is_manual_locked,
                COUNT(cm.archive_id) AS member_count,
                COUNT(cm.archive_id) - COUNT(DISTINCT cm.variant_group_key) AS variant_count,
                (SELECT COUNT(*) FROM collection_review_items r WHERE r.collection_id = c.id AND r.status = 'pending') AS review_count
         FROM collections c
         LEFT JOIN collection_members cm ON cm.collection_id = c.id",
    );
    if query.is_some() {
        sql.push_str(" WHERE c.display_title LIKE ? OR c.normalized_key LIKE ?");
    }
    sql.push_str(" GROUP BY c.id HAVING COUNT(cm.archive_id) > 1 OR c.is_manual_locked = TRUE ORDER BY c.updated_at DESC, c.display_title COLLATE NOCASE");

    let mut request = sqlx::query(&sql);
    if let Some(query) = query {
        let pattern = format!("%{}%", query.trim());
        request = request.bind(pattern.clone()).bind(pattern);
    }
    let rows = request
        .fetch_all(pool)
        .await
        .context("Failed to list collections")?;
    Ok(rows.into_iter().map(summary_from_row).collect())
}

pub async fn get_collection(pool: &Pool<Sqlite>, id: &str) -> Result<Option<CollectionDetail>> {
    let row = sqlx::query(
        "SELECT c.id, c.display_title, c.cover_archive_id, c.status, c.is_manual_locked,
                COUNT(cm.archive_id) AS member_count,
                COUNT(cm.archive_id) - COUNT(DISTINCT cm.variant_group_key) AS variant_count,
                (SELECT COUNT(*) FROM collection_review_items r WHERE r.collection_id = c.id AND r.status = 'pending') AS review_count
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

pub async fn rebuild_collections(pool: &Pool<Sqlite>) -> Result<CollectionRebuildResponse> {
    let rows = sqlx::query("SELECT id, title, path FROM archives ORDER BY id")
        .fetch_all(pool)
        .await
        .context("Failed to load archives for collection rebuild")?;

    let mut facts = Vec::with_capacity(rows.len());
    for row in rows {
        let archive = ArchiveRow {
            id: row.get("id"),
            title: row.get("title"),
            path: row.get("path"),
        };
        facts.push(parse_identity(&archive));
    }

    let mut tx = pool
        .begin()
        .await
        .context("Failed to start collection rebuild")?;
    for fact in &facts {
        sqlx::query(
            "INSERT INTO archive_identity_facts
                (archive_id, raw_filename, parent_path, normalized_key, display_title, creator,
                 unit_type, volume_number, chapter_number, issue_number, edition_marker,
                 confidence, evidence_json, parser_version, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
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
                edition_marker = excluded.edition_marker,
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
        .bind(&fact.edition_marker)
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
        let creator_key = fact
            .creator
            .as_deref()
            .map(normalize_text)
            .unwrap_or_default();
        let key = if creator_key.is_empty() {
            fact.normalized_key.clone()
        } else {
            format!("{}::{}", fact.normalized_key, creator_key)
        };
        grouped.entry(key).or_default().push(fact);
    }

    let mut created_collections = 0i64;
    let mut grouped_archives = 0i64;
    let mut pending_reviews = 0i64;
    for (group_key, mut group) in grouped {
        if group.len() < 2 {
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
            .map(|fact| fact.display_title.clone())
            .unwrap_or_else(|| "未命名合集".to_string());
        let cover_archive_id = group.first().map(|fact| fact.archive_id.clone());
        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO collections
                (id, display_title, normalized_key, cover_archive_id, status)
             VALUES (?, ?, ?, ?, 'auto')",
        )
        .bind(&collection_id)
        .bind(&title)
        .bind(&group_key)
        .bind(&cover_archive_id)
        .execute(&mut *tx)
        .await
        .context("Failed to create collection")?
        .rows_affected();
        created_collections += i64::from(inserted > 0);

        let needs_review = group.iter().any(|fact| fact.confidence < 0.75);
        if needs_review {
            sqlx::query("UPDATE collections SET status = 'needs_review', updated_at = CURRENT_TIMESTAMP WHERE id = ? AND is_manual_locked = FALSE")
                .bind(&collection_id)
                .execute(&mut *tx)
                .await?;
        }

        for fact in group {
            let variant_group_key = if fact.raw_number.is_none() {
                Some(format!("{}::variant", fact.normalized_key))
            } else {
                Some(format!(
                    "{}::{}::{}",
                    fact.normalized_key,
                    fact.unit_type,
                    fact.raw_number.as_deref().unwrap_or_default()
                ))
            };
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

async fn get_collection_summary(
    pool: &Pool<Sqlite>,
    id: &str,
) -> Result<Option<CollectionSummary>> {
    let row = sqlx::query(
        "SELECT c.id, c.display_title, c.cover_archive_id, c.status, c.is_manual_locked,
                COUNT(cm.archive_id) AS member_count,
                COUNT(cm.archive_id) - COUNT(DISTINCT cm.variant_group_key) AS variant_count,
                (SELECT COUNT(*) FROM collection_review_items r WHERE r.collection_id = c.id AND r.status = 'pending') AS review_count
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
        cover_archive_id: row.get("cover_archive_id"),
        status: row.get("status"),
        is_manual_locked: row.get("is_manual_locked"),
        member_count: row.get("member_count"),
        variant_count: row.get("variant_count"),
        review_count: row.get("review_count"),
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

    let display_title = clean_display_title(&body);
    let normalized_key = normalize_text(&clean_title_for_key(&body, &unit_type));
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
    let value = value.trim().trim_start_matches('0');
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
        let collections = list_collections(&pool, None).await.unwrap();
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
}
