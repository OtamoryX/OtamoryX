//! Deterministic ordinary-tag co-occurrence graph.
//!
//! The graph describes archive-level association only. It is intentionally kept separate from
//! profile features and preference feedback so a relation can expand recall without becoming an
//! alias, a learned rule, or a path for negative feedback propagation.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::Notify;

use super::namespace_policy::{
    is_system_managed_theme_namespace, load_metadata_namespace_set,
    METADATA_NAMESPACE_POLICY_VERSION,
};

pub const TAG_COOCCURRENCE_RELATION_KIND: &str = "cooccurrence";
pub const TAG_COOCCURRENCE_ALGORITHM_VERSION: &str = "tag-cooccurrence-v1";

const GRAPH_REBUILD_DEBOUNCE: Duration = Duration::from_millis(250);
const MAX_LEXICAL_SEEDS_PER_TRIGGER: usize = 32;
const MAX_NAME_GRAMS_PER_SEED: usize = 24;
const MAX_NAME_LOOKUP_ROWS_PER_SEED: usize = 300;
const MAX_LEXICAL_CANDIDATES_PER_SEED: usize = 20;
const MIN_NAME_SIMILARITY: f64 = 0.35;
const TAG_RELATION_SCAN_CHECKPOINT_KEY: &str = "tag_relation_scan_checkpoint";
const TAG_RELATION_SCAN_FORMAT_VERSION: u8 = 1;
const TAG_RELATION_CANDIDATE_PLANNER_VERSION: &str = "tag-relation-candidates-v2";
const TAG_RELATION_SCAN_PAGE_SIZE: usize = 1;
const TAG_RELATION_CANDIDATES_PER_SOURCE_PER_SEED: usize = 20;
static TAG_COOCCURRENCE_SIGNAL: OnceLock<Arc<Notify>> = OnceLock::new();
static TAG_COOCCURRENCE_REBUILD_PENDING: OnceLock<AtomicBool> = OnceLock::new();
static TAG_RELATION_SCAN_REQUEST_PENDING: OnceLock<AtomicBool> = OnceLock::new();

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TagRelationScanCheckpoint {
    format_version: u8,
    requested_generation: u64,
    working_generation: u64,
    completed_generation: u64,
    cursor: Option<String>,
    scan_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagRelationReconciliationProgress {
    Advanced,
    Blocked,
    Idle,
}

fn tag_cooccurrence_signal() -> &'static Arc<Notify> {
    TAG_COOCCURRENCE_SIGNAL.get_or_init(|| Arc::new(Notify::new()))
}

fn cooccurrence_rebuild_pending() -> &'static AtomicBool {
    TAG_COOCCURRENCE_REBUILD_PENDING.get_or_init(|| AtomicBool::new(false))
}

fn relation_scan_request_pending() -> &'static AtomicBool {
    TAG_RELATION_SCAN_REQUEST_PENDING.get_or_init(|| AtomicBool::new(false))
}

/// Coalesces ordinary tag changes into a graph rebuild and durable candidate scan.
pub fn notify_tag_cooccurrence_rebuild() {
    cooccurrence_rebuild_pending().store(true, Ordering::Release);
    request_tag_relation_reconciliation();
    tag_cooccurrence_signal().notify_one();
}

/// Requests another full tag relation candidate pass after a tag or scorer setting changes.
pub fn request_tag_relation_reconciliation() {
    relation_scan_request_pending().store(true, Ordering::Release);
    tag_cooccurrence_signal().notify_one();
}

/// Wakes reconciliation without marking the co-occurrence graph dirty.
pub fn notify_tag_relation_reconciliation_worker() {
    tag_cooccurrence_signal().notify_one();
}

