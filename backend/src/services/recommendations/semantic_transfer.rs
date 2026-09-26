//! One-hop, positive-only transfer from independently published synonym edges.
use anyhow::Result;
use serde_json::Value;
use sqlx::{Pool, Row, Sqlite};
use std::collections::{HashMap, HashSet};

use crate::models::Archive;
use crate::services::preferences::learning::{
    condition_feature_key, condition_is_ordinary_tag, observing_soft_lift,
};
use crate::services::recommendations::namespace_policy::load_metadata_namespace_set;

#[derive(Debug, Clone)]
pub struct Match {
    pub tag_a_id: String,
    pub tag_b_id: String,
    pub base: f64,
}

pub async fn positive_seeds(pool: &Pool<Sqlite>, user_id: &str) -> Result<HashMap<String, f64>> {
    let metadata = load_metadata_namespace_set(pool).await?;
    let rows = sqlx::query(
        "SELECT c.conditions_json, c.feature_kind, c.status, c.evidence_state,
                c.unique_archive_count, c.informative_result_count,
                c.positive_support, c.negative_support, c.lift, r.action
         FROM preference_rule_candidates c
         LEFT JOIN preference_rules r ON r.user_id = c.user_id
             AND r.conditions_json = c.conditions_json AND r.source = 'learned_cold_start'
             AND r.enabled = 1 AND r.auto_paused = 0
         WHERE c.user_id = ? AND c.source = 'cold_start_v1'
           AND ((c.status = 'observing' AND c.evidence_state = 'observing' AND c.lift > 0)
             OR (c.status = 'promoted' AND c.evidence_state = 'eligible' AND r.action = 'keep'))",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let mut identities = HashMap::new();
    for row in rows {
        if row.get::<Option<String>, _>("feature_kind").as_deref() != Some("binary") {
            continue;
        }
        let Ok(condition) = serde_json::from_str::<Value>(&row.get::<String, _>("conditions_json"))
        else {
            continue;
        };
        if !condition_is_ordinary_tag(&condition, &metadata) {
            continue;
        }
        let Some(key) = condition_feature_key(&condition) else {
            continue;
        };
        let Some(identity) = key.strip_prefix("tag:") else {
            continue;
        };
        let strength = if row.get::<String, _>("status") == "promoted" {
            1.0
        } else {
            observing_soft_lift(
                row.get("unique_archive_count"),
                row.get("informative_result_count"),
                row.get("positive_support"),
                row.get("negative_support"),
                row.get("lift"),
            )
            .unwrap_or(0.0)
        };
        if strength > 0.0 {
            identities
                .entry(identity.to_ascii_lowercase())
                .and_modify(|v: &mut f64| *v = v.max(strength))
                .or_insert(strength);
        }
    }
    if identities.is_empty() {
        return Ok(HashMap::new());
    }
    let mut seeds = HashMap::new();
    for row in sqlx::query(
        "SELECT id, lower(trim(namespace)) || ':' || lower(trim(name)) AS identity FROM tags",
    )
    .fetch_all(pool)
    .await?
    {
        if let Some(strength) = identities.get(&row.get::<String, _>("identity")) {
            seeds.insert(row.get("id"), *strength);
        }
    }
    Ok(seeds)
}

pub async fn published_neighbors(
    pool: &Pool<Sqlite>,
    seeds: &HashMap<String, f64>,
) -> Result<Vec<(String, String, String, f64)>> {
    if seeds.is_empty() {
        return Ok(Vec::new());
    }
    let mut ids = seeds.keys().collect::<Vec<_>>();
    ids.sort_by(|a, b| {
        seeds
            .get(*b)
            .unwrap_or(&0.0)
            .total_cmp(seeds.get(*a).unwrap_or(&0.0))
            .then_with(|| a.cmp(b))
    });
    ids.truncate(100);
    let placeholders = vec!["?"; ids.len()].join(",");
    let query = format!(
        "SELECT tag_a_id, tag_b_id FROM tag_semantic_edges
         WHERE status = 'published' AND relation_kind = 'same_meaning'
           AND forward_choice = 'same_meaning' AND reverse_choice = 'same_meaning'
           AND selected_confidence >= 0.95
           AND forward_confidence >= 0.95 AND reverse_confidence >= 0.95
           AND (tag_a_id IN ({placeholders}) OR tag_b_id IN ({placeholders}))"
    );
    let mut request = sqlx::query(&query);
    for id in &ids {
        request = request.bind(*id);
    }
    for id in &ids {
        request = request.bind(*id);
    }
    let mut neighbors = Vec::new();
    for row in request.fetch_all(pool).await? {
        let a: String = row.get("tag_a_id");
        let b: String = row.get("tag_b_id");
        if let Some(base) = seeds.get(&a) {
            neighbors.push((a.clone(), b.clone(), b.clone(), *base));
        }
        if let Some(base) = seeds.get(&b) {
            neighbors.push((a.clone(), b.clone(), a, *base));
        }
    }
    Ok(neighbors)
}

pub fn match_archives(
    archives: &[Archive],
    seeds: &HashMap<String, f64>,
    neighbors: &[(String, String, String, f64)],
) -> HashMap<String, Match> {
    let mut by_target: HashMap<&str, Vec<&(String, String, String, f64)>> = HashMap::new();
    for neighbor in neighbors {
        if !seeds.contains_key(&neighbor.0) && !seeds.contains_key(&neighbor.1) {
            continue;
        }
        by_target
            .entry(neighbor.2.as_str())
            .or_default()
            .push(neighbor);
    }
    let mut matched = HashMap::new();
    for archive in archives {
        let tags = archive
            .tags
            .iter()
            .map(|tag| tag.id.as_str())
            .collect::<HashSet<_>>();
        if tags.iter().any(|tag| seeds.contains_key(*tag)) {
            continue;
        }
        let best = tags
            .iter()
            .filter_map(|tag| by_target.get(tag))
            .flat_map(|edges| edges.iter().copied())
            .max_by(|a, b| {
                a.3.total_cmp(&b.3)
                    .then_with(|| b.0.cmp(&a.0))
                    .then_with(|| b.1.cmp(&a.1))
            });
        if let Some((a, b, _, base)) = best {
            matched.insert(
                archive.id.clone(),
                Match {
                    tag_a_id: a.clone(),
                    tag_b_id: b.clone(),
                    base: *base,
                },
            );
        }
    }
    matched
}

pub async fn policy_snapshot(pool: &Pool<Sqlite>) -> Result<(f64, i64)> {
    let row = sqlx::query("SELECT weight, version FROM semantic_transfer_policy WHERE id = 1")
        .fetch_one(pool)
        .await?;
    Ok((
        row.get::<f64, _>("weight").clamp(0.0, 0.3),
        row.get("version"),
    ))
}

pub async fn review_policy(pool: &Pool<Sqlite>) -> Result<()> {
    let row = sqlx::query(
        "SELECT weight, version FROM semantic_transfer_policy WHERE id = 1
         AND (reviewed_at IS NULL OR reviewed_at < datetime('now', '-7 days'))",
    )
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(());
    };
    let weight: f64 = row.get("weight");
    let version: i64 = row.get("version");
    // Only compare mature, attributed unknown-tier exposures under the same policy version.
    // No click/open proxy is treated as a successful read; deletion is a negative even unopened.
    let rows = sqlx::query(
        "SELECT s.semantic_arm AS arm, COUNT(*) AS exposures,
                COUNT(DISTINCT s.user_id) AS users, COUNT(DISTINCT s.id) AS sessions,
                SUM(CASE WHEN i.effective_read_at IS NOT NULL THEN 1 ELSE 0 END) AS reads,
                SUM(CASE WHEN i.manual_delete_at IS NOT NULL THEN 1 ELSE 0 END) AS deletes
         FROM random_recommendation_items i JOIN random_recommendation_sessions s ON s.id = i.session_id
         WHERE i.semantic_edge_a_id IS NOT NULL AND i.preference_tier = 'unknown'
           AND s.algorithm_variant = 'weighted-v1' AND s.semantic_weight = ?
           AND s.semantic_arm IN ('control', 'treatment')
           AND i.created_at >= datetime('now', '-30 days')
           AND i.created_at < datetime('now', '-2 days')
         GROUP BY s.semantic_arm",
    ).bind(weight).fetch_all(pool).await?;
    let arm = |name: &str| -> Option<(f64, f64)> {
        let r = rows.iter().find(|r| r.get::<String, _>("arm") == name)?;
        let count = r.get::<i64, _>("exposures");
        if count < 100 || r.get::<i64, _>("users") < 10 || r.get::<i64, _>("sessions") < 30 {
            return None;
        }
        Some((
            r.get::<i64, _>("reads") as f64 / count as f64,
            r.get::<i64, _>("deletes") as f64 / count as f64,
        ))
    };
    let (Some(control), Some(treatment)) = (arm("control"), arm("treatment")) else {
        sqlx::query(
            "UPDATE semantic_transfer_policy SET reviewed_at = CURRENT_TIMESTAMP
                     WHERE id = 1 AND version = ?",
        )
        .bind(version)
        .execute(pool)
        .await?;
        return Ok(());
    };
    let next = if treatment.1 > control.1 + 0.01 || treatment.0 + 0.02 < control.0 {
        (weight - 0.02).max(0.0)
    } else if treatment.0 > control.0 + 0.03 && treatment.1 <= control.1 {
        (weight + 0.02).min(0.3)
    } else {
        weight
    };
    sqlx::query(
        "UPDATE semantic_transfer_policy SET weight = ?, version = version + 1,
                reviewed_at = CURRENT_TIMESTAMP WHERE id = 1 AND version = ?",
    )
    .bind(next)
    .bind(version)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::tag::Tag;

    #[test]
    fn one_hop_is_single_best_edge_and_never_reinforces_direct_match() {
        let make = |id: &str, tags: &[&str]| Archive {
            id: id.into(),
            title: id.into(),
            subtitle: None,
            subtitle_language: None,
            path: id.into(),
            file_size: 1,
            page_count: 1,
            hash: id.into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: tags
                .iter()
                .map(|id| Tag {
                    id: (*id).into(),
                    name: (*id).into(),
                    namespace: "general".into(),
                    localized_name: None,
                })
                .collect(),
        };
        let seeds = HashMap::from([("a".into(), 0.7), ("c".into(), 0.5)]);
        let edges = vec![
            ("a".into(), "b".into(), "b".into(), 0.7),
            ("c".into(), "b".into(), "b".into(), 0.5),
            ("b".into(), "d".into(), "d".into(), 0.7),
        ];
        let matches = match_archives(
            &[
                make("one", &["b"]),
                make("direct", &["a", "b"]),
                make("two_hops", &["d"]),
            ],
            &seeds,
            &edges,
        );
        assert_eq!(matches["one"].base, 0.7);
        assert!(!matches.contains_key("direct"));
        assert!(!matches.contains_key("two_hops"));
    }
}
