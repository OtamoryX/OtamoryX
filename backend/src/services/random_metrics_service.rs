use anyhow::{Context, Result};
use serde::Serialize;
use sqlx::{Pool, Row, Sqlite};

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
}

#[derive(Debug, Clone, Serialize)]
pub struct RandomRecommendationMetrics {
    pub days: i64,
    pub overall: RandomRecommendationMetric,
    pub preferred: RandomRecommendationMetric,
    pub exploration: RandomRecommendationMetric,
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
            "SELECT COUNT(*) AS exposed,
                SUM(CASE WHEN i.opened_at IS NOT NULL THEN 1 ELSE 0 END) AS opened,
                SUM(CASE WHEN i.effective_read_at IS NOT NULL THEN 1 ELSE 0 END) AS effective_reads,
                SUM(CASE WHEN i.quick_exit_at IS NOT NULL THEN 1 ELSE 0 END) AS quick_exits,
                SUM(CASE WHEN i.manual_delete_at IS NOT NULL THEN 1 ELSE 0 END) AS manual_deletes,
                i.preference_tier
             FROM random_recommendation_items i
             WHERE {scope} AND i.created_at >= datetime('now', ?)
             GROUP BY i.preference_tier"
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
        for row in rows {
            let mut metric = RandomRecommendationMetric {
                exposed: row.get("exposed"),
                opened: row.get("opened"),
                effective_reads: row.get("effective_reads"),
                quick_exits: row.get("quick_exits"),
                manual_deletes: row.get("manual_deletes"),
                effective_read_rate: 0.0,
            };
            metric.effective_read_rate = if metric.opened == 0 {
                0.0
            } else {
                metric.effective_reads as f64 / metric.opened as f64
            };
            overall.exposed += metric.exposed;
            overall.opened += metric.opened;
            overall.effective_reads += metric.effective_reads;
            overall.quick_exits += metric.quick_exits;
            overall.manual_deletes += metric.manual_deletes;
            if row.get::<String, _>("preference_tier") == "unknown" {
                exploration = metric;
            } else {
                preferred.exposed += metric.exposed;
                preferred.opened += metric.opened;
                preferred.effective_reads += metric.effective_reads;
                preferred.quick_exits += metric.quick_exits;
                preferred.manual_deletes += metric.manual_deletes;
            }
        }
        overall.effective_read_rate = if overall.opened == 0 {
            0.0
        } else {
            overall.effective_reads as f64 / overall.opened as f64
        };
        preferred.effective_read_rate = if preferred.opened == 0 {
            0.0
        } else {
            preferred.effective_reads as f64 / preferred.opened as f64
        };
        exploration.effective_read_rate = if exploration.opened == 0 {
            0.0
        } else {
            exploration.effective_reads as f64 / exploration.opened as f64
        };
        Ok(RandomRecommendationMetrics {
            days,
            overall,
            preferred,
            exploration,
        })
    }

    pub async fn purge_expired(&self, limit: i64) -> Result<u64> {
        let result = sqlx::query("DELETE FROM random_recommendation_sessions WHERE id IN (SELECT id FROM random_recommendation_sessions WHERE expires_at < CURRENT_TIMESTAMP ORDER BY expires_at LIMIT ?)")
            .bind(limit.clamp(1, 10_000)).execute(&self.pool).await?;
        Ok(result.rows_affected())
    }
}

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