/// Starts the graph and relation reconciliation worker. The startup request advances a durable
/// scan generation but leaves any in-progress cursor intact, so restarts cannot reset the pass.
pub fn spawn_tag_cooccurrence_worker(pool: Pool<Sqlite>) {
    let signal = tag_cooccurrence_signal().clone();
    cooccurrence_rebuild_pending().store(true, Ordering::Release);
    request_tag_relation_reconciliation();
    signal.notify_one();
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
            if relation_scan_request_pending().swap(false, Ordering::AcqRel) {
                if let Err(error) = record_relation_scan_request(&pool).await {
                    relation_scan_request_pending().store(true, Ordering::Release);
                    tracing::warn!(%error, "failed to persist tag relation scan request");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    signal.notify_one();
                    continue;
                }
            }
            if cooccurrence_rebuild_pending().swap(false, Ordering::AcqRel) {
                if let Err(error) = rebuild_tag_cooccurrence_edges(&pool).await {
                    cooccurrence_rebuild_pending().store(true, Ordering::Release);
                    tracing::warn!(%error, "failed to rebuild tag co-occurrence graph after tag change");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    signal.notify_one();
                    continue;
                }
            }
            if cooccurrence_rebuild_pending().load(Ordering::Acquire) {
                continue;
            }

            loop {
                if relation_scan_request_pending().load(Ordering::Acquire)
                    || cooccurrence_rebuild_pending().load(Ordering::Acquire)
                {
                    break;
                }
                match run_tag_relation_reconciliation_once(&pool).await {
                    Ok(TagRelationReconciliationProgress::Advanced) => {}
                    Ok(
                        TagRelationReconciliationProgress::Blocked
                        | TagRelationReconciliationProgress::Idle,
                    ) => break,
                    Err(error) => {
                        tracing::warn!(%error, "tag relation reconciliation iteration failed");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        signal.notify_one();
                        break;
                    }
                }
            }
        }
    });
}

async fn load_tag_relation_scan_checkpoint(
    pool: &Pool<Sqlite>,
) -> Result<TagRelationScanCheckpoint> {
    let stored = sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(TAG_RELATION_SCAN_CHECKPOINT_KEY)
        .fetch_optional(pool)
        .await?;
    let mut checkpoint = stored
        .as_deref()
        .and_then(|raw| serde_json::from_str::<TagRelationScanCheckpoint>(raw).ok())
        .unwrap_or_default();
    if checkpoint.format_version != TAG_RELATION_SCAN_FORMAT_VERSION {
        checkpoint = TagRelationScanCheckpoint {
            format_version: TAG_RELATION_SCAN_FORMAT_VERSION,
            ..TagRelationScanCheckpoint::default()
        };
    }
    Ok(checkpoint)
}

async fn save_tag_relation_scan_checkpoint(
    pool: &Pool<Sqlite>,
    checkpoint: &TagRelationScanCheckpoint,
) -> Result<()> {
    let mut transaction = pool.begin_with("BEGIN IMMEDIATE").await?;
    let mut merged = checkpoint.clone();
    if let Some(current) =
        sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
            .bind(TAG_RELATION_SCAN_CHECKPOINT_KEY)
            .fetch_optional(&mut *transaction)
            .await?
            .and_then(|raw| serde_json::from_str::<TagRelationScanCheckpoint>(&raw).ok())
    {
        if current.requested_generation > merged.requested_generation {
            merged.requested_generation = current.requested_generation;
            if merged.working_generation <= merged.completed_generation {
                merged.working_generation = merged.requested_generation;
                merged.cursor = None;
                merged.scan_fingerprint = None;
            }
        }
        if current.completed_generation > merged.completed_generation {
            merged.completed_generation = current.completed_generation;
        }
    }
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(TAG_RELATION_SCAN_CHECKPOINT_KEY)
    .bind(serde_json::to_string(&merged)?)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn record_relation_scan_request(pool: &Pool<Sqlite>) -> Result<()> {
    let mut checkpoint = load_tag_relation_scan_checkpoint(pool).await?;
    checkpoint.requested_generation = checkpoint.requested_generation.saturating_add(1);
    if checkpoint.working_generation <= checkpoint.completed_generation {
        checkpoint.working_generation = checkpoint.requested_generation;
        checkpoint.cursor = None;
        checkpoint.scan_fingerprint = None;
    }
    save_tag_relation_scan_checkpoint(pool, &checkpoint).await
}

/// Returns true while a requested or active full reconciliation generation remains incomplete.
pub async fn is_tag_relation_reconciliation_pending(
    pool: &Pool<Sqlite>,
    settings: &crate::models::AISettings,
) -> Result<bool> {
    if relation_scan_request_pending().load(Ordering::Acquire) {
        return Ok(true);
    }
    let checkpoint = load_tag_relation_scan_checkpoint(pool).await?;
    let requested_or_active = checkpoint.requested_generation > checkpoint.completed_generation
        || checkpoint.working_generation > checkpoint.completed_generation;
    let current_fingerprint = tag_relation_scan_fingerprint(settings);
    let fingerprint_outdated = checkpoint.requested_generation > 0
        && checkpoint.scan_fingerprint.as_deref() != Some(current_fingerprint.as_str());
    Ok(requested_or_active || fingerprint_outdated)
}

fn tag_relation_scan_fingerprint(settings: &crate::models::AISettings) -> String {
    let scorer_version = crate::services::ai_service::tag_relation_scorer_version(
        &settings.features.recommendations.tag_relation,
        false,
    );
    let value = format!(
        "{}:{}:{}:{}:{}:{}",
        TAG_RELATION_SCAN_FORMAT_VERSION,
        TAG_RELATION_CANDIDATE_PLANNER_VERSION,
        TAG_COOCCURRENCE_ALGORITHM_VERSION,
        METADATA_NAMESPACE_POLICY_VERSION,
        settings
            .features
            .recommendations
            .tag_relation
            .candidate_algorithm_version
            .trim(),
        scorer_version
    );
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn tag_relation_queue_is_paused(pool: &Pool<Sqlite>) -> Result<bool> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS (SELECT 1 FROM ai_queue_controls \
         WHERE job_type = 'tag_relation_jev' AND manually_paused = 1)",
    )
    .fetch_one(pool)
    .await?
        != 0)
}

