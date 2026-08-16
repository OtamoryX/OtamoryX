use anyhow::{anyhow, Context, Result};
use rand::{seq::SliceRandom, Rng, RngExt};
use serde::Deserialize;
use sqlx::{Pool, Row, Sqlite};
use std::collections::HashMap;
use tracing::{debug, info};
use uuid::Uuid;

use crate::middleware::path_permission;
use crate::models::CategorySearchParams;
use crate::models::{deserialize_comma_separated, Archive};
use crate::services::{ArchiveFilters, ArchiveQueryService, PaginationParams, QueryOptions};

const DEFAULT_EXPLORATION_RATIO: f64 = 0.25;
const MIN_EXPLORATION_RATIO: f64 = 0.05;
const MAX_EXPLORATION_RATIO: f64 = 0.50;
const MIN_CANDIDATE_LIMIT: u64 = 500;
const MAX_CANDIDATE_LIMIT: u64 = 1_000;

#[derive(Debug, Clone, serde::Serialize)]
pub struct RandomRecommendationSession {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub archives: Vec<Archive>,
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct RandomArchiveParams {
    pub count: Option<u32>,
    #[serde(default, deserialize_with = "deserialize_comma_separated")]
    pub tags: Option<Vec<String>>,
    #[serde(rename = "minPages")]
    pub min_pages: Option<i32>,
    #[serde(rename = "maxPages")]
    pub max_pages: Option<i32>,
    #[serde(rename = "minFileSize")]
    pub min_file_size: Option<i64>,
    #[serde(rename = "maxFileSize")]
    pub max_file_size: Option<i64>,
    #[serde(rename = "createdAfter")]
    pub created_after: Option<String>,
    #[serde(rename = "createdBefore")]
    pub created_before: Option<String>,
    pub exclude_new: Option<bool>,
    pub category_id: Option<String>,
    pub query: Option<String>,
    #[serde(rename = "explorationRatio")]
    pub exploration_ratio: Option<f64>,
}

impl RandomArchiveParams {
    pub fn exploration_ratio(&self) -> Result<f64> {
        let ratio = self.exploration_ratio.unwrap_or(DEFAULT_EXPLORATION_RATIO);
        if !ratio.is_finite() || !(MIN_EXPLORATION_RATIO..=MAX_EXPLORATION_RATIO).contains(&ratio) {
            return Err(anyhow!(
                "explorationRatio must be between {MIN_EXPLORATION_RATIO} and {MAX_EXPLORATION_RATIO}"
            ));
        }
        Ok(ratio)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreferenceTier {
    Keep,
    Unknown,
    Downrank,
    AutoDelete,
}

#[derive(Debug)]
struct WeightedArchive {
    archive: Archive,
    tier: PreferenceTier,
    weight: f64,
}

#[derive(Debug, Default)]
struct PreferenceScore {
    signed_score: f64,
    auto_delete: bool,
    behavior_boost: f64,
}

pub struct RandomService {
    query_service: ArchiveQueryService,
}

impl RandomService {
    pub fn new(db: Pool<Sqlite>) -> Self {
        let query_service = ArchiveQueryService::new(db);
        Self { query_service }
    }

    /// Backwards-compatible service entry point for internal callers that do not
    /// have a request user. HTTP handlers use `get_random_archives_for_user`.
    pub async fn get_random_archives(&self, params: RandomArchiveParams) -> Result<Vec<Archive>> {
        self.get_random_archives_for_user(params, "", "admin").await
    }

    pub async fn get_random_archives_for_user(
        &self,
        params: RandomArchiveParams,
        user_id: &str,
        role: &str,
    ) -> Result<Vec<Archive>> {
        Ok(self
            .get_random_archive_session_for_user(params, user_id, role)
            .await?
            .archives)
    }

    pub async fn get_random_archive_session_for_user(
        &self,
        params: RandomArchiveParams,
        user_id: &str,
        role: &str,
    ) -> Result<RandomRecommendationSession> {
        debug!("Getting random archives with filters: {:?}", params);
        let exploration_ratio = params.exploration_ratio()?;
        let requested_count = params.count.unwrap_or(20).min(100) as usize;
        if requested_count == 0 {
            return Ok(RandomRecommendationSession {
                session_id: Uuid::new_v4().to_string(),
                archives: Vec::new(),
            });
        }

        let mut filters = ArchiveFilters::from_random_params(&params);

        // 如果指定了分类，按分类类型应用过滤：
        // - static: 使用 category_archives 关联表
        // - dynamic: 使用 categories.search_criteria 中保存的搜索条件
        if let Some(ref category_id) = params.category_id {
            let category_row =
                sqlx::query("SELECT category_type, search_criteria FROM categories WHERE id = ?")
                    .bind(category_id)
                    .fetch_optional(self.query_service.db())
                    .await?;

            let Some(category_row) = category_row else {
                return Ok(RandomRecommendationSession {
                    session_id: Uuid::new_v4().to_string(),
                    archives: Vec::new(),
                });
            };

            let category_type: String = category_row.get("category_type");
            if category_type == "static" {
                let archive_ids: Vec<String> = sqlx::query_scalar(
                    "SELECT archive_id FROM category_archives WHERE category_id = ?",
                )
                .bind(category_id)
                .fetch_all(self.query_service.db())
                .await?;

                if archive_ids.is_empty() {
                    return Ok(RandomRecommendationSession {
                        session_id: Uuid::new_v4().to_string(),
                        archives: Vec::new(),
                    });
                }
                filters.archive_ids = Some(archive_ids);
            } else {
                let search_criteria: Option<String> = category_row.get("search_criteria");
                let Some(search_criteria) = search_criteria else {
                    return Ok(RandomRecommendationSession {
                        session_id: Uuid::new_v4().to_string(),
                        archives: Vec::new(),
                    });
                };

                let dynamic_params: CategorySearchParams = serde_json::from_str(&search_criteria)?;

                // 动态分类的范围由保存的搜索条件定义
                filters.query = dynamic_params.query;
                filters.tags = dynamic_params.tags;
                filters.min_pages = dynamic_params.min_pages;
                filters.max_pages = dynamic_params.max_pages;
                filters.min_file_size = dynamic_params.min_file_size;
                filters.max_file_size = dynamic_params.max_file_size;
                filters.created_after = dynamic_params.created_after;
                filters.created_before = dynamic_params.created_before;
            }
        }

        let active_trash_ids = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT archive_id FROM trash_entries WHERE status = 'active'",
        )
        .fetch_all(self.query_service.db())
        .await
        .context("failed to load active trash exclusions")?;
        if !active_trash_ids.is_empty() {
            filters
                .exclude_archive_ids
                .get_or_insert_with(Vec::new)
                .extend(active_trash_ids);
        }

        let candidate_limit =
            ((requested_count as u64) * 20).clamp(MIN_CANDIDATE_LIMIT, MAX_CANDIDATE_LIMIT);
        let pagination = PaginationParams::from_random_params(Some(candidate_limit as u32));
        let options = QueryOptions {
            random: true,
            include_tags: true,
            user_id: Some(user_id.to_string()),
        };

        let response = self
            .query_service
            .query_archives(filters, pagination, options)
            .await?;
        let user_paths = if role == "admin" {
            Vec::new()
        } else {
            sqlx::query_scalar::<_, String>("SELECT path FROM user_paths WHERE user_id = ?")
                .bind(user_id)
                .fetch_all(self.query_service.db())
                .await
                .context("failed to load random archive path permissions")?
        };
        let candidates: Vec<Archive> = response
            .data
            .into_iter()
            .filter(|archive| {
                path_permission::has_path_permission_with_paths(role, &user_paths, &archive.path)
            })
            .collect();

        let weighted = self.score_candidates(user_id, candidates).await?;
        let keep_count = weighted
            .iter()
            .filter(|item| item.tier == PreferenceTier::Keep)
            .count();
        let unknown_count = weighted
            .iter()
            .filter(|item| item.tier == PreferenceTier::Unknown)
            .count();
        let downrank_count = weighted
            .iter()
            .filter(|item| item.tier == PreferenceTier::Downrank)
            .count();
        let auto_delete_count = weighted
            .iter()
            .filter(|item| item.tier == PreferenceTier::AutoDelete)
            .count();
        let weighted_snapshot: Vec<(String, PreferenceTier, f64)> = weighted
            .iter()
            .map(|item| (item.archive.id.clone(), item.tier, item.weight))
            .collect();
        let (selected, explored_count) = {
            let mut rng = rand::rng();
            select_weighted_archives(weighted, requested_count, exploration_ratio, &mut rng)
        };

        let session_id = Uuid::new_v4().to_string();
        let filters_json = serde_json::to_string(&params).unwrap_or_else(|_| "{}".to_string());
        let session_insert = sqlx::query("INSERT INTO random_recommendation_sessions (id,user_id,filters_json,exploration_ratio,candidate_count,keep_count,unknown_count,downrank_count,returned_count,explored_count,algorithm_version) VALUES (?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&session_id)
            .bind(user_id)
            .bind(filters_json)
            .bind(exploration_ratio)
            .bind((keep_count + unknown_count + downrank_count + auto_delete_count) as i64)
            .bind(keep_count as i64)
            .bind(unknown_count as i64)
            .bind(downrank_count as i64)
            .bind(selected.len() as i64)
            .bind(explored_count as i64)
            .bind("weighted-v1")
            .execute(self.query_service.db()).await;
        if let Err(error) = session_insert {
            tracing::warn!(%error, "random recommendation audit tables unavailable");
        }
        for (position, archive) in selected.iter().enumerate() {
            let (_, tier, weight) = weighted_snapshot
                .iter()
                .find(|(id, _, _)| id == &archive.id)
                .cloned()
                .unwrap_or_else(|| (archive.id.clone(), PreferenceTier::Unknown, 1.0));
            let item_insert = sqlx::query("INSERT INTO random_recommendation_items (id,session_id,user_id,archive_id,position,preference_tier,sampling_weight,is_exploration) VALUES (?,?,?,?,?,?,?,?)")
                .bind(Uuid::new_v4().to_string())
                .bind(&session_id)
                .bind(user_id)
                .bind(&archive.id)
                .bind(position as i64)
                .bind(preference_tier_name(tier))
                .bind(weight)
                .bind((tier == PreferenceTier::Unknown) as i64)
                .execute(self.query_service.db()).await;
            if let Err(error) = item_insert {
                tracing::warn!(%error, "random recommendation item audit unavailable");
            }
        }

        info!(
            user_id,
            candidate_count = keep_count + unknown_count + downrank_count + auto_delete_count,
            keep_count,
            unknown_count,
            downrank_count,
            auto_delete_count,
            explored_count,
            returned_count = selected.len(),
            exploration_ratio,
            "preference-weighted random archives selected"
        );
        Ok(RandomRecommendationSession {
            session_id,
            archives: selected,
        })
    }

    async fn score_candidates(
        &self,
        user_id: &str,
        candidates: Vec<Archive>,
    ) -> Result<Vec<WeightedArchive>> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let archive_ids: Vec<&str> = candidates
            .iter()
            .map(|archive| archive.id.as_str())
            .collect();
        let placeholders = archive_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let mut scores: HashMap<String, PreferenceScore> = archive_ids
            .iter()
            .map(|id| ((*id).to_string(), PreferenceScore::default()))
            .collect();

        let evaluation_query = format!(
            "SELECT a.archive_id, e.decision, e.matched_conditions_json, r.confidence_threshold, COALESCE(r.preference_weight, 1.0) AS preference_weight \
             FROM preference_rule_evaluations e \
             JOIN content_analyses a ON a.id = e.analysis_id \
             JOIN preference_rules r ON r.id = e.rule_id AND r.rule_version = e.rule_version \
             WHERE a.archive_id IN ({placeholders}) \
               AND a.status = 'completed' AND e.matched = 1 \
               AND e.decision IN ('keep', 'downrank', 'auto_delete') \
               AND r.enabled = 1 AND r.auto_paused = 0 \
               AND (r.user_id = ? OR r.owner_role = 'system') \
               AND a.id = (SELECT latest.id FROM content_analyses latest \
                           WHERE latest.archive_id = a.archive_id AND latest.status = 'completed' \
                           ORDER BY latest.created_at DESC, latest.id DESC LIMIT 1)"
        );
        let mut evaluation_request = sqlx::query(&evaluation_query);
        for archive_id in &archive_ids {
            evaluation_request = evaluation_request.bind(archive_id);
        }
        evaluation_request = evaluation_request.bind(user_id);
        for row in evaluation_request
            .fetch_all(self.query_service.db())
            .await
            .context("failed to load preference evaluations for random candidates")?
        {
            let archive_id: String = row.get("archive_id");
            let decision: String = row.get("decision");
            let threshold: f64 = row.get("confidence_threshold");
            let preference_weight: f64 = row.get("preference_weight");
            let detail: String = row.get("matched_conditions_json");
            let confidence = serde_json::from_str(&detail)
                .ok()
                .and_then(|value| minimum_json_confidence(&value))
                .unwrap_or(1.0)
                .clamp(0.0, 1.0);
            let rule_score = confidence
                * (0.5 + threshold.clamp(0.0, 1.0) * 0.5)
                * preference_weight.clamp(0.1, 2.0);
            if let Some(score) = scores.get_mut(&archive_id) {
                match decision.as_str() {
                    "keep" => score.signed_score += rule_score,
                    "downrank" => score.signed_score -= rule_score,
                    "auto_delete" => score.auto_delete = true,
                    _ => {}
                }
            }
        }

        let disposition_query = format!(
            "SELECT d.archive_id, d.disposition, d.confidence FROM archive_dispositions d \
             WHERE d.user_id = ? AND d.archive_id IN ({placeholders}) \
               AND d.created_at = (SELECT MAX(latest.created_at) FROM archive_dispositions latest \
                                   WHERE latest.user_id = d.user_id AND latest.archive_id = d.archive_id)"
        );
        let mut disposition_request = sqlx::query(&disposition_query).bind(user_id);
        for archive_id in &archive_ids {
            disposition_request = disposition_request.bind(archive_id);
        }
        for row in disposition_request
            .fetch_all(self.query_service.db())
            .await
            .context("failed to load archive dispositions for random candidates")?
        {
            let archive_id: String = row.get("archive_id");
            let disposition: String = row.get("disposition");
            let confidence = row
                .get::<Option<f64>, _>("confidence")
                .unwrap_or(1.0)
                .clamp(0.0, 1.0);
            if let Some(score) = scores.get_mut(&archive_id) {
                match disposition.as_str() {
                    "keep" => score.signed_score += confidence,
                    "downrank" => score.signed_score -= confidence,
                    "auto_delete" => score.auto_delete = true,
                    _ => {}
                }
            }
        }

        let behavior_query = format!(
            "SELECT archive_id, \
                    SUM(CASE WHEN event_type = 'open' THEN 1 ELSE 0 END) AS opens, \
                    SUM(CASE WHEN event_type = 'continue_reading' THEN 1 ELSE 0 END) AS continues, \
                    SUM(CASE WHEN event_type = 'repeat_open' THEN 1 ELSE 0 END) AS repeats, \
                    MAX(CASE WHEN occurred_at >= datetime('now', '-30 days') THEN 1 ELSE 0 END) AS recent \
             FROM user_behavior_events \
             WHERE user_id = ? AND archive_id IN ({placeholders}) \
               AND event_type IN ('open', 'continue_reading', 'repeat_open') \
             GROUP BY archive_id"
        );
        let mut behavior_request = sqlx::query(&behavior_query).bind(user_id);
        for archive_id in &archive_ids {
            behavior_request = behavior_request.bind(archive_id);
        }
        for row in behavior_request
            .fetch_all(self.query_service.db())
            .await
            .context("failed to load positive behavior signals for random candidates")?
        {
            let archive_id: String = row.get("archive_id");
            let opens = row.get::<i64, _>("opens").min(3) as f64;
            let continues = row.get::<i64, _>("continues").min(2) as f64;
            let repeats = row.get::<i64, _>("repeats").min(2) as f64;
            let recent = row.get::<i64, _>("recent") as f64;
            if let Some(score) = scores.get_mut(&archive_id) {
                score.behavior_boost =
                    opens * 0.03 + continues * 0.10 + repeats * 0.08 + recent * 0.05;
            }
        }

        Ok(candidates
            .into_iter()
            .map(|archive| {
                let score = scores.remove(&archive.id).unwrap_or_default();
                let (tier, weight) = if score.auto_delete {
                    (PreferenceTier::AutoDelete, 0.0)
                } else if score.signed_score > f64::EPSILON {
                    (
                        PreferenceTier::Keep,
                        1.5 + score.signed_score.min(2.0) + score.behavior_boost,
                    )
                } else if score.signed_score < -f64::EPSILON {
                    (
                        PreferenceTier::Downrank,
                        (0.08 / (1.0 + score.signed_score.abs()) + score.behavior_boost * 0.05)
                            .max(0.01),
                    )
                } else {
                    (PreferenceTier::Unknown, 1.0 + score.behavior_boost)
                };
                WeightedArchive {
                    archive,
                    tier,
                    weight,
                }
            })
            .collect())
    }

    pub async fn get_random_archive_by_tag(&self, tag_name: &str) -> Result<Option<Archive>> {
        debug!("Getting random archive with tag: {}", tag_name);

        let filters = ArchiveFilters {
            tags: Some(vec![tag_name.to_string()]),
            ..Default::default()
        };
        let pagination = PaginationParams::from_random_params(Some(1));
        let options = QueryOptions {
            random: true,
            include_tags: true,
            user_id: None,
        };

        let response = self
            .query_service
            .query_archives(filters, pagination, options)
            .await?;
        Ok(response.data.into_iter().next())
    }

    pub async fn get_unread_random_archives(&self, count: Option<u32>) -> Result<Vec<Archive>> {
        debug!("Getting random unread archives");

        let filters = ArchiveFilters {
            unread_only: Some(true),
            ..Default::default()
        };
        let pagination = PaginationParams::from_random_params(count);
        let options = QueryOptions {
            random: true,
            include_tags: true,
            user_id: None,
        };

        let response = self
            .query_service
            .query_archives(filters, pagination, options)
            .await?;
        Ok(response.data)
    }

    pub async fn get_random_archives_by_date_range(
        &self,
        start_date: &str,
        end_date: &str,
        count: Option<u32>,
    ) -> Result<Vec<Archive>> {
        debug!(
            "Getting random archives between {} and {}",
            start_date, end_date
        );

        let filters = ArchiveFilters {
            created_after: Some(start_date.to_string()),
            created_before: Some(end_date.to_string()),
            ..Default::default()
        };
        let pagination = PaginationParams::from_random_params(count);
        let options = QueryOptions {
            random: true,
            include_tags: true,
            user_id: None,
        };

        let response = self
            .query_service
            .query_archives(filters, pagination, options)
            .await?;
        Ok(response.data)
    }

    pub async fn get_random_archives_with_minimum_pages(
        &self,
        min_pages: i32,
        count: Option<u32>,
    ) -> Result<Vec<Archive>> {
        debug!("Getting random archives with at least {} pages", min_pages);

        let filters = ArchiveFilters {
            min_pages: Some(min_pages),
            ..Default::default()
        };
        let pagination = PaginationParams::from_random_params(count);
        let options = QueryOptions {
            random: true,
            include_tags: true,
            user_id: None,
        };

        let response = self
            .query_service
            .query_archives(filters, pagination, options)
            .await?;
        Ok(response.data)
    }
}

fn minimum_json_confidence(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Object(map) => {
            let own = map.get("confidence").and_then(serde_json::Value::as_f64);
            map.values()
                .filter_map(minimum_json_confidence)
                .chain(own)
                .reduce(f64::min)
        }
        serde_json::Value::Array(values) => values
            .iter()
            .filter_map(minimum_json_confidence)
            .reduce(f64::min),
        _ => None,
    }
}

fn select_weighted_archives<R: Rng + ?Sized>(
    candidates: Vec<WeightedArchive>,
    count: usize,
    exploration_ratio: f64,
    rng: &mut R,
) -> (Vec<Archive>, usize) {
    let mut keep = Vec::new();
    let mut unknown = Vec::new();
    let mut downrank = Vec::new();
    for candidate in candidates {
        match candidate.tier {
            PreferenceTier::Keep => keep.push(candidate),
            PreferenceTier::Unknown => unknown.push(candidate),
            PreferenceTier::Downrank => downrank.push(candidate),
            PreferenceTier::AutoDelete => {}
        }
    }

    let target = count.min(keep.len() + unknown.len() + downrank.len());
    let exploration_target = ((target as f64) * exploration_ratio).round() as usize;
    let preferred_target = target.saturating_sub(exploration_target);
    let mut selected = Vec::with_capacity(target);
    let mut explored_count = 0;

    for _ in 0..preferred_target {
        let item = if !keep.is_empty() {
            take_from_preference_pools(&mut keep, &mut downrank, rng)
        } else if !unknown.is_empty() {
            take_weighted(&mut unknown, rng)
        } else {
            take_weighted(&mut downrank, rng)
        };
        if let Some(item) = item {
            if item.tier == PreferenceTier::Unknown {
                explored_count += 1;
            }
            selected.push(item.archive);
        }
    }

    for _ in 0..exploration_target {
        let item = if !unknown.is_empty() {
            let selected = take_weighted(&mut unknown, rng);
            if selected.is_some() {
                explored_count += 1;
            }
            selected
        } else {
            take_from_preference_pools(&mut keep, &mut downrank, rng)
        };
        if let Some(item) = item {
            selected.push(item.archive);
        }
    }

    while selected.len() < target {
        let item = take_from_all_pools(&mut keep, &mut unknown, &mut downrank, rng);
        let Some(item) = item else { break };
        if item.tier == PreferenceTier::Unknown {
            explored_count += 1;
        }
        selected.push(item.archive);
    }
    selected.shuffle(rng);
    (selected, explored_count)
}

fn preference_tier_name(tier: PreferenceTier) -> &'static str {
    match tier {
        PreferenceTier::Keep => "keep",
        PreferenceTier::Unknown => "unknown",
        PreferenceTier::Downrank => "downrank",
        PreferenceTier::AutoDelete => "auto_delete",
    }
}

