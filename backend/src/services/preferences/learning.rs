use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use sqlx::{Pool, Row, Sqlite};
use std::collections::{BTreeSet, HashSet};
use uuid::Uuid;

use crate::models::ContentAnalysisResult;

const MIN_INDEPENDENT_ARCHIVES: usize = 3;
const MIN_EFFECTIVE_SUPPORT: f64 = 3.0;
const DEFAULT_SIGNAL_HALF_LIFE_DAYS: f64 = 30.0;
const MAX_RETRIES: i64 = 5;

#[derive(Clone)]
pub struct PreferenceLearningService {
    pool: Pool<Sqlite>,
}

impl PreferenceLearningService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn enqueue_events(&self) -> Result<u64> {
        let result = sqlx::query(
            "INSERT OR IGNORE INTO preference_learning_events (id, behavior_event_id, user_id)
             SELECT lower(hex(randomblob(16))), id, user_id FROM user_behavior_events
             WHERE event_type IN ('manual_delete','auto_delete','restore','rule_correction','exit','continue_reading','repeat_open')",
        ).execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    pub async fn process_next(&self) -> Result<bool> {
        self.enqueue_events().await?;
        let row = sqlx::query("SELECT id, behavior_event_id, user_id, attempts FROM preference_learning_events WHERE status IN ('pending','retryable','waiting_analysis') AND (next_attempt_at IS NULL OR next_attempt_at <= CURRENT_TIMESTAMP) ORDER BY updated_at ASC LIMIT 1").fetch_optional(&self.pool).await?;
        let Some(row) = row else { return Ok(false) };
        let id: String = row.get("id");
        let event_id: String = row.get("behavior_event_id");
        let user_id: String = row.get("user_id");
        let attempts: i64 = row.get("attempts");
        let claimed = sqlx::query("UPDATE preference_learning_events SET status='running', attempts=attempts+1, updated_at=CURRENT_TIMESTAMP WHERE id=? AND status IN ('pending','retryable','waiting_analysis') AND (next_attempt_at IS NULL OR next_attempt_at <= CURRENT_TIMESTAMP)").bind(&id).execute(&self.pool).await?;
        if claimed.rows_affected() != 1 {
            return Ok(true);
        }
        match self.process_event(&event_id, &user_id).await {
            Ok(()) => {
                sqlx::query("UPDATE preference_learning_events SET status='completed', last_error=NULL, next_attempt_at=NULL, updated_at=CURRENT_TIMESTAMP WHERE id=?").bind(&id).execute(&self.pool).await?;
            }
            Err(error) if error.to_string() == "analysis pending" => {
                sqlx::query("UPDATE preference_learning_events SET status='waiting_analysis', next_attempt_at=?, last_error=?, updated_at=CURRENT_TIMESTAMP WHERE id=?").bind(Utc::now() + Duration::seconds(30)).bind(error.to_string()).bind(&id).execute(&self.pool).await?;
            }
            Err(error) => {
                let status = if attempts + 1 >= MAX_RETRIES {
                    "failed"
                } else {
                    "retryable"
                };
                let delay = 2_i64.pow((attempts as u32).min(5));
                sqlx::query("UPDATE preference_learning_events SET status=?, next_attempt_at=?, last_error=?, updated_at=CURRENT_TIMESTAMP WHERE id=?").bind(status).bind(Utc::now() + Duration::seconds(delay)).bind(error.to_string()).bind(&id).execute(&self.pool).await?;
                tracing::warn!(%event_id, %error, "preference learning event failed");
            }
        }
        Ok(true)
    }

