//! Deterministic ordinary-tag co-occurrence graph.
//!
//! The graph describes archive-level association only. It is intentionally kept separate from
//! profile features and preference feedback so a relation can expand recall without becoming an
//! alias, a learned rule, or a path for negative feedback propagation.

use anyhow::Result;
use serde::Serialize;
use sqlx::{Pool, Row, Sqlite};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Mutex;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::Notify;

use super::namespace_policy::{is_system_managed_theme_namespace, load_metadata_namespace_set};

pub const TAG_COOCCURRENCE_RELATION_KIND: &str = "cooccurrence";
pub const TAG_COOCCURRENCE_ALGORITHM_VERSION: &str = "tag-cooccurrence-v1";

const GRAPH_REBUILD_DEBOUNCE: Duration = Duration::from_millis(250);
const MAX_LEXICAL_SEEDS_PER_TRIGGER: usize = 32;
const MAX_NAME_GRAMS_PER_SEED: usize = 24;
const MAX_NAME_LOOKUP_ROWS_PER_SEED: usize = 300;
const MAX_LEXICAL_CANDIDATES_PER_SEED: usize = 20;
const MIN_NAME_SIMILARITY: f64 = 0.35;
static TAG_COOCCURRENCE_SIGNAL: OnceLock<Arc<Notify>> = OnceLock::new();
static TAG_RELATION_PENDING_IDS: OnceLock<Arc<Mutex<BTreeSet<String>>>> = OnceLock::new();

fn tag_cooccurrence_signal() -> &'static Arc<Notify> {
    TAG_COOCCURRENCE_SIGNAL.get_or_init(|| Arc::new(Notify::new()))
}

fn pending_tag_relation_ids() -> &'static Arc<Mutex<BTreeSet<String>>> {
    TAG_RELATION_PENDING_IDS.get_or_init(|| Arc::new(Mutex::new(BTreeSet::new())))
}

/// Coalesces tag changes into an asynchronous graph rebuild.
pub fn notify_tag_cooccurrence_rebuild() {
    tag_cooccurrence_signal().notify_one();
}

/// Coalesces an ordinary tag mutation into the graph rebuild and bounded semantic candidate
/// planner. Only changed tag IDs are retained; lexical lookup limits returned rows but its
/// `LIKE` predicates may still scan the tags table until an indexed search path is measured.
pub fn notify_tag_cooccurrence_rebuild_for_tags(tag_ids: impl IntoIterator<Item = String>) {
    if let Ok(mut pending) = pending_tag_relation_ids().lock() {
        pending.extend(tag_ids.into_iter().filter(|id| !id.trim().is_empty()));
    }
    notify_tag_cooccurrence_rebuild();
}

/// Starts the process-local graph refresh worker. The source tag tables remain authoritative;
/// this worker only refreshes the derived co-occurrence snapshot after a short quiet period.
pub fn spawn_tag_cooccurrence_worker(pool: Pool<Sqlite>) {
    let signal = tag_cooccurrence_signal().clone();
    tokio::spawn(async move {
        loop {
            signal.notified().await;
            loop {
                let quiet_period = tokio::time::sleep(GRAPH_REBUILD_DEBOUNCE);
                tokio::pin!(quiet_period);
                tokio::select! {
                    _ = &mut quiet_period => break,
                    _ = signal.notified() => {}
                }
            }
            let rebuild_succeeded = match rebuild_tag_cooccurrence_edges(&pool).await {
                Ok(_) => true,
                Err(error) => {
                    tracing::warn!(%error, "failed to rebuild tag co-occurrence graph after tag change");
                    false
                }
            };
            if rebuild_succeeded {
                let changed_tag_ids = pending_tag_relation_ids()
                    .lock()
                    .map(|mut pending| std::mem::take(&mut *pending))
                    .unwrap_or_default();
                if !changed_tag_ids.is_empty() {
                    let changed_tag_ids = changed_tag_ids.into_iter().collect::<Vec<_>>();
                    if let Err(error) =
                        enqueue_semantic_candidates_for_changed_tags(&pool, &changed_tag_ids).await
                    {
                        if let Ok(mut pending) = pending_tag_relation_ids().lock() {
                            pending.extend(changed_tag_ids);
                        }
                        tracing::warn!(%error, "failed to enqueue bounded JEV tag relation candidates");
                    }
                }
            }
        }
    });
}

