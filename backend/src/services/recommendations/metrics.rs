use anyhow::{Context, Result};
use serde::Serialize;
use sqlx::{Pool, Row, Sqlite};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Serialize, Default)]
pub struct RandomRecommendationMetric {
    pub exposed: i64,
    pub opened: i64,
    #[serde(rename = "effectiveReads")]
    pub effective_reads: i64,
    #[serde(rename = "quickExits")]
    pub quick_exits: i64,
    #[serde(rename = "manualDeletes")]
    pub manual_deletes: i64,
    #[serde(rename = "effectiveReadRate")]
    pub effective_read_rate: f64,
    #[serde(rename = "manualDeletesPer100Opens")]
    pub manual_deletes_per_100_opens: f64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RandomRecommendationTopicCoverage {
    #[serde(rename = "candidateTopicCount")]
    pub candidate_topic_count: i64,
    #[serde(rename = "exposedTopicCount")]
    pub exposed_topic_count: i64,
    #[serde(rename = "explorationTopicCount")]
    pub exploration_topic_count: i64,
    #[serde(rename = "exposureCoverage")]
    pub exposure_coverage: f64,
    #[serde(rename = "explorationCoverage")]
    pub exploration_coverage: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RandomRecommendationAlgorithmMetrics {
    #[serde(rename = "algorithmVariant")]
    pub algorithm_variant: String,
    pub overall: RandomRecommendationMetric,
    pub preferred: RandomRecommendationMetric,
    pub exploration: RandomRecommendationMetric,
    pub topics: RandomRecommendationTopicCoverage,
}

#[derive(Debug, Clone, Serialize)]
pub struct RandomRecommendationMetrics {
    pub days: i64,
    pub overall: RandomRecommendationMetric,
    pub preferred: RandomRecommendationMetric,
    pub exploration: RandomRecommendationMetric,
    pub topics: RandomRecommendationTopicCoverage,
    #[serde(rename = "byAlgorithm")]
    pub by_algorithm: Vec<RandomRecommendationAlgorithmMetrics>,
}

#[derive(Clone)]
pub struct RandomMetricsService {
    pool: Pool<Sqlite>,
}

impl RandomMetricsService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn metrics(
        &self,
        user_id: &str,
        admin: bool,
        days: i64,
    ) -> Result<RandomRecommendationMetrics> {
        let days = days.clamp(7, 90);
        let scope = if admin { "1=1" } else { "i.user_id = ?" };
        let sql = format!(
            "SELECT s.algorithm_variant, COUNT(*) AS exposed,
                SUM(CASE WHEN i.opened_at IS NOT NULL THEN 1 ELSE 0 END) AS opened,
                SUM(CASE WHEN i.effective_read_at IS NOT NULL THEN 1 ELSE 0 END) AS effective_reads,
                SUM(CASE WHEN i.quick_exit_at IS NOT NULL THEN 1 ELSE 0 END) AS quick_exits,
                SUM(CASE WHEN i.manual_delete_at IS NOT NULL THEN 1 ELSE 0 END) AS manual_deletes,
                i.preference_tier
             FROM random_recommendation_items i
             JOIN random_recommendation_sessions s ON s.id = i.session_id
             WHERE {scope} AND i.created_at >= datetime('now', ?)
             GROUP BY s.algorithm_variant, i.preference_tier"
        );
        let mut request = sqlx::query(&sql);
        if !admin {
            request = request.bind(user_id);
        }
        request = request.bind(format!("-{days} days"));
        let rows = request
            .fetch_all(&self.pool)
            .await
            .context("failed to query random recommendation metrics")?;
        let mut overall = RandomRecommendationMetric::default();
        let mut preferred = RandomRecommendationMetric::default();
        let mut exploration = RandomRecommendationMetric::default();
        let mut variants: BTreeMap<String, MetricBreakdown> = BTreeMap::new();
        for row in rows {
            let metric = RandomRecommendationMetric {
                exposed: row.get("exposed"),
                opened: row.get("opened"),
                effective_reads: row.get("effective_reads"),
                quick_exits: row.get("quick_exits"),
                manual_deletes: row.get("manual_deletes"),
                effective_read_rate: 0.0,
                manual_deletes_per_100_opens: 0.0,
            };
            let variant: String = row.get("algorithm_variant");
            let variant_metrics = variants.entry(variant).or_default();
            add_metric(&mut overall, &metric);
            add_metric(&mut variant_metrics.overall, &metric);
            if row.get::<String, _>("preference_tier") == "unknown" {
                add_metric(&mut exploration, &metric);
                add_metric(&mut variant_metrics.exploration, &metric);
            } else {
                add_metric(&mut preferred, &metric);
                add_metric(&mut variant_metrics.preferred, &metric);
            }
        }
        finalize_metric(&mut overall);
        finalize_metric(&mut preferred);
        finalize_metric(&mut exploration);

        let mut topic_coverage = self.topic_coverage(user_id, admin, days).await?;
        let mut by_algorithm = Vec::with_capacity(variants.len());
        for (algorithm_variant, mut metric) in variants {
            finalize_metric(&mut metric.overall);
            finalize_metric(&mut metric.preferred);
            finalize_metric(&mut metric.exploration);
            by_algorithm.push(RandomRecommendationAlgorithmMetrics {
                topics: topic_coverage
                    .by_algorithm
                    .remove(&algorithm_variant)
                    .unwrap_or_default(),
                algorithm_variant,
                overall: metric.overall,
                preferred: metric.preferred,
                exploration: metric.exploration,
            });
        }
        for (algorithm_variant, topics) in topic_coverage.by_algorithm {
            by_algorithm.push(RandomRecommendationAlgorithmMetrics {
                algorithm_variant,
                overall: RandomRecommendationMetric::default(),
                preferred: RandomRecommendationMetric::default(),
                exploration: RandomRecommendationMetric::default(),
                topics,
            });
        }
        Ok(RandomRecommendationMetrics {
            days,
            overall,
            preferred,
            exploration,
            topics: topic_coverage.overall,
            by_algorithm,
        })
    }

    async fn topic_coverage(&self, user_id: &str, admin: bool, days: i64) -> Result<TopicCoverage> {
        let scope = if admin { "1=1" } else { "s.user_id = ?" };
        let session_sql = format!(
            "SELECT s.algorithm_variant, s.candidate_topics_json, s.exploration_topics_json
             FROM random_recommendation_sessions s
             WHERE {scope} AND s.created_at >= datetime('now', ?)"
        );
        let mut session_request = sqlx::query(&session_sql);
        if !admin {
            session_request = session_request.bind(user_id);
        }
        let session_rows = session_request
            .bind(format!("-{days} days"))
            .fetch_all(&self.pool)
            .await
            .context("failed to query random recommendation topic sessions")?;
        let item_scope = if admin { "1=1" } else { "i.user_id = ?" };
        let item_sql = format!(
            "SELECT s.algorithm_variant, i.topics_json
             FROM random_recommendation_items i
             JOIN random_recommendation_sessions s ON s.id=i.session_id
             WHERE {item_scope} AND i.created_at >= datetime('now', ?)"
        );
        let mut item_request = sqlx::query(&item_sql);
        if !admin {
            item_request = item_request.bind(user_id);
        }
        let item_rows = item_request
            .bind(format!("-{days} days"))
            .fetch_all(&self.pool)
            .await
            .context("failed to query random recommendation exposed topics")?;

        let mut topic_sets: BTreeMap<String, TopicSets> = BTreeMap::new();
        for row in session_rows {
            let variant: String = row.get("algorithm_variant");
            let values = topic_sets.entry(variant).or_default();
            values
                .candidate
                .extend(parse_topics(row.get::<String, _>("candidate_topics_json")));
            values.exploration.extend(parse_topics(
                row.get::<String, _>("exploration_topics_json"),
            ));
        }
        for row in item_rows {
            let variant: String = row.get("algorithm_variant");
            topic_sets
                .entry(variant)
                .or_default()
                .exposed
                .extend(parse_topics(row.get::<String, _>("topics_json")));
        }
        let mut overall_sets = TopicSets::default();
        let mut by_algorithm = BTreeMap::new();
        for (variant, sets) in topic_sets {
            overall_sets
                .candidate
                .extend(sets.candidate.iter().cloned());
            overall_sets.exposed.extend(sets.exposed.iter().cloned());
            overall_sets
                .exploration
                .extend(sets.exploration.iter().cloned());
            by_algorithm.insert(variant, coverage_metric(&sets));
        }
        Ok(TopicCoverage {
            overall: coverage_metric(&overall_sets),
            by_algorithm,
        })
    }

    pub async fn purge_expired(&self, limit: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM random_recommendation_sessions WHERE id IN (SELECT id FROM random_recommendation_sessions WHERE expires_at < CURRENT_TIMESTAMP ORDER BY expires_at LIMIT ?)")
            .bind(limit.clamp(1, 10_000)).execute(&self.pool).await?;
        Ok(result.rows_affected())
    }
}

#[derive(Default)]
struct MetricBreakdown {
    overall: RandomRecommendationMetric,
    preferred: RandomRecommendationMetric,
    exploration: RandomRecommendationMetric,
}

#[derive(Default)]
struct TopicSets {
    candidate: HashSet<String>,
    exposed: HashSet<String>,
    exploration: HashSet<String>,
}

struct TopicCoverage {
    overall: RandomRecommendationTopicCoverage,
    by_algorithm: BTreeMap<String, RandomRecommendationTopicCoverage>,
}

fn coverage_metric(sets: &TopicSets) -> RandomRecommendationTopicCoverage {
    let candidate_topic_count = sets.candidate.len() as i64;
    let exposed_topic_count = sets.exposed.len() as i64;
    let exploration_topic_count = sets.exploration.len() as i64;
    RandomRecommendationTopicCoverage {
        candidate_topic_count,
        exposed_topic_count,
        exploration_topic_count,
        exposure_coverage: coverage_ratio(exposed_topic_count, candidate_topic_count),
        exploration_coverage: coverage_ratio(exploration_topic_count, candidate_topic_count),
    }
}

fn coverage_ratio(numerator: i64, denominator: i64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn parse_topics(value: String) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(&value).unwrap_or_default()
}

fn add_metric(target: &mut RandomRecommendationMetric, source: &RandomRecommendationMetric) {
    target.exposed += source.exposed;
    target.opened += source.opened;
    target.effective_reads += source.effective_reads;
    target.quick_exits += source.quick_exits;
    target.manual_deletes += source.manual_deletes;
}

fn finalize_metric(metric: &mut RandomRecommendationMetric) {
    metric.effective_read_rate = if metric.opened == 0 {
        0.0
    } else {
        metric.effective_reads as f64 / metric.opened as f64
    };
    metric.manual_deletes_per_100_opens = if metric.opened == 0 {
        0.0
    } else {
        metric.manual_deletes as f64 * 100.0 / metric.opened as f64
    };
}

#[cfg(test)]
#[path = "metrics_tests.rs"]
mod metrics_tests;
pub fn spawn_random_recommendation_cleanup(pool: Pool<Sqlite>) {
    tokio::spawn(async move {
        let service = RandomMetricsService::new(pool);
        loop {
            match service.purge_expired(500).await {
                Ok(count) if count > 0 => {
                    tracing::info!(count, "expired random recommendation sessions purged")
                }
                Err(error) => {
                    tracing::warn!(%error, "random recommendation retention cleanup failed")
                }
                _ => {}
            }
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    });
}
