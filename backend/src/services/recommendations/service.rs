use anyhow::{anyhow, Context, Result};
use rand::{seq::SliceRandom, Rng, RngExt};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{Pool, Row, Sqlite};
use std::collections::{BTreeSet, HashMap, HashSet};
use tracing::{debug, info};
use uuid::Uuid;

use crate::middleware::path_permission;
use crate::models::{deserialize_comma_separated, Archive};
use crate::models::{
    ArchiveContentProfileDocument, CategorySearchParams, CANONICAL_THEME_FEATURE_KIND,
};
use crate::services::archive::query::{
    ArchiveDeleteTarget, ArchiveFilters, ArchiveQueryService, PaginationParams, QueryOptions,
};
use crate::services::content_profile::{ContentProfileService, CONTENT_PROFILE_VERSION};
use crate::services::load_ai_settings;
use crate::services::preferences::learning::{
    condition_feature_key, condition_is_ordinary_tag, observing_soft_lift,
    profile_condition_matches,
};
use crate::services::recommendations::namespace_policy::load_metadata_namespace_set;
use crate::services::recommendations::tag_cooccurrence::expand_tag_cooccurrence_ids;

const DEFAULT_EXPLORATION_RATIO: f64 = 0.25;
const MIN_EXPLORATION_RATIO: f64 = 0.05;
const MAX_EXPLORATION_RATIO: f64 = 0.50;
const MIN_CANDIDATE_LIMIT: u64 = 500;
const MAX_CANDIDATE_LIMIT: u64 = 1_000;

#[derive(Debug, Clone, serde::Serialize)]
pub struct RandomRecommendationSession {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "algorithmVariant")]
    pub algorithm_variant: String,
    pub archives: Vec<Archive>,
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct RandomArchiveParams {
    pub count: Option<u32>,
    #[serde(default, deserialize_with = "deserialize_comma_separated")]
    pub tags: Option<Vec<String>>,
    #[serde(
        rename = "themeIds",
        default,
        deserialize_with = "deserialize_comma_separated"
    )]
    pub theme_ids: Option<Vec<String>>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecommendationAlgorithm {
    WeightedV1,
    UniformV1,
}

impl RecommendationAlgorithm {
    fn name(self) -> &'static str {
        match self {
            Self::WeightedV1 => "weighted-v1",
            Self::UniformV1 => "uniform-v1",
        }
    }
}

#[derive(Debug)]
struct WeightedArchive {
    archive: Archive,
    tier: PreferenceTier,
    weight: f64,
}

#[derive(Debug)]
struct PreferenceScore {
    signed_score: f64,
    auto_delete: bool,
    behavior_boost: f64,
    soft_multiplier: f64,
}

impl Default for PreferenceScore {
    fn default() -> Self {
        Self {
            signed_score: 0.0,
            auto_delete: false,
            behavior_boost: 0.0,
            soft_multiplier: 1.0,
        }
    }
}

pub struct RandomService {
    query_service: ArchiveQueryService,
}