async fn enqueue_semantic_candidates_for_changed_tags(
    pool: &Pool<Sqlite>,
    changed_tag_ids: &[String],
) -> Result<()> {
    const NEIGHBORS_PER_TAG: usize = 20;
    // The neighbor query binds each seed twice. Its result can contain up to
    // `2 * NEIGHBORS_PER_TAG` distinct tag IDs per seed for the follow-up tag
    // lookup, so keep each batch comfortably below SQLite's bind-variable cap.
    const MAX_SEED_TAGS_PER_BATCH: usize = 8;
    const MAX_CANDIDATES: usize = 100;
    let settings = crate::services::load_ai_settings(pool).await?;
    if !crate::services::ai_service::tag_relation_is_available(&settings) {
        return Ok(());
    }
    let metadata_namespaces = load_metadata_namespace_set(pool).await?;
    let mut candidates = BTreeMap::new();
    'name_batches: for seed_batch in changed_tag_ids.chunks(MAX_SEED_TAGS_PER_BATCH) {
        let placeholders = std::iter::repeat("?")
            .take(seed_batch.len())
            .collect::<Vec<_>>()
            .join(",");
        let query = format!(
            "SELECT source.id AS source_id, source.namespace AS source_namespace,
                    source.name AS source_name, target.id AS target_id,
                    target.namespace AS target_namespace, target.name AS target_name,
                    (SELECT COUNT(DISTINCT archive_id) FROM archive_tags
                     WHERE tag_id = source.id) AS source_support,
                    (SELECT COUNT(DISTINCT archive_id) FROM archive_tags
                     WHERE tag_id = target.id) AS target_support
             FROM tags source JOIN tags target
               ON lower(trim(target.name)) = lower(trim(source.name))
              AND lower(trim(target.namespace)) <> lower(trim(source.namespace))
              AND target.id <> source.id
             WHERE source.id IN ({placeholders})
             ORDER BY lower(trim(source.name)), source.id, target.id
             LIMIT ?"
        );
        let mut request = sqlx::query(&query);
        for tag_id in seed_batch {
            request = request.bind(tag_id);
        }
        let rows = request.bind(MAX_CANDIDATES as i64).fetch_all(pool).await?;
        for row in rows {
            let source_namespace: String = row.try_get("source_namespace")?;
            let target_namespace: String = row.try_get("target_namespace")?;
            let source_namespace_normalized = source_namespace.trim().to_ascii_lowercase();
            let target_namespace_normalized = target_namespace.trim().to_ascii_lowercase();
            if is_system_managed_theme_namespace(&source_namespace_normalized)
                || is_system_managed_theme_namespace(&target_namespace_normalized)
                || metadata_namespaces.contains(&source_namespace_normalized)
                || metadata_namespaces.contains(&target_namespace_normalized)
                || normalized_tag_name(row.try_get::<String, _>("source_name")?.as_str())
                    != normalized_tag_name(row.try_get::<String, _>("target_name")?.as_str())
            {
                continue;
            }
            let source = crate::services::recommendations::semantic_edges::TagRelationTag {
                id: row.try_get("source_id")?,
                namespace: source_namespace,
                name: row.try_get("source_name")?,
                support_count: row.try_get::<i64, _>("source_support")?.max(0) as u32,
            };
            let target = crate::services::recommendations::semantic_edges::TagRelationTag {
                id: row.try_get("target_id")?,
                namespace: target_namespace,
                name: row.try_get("target_name")?,
                support_count: row.try_get::<i64, _>("target_support")?.max(0) as u32,
            };
            let pair_id = crate::services::recommendations::semantic_edges::canonical_pair_id(
                &source.id, &target.id,
            );
            candidates
                .entry(pair_id)
                .or_insert_with(|| (source, target));
            if candidates.len() >= MAX_CANDIDATES {
                break 'name_batches;
            }
        }
    }
    'seed_batches: for seed_batch in changed_tag_ids.chunks(MAX_SEED_TAGS_PER_BATCH) {
        if candidates.len() >= MAX_CANDIDATES {
            break;
        }
        let edges = load_tag_cooccurrence_neighbors(pool, seed_batch, NEIGHBORS_PER_TAG).await?;
        let mut ids = BTreeSet::new();
        for edge in &edges {
            ids.insert(edge.tag_a_id.clone());
            ids.insert(edge.tag_b_id.clone());
        }
        if ids.len() < 2 {
            continue;
        }

        let placeholders = std::iter::repeat("?")
            .take(ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let query = format!(
            "SELECT t.id, t.namespace, t.name,
                    (SELECT COUNT(DISTINCT at.archive_id) FROM archive_tags at WHERE at.tag_id = t.id) AS support_count
             FROM tags t WHERE t.id IN ({placeholders})"
        );
        let mut request = sqlx::query(&query);
        for id in &ids {
            request = request.bind(id);
        }
        let mut tags = HashMap::new();
        for row in request.fetch_all(pool).await? {
            let namespace: String = row.try_get("namespace")?;
            let normalized_namespace = namespace.trim().to_ascii_lowercase();
            if is_system_managed_theme_namespace(&normalized_namespace)
                || metadata_namespaces.contains(&normalized_namespace)
            {
                continue;
            }
            tags.insert(
                row.try_get::<String, _>("id")?,
                crate::services::recommendations::semantic_edges::TagRelationTag {
                    id: row.try_get("id")?,
                    namespace,
                    name: row.try_get("name")?,
                    support_count: row.try_get::<i64, _>("support_count")?.max(0) as u32,
                },
            );
        }

        for edge in edges {
            let Some(tag_a) = tags.get(&edge.tag_a_id) else {
                continue;
            };
            let Some(tag_b) = tags.get(&edge.tag_b_id) else {
                continue;
            };
            candidates
                .entry(format!("{}:{}", edge.tag_a_id, edge.tag_b_id))
                .or_insert_with(|| (tag_a.clone(), tag_b.clone()));
            if candidates.len() >= MAX_CANDIDATES {
                break 'seed_batches;
            }
        }
    }
    for changed_tag_id in changed_tag_ids.iter().take(MAX_LEXICAL_SEEDS_PER_TRIGGER) {
        if candidates.len() >= MAX_CANDIDATES {
            break;
        }
        let Some(row) = sqlx::query(
            "SELECT t.id, t.namespace, t.name,
                    (SELECT COUNT(DISTINCT at.archive_id) FROM archive_tags at \
                     WHERE at.tag_id = t.id) AS support_count \
             FROM tags t WHERE t.id = ?",
        )
        .bind(changed_tag_id)
        .fetch_optional(pool)
        .await?
        else {
            continue;
        };
        let source_namespace: String = row.try_get("namespace")?;
        let normalized_namespace = source_namespace.trim().to_ascii_lowercase();
        if is_system_managed_theme_namespace(&normalized_namespace)
            || metadata_namespaces.contains(&normalized_namespace)
        {
            continue;
        }
        let source = crate::services::recommendations::semantic_edges::TagRelationTag {
            id: row.try_get("id")?,
            namespace: source_namespace,
            name: row.try_get("name")?,
            support_count: row.try_get::<i64, _>("support_count")?.max(0) as u32,
        };
        let grams = lexical_name_grams(&source.name);
        if grams.is_empty() {
            continue;
        }
        let excluded_namespaces = metadata_namespaces
            .iter()
            .cloned()
            .chain(std::iter::once("theme".to_string()))
            .collect::<BTreeSet<_>>();
        let clauses = grams
            .iter()
            .map(|_| "lower(t.name) LIKE ?")
            .collect::<Vec<_>>()
            .join(" OR ");
        let namespace_placeholders = std::iter::repeat("?")
            .take(excluded_namespaces.len())
            .collect::<Vec<_>>()
            .join(",");
        let query = format!(
            "SELECT t.id, t.namespace, t.name, \
                    (SELECT COUNT(DISTINCT at.archive_id) FROM archive_tags at \
                     WHERE at.tag_id = t.id) AS support_count \
             FROM tags t WHERE t.id <> ? \
               AND lower(trim(t.namespace)) NOT IN ({namespace_placeholders}) \
               AND ({clauses}) \
             ORDER BY ABS(length(trim(t.name)) - ?), lower(trim(t.name)), t.id LIMIT ?"
        );
        let mut request = sqlx::query(&query).bind(&source.id);
        for namespace in &excluded_namespaces {
            request = request.bind(namespace);
        }
        for gram in &grams {
            request = request.bind(format!("%{gram}%"));
        }
        let source_name_length = source.name.chars().count() as i64;
        let rows = request
            .bind(source_name_length)
            .bind(MAX_NAME_LOOKUP_ROWS_PER_SEED as i64)
            .fetch_all(pool)
            .await?;
        let mut lexical_candidates = Vec::new();
        for row in rows {
            let namespace: String = row.try_get("namespace")?;
            let normalized_namespace = namespace.trim().to_ascii_lowercase();
            if is_system_managed_theme_namespace(&normalized_namespace)
                || metadata_namespaces.contains(&normalized_namespace)
            {
                continue;
            }
            let target = crate::services::recommendations::semantic_edges::TagRelationTag {
                id: row.try_get("id")?,
                namespace,
                name: row.try_get("name")?,
                support_count: row.try_get::<i64, _>("support_count")?.max(0) as u32,
            };
            if normalized_tag_name(&source.name) == normalized_tag_name(&target.name) {
                continue;
            }
            let similarity = lexical_name_similarity(&source.name, &target.name);
            if similarity < MIN_NAME_SIMILARITY {
                continue;
            }
            let pair_id = crate::services::recommendations::semantic_edges::canonical_pair_id(
                &source.id, &target.id,
            );
            lexical_candidates.push((similarity, pair_id, target));
        }
        lexical_candidates.sort_by(|left, right| {
            right
                .0
                .total_cmp(&left.0)
                .then_with(|| left.1.cmp(&right.1))
        });
        for (_, pair_id, target) in lexical_candidates
            .into_iter()
            .take(MAX_LEXICAL_CANDIDATES_PER_SEED)
        {
            candidates
                .entry(pair_id)
                .or_insert_with(|| (source.clone(), target));
            if candidates.len() >= MAX_CANDIDATES {
                break;
            }
        }
    }
    let pairs = candidates
        .into_iter()
        .take(MAX_CANDIDATES)
        .map(|(_, (tag_a, tag_b))| {
            crate::services::recommendations::semantic_edges::TagRelationPair {
                pair_id: String::new(),
                tag_a,
                tag_b,
                pair_input_hash: String::new(),
            }
            .canonicalize()
        })
        .collect::<Result<Vec<_>>>()?;
    let _ = crate::services::enqueue_tag_relation_jev_candidates(pool, &settings, &pairs).await?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TagCooccurrenceEdge {
    pub tag_a_id: String,
    pub tag_b_id: String,
    pub coarchive_count: i64,
    pub tag_a_archive_count: i64,
    pub tag_b_archive_count: i64,
    pub jaccard: f64,
    pub relation_kind: String,
    pub algorithm_version: String,
    pub updated_at: String,
}