async fn load_next_tag_relation_scan_page(
    pool: &Pool<Sqlite>,
    cursor: Option<&str>,
) -> Result<Vec<String>> {
    let mut excluded_namespaces = load_metadata_namespace_set(pool)
        .await?
        .into_iter()
        .collect::<BTreeSet<_>>();
    excluded_namespaces.insert("theme".to_string());
    let exclusions = std::iter::repeat("?")
        .take(excluded_namespaces.len())
        .collect::<Vec<_>>()
        .join(",");
    let cursor_clause = cursor.map(|_| "AND t.id > ? ").unwrap_or_default();
    let query = format!(
        "SELECT t.id FROM tags t \
         WHERE EXISTS (SELECT 1 FROM archive_tags at WHERE at.tag_id = t.id) \
           AND lower(trim(t.namespace)) NOT IN ({exclusions}) {cursor_clause} \
         ORDER BY t.id LIMIT ?"
    );
    let mut request = sqlx::query_scalar::<_, String>(&query);
    for namespace in &excluded_namespaces {
        request = request.bind(namespace);
    }
    if let Some(cursor) = cursor {
        request = request.bind(cursor);
    }
    Ok(request
        .bind(TAG_RELATION_SCAN_PAGE_SIZE as i64)
        .fetch_all(pool)
        .await?)
}