impl RandomService {
    pub fn new(db: Pool<Sqlite>) -> Self {
        let query_service = ArchiveQueryService::new(db);
        Self { query_service }
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
        let algorithm = self.recommendation_algorithm(user_id).await;
        let requested_count = params.count.unwrap_or(20).min(100) as usize;
        if requested_count == 0 {
            return Ok(RandomRecommendationSession {
                session_id: Uuid::new_v4().to_string(),
                algorithm_variant: algorithm.name().to_string(),
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
                    algorithm_variant: algorithm.name().to_string(),
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
                        algorithm_variant: algorithm.name().to_string(),
                        archives: Vec::new(),
                    });
                }
                filters.archive_ids = Some(archive_ids);
            } else {
                let search_criteria: Option<String> = category_row.get("search_criteria");
                let Some(search_criteria) = search_criteria else {
                    return Ok(RandomRecommendationSession {
                        session_id: Uuid::new_v4().to_string(),
                        algorithm_variant: algorithm.name().to_string(),
                        archives: Vec::new(),
                    });
                };

                let dynamic_params: CategorySearchParams = serde_json::from_str(&search_criteria)?;

                // 动态分类的范围由保存的搜索条件定义
                filters.query = dynamic_params.query;
                filters.tags = dynamic_params.tags;
                filters.theme_ids = dynamic_params.theme_ids;
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

        let user_paths = if role == "admin" {
            Vec::new()
        } else {
            sqlx::query_scalar::<_, String>("SELECT path FROM user_paths WHERE user_id = ?")
                .bind(user_id)
                .fetch_all(self.query_service.db())
                .await
                .context("failed to load random archive path permissions")?
        };

        // Apply path permissions before the random LIMIT. Sampling the whole
        // library first can otherwise produce an empty result when a user has
        // access to only a small subset of archives.
        if role != "admin" && !user_paths.is_empty() {
            let targets = self
                .query_service
                .query_delete_targets(
                    filters.clone(),
                    QueryOptions {
                        random: false,
                        include_tags: false,
                        user_id: Some(user_id.to_string()),
                    },
                )
                .await?;
            let permitted_ids = permitted_archive_ids(role, &user_paths, targets);

            if permitted_ids.is_empty() {
                return Ok(RandomRecommendationSession {
                    session_id: Uuid::new_v4().to_string(),
                    algorithm_variant: algorithm.name().to_string(),
                    archives: Vec::new(),
                });
            }

            let mut permitted_ids = permitted_ids;
            if permitted_ids.len() > candidate_limit as usize {
                permitted_ids.shuffle(&mut rand::rng());
                permitted_ids.truncate(candidate_limit as usize);
            }
            filters.archive_ids = Some(permitted_ids);
        }

        let pagination = PaginationParams::from_random_params(Some(candidate_limit as u32));
        let options = QueryOptions {
            random: true,
            include_tags: true,
            user_id: Some(user_id.to_string()),
        };

        let response = self
            .query_service
            .query_archives(filters.clone(), pagination, options)
            .await?;
        let mut candidates: Vec<Archive> = response
            .data
            .into_iter()
            .filter(|archive| {
                path_permission::has_path_permission_with_paths(role, &user_paths, &archive.path)
            })
            .collect();

        // A positive ordinary-tag preference can recall a small set of archives connected by
        // the deterministic co-occurrence graph. Explicit tag filters retain their exact
        // intersection semantics; graph expansion is used only for the unfiltered main feed.
        if filters.tags.as_ref().is_none_or(|tags| tags.is_empty()) {
            let graph_archives = self
                .load_graph_recall_archives(user_id, role, &user_paths, &filters, &candidates)
                .await?;
            let existing_ids = candidates
                .iter()
                .map(|archive| archive.id.clone())
                .collect::<HashSet<_>>();
            candidates.extend(
                graph_archives
                    .into_iter()
                    .filter(|archive| !existing_ids.contains(&archive.id)),
            );
        }

        let topic_snapshots = self.load_topic_snapshots(&candidates).await?;
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
            match algorithm {
                RecommendationAlgorithm::WeightedV1 => {
                    select_weighted_archives(weighted, requested_count, exploration_ratio, &mut rng)
                }
                RecommendationAlgorithm::UniformV1 => {
                    select_uniform_archives(weighted, requested_count, &mut rng)
                }
            }
        };

        let candidate_topics = topics_for_archives(
            weighted_snapshot
                .iter()
                .filter(|(_, tier, _)| *tier != PreferenceTier::AutoDelete)
                .map(|(archive_id, _, _)| archive_id.as_str()),
            &topic_snapshots,
        );
        let exploration_topics = topics_for_archives(
            selected.iter().filter_map(|archive| {
                weighted_snapshot
                    .iter()
                    .find(|(archive_id, tier, _)| {
                        archive_id == &archive.id && *tier == PreferenceTier::Unknown
                    })
                    .map(|_| archive.id.as_str())
            }),
            &topic_snapshots,
        );