/// Rebuilds the current graph from distinct ordinary tags per archive.
///
/// This is an explicit, deterministic maintenance entry point. It does not infer semantics and
/// does not alter the source tag associations or any user preference data.
pub async fn rebuild_tag_cooccurrence_edges(pool: &Pool<Sqlite>) -> Result<usize> {
    let metadata_namespaces = load_metadata_namespace_set(pool).await?;
    let rows = sqlx::query(
        "SELECT at.archive_id, t.id, t.namespace
         FROM archive_tags at
         JOIN tags t ON t.id = at.tag_id
         ORDER BY at.archive_id, t.id",
    )
    .fetch_all(pool)
    .await?;

    let mut tags_by_archive: HashMap<String, BTreeSet<String>> = HashMap::new();
    for row in rows {
        let namespace: String = row.get("namespace");
        let normalized_namespace = namespace.trim().to_ascii_lowercase();
        if is_system_managed_theme_namespace(&normalized_namespace)
            || metadata_namespaces.contains(&normalized_namespace)
        {
            continue;
        }
        tags_by_archive
            .entry(row.get("archive_id"))
            .or_default()
            .insert(row.get("id"));
    }

    let mut archive_counts: HashMap<String, i64> = HashMap::new();
    let mut pair_counts: HashMap<(String, String), i64> = HashMap::new();
    for tag_ids in tags_by_archive.values() {
        let tag_ids = tag_ids.iter().cloned().collect::<Vec<_>>();
        for tag_id in &tag_ids {
            *archive_counts.entry(tag_id.clone()).or_default() += 1;
        }
        for (index, left) in tag_ids.iter().enumerate() {
            for right in tag_ids.iter().skip(index + 1) {
                pair_counts
                    .entry((left.clone(), right.clone()))
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
            }
        }
    }

    let mut transaction = pool.begin().await?;
    sqlx::query("DELETE FROM tag_cooccurrence_edges WHERE relation_kind = ?")
        .bind(TAG_COOCCURRENCE_RELATION_KIND)
        .execute(&mut *transaction)
        .await?;

    let mut inserted = 0;
    for ((tag_a_id, tag_b_id), coarchive_count) in pair_counts {
        if coarchive_count < 2 {
            continue;
        }
        let tag_a_archive_count = archive_counts.get(&tag_a_id).copied().unwrap_or_default();
        let tag_b_archive_count = archive_counts.get(&tag_b_id).copied().unwrap_or_default();
        let union_count = tag_a_archive_count + tag_b_archive_count - coarchive_count;
        if union_count <= 0 {
            continue;
        }
        let jaccard = coarchive_count as f64 / union_count as f64;
        sqlx::query(
            "INSERT INTO tag_cooccurrence_edges
             (tag_a_id, tag_b_id, coarchive_count, tag_a_archive_count,
              tag_b_archive_count, jaccard, relation_kind, algorithm_version, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)",
        )
        .bind(&tag_a_id)
        .bind(&tag_b_id)
        .bind(coarchive_count)
        .bind(tag_a_archive_count)
        .bind(tag_b_archive_count)
        .bind(jaccard)
        .bind(TAG_COOCCURRENCE_RELATION_KIND)
        .bind(TAG_COOCCURRENCE_ALGORITHM_VERSION)
        .execute(&mut *transaction)
        .await?;
        inserted += 1;
    }
    transaction.commit().await?;
    Ok(inserted)
}