fn take_from_preference_pools<R: Rng + ?Sized>(
    keep: &mut Vec<WeightedArchive>,
    downrank: &mut Vec<WeightedArchive>,
    rng: &mut R,
) -> Option<WeightedArchive> {
    take_from_weighted_pools(&mut [keep, downrank], rng)
}

fn take_from_all_pools<R: Rng + ?Sized>(
    keep: &mut Vec<WeightedArchive>,
    unknown: &mut Vec<WeightedArchive>,
    downrank: &mut Vec<WeightedArchive>,
    rng: &mut R,
) -> Option<WeightedArchive> {
    take_from_weighted_pools(&mut [keep, unknown, downrank], rng)
}

fn take_from_weighted_pools<R: Rng + ?Sized>(
    pools: &mut [&mut Vec<WeightedArchive>],
    rng: &mut R,
) -> Option<WeightedArchive> {
    let total: f64 = pools
        .iter()
        .flat_map(|pool| pool.iter())
        .map(|item| item.weight.max(0.0))
        .sum();
    if total <= 0.0 {
        return pools.iter_mut().find_map(|pool| pool.pop());
    }
    let mut roll = rng.random_range(0.0..total);
    for pool in pools {
        let pool_total: f64 = pool.iter().map(|item| item.weight.max(0.0)).sum();
        if roll < pool_total {
            return take_weighted(pool, rng);
        }
        roll -= pool_total;
    }
    None
}