        let session_id = Uuid::new_v4().to_string();
        let filters_json = serde_json::to_string(&params).unwrap_or_else(|_| "{}".to_string());
        let session_insert = sqlx::query("INSERT INTO random_recommendation_sessions (id,user_id,filters_json,exploration_ratio,candidate_count,keep_count,unknown_count,downrank_count,returned_count,explored_count,algorithm_version,algorithm_variant,candidate_topics_json,exploration_topics_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
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
            .bind(algorithm.name())
            .bind(algorithm.name())
            .bind(serde_json::to_string(&candidate_topics).unwrap_or_else(|_| "[]".to_string()))
            .bind(serde_json::to_string(&exploration_topics).unwrap_or_else(|_| "[]".to_string()))
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
            let topics = topic_snapshots
                .get(&archive.id)
                .cloned()
                .unwrap_or_default();
            let item_insert = sqlx::query("INSERT INTO random_recommendation_items (id,session_id,user_id,archive_id,position,preference_tier,sampling_weight,is_exploration,topics_json) VALUES (?,?,?,?,?,?,?,?,?)")
                .bind(Uuid::new_v4().to_string())
                .bind(&session_id)
                .bind(user_id)
                .bind(&archive.id)
                .bind(position as i64)
                .bind(preference_tier_name(tier))
                .bind(weight)
                .bind((tier == PreferenceTier::Unknown) as i64)
                .bind(serde_json::to_string(&topics).unwrap_or_else(|_| "[]".to_string()))
                .execute(self.query_service.db()).await;
            if let Err(error) = item_insert {
                tracing::warn!(%error, "random recommendation item audit unavailable");
            }
        }