/// Loads graph edges touching any seed tag. Edges are returned once even when multiple seeds
/// share the same relation; callers can use the stored counts for explanations.
pub async fn load_tag_cooccurrence_neighbors(
    pool: &Pool<Sqlite>,
    seed_tag_ids: &[String],
    limit_per_seed: usize,
) -> Result<Vec<TagCooccurrenceEdge>> {
    let seeds = seed_tag_ids
        .iter()
        .filter(|tag_id| !tag_id.trim().is_empty())
        .cloned()
        .collect::<BTreeSet<_>>();
    if seeds.is_empty() || limit_per_seed == 0 {
        return Ok(Vec::new());
    }
    let placeholders = seeds.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT tag_a_id, tag_b_id, coarchive_count, tag_a_archive_count,
                tag_b_archive_count, jaccard, relation_kind, algorithm_version, updated_at
         FROM tag_cooccurrence_edges
         WHERE relation_kind = ? AND algorithm_version = ?
           AND (tag_a_id IN ({placeholders}) OR tag_b_id IN ({placeholders}))
         ORDER BY jaccard DESC, coarchive_count DESC, tag_a_id, tag_b_id"
    );
    let mut request = sqlx::query(&query)
        .bind(TAG_COOCCURRENCE_RELATION_KIND)
        .bind(TAG_COOCCURRENCE_ALGORITHM_VERSION);
    for tag_id in &seeds {
        request = request.bind(tag_id);
    }
    for tag_id in &seeds {
        request = request.bind(tag_id);
    }

    let mut edges = Vec::new();
    let mut per_seed_counts: HashMap<String, usize> = HashMap::new();
    for row in request.fetch_all(pool).await? {
        let edge = edge_from_row(&row);
        let touching_seeds = seeds
            .iter()
            .filter(|seed| **seed == edge.tag_a_id || **seed == edge.tag_b_id)
            .cloned()
            .collect::<Vec<_>>();
        if touching_seeds
            .iter()
            .any(|seed| per_seed_counts.get(seed).copied().unwrap_or_default() < limit_per_seed)
        {
            for seed in touching_seeds {
                let count = per_seed_counts.entry(seed).or_default();
                if *count < limit_per_seed {
                    *count += 1;
                }
            }
            edges.push(edge);
        }
    }
    Ok(edges)
}