fn take_weighted<R: Rng + ?Sized>(
    pool: &mut Vec<WeightedArchive>,
    rng: &mut R,
) -> Option<WeightedArchive> {
    if pool.is_empty() {
        return None;
    }
    let total: f64 = pool.iter().map(|item| item.weight.max(0.0)).sum();
    if total <= 0.0 {
        return pool.pop();
    }
    let mut roll = rng.random_range(0.0..total);
    let mut selected = pool.len() - 1;
    for (index, item) in pool.iter().enumerate() {
        roll -= item.weight.max(0.0);
        if roll <= 0.0 {
            selected = index;
            break;
        }
    }
    Some(pool.swap_remove(selected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};
    use std::collections::HashSet;

    fn archive(id: &str) -> Archive {
        let now = chrono::Utc::now();
        Archive {
            id: id.to_string(),
            title: id.to_string(),
            subtitle: None,
            subtitle_language: None,
            path: format!("/{id}.cbz"),
            file_size: 1,
            page_count: 1,
            hash: id.to_string(),
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
        }
    }

    fn weighted(id: &str, tier: PreferenceTier, weight: f64) -> WeightedArchive {
        WeightedArchive {
            archive: archive(id),
            tier,
            weight,
        }
    }

    #[test]
    fn exploration_ratio_defaults_and_rejects_out_of_range_values() {
        let params = RandomArchiveParams {
            count: None,
            tags: None,
            min_pages: None,
            max_pages: None,
            min_file_size: None,
            max_file_size: None,
            created_after: None,
            created_before: None,
            exclude_new: None,
            category_id: None,
            query: None,
            exploration_ratio: None,
        };
        assert_eq!(params.exploration_ratio().unwrap(), 0.25);
        assert!(RandomArchiveParams {
            exploration_ratio: Some(0.04),
            ..params.clone()
        }
        .exploration_ratio()
        .is_err());
        assert!(RandomArchiveParams {
            exploration_ratio: Some(0.51),
            ..params
        }
        .exploration_ratio()
        .is_err());
    }

    #[test]
    fn weighted_selection_is_unique_and_honors_exploration_quota() {
        let candidates = vec![
            weighted("keep-1", PreferenceTier::Keep, 2.0),
            weighted("keep-2", PreferenceTier::Keep, 2.0),
            weighted("unknown-1", PreferenceTier::Unknown, 1.0),
            weighted("unknown-2", PreferenceTier::Unknown, 1.0),
            weighted("unknown-3", PreferenceTier::Unknown, 1.0),
            weighted("downrank-1", PreferenceTier::Downrank, 0.01),
        ];
        let mut rng = StdRng::seed_from_u64(7);
        let (selected, explored) = select_weighted_archives(candidates, 4, 0.25, &mut rng);
        let ids: HashSet<&str> = selected.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(selected.len(), ids.len());
        assert_eq!(selected.len(), 4);
        assert_eq!(explored, 2);
    }

    #[test]
    fn empty_preference_pool_falls_back_to_unknown_and_downrank() {
        let candidates = vec![
            weighted("unknown-1", PreferenceTier::Unknown, 1.0),
            weighted("downrank-1", PreferenceTier::Downrank, 0.01),
        ];
        let mut rng = StdRng::seed_from_u64(11);
        let (selected, explored) = select_weighted_archives(candidates, 3, 0.25, &mut rng);
        assert_eq!(selected.len(), 2);
        assert_eq!(explored, 1);
    }

    #[test]
    fn keep_weight_dominates_downrank_weight() {
        let mut keep_hits = 0;
        let mut downrank_hits = 0;
        for seed in 0..500 {
            let mut keep = vec![weighted("keep", PreferenceTier::Keep, 3.0)];
            let mut downrank = vec![weighted("downrank", PreferenceTier::Downrank, 0.01)];
            let mut rng = StdRng::seed_from_u64(seed);
            let picked = take_from_preference_pools(&mut keep, &mut downrank, &mut rng)
                .expect("one preference candidate");
            if picked.tier == PreferenceTier::Keep {
                keep_hits += 1;
            } else {
                downrank_hits += 1;
            }
        }
        assert!(
            keep_hits > 450,
            "keep={keep_hits}, downrank={downrank_hits}"
        );
    }

    #[test]
    fn confidence_is_derived_from_nested_evidence() {
        let value =
            serde_json::json!({"all": [{"confidence": 0.9}, {"concept": {"confidence": 0.72}}]});
        assert_eq!(minimum_json_confidence(&value), Some(0.72));
    }

    #[tokio::test]
    async fn random_candidates_are_user_scoped_and_exclude_trash_and_paths() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite pool");
        for statement in [
            "CREATE TABLE archives (id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT, path TEXT NOT NULL, file_hash TEXT UNIQUE NOT NULL, file_size INTEGER NOT NULL, page_count INTEGER NOT NULL, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)",
            "CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL)",
            "CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL)",
            "CREATE TABLE trash_entries (archive_id TEXT NOT NULL, status TEXT NOT NULL)",
            "CREATE TABLE user_paths (user_id TEXT NOT NULL, path TEXT NOT NULL)",
            "CREATE TABLE content_analyses (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, status TEXT NOT NULL, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)",
            "CREATE TABLE preference_rules (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, rule_version TEXT NOT NULL, confidence_threshold REAL NOT NULL, preference_weight REAL NOT NULL DEFAULT 1.0, enabled INTEGER NOT NULL, auto_paused INTEGER NOT NULL, owner_role TEXT NOT NULL)",
            "CREATE TABLE preference_rule_evaluations (id TEXT PRIMARY KEY, analysis_id TEXT NOT NULL, rule_id TEXT NOT NULL, rule_version TEXT NOT NULL, matched INTEGER NOT NULL, decision TEXT NOT NULL, matched_conditions_json TEXT NOT NULL)",
            "CREATE TABLE archive_dispositions (archive_id TEXT NOT NULL, user_id TEXT NOT NULL, disposition TEXT NOT NULL, confidence REAL, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)",
            "CREATE TABLE user_behavior_events (archive_id TEXT, user_id TEXT NOT NULL, event_type TEXT NOT NULL, occurred_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)",
        ] {
            sqlx::query(statement)
                .execute(&pool)
                .await
                .expect("create random test table");
        }
        for (id, path) in [
            ("a", "/library/a.cbz"),
            ("b", "/library/b.cbz"),
            ("c", "/library/c.cbz"),
            ("private", "/private/private.cbz"),
        ] {
            sqlx::query(
                "INSERT INTO archives (id,title,path,file_hash,file_size,page_count) VALUES (?,?,?,?,1,1)",
            )
            .bind(id)
            .bind(id)
            .bind(path)
            .bind(format!("hash-{id}"))
            .execute(&pool)
            .await
            .expect("insert random archive");
        }
        sqlx::query("INSERT INTO trash_entries (archive_id,status) VALUES ('c','active')")
            .execute(&pool)
            .await
            .expect("insert active trash entry");
        sqlx::query("INSERT INTO user_paths (user_id,path) VALUES ('user-a','/library/*')")
            .execute(&pool)
            .await
            .expect("insert user path");
        sqlx::query("INSERT INTO content_analyses (id,archive_id,status) VALUES ('analysis-a','a','completed'),('analysis-b','b','completed')")
            .execute(&pool)
            .await
            .expect("insert analyses");
        sqlx::query("INSERT INTO preference_rules (id,user_id,rule_version,confidence_threshold,enabled,auto_paused,owner_role) VALUES ('rule-a','user-a','1',0.8,1,0,'user'),('rule-b','user-b','1',0.8,1,0,'user')")
            .execute(&pool)
            .await
            .expect("insert rules");
        sqlx::query("INSERT INTO preference_rule_evaluations (id,analysis_id,rule_id,rule_version,matched,decision,matched_conditions_json) VALUES ('eval-a','analysis-a','rule-a','1',1,'keep','{\"confidence\":0.95}'),('eval-b','analysis-b','rule-b','1',1,'keep','{\"confidence\":0.95}')")
            .execute(&pool)
            .await
            .expect("insert evaluations");

        let service = RandomService::new(pool.clone());
        let scored = service
            .score_candidates("user-a", vec![archive("a"), archive("b")])
            .await
            .expect("score candidates");
        let tiers: HashMap<String, PreferenceTier> = scored
            .into_iter()
            .map(|item| (item.archive.id, item.tier))
            .collect();
        assert_eq!(tiers.get("a"), Some(&PreferenceTier::Keep));
        assert_eq!(tiers.get("b"), Some(&PreferenceTier::Unknown));

        let params = RandomArchiveParams {
            count: Some(20),
            tags: None,
            min_pages: None,
            max_pages: None,
            min_file_size: None,
            max_file_size: None,
            created_after: None,
            created_before: None,
            exclude_new: None,
            category_id: None,
            query: None,
            exploration_ratio: Some(0.25),
        };
        let result = service
            .get_random_archives_for_user(params, "user-a", "user")
            .await
            .expect("select random candidates");
        let ids: HashSet<&str> = result.iter().map(|item| item.id.as_str()).collect();
        assert!(ids.contains("a") && ids.contains("b"));
        assert!(!ids.contains("c"));
        assert!(!ids.contains("private"));
    }
}