/// Plans and settles at most one keyset page, allowing isolated database copies to replay the
/// durable reconciliation without starting the long-lived worker.
pub async fn run_tag_relation_reconciliation_once(
    pool: &Pool<Sqlite>,
) -> Result<TagRelationReconciliationProgress> {
    let settings = crate::services::load_ai_settings(pool).await?;
    if !crate::services::ai_service::tag_relation_is_available(&settings)
        || tag_relation_queue_is_paused(pool).await?
    {
        return Ok(TagRelationReconciliationProgress::Blocked);
    }

    let mut checkpoint = load_tag_relation_scan_checkpoint(pool).await?;
    let fingerprint = tag_relation_scan_fingerprint(&settings);
    let fingerprint_outdated = checkpoint.requested_generation > 0
        && checkpoint.scan_fingerprint.as_deref() != Some(fingerprint.as_str());
    if checkpoint.requested_generation <= checkpoint.completed_generation
        && checkpoint.working_generation <= checkpoint.completed_generation
        && !fingerprint_outdated
    {
        return Ok(TagRelationReconciliationProgress::Idle);
    }
    if fingerprint_outdated
        && checkpoint.requested_generation <= checkpoint.completed_generation
        && checkpoint.working_generation <= checkpoint.completed_generation
    {
        checkpoint.requested_generation = checkpoint.completed_generation.saturating_add(1);
    }
    if checkpoint.working_generation <= checkpoint.completed_generation {
        checkpoint.working_generation = checkpoint.requested_generation;
        checkpoint.cursor = None;
        checkpoint.scan_fingerprint = Some(fingerprint.clone());
        save_tag_relation_scan_checkpoint(pool, &checkpoint).await?;
    } else if checkpoint.scan_fingerprint.is_none() {
        checkpoint.scan_fingerprint = Some(fingerprint.clone());
        save_tag_relation_scan_checkpoint(pool, &checkpoint).await?;
    }

    let page = load_next_tag_relation_scan_page(pool, checkpoint.cursor.as_deref()).await?;
    let Some(seed_id) = page.first() else {
        checkpoint.completed_generation = checkpoint.working_generation;
        checkpoint.cursor = None;
        checkpoint.scan_fingerprint = Some(fingerprint);
        save_tag_relation_scan_checkpoint(pool, &checkpoint).await?;
        return Ok(TagRelationReconciliationProgress::Advanced);
    };

    if !enqueue_semantic_candidates_for_seeds(pool, page.as_slice()).await? {
        return Ok(TagRelationReconciliationProgress::Blocked);
    }
    checkpoint.cursor = Some(seed_id.clone());
    checkpoint.scan_fingerprint = Some(fingerprint);
    save_tag_relation_scan_checkpoint(pool, &checkpoint).await?;
    Ok(TagRelationReconciliationProgress::Advanced)
}

async fn enqueue_semantic_candidates_for_seeds(
    pool: &Pool<Sqlite>,
    seed_tag_ids: &[String],
) -> Result<bool> {
    let settings = crate::services::load_ai_settings(pool).await?;
    if !crate::services::ai_service::tag_relation_is_available(&settings) {
        return Ok(false);
    }
    let metadata_namespaces = load_metadata_namespace_set(pool).await?;
    let mut candidates = BTreeMap::new();
    for tag_id in seed_tag_ids.iter().take(MAX_LEXICAL_SEEDS_PER_TRIGGER) {
        for pair in candidates_for_tag_relation_seed(pool, tag_id, &metadata_namespaces).await? {
            candidates.entry(pair.pair_id.clone()).or_insert(pair);
        }
    }
    let pairs = candidates.into_values().collect::<Vec<_>>();
    let admission =
        crate::services::enqueue_tag_relation_jev_candidates(pool, &settings, &pairs).await?;
    Ok(admission.settled_page)
}

