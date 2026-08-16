use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::{Pool, Row, Sqlite};
use std::collections::BTreeSet;
use uuid::Uuid;

use crate::models::ContentAnalysisResult;

const MIN_CONCEPT_CONFIDENCE: f64 = 0.75;
const MIN_INDEPENDENT_ARCHIVES: usize = 3;
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
        let event = sqlx::query("SELECT archive_id,event_type,metadata_json FROM user_behavior_events WHERE id=? AND user_id=?").bind(event_id).bind(user_id).fetch_optional(&self.pool).await?.ok_or_else(|| anyhow!("behavior event not found"))?;
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
        let concepts = concept_keys(&result);
        for size in 2..=3.min(concepts.len()) {
            for key in combinations(&concepts, size)
                .into_iter()
                .map(|v| v.join("+"))
            {
                self.update_candidate(user_id, &archive_id, &key, signal)
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
        signal: f64,
    ) -> Result<()> {
        let conditions: Vec<Value> = key
            .split('+')
            .map(|concept| json!({"concept": concept, "minConfidence": MIN_CONCEPT_CONFIDENCE}))
            .collect();
        let condition_json = serde_json::to_string(&json!({"all": conditions}))?;
        let row = sqlx::query("SELECT id,sample_archives_json,positive_score,negative_score,positive_support,negative_support FROM preference_rule_candidates WHERE user_id=? AND condition_key=?").bind(user_id).bind(key).fetch_optional(&self.pool).await?;
        let (id, mut samples, mut pos, mut neg, mut pos_count, mut neg_count) =
            if let Some(row) = row {
                (
                    row.get::<String, _>("id"),
                    serde_json::from_str::<Vec<String>>(
                        row.get::<String, _>("sample_archives_json").as_str(),
                    )
                    .unwrap_or_default(),
                    row.get::<f64, _>("positive_score"),
                    row.get::<f64, _>("negative_score"),
                    row.get::<i64, _>("positive_support"),
                    row.get::<i64, _>("negative_support"),
                )
            } else {
                (Uuid::new_v4().to_string(), Vec::new(), 0.0, 0.0, 0, 0)
            };
        if !samples.iter().any(|sample| sample == archive_id) {
            samples.push(archive_id.to_string());
            if signal > 0.0 {
                pos_count += 1;
            } else {
                neg_count += 1;
            }
        }
        if signal > 0.0 {
            pos += signal;
        } else {
            neg += -signal;
        }
        let confidence = ((pos - neg) / (pos + neg).max(1.0) + 1.0) / 2.0;
        let status = if samples.len() >= MIN_INDEPENDENT_ARCHIVES {
            "promoted"
        } else {
            "observing"
        };
        sqlx::query("INSERT INTO preference_rule_candidates (id,user_id,condition_key,conditions_json,positive_score,negative_score,positive_support,negative_support,sample_archives_json,confidence,status,last_learned_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP) ON CONFLICT(user_id,condition_key) DO UPDATE SET positive_score=excluded.positive_score,negative_score=excluded.negative_score,positive_support=excluded.positive_support,negative_support=excluded.negative_support,sample_archives_json=excluded.sample_archives_json,confidence=excluded.confidence,status=excluded.status,last_learned_at=CURRENT_TIMESTAMP,updated_at=CURRENT_TIMESTAMP)").bind(id).bind(user_id).bind(key).bind(&condition_json).bind(pos).bind(neg).bind(pos_count).bind(neg_count).bind(serde_json::to_string(&samples)?).bind(confidence).bind(status).execute(&self.pool).await?;
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
        let exists: Option<String> = sqlx::query_scalar("SELECT id FROM preference_rules WHERE user_id=? AND source='learned' AND conditions_json=? LIMIT 1").bind(user_id).bind(&condition_json).fetch_optional(&self.pool).await?;
        if exists.is_some() {
            return Ok(());
        }
        let action = if confidence >= 0.5 {
            "keep"
        } else {
            "downrank"
        };
        sqlx::query("INSERT INTO preference_rules (id,user_id,name,rule_version,conditions_json,exceptions_json,action,confidence_threshold,enabled,owner_role,source,preference_weight,positive_support,negative_support,last_learned_at) VALUES (?,?,?,?,?,?,?,?,0,'user','learned',?,?,?,CURRENT_TIMESTAMP)").bind(Uuid::new_v4().to_string()).bind(user_id).bind(format!("Learned: {key}")).bind(format!("learned-{}", Utc::now().timestamp())).bind(condition_json).bind("{}").bind(action).bind(0.85_f64).bind((confidence * 2.0).clamp(0.1, 2.0)).bind(pos).bind(neg).execute(&self.pool).await?;
        Ok(())
    }
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

fn concept_keys(result: &ContentAnalysisResult) -> Vec<String> {
    let mut values: BTreeSet<String> = result
        .concepts
        .iter()
        .filter(|c| c.confidence as f64 >= MIN_CONCEPT_CONFIDENCE && !c.evidence_pages.is_empty())
        .map(|c| c.name.trim().to_lowercase())
        .filter(|c| !c.is_empty())
        .collect();
    values.extend(
        result
            .themes
            .iter()
            .map(|v| v.trim().to_lowercase())
            .filter(|v| !v.is_empty()),
    );
    values.into_iter().collect()
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