    async fn process_event(&self, event_id: &str, user_id: &str) -> Result<()> {
        let event = sqlx::query("SELECT id,archive_id,event_type,metadata_json,occurred_at FROM user_behavior_events WHERE id=? AND user_id=?").bind(event_id).bind(user_id).fetch_optional(&self.pool).await?.ok_or_else(|| anyhow!("behavior event not found"))?;
        let archive_id: String = event
            .try_get::<Option<String>, _>("archive_id")?
            .ok_or_else(|| anyhow!("event has no archive"))?;
        let analysis = sqlx::query("SELECT result_json FROM content_analyses WHERE archive_id=? AND status='completed' ORDER BY created_at DESC LIMIT 1").bind(&archive_id).fetch_optional(&self.pool).await?;
        let Some(analysis) = analysis else {
            return Err(anyhow!("analysis pending"));
        };
        let result: ContentAnalysisResult =
            serde_json::from_str(analysis.get::<String, _>("result_json").as_str())
                .context("invalid completed analysis")?;
        let metadata: Value =
            serde_json::from_str(event.get::<String, _>("metadata_json").as_str())
                .unwrap_or_else(|_| json!({}));
        let signal = signal_for_event(event.get::<String, _>("event_type").as_str(), &metadata);
        if signal == 0.0 {
            return Ok(());
        }
        let themes = theme_keys(&result);
        for size in 2..=3.min(themes.len()) {
            for key in combinations(&themes, size)
                .into_iter()
                .map(|v| v.join("+"))
            {
                self.update_candidate(
                    user_id,
                    &archive_id,
                    &key,
                    event.get::<String, _>("id").as_str(),
                    event.get::<String, _>("event_type").as_str(),
                    event.get::<DateTime<Utc>, _>("occurred_at"),
                    signal,
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn update_candidate(
        &self,
        user_id: &str,
        archive_id: &str,
        key: &str,
        behavior_event_id: &str,
        event_type: &str,
        occurred_at: DateTime<Utc>,
        signal: f64,
    ) -> Result<()> {
        let conditions: Vec<Value> = key
            .split('+')
            .map(|theme| json!({"theme": theme}))
            .collect();
        let condition_json = serde_json::to_string(&json!({"all": conditions}))?;
        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO preference_candidate_signals
             (id,user_id,condition_key,archive_id,behavior_event_id,event_type,raw_score,occurred_at)
             VALUES (?,?,?,?,?,?,?,?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(key)
        .bind(archive_id)
        .bind(behavior_event_id)
        .bind(event_type)
        .bind(signal)
        .bind(occurred_at)
        .execute(&self.pool)
        .await?;
        if inserted.rows_affected() == 0 {
            return Ok(());
        }

        let rows = sqlx::query(
            "SELECT archive_id, raw_score, occurred_at FROM preference_candidate_signals
             WHERE user_id=? AND condition_key=?",
        )
        .bind(user_id)
        .bind(key)
        .fetch_all(&self.pool)
        .await?;
        let now = Utc::now();
        let mut samples = BTreeSet::new();
        let mut positive_archives = HashSet::new();
        let mut negative_archives = HashSet::new();
        let mut pos = 0.0;
        let mut neg = 0.0;
        for row in rows {
            let sample_archive_id: String = row.get("archive_id");
            let raw_score: f64 = row.get("raw_score");
            let signal_time: DateTime<Utc> = row.get("occurred_at");
            let effective_score = decayed_score(raw_score, signal_time, now);
            samples.insert(sample_archive_id.clone());
            if effective_score > 0.0 {
                pos += effective_score;
                positive_archives.insert(sample_archive_id);
            } else if effective_score < 0.0 {
                neg += -effective_score;
                negative_archives.insert(sample_archive_id);
            }
        }
        let id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM preference_rule_candidates WHERE user_id=? AND condition_key=?",
        )
        .bind(user_id)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        let id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let pos_count = positive_archives.len() as i64;
        let neg_count = negative_archives.len() as i64;
        let confidence = ((pos - neg) / (pos + neg).max(1.0) + 1.0) / 2.0;
        let status =
            if samples.len() >= MIN_INDEPENDENT_ARCHIVES && pos + neg >= MIN_EFFECTIVE_SUPPORT {
                "promoted"
            } else {
                "observing"
            };
        sqlx::query("INSERT INTO preference_rule_candidates (id,user_id,condition_key,conditions_json,positive_score,negative_score,positive_support,negative_support,sample_archives_json,confidence,status,last_learned_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP) ON CONFLICT(user_id,condition_key) DO UPDATE SET positive_score=excluded.positive_score,negative_score=excluded.negative_score,positive_support=excluded.positive_support,negative_support=excluded.negative_support,sample_archives_json=excluded.sample_archives_json,confidence=excluded.confidence,status=excluded.status,last_learned_at=CURRENT_TIMESTAMP,updated_at=CURRENT_TIMESTAMP)").bind(id).bind(user_id).bind(key).bind(&condition_json).bind(pos).bind(neg).bind(pos_count).bind(neg_count).bind(serde_json::to_string(&samples.into_iter().collect::<Vec<_>>())?).bind(confidence).bind(status).execute(&self.pool).await?;
        if status == "promoted" {
            self.promote_rule(
                user_id,
                key,
                &serde_json::from_str::<Value>(&condition_json)?
                    .get("all")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
                confidence,
                pos_count,
                neg_count,
            )
            .await?;
        }
        Ok(())
    }

    async fn promote_rule(
        &self,
        user_id: &str,
        key: &str,
        conditions: &[Value],
        confidence: f64,
        pos: i64,
        neg: i64,
    ) -> Result<()> {
        let condition_json = serde_json::to_string(&json!({"all": conditions}))?;
        let action = if confidence >= 0.5 {
            "keep"
        } else {
            "downrank"
        };
        let rule_version = format!("learned-{}", Utc::now().timestamp_millis());
        let existing: Option<String> = sqlx::query_scalar("SELECT id FROM preference_rules WHERE user_id=? AND source='learned' AND conditions_json=? LIMIT 1").bind(user_id).bind(&condition_json).fetch_optional(&self.pool).await?;
        if let Some(id) = existing {
            sqlx::query("UPDATE preference_rules SET rule_version=?, action=?, preference_weight=?, positive_support=?, negative_support=?, last_learned_at=CURRENT_TIMESTAMP, updated_at=CURRENT_TIMESTAMP WHERE id=?")
                .bind(rule_version).bind(action).bind((confidence * 2.0).clamp(0.1, 2.0)).bind(pos).bind(neg).bind(id).execute(&self.pool).await?;
        } else {
            sqlx::query("INSERT INTO preference_rules (id,user_id,name,rule_version,conditions_json,exceptions_json,action,confidence_threshold,enabled,owner_role,source,preference_weight,positive_support,negative_support,last_learned_at) VALUES (?,?,?,?,?,?,?,?,0,'user','learned',?,?,?,CURRENT_TIMESTAMP)").bind(Uuid::new_v4().to_string()).bind(user_id).bind(format!("Learned: {key}")).bind(rule_version).bind(condition_json).bind("{}").bind(action).bind(0.85_f64).bind((confidence * 2.0).clamp(0.1, 2.0)).bind(pos).bind(neg).execute(&self.pool).await?;
        }
        Ok(())
    }
}

fn decayed_score(raw_score: f64, occurred_at: DateTime<Utc>, now: DateTime<Utc>) -> f64 {
    let age_days = (now - occurred_at).num_milliseconds().max(0) as f64 / 86_400_000.0;
    raw_score * 2_f64.powf(-age_days / DEFAULT_SIGNAL_HALF_LIFE_DAYS)
}

fn signal_for_event(event_type: &str, metadata: &Value) -> f64 {
    match event_type {
        "manual_delete" => -5.0,
        "restore" | "rule_correction" => 6.0,
        "continue_reading" | "repeat_open" => 3.0,
        "exit" => {
            let start = metadata
                .get("startPage")
                .or_else(|| metadata.get("start_page"))
                .and_then(Value::as_i64)
                .unwrap_or(1);
            let end = metadata
                .get("endPage")
                .or_else(|| metadata.get("end_page"))
                .and_then(Value::as_i64)
                .unwrap_or(1);
            let total = metadata
                .get("totalPages")
                .or_else(|| metadata.get("total_pages"))
                .and_then(Value::as_i64)
                .unwrap_or(1)
                .max(1);
            let duration = metadata
                .get("durationMs")
                .or_else(|| metadata.get("duration_ms"))
                .and_then(Value::as_i64)
                .unwrap_or(0);
            if duration > 0 && duration < 15_000 && end <= start + 1 {
                -0.5
            } else if end >= total * 3 / 4 || end - start >= 10 {
                3.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

fn theme_keys(result: &ContentAnalysisResult) -> Vec<String> {
    result
        .themes
        .iter()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())
        .collect()
}

fn combinations(values: &[String], size: usize) -> Vec<Vec<String>> {
    fn visit(
        values: &[String],
        size: usize,
        start: usize,
        current: &mut Vec<String>,
        output: &mut Vec<Vec<String>>,
    ) {
        if current.len() == size {
            output.push(current.clone());
            return;
        }
        for index in start..values.len() {
            current.push(values[index].clone());
            visit(values, size, index + 1, current, output);
            current.pop();
        }
    }
    let mut output = Vec::new();
    visit(values, size, 0, &mut Vec::new(), &mut output);
    output
}

pub fn spawn_preference_learning_worker(pool: Pool<Sqlite>) {
    tokio::spawn(async move {
        let service = PreferenceLearningService::new(pool);
        loop {
            match service.process_next().await {
                Ok(true) => {}
                Ok(false) => tokio::time::sleep(std::time::Duration::from_secs(10)).await,
                Err(error) => {
                    tracing::warn!(%error, "preference learning worker iteration failed");
                    tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn derives_signals() {
        assert_eq!(signal_for_event("manual_delete", &json!({})), -5.0);
        assert_eq!(
            signal_for_event(
                "exit",
                &json!({"startPage":1,"endPage":2,"totalPages":100,"durationMs":1000})
            ),
            -0.5
        );
    }
    #[test]
    fn combinations_are_stable() {
        assert_eq!(
            combinations(&["a".into(), "b".into(), "c".into()], 2).len(),
            3
        );
    }
}