async fn candidates_for_tag_relation_seed(
    pool: &Pool<Sqlite>,
    seed_tag_id: &str,
    metadata_namespaces: &HashSet<String>,
) -> Result<Vec<crate::services::recommendations::semantic_edges::TagRelationPair>> {
    use crate::services::recommendations::semantic_edges::{TagRelationPair, TagRelationTag};

    let Some(row) = sqlx::query(
        "SELECT t.id, t.namespace, t.name,
                (SELECT COUNT(DISTINCT at.archive_id) FROM archive_tags at \
                 WHERE at.tag_id = t.id) AS support_count \
         FROM tags t WHERE t.id = ?",
    )
    .bind(seed_tag_id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(Vec::new());
    };
    let source_namespace: String = row.try_get("namespace")?;
    let normalized_source_namespace = source_namespace.trim().to_ascii_lowercase();
    if is_system_managed_theme_namespace(&normalized_source_namespace)
        || metadata_namespaces.contains(&normalized_source_namespace)
    {
        return Ok(Vec::new());
    }
    let source = TagRelationTag {
        id: row.try_get("id")?,
        namespace: source_namespace,
        name: row.try_get("name")?,
        support_count: row.try_get::<i64, _>("support_count")?.max(0) as u32,
    };
    let mut candidates = BTreeMap::<String, TagRelationPair>::new();
    let mut excluded_namespaces = metadata_namespaces.iter().cloned().collect::<BTreeSet<_>>();
    excluded_namespaces.insert("theme".to_string());

    let namespace_placeholders = std::iter::repeat("?")
        .take(excluded_namespaces.len())
        .collect::<Vec<_>>()
        .join(",");
    let exact_query = format!(
        "SELECT t.id, t.namespace, t.name, \
                (SELECT COUNT(DISTINCT at.archive_id) FROM archive_tags at WHERE at.tag_id = t.id) AS support_count \
         FROM tags t WHERE t.id <> ? \
           AND lower(trim(t.name)) = lower(trim(?)) \
           AND lower(trim(t.namespace)) <> lower(trim(?)) \
           AND lower(trim(t.namespace)) NOT IN ({namespace_placeholders}) \
         ORDER BY lower(trim(t.name)), t.id LIMIT ?"
    );
    let mut exact_request = sqlx::query(&exact_query)
        .bind(&source.id)
        .bind(&source.name)
        .bind(&source.namespace);
    for namespace in &excluded_namespaces {
        exact_request = exact_request.bind(namespace);
    }
    for row in exact_request
        .bind(TAG_RELATION_CANDIDATES_PER_SOURCE_PER_SEED as i64)
        .fetch_all(pool)
        .await?
    {
        let target = TagRelationTag {
            id: row.try_get("id")?,
            namespace: row.try_get("namespace")?,
            name: row.try_get("name")?,
            support_count: row.try_get::<i64, _>("support_count")?.max(0) as u32,
        };
        if normalized_tag_name(&source.name) != normalized_tag_name(&target.name) {
            continue;
        }
        let pair = TagRelationPair {
            pair_id: String::new(),
            tag_a: source.clone(),
            tag_b: target,
            pair_input_hash: String::new(),
        }
        .canonicalize()?;
        candidates.entry(pair.pair_id.clone()).or_insert(pair);
    }

    let edges = load_tag_cooccurrence_neighbors(
        pool,
        &[source.id.clone()],
        TAG_RELATION_CANDIDATES_PER_SOURCE_PER_SEED,
    )
    .await?;
    let mut neighbor_ids = BTreeSet::new();
    for edge in &edges {
        neighbor_ids.insert(edge.tag_a_id.clone());
        neighbor_ids.insert(edge.tag_b_id.clone());
    }
    if !neighbor_ids.is_empty() {
        let placeholders = std::iter::repeat("?")
            .take(neighbor_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let query = format!(
            "SELECT t.id, t.namespace, t.name, \
                    (SELECT COUNT(DISTINCT at.archive_id) FROM archive_tags at WHERE at.tag_id = t.id) AS support_count \
             FROM tags t WHERE t.id IN ({placeholders})"
        );
        let mut request = sqlx::query(&query);
        for id in &neighbor_ids {
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
                TagRelationTag {
                    id: row.try_get("id")?,
                    namespace,
                    name: row.try_get("name")?,
                    support_count: row.try_get::<i64, _>("support_count")?.max(0) as u32,
                },
            );
        }
        for edge in edges {
            let (Some(tag_a), Some(tag_b)) = (tags.get(&edge.tag_a_id), tags.get(&edge.tag_b_id))
            else {
                continue;
            };
            let pair = TagRelationPair {
                pair_id: String::new(),
                tag_a: tag_a.clone(),
                tag_b: tag_b.clone(),
                pair_input_hash: String::new(),
            }
            .canonicalize()?;
            candidates.entry(pair.pair_id.clone()).or_insert(pair);
        }
    }

    let grams = lexical_name_grams(&source.name);
    if !grams.is_empty() {
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
                    (SELECT COUNT(DISTINCT at.archive_id) FROM archive_tags at WHERE at.tag_id = t.id) AS support_count \
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
        let rows = request
            .bind(source.name.chars().count() as i64)
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
            let target = TagRelationTag {
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
        for (_, _, target) in lexical_candidates
            .into_iter()
            .take(MAX_LEXICAL_CANDIDATES_PER_SEED)
        {
            let pair = TagRelationPair {
                pair_id: String::new(),
                tag_a: source.clone(),
                tag_b: target,
                pair_input_hash: String::new(),
            }
            .canonicalize()?;
            candidates.entry(pair.pair_id.clone()).or_insert(pair);
        }
    }

    Ok(candidates.into_values().collect())
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

    async fn attach_tag(pool: &Pool<Sqlite>, tag_id: &str, name: &str, namespace: &str) {
        let archive_id = format!("archive-{tag_id}");
        sqlx::query(
            "INSERT INTO archives (id, title, path, file_hash, file_size, page_count) \
             VALUES (?, ?, ?, ?, 1, 1)",
        )
        .bind(&archive_id)
        .bind(name)
        .bind(format!("/{archive_id}.cbz"))
        .bind(format!("hash-{tag_id}"))
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO tags (id, name, namespace) VALUES (?, ?, ?)")
            .bind(tag_id)
            .bind(name)
            .bind(namespace)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO archive_tags (archive_id, tag_id) VALUES (?, ?)")
            .bind(archive_id)
            .bind(tag_id)
            .execute(pool)
            .await
            .unwrap();
    }

    async fn configure_relation_scoring(pool: &Pool<Sqlite>, max_pairs: usize) {
        let mut settings = crate::services::load_ai_settings(pool).await.unwrap();
        settings.features.recommendations.tag_graph_enabled = true;
        settings.features.recommendations.tag_relation.api_key =
            Some("test-provider-key".to_string());
        settings
            .features
            .recommendations
            .tag_relation
            .max_pairs_per_trigger = max_pairs;
        crate::services::save_ai_settings(pool, settings)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn relation_scan_keeps_cursor_and_replays_earlier_insert_in_next_generation() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES ('tag-00', 'orphan signal', 'general')",
        )
        .execute(&pool)
        .await
        .unwrap();
        attach_tag(&pool, "tag-ab", "metadata signal", "artist").await;
        attach_tag(&pool, "tag-b", "quiet meadow", "general").await;
        attach_tag(&pool, "tag-c", "silver lantern", "general").await;
        configure_relation_scoring(&pool, 100).await;
        record_relation_scan_request(&pool).await.unwrap();

        let settings = crate::services::load_ai_settings(&pool).await.unwrap();
        assert!(is_tag_relation_reconciliation_pending(&pool, &settings)
            .await
            .unwrap());
        assert_eq!(
            run_tag_relation_reconciliation_once(&pool).await.unwrap(),
            TagRelationReconciliationProgress::Advanced
        );
        let first_page = load_tag_relation_scan_checkpoint(&pool).await.unwrap();
        assert_eq!(first_page.cursor.as_deref(), Some("tag-b"));

        attach_tag(&pool, "tag-a", "orchid field", "general").await;
        let mut changed_settings = crate::services::load_ai_settings(&pool).await.unwrap();
        changed_settings.features.recommendations.tag_relation.model =
            "alternate-jev-model".to_string();
        crate::services::save_ai_settings(&pool, changed_settings)
            .await
            .unwrap();
        record_relation_scan_request(&pool).await.unwrap();
        let after_restart_request = load_tag_relation_scan_checkpoint(&pool).await.unwrap();
        assert_eq!(after_restart_request.cursor.as_deref(), Some("tag-b"));
        assert_eq!(after_restart_request.working_generation, 1);
        assert_eq!(after_restart_request.requested_generation, 2);
        assert_eq!(
            after_restart_request.scan_fingerprint,
            first_page.scan_fingerprint
        );

        assert_eq!(
            run_tag_relation_reconciliation_once(&pool).await.unwrap(),
            TagRelationReconciliationProgress::Advanced
        );
        assert_eq!(
            load_tag_relation_scan_checkpoint(&pool)
                .await
                .unwrap()
                .cursor
                .as_deref(),
            Some("tag-c")
        );
        assert_eq!(
            run_tag_relation_reconciliation_once(&pool).await.unwrap(),
            TagRelationReconciliationProgress::Advanced
        );
        let first_generation_done = load_tag_relation_scan_checkpoint(&pool).await.unwrap();
        assert_eq!(first_generation_done.completed_generation, 1);
        assert_eq!(first_generation_done.cursor, None);
        assert!(is_tag_relation_reconciliation_pending(&pool, &settings)
            .await
            .unwrap());

        assert_eq!(
            run_tag_relation_reconciliation_once(&pool).await.unwrap(),
            TagRelationReconciliationProgress::Advanced
        );
        let second_generation = load_tag_relation_scan_checkpoint(&pool).await.unwrap();
        assert_eq!(second_generation.working_generation, 2);
        assert_eq!(second_generation.cursor.as_deref(), Some("tag-a"));
    }

    #[tokio::test]
    async fn relation_scan_waits_for_configuration_and_queue_resume() {
        let pool = test_pool().await;
        attach_tag(&pool, "tag-a", "quiet meadow", "general").await;
        record_relation_scan_request(&pool).await.unwrap();

        assert_eq!(
            run_tag_relation_reconciliation_once(&pool).await.unwrap(),
            TagRelationReconciliationProgress::Blocked
        );
        assert_eq!(
            load_tag_relation_scan_checkpoint(&pool)
                .await
                .unwrap()
                .cursor,
            None
        );

        configure_relation_scoring(&pool, 100).await;
        sqlx::query(
            "INSERT INTO ai_queue_controls (job_type, manually_paused) VALUES ('tag_relation_jev', 1) \
             ON CONFLICT(job_type) DO UPDATE SET manually_paused = 1",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            run_tag_relation_reconciliation_once(&pool).await.unwrap(),
            TagRelationReconciliationProgress::Blocked
        );
        sqlx::query("UPDATE ai_queue_controls SET manually_paused = 0 WHERE job_type = ?")
            .bind("tag_relation_jev")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            run_tag_relation_reconciliation_once(&pool).await.unwrap(),
            TagRelationReconciliationProgress::Advanced
        );
        assert_eq!(
            load_tag_relation_scan_checkpoint(&pool)
                .await
                .unwrap()
                .cursor
                .as_deref(),
            Some("tag-a")
        );
    }

    #[tokio::test]
    async fn relation_scan_does_not_advance_when_unresolved_pairs_exceed_capacity() {
        let pool = test_pool().await;
        attach_tag(&pool, "tag-a", "shared signal", "general").await;
        attach_tag(&pool, "tag-b", "shared signal", "series").await;
        attach_tag(&pool, "tag-x", "isolated violet", "general").await;
        attach_tag(&pool, "tag-y", "distant orchard", "general").await;
        configure_relation_scoring(&pool, 1).await;

        let fill_pair = crate::services::recommendations::semantic_edges::TagRelationPair {
            pair_id: String::new(),
            tag_a: crate::services::recommendations::semantic_edges::TagRelationTag {
                id: "tag-x".to_string(),
                namespace: "general".to_string(),
                name: "isolated violet".to_string(),
                support_count: 1,
            },
            tag_b: crate::services::recommendations::semantic_edges::TagRelationTag {
                id: "tag-y".to_string(),
                namespace: "general".to_string(),
                name: "distant orchard".to_string(),
                support_count: 1,
            },
            pair_input_hash: String::new(),
        }
        .canonicalize()
        .unwrap();
        let settings = crate::services::load_ai_settings(&pool).await.unwrap();
        let admission =
            crate::services::enqueue_tag_relation_jev_candidates(&pool, &settings, &[fill_pair])
                .await
                .unwrap();
        assert!(admission.settled_page);
        assert_eq!(admission.admitted_pairs, 1);

        record_relation_scan_request(&pool).await.unwrap();
        assert_eq!(
            run_tag_relation_reconciliation_once(&pool).await.unwrap(),
            TagRelationReconciliationProgress::Blocked
        );
        assert_eq!(
            load_tag_relation_scan_checkpoint(&pool)
                .await
                .unwrap()
                .cursor,
            None
        );

        sqlx::query("DELETE FROM tag_relation_pair_reservations")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE ai_processing_queue SET status = 'completed' WHERE job_type = ?")
            .bind("tag_relation_jev")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            run_tag_relation_reconciliation_once(&pool).await.unwrap(),
            TagRelationReconciliationProgress::Advanced
        );
        assert_eq!(
            load_tag_relation_scan_checkpoint(&pool)
                .await
                .unwrap()
                .cursor
                .as_deref(),
            Some("tag-a")
        );
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

        let seed_tag_ids = (0..20_000)
            .map(|index| {
                if index < 128 {
                    format!("changed-{index:03}")
                } else {
                    format!("missing-{index:05}")
                }
            })
            .collect::<Vec<_>>();
        enqueue_semantic_candidates_for_seeds(&pool, &seed_tag_ids)
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

        enqueue_semantic_candidates_for_seeds(&pool, &["tag-seed".to_string()])
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