/// Returns unique neighbor IDs for candidate recall. Seed IDs are excluded from the result, and
/// a neighbor shared by several seeds is returned only once.
pub async fn expand_tag_cooccurrence_ids(
    pool: &Pool<Sqlite>,
    seed_tag_ids: &[String],
    limit_per_seed: usize,
    total_limit: usize,
) -> Result<Vec<String>> {
    if total_limit == 0 {
        return Ok(Vec::new());
    }
    let seeds = seed_tag_ids.iter().cloned().collect::<HashSet<_>>();
    let edges = load_tag_cooccurrence_neighbors(pool, seed_tag_ids, limit_per_seed).await?;
    let mut ranked: HashMap<String, (f64, i64)> = HashMap::new();
    for edge in edges {
        let neighbor = if seeds.contains(&edge.tag_a_id) {
            edge.tag_b_id
        } else {
            edge.tag_a_id
        };
        if seeds.contains(&neighbor) {
            continue;
        }
        let entry = ranked
            .entry(neighbor)
            .or_insert((edge.jaccard, edge.coarchive_count));
        if (edge.jaccard, edge.coarchive_count) > *entry {
            *entry = (edge.jaccard, edge.coarchive_count);
        }
    }
    let mut ranked = ranked.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|(left_id, left_score), (right_id, right_score)| {
        right_score
            .partial_cmp(left_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left_id.cmp(right_id))
    });
    ranked.truncate(total_limit);
    Ok(ranked.into_iter().map(|(tag_id, _)| tag_id).collect())
}

