use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::models::{Archive, VersionCandidate, VersionCleanupResponse, VersionGroup};
use crate::services::trash::TrashService;

use super::identity::{content_unit_key, IdentityFact};
use super::query::load_archive;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VersionCleanupRequestSnapshot {
    #[serde(default)]
    pub(crate) version: u8,
    pub(crate) group_id: String,
    pub(crate) group_key: String,
    pub(crate) keep_archive_id: String,
    pub(crate) delete_archive_ids: Vec<String>,
    #[serde(default)]
    pub(crate) failed_archive_ids: Vec<String>,
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
            Some("recognitionPriority") => version_priority(left).cmp(&version_priority(right)),
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
        if sort_by == Some("recognitionPriority")
            || sort_by
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
    archive_cache: &std::sync::Arc<crate::services::archive::cache::ArchiveCacheService>,
    user_id: &str,
    group_id: &str,
    keep_archive_id: &str,
    delete_archive_ids: &[String],
    idempotency_key: Option<&str>,
) -> Result<VersionCleanupResponse> {
    let sorted_deletions = canonical_archive_ids(delete_archive_ids);
    let derived_key = format!(
        "version-cleanup:{}:{}:{}",
        group_id,
        keep_archive_id,
        sorted_deletions.join(",")
    );
    let idempotency_key = idempotency_key.unwrap_or(&derived_key);
    if let Some(response) = load_idempotent_version_cleanup(
        pool,
        user_id,
        idempotency_key,
        group_id,
        keep_archive_id,
        &sorted_deletions,
    )
    .await?
    {
        return Ok(response);
    }

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
    let valid_deletions = group
        .members
        .iter()
        .filter(|member| member.archive.id != keep_archive_id)
        .map(|member| member.archive.id.clone())
        .collect::<HashSet<_>>();
    let requested_deletions = delete_archive_ids.iter().cloned().collect::<HashSet<_>>();
    if requested_deletions.is_empty() || !requested_deletions.is_subset(&valid_deletions) {
        return Err(anyhow::anyhow!(
            "cleanup request includes invalid version members"
        ));
    }

    let operation_id = Uuid::new_v4().to_string();
    let request_snapshot = VersionCleanupRequestSnapshot {
        version: 1,
        group_id: group_id.to_string(),
        group_key: group.group_key.clone(),
        keep_archive_id: keep_archive_id.to_string(),
        delete_archive_ids: sorted_deletions.clone(),
        failed_archive_ids: Vec::new(),
    };
    let insert_operation = sqlx::query(
        "INSERT INTO trash_operations
         (id, user_id, operation_type, group_key, keep_archive_id, idempotency_key, migration_snapshot_json, status)
         VALUES (?, ?, 'version_cleanup', ?, ?, ?, ?, 'pending')",
    )
    .bind(&operation_id)
    .bind(user_id)
    .bind(&group.group_key)
    .bind(keep_archive_id)
    .bind(idempotency_key)
    .bind(serde_json::to_string(&request_snapshot)?);
    if let Err(error) = insert_operation.execute(pool).await {
        if let Some(response) = load_idempotent_version_cleanup(
            pool,
            user_id,
            idempotency_key,
            group_id,
            keep_archive_id,
            &sorted_deletions,
        )
        .await?
        {
            return Ok(response);
        }
        return Err(error.into());
    }
    let keeper_pages = group
        .members
        .iter()
        .find(|member| member.archive.id == keep_archive_id)
        .map(|member| member.archive.page_count)
        .unwrap_or(0);
    let mut failed_archive_ids = Vec::new();
    let mut deleted = 0;
    let trash_service = TrashService::new(pool.clone());
    let mut moved_archive_ids = Vec::new();
    for member in group
        .members
        .into_iter()
        .filter(|member| requested_deletions.contains(&member.archive.id))
    {
        match trash_service
            .move_version_group_member_to_trash(
                user_id,
                &operation_id,
                &member.archive.id,
                keep_archive_id,
                keeper_pages,
            )
            .await
        {
            Ok(_) => {
                deleted += 1;
                moved_archive_ids.push(member.archive.id.clone());
                archive_cache.clear_archive_cache(&member.archive.id).await;
                archive_cache.clear_archive_cache(keep_archive_id).await;
            }
            Err(error) => {
                tracing::error!(archive_id = %member.archive.id, "Failed to clean up version: {error:#}");
                failed_archive_ids.push(member.archive.id);
                let failed_snapshot = VersionCleanupRequestSnapshot {
                    failed_archive_ids: failed_archive_ids.clone(),
                    ..request_snapshot.clone()
                };
                sqlx::query(
                    "UPDATE trash_operations SET migration_snapshot_json = ? WHERE id = ? AND status = 'pending'",
                )
                .bind(serde_json::to_string(&failed_snapshot)?)
                .bind(&operation_id)
                .execute(pool)
                .await?;
                if let Err(rollback_error) = trash_service
                    .rollback_version_cleanup(user_id, &operation_id)
                    .await
                {
                    sqlx::query("UPDATE trash_operations SET status = 'failed' WHERE id = ?")
                        .bind(&operation_id)
                        .execute(pool)
                        .await?;
                    return Err(anyhow::anyhow!(
                        "version cleanup failed for {} and rollback failed: {rollback_error:#}",
                        failed_archive_ids.join(", ")
                    ));
                }
                for archive_id in &moved_archive_ids {
                    archive_cache.clear_archive_cache(archive_id).await;
                }
                archive_cache.clear_archive_cache(keep_archive_id).await;
                return Ok(VersionCleanupResponse {
                    kept_archive_id: keep_archive_id.to_string(),
                    deleted: 0,
                    failed_archive_ids,
                    operation_id,
                });
            }
        }
    }

    let activated = sqlx::query(
        "UPDATE trash_operations SET status = 'active' WHERE id = ? AND user_id = ? AND status = 'pending'",
    )
    .bind(&operation_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    if activated.rows_affected() != 1 {
        let rollback_result = trash_service
            .rollback_version_cleanup(user_id, &operation_id)
            .await;
        for archive_id in &moved_archive_ids {
            archive_cache.clear_archive_cache(archive_id).await;
        }
        archive_cache.clear_archive_cache(keep_archive_id).await;
        if let Err(rollback_error) = rollback_result {
            return Err(anyhow::anyhow!(
                "version cleanup could not be activated and rollback failed: {rollback_error:#}"
            ));
        }
        return Err(anyhow::anyhow!("version cleanup could not be activated"));
    }

    Ok(VersionCleanupResponse {
        kept_archive_id: keep_archive_id.to_string(),
        deleted,
        failed_archive_ids,
        operation_id,
    })
}

pub(crate) fn canonical_archive_ids(ids: &[String]) -> Vec<String> {
    let mut canonical = ids
        .iter()
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    canonical.sort();
    canonical
}

pub(crate) async fn load_idempotent_version_cleanup(
    pool: &Pool<Sqlite>,
    user_id: &str,
    idempotency_key: &str,
    group_id: &str,
    keep_archive_id: &str,
    delete_archive_ids: &[String],
) -> Result<Option<VersionCleanupResponse>> {
    let Some(existing) = sqlx::query(
        "SELECT id, status, migration_snapshot_json FROM trash_operations
         WHERE user_id = ? AND idempotency_key = ? AND operation_type = 'version_cleanup'",
    )
    .bind(user_id)
    .bind(idempotency_key)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    let operation_id: String = existing.get("id");
    let status: String = existing.get("status");
    let snapshot: VersionCleanupRequestSnapshot = serde_json::from_str(
        existing
            .get::<String, _>("migration_snapshot_json")
            .as_str(),
    )
    .context("failed to decode version cleanup idempotency snapshot")?;
    if !matches!(snapshot.version, 0 | 1)
        || snapshot.group_id != group_id
        || snapshot.keep_archive_id != keep_archive_id
        || canonical_archive_ids(&snapshot.delete_archive_ids) != delete_archive_ids
    {
        return Err(anyhow::anyhow!(
            "idempotency key was used for a different version cleanup"
        ));
    }

    match status.as_str() {
        "active" => {
            let member_ids = sqlx::query_scalar::<_, String>(
                "SELECT m.archive_id FROM trash_operation_members m
                 JOIN trash_entries t ON t.id = m.trash_entry_id
                 WHERE m.operation_id = ? AND t.status = 'active'
                 ORDER BY m.archive_id",
            )
            .bind(&operation_id)
            .fetch_all(pool)
            .await?;
            if member_ids != snapshot.delete_archive_ids {
                return Err(anyhow::anyhow!(
                    "version cleanup operation is no longer active"
                ));
            }
            Ok(Some(VersionCleanupResponse {
                kept_archive_id: snapshot.keep_archive_id,
                deleted: member_ids.len(),
                failed_archive_ids: Vec::new(),
                operation_id,
            }))
        }
        "failed" => Ok(Some(VersionCleanupResponse {
            kept_archive_id: snapshot.keep_archive_id,
            deleted: 0,
            failed_archive_ids: snapshot.failed_archive_ids,
            operation_id,
        })),
        "pending" | "restoring" => Err(anyhow::anyhow!(
            "version cleanup operation is already in progress"
        )),
        "restored" | "expired" => Err(anyhow::anyhow!(
            "version cleanup operation is no longer active"
        )),
        _ => Err(anyhow::anyhow!(
            "version cleanup operation has an invalid status"
        )),
    }
}
pub(crate) fn version_priority(group: &VersionGroup) -> u8 {
    if group.status == "keep_all" {
        2
    } else if group.recommended_archive_id.is_some() {
        0
    } else {
        1
    }
}

pub(crate) fn version_group_id(group_key: &str) -> String {
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

pub(crate) fn unit_label(fact: &IdentityFact) -> String {
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
pub(crate) fn recommend_version(
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