        if let Err(error) = ContentProfileService::new(self.query_service.db().clone())
            .enqueue_for_archives(
                selected.iter().map(|archive| archive.id.clone()).collect(),
                "recommendation",
            )
            .await
        {
            tracing::warn!(%error, "deterministic content profiles were not queued for recommendation exposure");
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
            algorithm_variant = algorithm.name(),
            "preference-weighted random archives selected"
        );
        Ok(RandomRecommendationSession {
            session_id,
            algorithm_variant: algorithm.name().to_string(),
            archives: selected,
        })
    }

    // Default to personalized weighting. A comparison group is opt-in and keeps users in the
    // same arm for the lifetime of the experiment so the resulting metrics are interpretable.
    async fn recommendation_algorithm(&self, user_id: &str) -> RecommendationAlgorithm {
        let experiment_enabled = match load_ai_settings(self.query_service.db()).await {
            Ok(settings) => {
                settings
                    .features
                    .recommendations
                    .multi_user_experiment_enabled
            }
            Err(error) => {
                tracing::warn!(%error, "recommendation settings unavailable; using personalized weighting");
                false
            }
        };
        if experiment_enabled && stable_experiment_bucket(user_id) < 20 {
            RecommendationAlgorithm::UniformV1
        } else {
            RecommendationAlgorithm::WeightedV1
        }
    }

    async fn load_topic_snapshots(
        &self,
        candidates: &[Archive],
    ) -> Result<HashMap<String, Vec<String>>> {
        if candidates.is_empty() {
            return Ok(HashMap::new());
        }
        let ids: Vec<&str> = candidates
            .iter()
            .map(|archive| archive.id.as_str())
            .collect();
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT analysis.archive_id, themes.theme_tag_id
             FROM content_analyses analysis
             JOIN archives current_archive ON current_archive.id = analysis.archive_id
             JOIN content_analysis_themes themes ON themes.analysis_id = analysis.id
             JOIN tags theme_tags
               ON theme_tags.id = themes.theme_tag_id
              AND lower(trim(theme_tags.namespace)) = 'theme'
             WHERE analysis.archive_id IN ({placeholders})
               AND analysis.content_fingerprint = current_archive.file_hash
               AND analysis.status = 'completed'
               AND analysis.canonicalization_status = 'completed'
               AND themes.canonicalization_status = 'completed'
               AND themes.theme_tag_id IS NOT NULL
               AND analysis.id = (SELECT latest.id FROM content_analyses latest
                                  WHERE latest.archive_id = analysis.archive_id
                                    AND latest.content_fingerprint = current_archive.file_hash
                                    AND latest.status = 'completed'
                                    AND latest.canonicalization_status = 'completed'
                                  ORDER BY latest.created_at DESC, latest.id DESC LIMIT 1)
             ORDER BY analysis.archive_id, themes.ordinal"
        );
        let mut request = sqlx::query(&query);
        for id in ids {
            request = request.bind(id);
        }
        let rows = match request.fetch_all(self.query_service.db()).await {
            Ok(rows) => rows,
            Err(error) => {
                debug!(%error, "canonical theme snapshots are unavailable");
                return Ok(HashMap::new());
            }
        };
        let mut snapshots = HashMap::new();
        for row in rows {
            let archive_id: String = row.get("archive_id");
            let Some(theme_tag_id) = row.try_get::<Option<String>, _>("theme_tag_id")? else {
                continue;
            };
            snapshots
                .entry(archive_id)
                .or_insert_with(Vec::new)
                .push(format!("theme:{theme_tag_id}"));
        }
        for topics in snapshots.values_mut() {
            topics.sort();
            topics.dedup();
        }
        Ok(snapshots)
    }

    async fn load_graph_recall_archives(
        &self,
        user_id: &str,
        role: &str,
        user_paths: &[String],
        filters: &ArchiveFilters,
        current_candidates: &[Archive],
    ) -> Result<Vec<Archive>> {
        // An empty path list is not a grant for graph recall. Keep this explicit because the
        // shared path helper treats an empty list as unrestricted for legacy list endpoints.
        if !graph_recall_has_path_scope(role, user_paths) {
            return Ok(Vec::new());
        }
        let metadata_namespaces = match load_metadata_namespace_set(self.query_service.db()).await {
            Ok(namespaces) => namespaces,
            Err(error) => {
                debug!(%error, "metadata namespace policy is unavailable for graph recall");
                return Ok(Vec::new());
            }
        };
        let preference_rows = match sqlx::query(
            "SELECT candidate.conditions_json, candidate.feature_kind,
                    candidate.status, candidate.evidence_state,
                    candidate.unique_archive_count, candidate.informative_result_count,
                    candidate.positive_support, candidate.negative_support, candidate.lift,
                    rule.action
             FROM preference_rule_candidates candidate
             LEFT JOIN preference_rules rule
               ON rule.user_id = candidate.user_id
              AND rule.conditions_json = candidate.conditions_json
              AND rule.source = 'learned_cold_start'
             WHERE candidate.user_id = ? AND candidate.source = 'cold_start_v1'
               AND ((candidate.status = 'observing' AND candidate.evidence_state = 'observing'
                     AND candidate.lift > 0)
                    OR (candidate.status = 'promoted' AND candidate.evidence_state = 'eligible'
                        AND rule.action = 'keep'))",
        )
        .bind(user_id)
        .fetch_all(self.query_service.db())
        .await
        {
            Ok(rows) => rows,
            Err(error) => {
                debug!(%error, "preference candidates are unavailable for graph recall");
                return Ok(Vec::new());
            }
        };
        let mut seed_tag_ids = BTreeSet::new();
        for row in preference_rows {
            let feature_kind = row.get::<Option<String>, _>("feature_kind");
            let status: String = row.get("status");
            let evidence_state: String = row.get("evidence_state");
            let action = row.get::<Option<String>, _>("action");
            let observing_supported = feature_kind.as_deref() == Some("binary")
                && observing_soft_lift(
                    row.get("unique_archive_count"),
                    row.get("informative_result_count"),
                    row.get("positive_support"),
                    row.get("negative_support"),
                    row.get("lift"),
                )
                .is_some();
            let positive_rule = status == "promoted"
                && evidence_state == "eligible"
                && action.as_deref() == Some("keep");
            if !observing_supported && !positive_rule {
                continue;
            }
            let condition: Value =
                match serde_json::from_str(row.get::<String, _>("conditions_json").as_str()) {
                    Ok(condition) => condition,
                    Err(_) => continue,
                };
            if !condition_is_ordinary_tag(&condition, &metadata_namespaces) {
                continue;
            }
            let Some(feature_key) = condition_feature_key(&condition) else {
                continue;
            };
            let Some(tag_key) = feature_key.strip_prefix("tag:") else {
                continue;
            };
            let Some((namespace, name)) = tag_key.split_once(':') else {
                continue;
            };
            let tag_identity = format!(
                "{}:{}",
                namespace.trim().to_ascii_lowercase(),
                name.trim().to_ascii_lowercase()
            );
            if let Some(tag_id) = sqlx::query_scalar::<_, String>(
                "SELECT id FROM tags
                 WHERE lower(trim(namespace)) || ':' || lower(trim(name)) = ?
                 ORDER BY id LIMIT 1",
            )
            .bind(tag_identity)
            .fetch_optional(self.query_service.db())
            .await?
            {
                seed_tag_ids.insert(tag_id);
            }
        }
        if seed_tag_ids.is_empty() {
            return Ok(Vec::new());
        }
        let neighbor_tag_ids = expand_tag_cooccurrence_ids(
            self.query_service.db(),
            &seed_tag_ids.iter().cloned().collect::<Vec<_>>(),
            20,
            100,
        )
        .await?;
        if neighbor_tag_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = neighbor_tag_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let archive_query = format!(
            "SELECT DISTINCT archive_id FROM archive_tags WHERE tag_id IN ({placeholders})"
        );
        let archive_ids = graph_recall_archive_ids(
            {
                let mut request = sqlx::query(&archive_query);
                for tag_id in &neighbor_tag_ids {
                    request = request.bind(tag_id);
                }
                request
                    .fetch_all(self.query_service.db())
                    .await?
                    .into_iter()
                    .map(|row| row.get::<String, _>("archive_id"))
                    .collect::<Vec<_>>()
            },
            filters,
            current_candidates,
        );
        if archive_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut recall_filters = filters.clone();
        recall_filters.archive_ids = Some(archive_ids);
        let response = self
            .query_service
            .query_archives(
                recall_filters,
                PaginationParams::from_random_params(Some(100)),
                QueryOptions {
                    random: false,
                    include_tags: true,
                    user_id: Some(user_id.to_string()),
                },
            )
            .await?;
        Ok(filter_graph_recall_archives(
            role,
            user_paths,
            response.data,
        ))
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
               AND d.id = (SELECT latest.id FROM archive_dispositions latest \
                           WHERE latest.user_id = d.user_id AND latest.archive_id = d.archive_id \
                           ORDER BY latest.created_at DESC, latest.id DESC LIMIT 1)"
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

        // Learned cold-start rules are evaluated directly against the deterministic profile. The
        // formal path still requires promoted/eligible evidence. Observing ordinary-tag candidates
        // use the separate bounded multiplier below and never become hard rules.
        let learned_rules = match sqlx::query(
            "SELECT candidate.conditions_json, candidate.direction_probability,
                    candidate.lift, rule.action,
                    COALESCE(rule.preference_weight, 1.0) AS preference_weight
             FROM preference_rule_candidates candidate
             JOIN preference_rules rule
               ON rule.user_id = candidate.user_id
              AND rule.conditions_json = candidate.conditions_json
              AND rule.source = 'learned_cold_start'
             WHERE candidate.user_id = ?
               AND candidate.source = 'cold_start_v1'
               AND candidate.status = 'promoted'
               AND candidate.evidence_state = 'eligible'
               AND COALESCE(candidate.feature_kind, '') <> ?
               AND candidate.condition_key NOT LIKE 'profile:%:theme:%'
               AND rule.enabled = 1 AND rule.auto_paused = 0",
        )
        .bind(user_id)
        .bind(CANONICAL_THEME_FEATURE_KIND)
        .fetch_all(self.query_service.db())
        .await
        {
            Ok(rows) => rows,
            Err(error) => {
                debug!(%error, "cold-start learned candidates are not available yet");
                Vec::new()
            }
        };
        let observing_candidates = match sqlx::query(
            "SELECT conditions_json, feature_kind, unique_archive_count,
                    informative_result_count, positive_support, negative_support, lift
             FROM preference_rule_candidates
             WHERE user_id = ? AND source = 'cold_start_v1'
               AND status = 'observing' AND evidence_state = 'observing'",
        )
        .bind(user_id)
        .fetch_all(self.query_service.db())
        .await
        {
            Ok(rows) => rows,
            Err(error) => {
                debug!(%error, "observing cold-start candidates are not available yet");
                Vec::new()
            }
        };
        let metadata_namespaces = if observing_candidates.is_empty() {
            None
        } else {
            match load_metadata_namespace_set(self.query_service.db()).await {
                Ok(namespaces) => Some(namespaces),
                Err(error) => {
                    debug!(%error, "metadata namespace policy is unavailable for observing candidates");
                    None
                }
            }
        };
        if !learned_rules.is_empty() || !observing_candidates.is_empty() {
            let profile_query = format!(
                "SELECT archive_id, profile_json FROM archive_content_profiles
                 WHERE archive_id IN ({placeholders})
                   AND profile_version = '{CONTENT_PROFILE_VERSION}'
                   AND status IN ('completed','partial')
                   AND coverage >= 0.60
                   AND id = (SELECT latest.id FROM archive_content_profiles latest
                             WHERE latest.archive_id = archive_content_profiles.archive_id
                               AND latest.profile_version = '{CONTENT_PROFILE_VERSION}'
                               AND latest.status IN ('completed','partial')
                             ORDER BY latest.updated_at DESC, latest.id DESC LIMIT 1)"
            );
            let mut profile_request = sqlx::query(&profile_query);
            for archive_id in &archive_ids {
                profile_request = profile_request.bind(archive_id);
            }
            if let Ok(profile_rows) = profile_request.fetch_all(self.query_service.db()).await {
                for row in profile_rows {
                    let archive_id: String = row.get("archive_id");
                    let profile = serde_json::from_str::<ArchiveContentProfileDocument>(
                        row.get::<String, _>("profile_json").as_str(),
                    );
                    let Ok(profile) = profile else { continue };
                    for learned_rule in &learned_rules {
                        let condition: Value = match serde_json::from_str(
                            learned_rule.get::<String, _>("conditions_json").as_str(),
                        ) {
                            Ok(condition) => condition,
                            Err(_) => continue,
                        };
                        if !profile_condition_matches(&condition, &profile) {
                            continue;
                        }
                        let probability: f64 = learned_rule.get("direction_probability");
                        let lift: f64 = learned_rule.get("lift");
                        let preference_weight: f64 = learned_rule.get("preference_weight");
                        let rule_score = probability
                            * (0.5 + lift.abs().min(1.0))
                            * preference_weight.clamp(0.5, 2.0);
                        if let Some(score) = scores.get_mut(&archive_id) {
                            match learned_rule.get::<String, _>("action").as_str() {
                                "keep" => score.signed_score += rule_score,
                                "downrank" => score.signed_score -= rule_score,
                                _ => {}
                            }
                        }
                    }
                    if let Some(metadata_namespaces) = metadata_namespaces.as_ref() {
                        for candidate in &observing_candidates {
                            if candidate
                                .get::<Option<String>, _>("feature_kind")
                                .as_deref()
                                != Some("binary")
                            {
                                continue;
                            }
                            let condition: Value = match serde_json::from_str(
                                candidate.get::<String, _>("conditions_json").as_str(),
                            ) {
                                Ok(condition) => condition,
                                Err(_) => continue,
                            };
                            if !condition_is_ordinary_tag(&condition, metadata_namespaces)
                                || !profile_condition_matches(&condition, &profile)
                            {
                                continue;
                            }
                            let Some(soft_lift) = observing_soft_lift(
                                candidate.get("unique_archive_count"),
                                candidate.get("informative_result_count"),
                                candidate.get("positive_support"),
                                candidate.get("negative_support"),
                                candidate.get("lift"),
                            ) else {
                                continue;
                            };
                            if let Some(score) = scores.get_mut(&archive_id) {
                                score.soft_multiplier =
                                    (score.soft_multiplier * (1.0 + soft_lift)).clamp(0.25, 4.0);
                            }
                        }
                    }
                }
            }
        }

        Ok(candidates
            .into_iter()
            .map(|archive| {
                let score = scores.remove(&archive.id).unwrap_or_default();
                let (tier, weight) = tier_and_weight(&score);
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

fn graph_recall_archive_ids(
    archive_ids: Vec<String>,
    filters: &ArchiveFilters,
    current_candidates: &[Archive],
) -> Vec<String> {
    let current_ids = current_candidates
        .iter()
        .map(|archive| archive.id.as_str())
        .collect::<HashSet<_>>();
    let scoped_ids = filters
        .archive_ids
        .as_ref()
        .filter(|archive_ids| !archive_ids.is_empty())
        .map(|archive_ids| {
            archive_ids
                .iter()
                .map(String::as_str)
                .collect::<HashSet<_>>()
        });
    let excluded_ids = filters.exclude_archive_ids.as_ref().map(|archive_ids| {
        archive_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>()
    });

    archive_ids
        .into_iter()
        .filter(|archive_id| {
            !current_ids.contains(archive_id.as_str())
                && excluded_ids
                    .as_ref()
                    .is_none_or(|excluded| !excluded.contains(archive_id.as_str()))
                && scoped_ids
                    .as_ref()
                    .is_none_or(|allowed_ids| allowed_ids.contains(archive_id.as_str()))
        })
        .collect()
}

fn graph_recall_has_path_scope(role: &str, user_paths: &[String]) -> bool {
    role == "admin" || !user_paths.is_empty()
}

fn filter_graph_recall_archives(
    role: &str,
    user_paths: &[String],
    archives: Vec<Archive>,
) -> Vec<Archive> {
    archives
        .into_iter()
        .filter(|archive| {
            path_permission::has_path_permission_with_paths(role, user_paths, &archive.path)
        })
        .collect()
}

fn soft_multiplier_for_tier(tier: PreferenceTier, multiplier: f64) -> f64 {
    if tier == PreferenceTier::Unknown {
        multiplier
    } else {
        1.0
    }
}

fn tier_and_weight(score: &PreferenceScore) -> (PreferenceTier, f64) {
    if score.auto_delete {
        return (PreferenceTier::AutoDelete, 0.0);
    }
    if score.signed_score > f64::EPSILON {
        return (
            PreferenceTier::Keep,
            (1.5 + score.signed_score.min(2.0) + score.behavior_boost)
                * soft_multiplier_for_tier(PreferenceTier::Keep, score.soft_multiplier),
        );
    }
    if score.signed_score < -f64::EPSILON {
        return (
            PreferenceTier::Downrank,
            (0.08 / (1.0 + score.signed_score.abs()) + score.behavior_boost * 0.05).max(0.01)
                * soft_multiplier_for_tier(PreferenceTier::Downrank, score.soft_multiplier),
        );
    }
    (
        PreferenceTier::Unknown,
        (1.0 + score.behavior_boost)
            * soft_multiplier_for_tier(PreferenceTier::Unknown, score.soft_multiplier),
    )
}

fn permitted_archive_ids(
    role: &str,
    user_paths: &[String],
    targets: Vec<ArchiveDeleteTarget>,
) -> Vec<String> {
    targets
        .into_iter()
        .filter(|target| {
            path_permission::has_path_permission_with_paths(role, user_paths, &target.path)
        })
        .map(|target| target.id)
        .collect()
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

fn select_uniform_archives<R: Rng + ?Sized>(
    candidates: Vec<WeightedArchive>,
    count: usize,
    rng: &mut R,
) -> (Vec<Archive>, usize) {
    let mut eligible: Vec<WeightedArchive> = candidates
        .into_iter()
        .filter(|candidate| candidate.tier != PreferenceTier::AutoDelete)
        .collect();
    eligible.shuffle(rng);
    eligible.truncate(count);
    let explored_count = eligible
        .iter()
        .filter(|candidate| candidate.tier == PreferenceTier::Unknown)
        .count();
    (
        eligible
            .into_iter()
            .map(|candidate| candidate.archive)
            .collect(),
        explored_count,
    )
}

fn stable_experiment_bucket(user_id: &str) -> u8 {
    // FNV-1a is deliberately explicit instead of DefaultHasher: experiment
    // assignment must not change between processes or Rust versions.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in user_id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    (hash % 100) as u8
}

fn topics_for_archives<'a>(
    archive_ids: impl Iterator<Item = &'a str>,
    snapshots: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut topics = HashSet::new();
    for archive_id in archive_ids {
        if let Some(values) = snapshots.get(archive_id) {
            topics.extend(values.iter().cloned());
        }
    }
    let mut topics: Vec<String> = topics.into_iter().collect();
    topics.sort();
    topics
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
#[path = "service_tests.rs"]
mod service_tests;