fn edge_from_row(row: &sqlx::sqlite::SqliteRow) -> TagCooccurrenceEdge {
    TagCooccurrenceEdge {
        tag_a_id: row.get("tag_a_id"),
        tag_b_id: row.get("tag_b_id"),
        coarchive_count: row.get("coarchive_count"),
        tag_a_archive_count: row.get("tag_a_archive_count"),
        tag_b_archive_count: row.get("tag_b_archive_count"),
        jaccard: row.get("jaccard"),
        relation_kind: row.get("relation_kind"),
        algorithm_version: row.get("algorithm_version"),
        updated_at: row.get("updated_at"),
    }
}

fn normalized_tag_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn normalized_name_tokens(name: &str) -> BTreeSet<String> {
    name.split(|character: char| !character.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|token| token.chars().count() >= 3)
        .collect()
}

fn lexical_name_grams(name: &str) -> Vec<String> {
    let tokens = normalized_name_tokens(name);
    let mut grams = BTreeSet::new();
    for token in tokens {
        grams.insert(token.clone());
        let characters = token.chars().collect::<Vec<_>>();
        for trigram in characters.windows(3) {
            grams.insert(trigram.iter().copied().collect());
        }
    }
    grams.into_iter().take(MAX_NAME_GRAMS_PER_SEED).collect()
}

fn lexical_name_similarity(left: &str, right: &str) -> f64 {
    let left_tokens = normalized_name_tokens(left);
    let right_tokens = normalized_name_tokens(right);
    let token_similarity = set_jaccard(&left_tokens, &right_tokens);
    let left_grams = name_trigrams(left);
    let right_grams = name_trigrams(right);
    token_similarity.max(set_jaccard(&left_grams, &right_grams))
}

fn name_trigrams(name: &str) -> BTreeSet<String> {
    normalized_name_tokens(name)
        .into_iter()
        .flat_map(|token| {
            let characters = token.chars().collect::<Vec<_>>();
            characters
                .windows(3)
                .map(|trigram| trigram.iter().copied().collect::<String>())
                .collect::<Vec<String>>()
        })
        .collect()
}

fn set_jaccard<T: Ord>(left: &BTreeSet<T>, right: &BTreeSet<T>) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let intersection = left.intersection(right).count();
    let union = left.union(right).count();
    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("SQLite test database should connect");
        crate::database::run_sqlite_migrations(&pool)
            .await
            .expect("co-occurrence migration should succeed");
        pool
    }

    #[tokio::test]
    async fn rebuild_uses_distinct_archive_support_and_excludes_theme_metadata() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO archives (id, title, path, file_hash, file_size, page_count) VALUES
             ('archive-1', 'one', '/one.cbz', 'hash-1', 1, 1),
             ('archive-2', 'two', '/two.cbz', 'hash-2', 1, 1),
             ('archive-3', 'three', '/three.cbz', 'hash-3', 1, 1),
             ('archive-4', 'four', '/four.cbz', 'hash-4', 1, 1)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES
             ('tag-a', 'A', 'general'),
             ('tag-b', 'B', 'general'),
             ('tag-c', 'C', 'general'),
             ('tag-theme', 'Theme', 'theme'),
             ('tag-artist', 'Artist', 'artist')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO archive_tags (archive_id, tag_id) VALUES
             ('archive-1', 'tag-a'), ('archive-1', 'tag-b'), ('archive-1', 'tag-artist'),
             ('archive-2', 'tag-a'), ('archive-2', 'tag-b'),
             ('archive-2', 'tag-c'), ('archive-3', 'tag-a'), ('archive-3', 'tag-c'),
             ('archive-4', 'tag-b'), ('archive-4', 'tag-c')",
        )
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(rebuild_tag_cooccurrence_edges(&pool).await.unwrap(), 3);
        let edge = sqlx::query(
            "SELECT coarchive_count, tag_a_archive_count, tag_b_archive_count, jaccard
             FROM tag_cooccurrence_edges WHERE tag_a_id = 'tag-a' AND tag_b_id = 'tag-b'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(edge.get::<i64, _>("coarchive_count"), 2);
        assert_eq!(edge.get::<i64, _>("tag_a_archive_count"), 3);
        assert_eq!(edge.get::<i64, _>("tag_b_archive_count"), 3);
        assert!((edge.get::<f64, _>("jaccard") - 0.5).abs() < 1e-9);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM tag_cooccurrence_edges
                 WHERE tag_a_id = 'tag-theme' OR tag_b_id = 'tag-theme'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn neighbor_expansion_is_unique_and_does_not_return_seed_tags() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO archives (id, title, path, file_hash, file_size, page_count) VALUES
             ('archive-1', 'one', '/one.cbz', 'hash-1', 1, 1),
             ('archive-2', 'two', '/two.cbz', 'hash-2', 1, 1),
             ('archive-3', 'three', '/three.cbz', 'hash-3', 1, 1)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES
             ('tag-a', 'A', 'general'), ('tag-b', 'B', 'general'),
             ('tag-c', 'C', 'general'), ('tag-d', 'D', 'general')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
             "INSERT INTO archive_tags (archive_id, tag_id) VALUES
             ('archive-1', 'tag-a'), ('archive-1', 'tag-b'), ('archive-1', 'tag-c'),
             ('archive-2', 'tag-a'), ('archive-2', 'tag-b'), ('archive-2', 'tag-c'), ('archive-2', 'tag-d'),
             ('archive-3', 'tag-a'), ('archive-3', 'tag-b'), ('archive-3', 'tag-d')",
        )
        .execute(&pool)
        .await
        .unwrap();
        rebuild_tag_cooccurrence_edges(&pool).await.unwrap();

        let expanded =
            expand_tag_cooccurrence_ids(&pool, &["tag-a".to_string(), "tag-b".to_string()], 20, 20)
                .await
                .unwrap();
        assert_eq!(expanded, vec!["tag-c".to_string(), "tag-d".to_string()]);
    }

    #[tokio::test]
    async fn semantic_candidate_planner_chunks_large_changed_tag_sets() {
        let pool = test_pool().await;
        let mut settings = crate::services::load_ai_settings(&pool).await.unwrap();
        settings.features.recommendations.tag_graph_enabled = true;
        crate::services::save_ai_settings(&pool, settings)
            .await
            .unwrap();

        // Only the first 128 changed tags need backing edges. The remaining IDs model a large
        // pending set and must not be expanded into one oversized IN (...) expression.
        for index in 0..128 {
            let left_id = format!("changed-{index:03}");
            let right_id = format!("neighbor-{index:03}");
            sqlx::query(
                "INSERT INTO tags (id, name, namespace) VALUES (?, ?, 'general'), (?, ?, 'general')",
            )
            .bind(&left_id)
            .bind(format!("changed {index}"))
            .bind(&right_id)
            .bind(format!("neighbor {index}"))
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query(
                "INSERT INTO tag_cooccurrence_edges
                 (tag_a_id, tag_b_id, coarchive_count, tag_a_archive_count,
                  tag_b_archive_count, jaccard, relation_kind, algorithm_version)
                 VALUES (?, ?, 2, 2, 2, 1.0, ?, ?)",
            )
            .bind(&left_id)
            .bind(&right_id)
            .bind(TAG_COOCCURRENCE_RELATION_KIND)
            .bind(TAG_COOCCURRENCE_ALGORITHM_VERSION)
            .execute(&pool)
            .await
            .unwrap();
        }

        let changed_tag_ids = (0..20_000)
            .map(|index| {
                if index < 128 {
                    format!("changed-{index:03}")
                } else {
                    format!("missing-{index:05}")
                }
            })
            .collect::<Vec<_>>();
        enqueue_semantic_candidates_for_changed_tags(&pool, &changed_tag_ids)
            .await
            .expect("large changed-tag batches should stay within SQLite bind limits");
    }

    #[test]
    fn lexical_similarity_recalls_token_order_and_spelling_variants() {
        assert_eq!(
            lexical_name_similarity("silver meadow", "meadow silver"),
            1.0
        );
        assert!(lexical_name_similarity("silver meadow", "silvery meadow") >= MIN_NAME_SIMILARITY);
        assert_eq!(
            lexical_name_similarity("paper lantern", "orchid river"),
            0.0
        );
    }

    #[tokio::test]
    async fn lexical_recall_queues_a_noncooccurring_ordinary_tag_pair_only() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES
             ('tag-seed', 'luminous garden', 'general'),
             ('tag-lexical', 'garden luminous', 'general'),
             ('tag-theme', 'garden luminous', 'theme'),
             ('tag-unrelated', 'paper lantern', 'general')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let mut settings = crate::services::load_ai_settings(&pool).await.unwrap();
        settings.features.recommendations.tag_graph_enabled = true;
        settings.features.recommendations.tag_relation.api_key =
            Some("test-provider-key".to_string());
        crate::services::save_ai_settings(&pool, settings)
            .await
            .unwrap();

        enqueue_semantic_candidates_for_changed_tags(&pool, &["tag-seed".to_string()])
            .await
            .unwrap();
        let payloads = sqlx::query_scalar::<_, String>(
            "SELECT payload FROM ai_processing_queue WHERE job_type = 'tag_relation_jev' \
             AND status IN ('pending', 'processing', 'waiting_dependency')",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let candidate_pairs = payloads
            .iter()
            .flat_map(|payload| {
                let value: serde_json::Value = serde_json::from_str(payload).unwrap();
                value["pairs"].as_array().unwrap().clone()
            })
            .collect::<Vec<_>>();
        assert_eq!(candidate_pairs.len(), 1);
        let pair_ids = [
            candidate_pairs[0]["tag_a"]["id"].as_str().unwrap(),
            candidate_pairs[0]["tag_b"]["id"].as_str().unwrap(),
        ];
        assert!(pair_ids.contains(&"tag-seed"));
        assert!(pair_ids.contains(&"tag-lexical"));
        assert!(!pair_ids.contains(&"tag-theme"));
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM tag_cooccurrence_edges WHERE tag_a_id = 'tag-seed' \
                 OR tag_b_id = 'tag-seed'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            0,
            "the retrieved pair has no co-occurrence support"
        );
    }
}
