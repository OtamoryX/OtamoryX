use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use sqlx::{Pool, Row, Sqlite};
use std::{
    sync::{Arc, OnceLock},
    time::Duration as StdDuration,
};
use tokio::sync::Notify;
use uuid::Uuid;

use crate::models::{
    AutoDeleteDecision, ContentAnalysisResult, PreferenceRule, PreferenceRuleEvaluation,
    PreferenceRuleInput, PreferenceRuleVersionStats, PreferenceRuleVersionWindowStats,
};
use crate::services::{AutoDeleteResult, AutoDeleteService};

pub struct PreferenceDecisionService {
    pool: Pool<Sqlite>,
}

static PREFERENCE_DECISION_SIGNAL: OnceLock<Arc<Notify>> = OnceLock::new();

const ELIGIBLE_RETRYABLE_EVALUATIONS_SQL: &str = "
    SELECT e.analysis_id, e.rule_id, e.rule_version, e.next_attempt_at
    FROM preference_rule_evaluations e
    JOIN content_analyses a ON a.id = e.analysis_id
    JOIN archives archive ON archive.id = a.archive_id
    JOIN preference_rules r ON r.id = e.rule_id
    WHERE e.execution_status = 'retryable'
      AND e.next_attempt_at IS NOT NULL
      AND a.status = 'completed'
      AND a.content_fingerprint = archive.file_hash
      AND r.enabled = 1
      AND r.auto_paused = 0
      AND COALESCE(r.source, 'manual') <> 'learned_cold_start'
      AND e.rule_version = r.rule_version
";

fn preference_decision_signal() -> &'static Arc<Notify> {
    PREFERENCE_DECISION_SIGNAL.get_or_init(|| Arc::new(Notify::new()))
}

pub fn notify_preference_decision_worker() {
    preference_decision_signal().notify_waiters();
}

impl PreferenceDecisionService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn create_rule(
        &self,
        user_id: &str,
        role: &str,
        input: PreferenceRuleInput,
    ) -> Result<PreferenceRule> {
        validate_input(&input, role)?;
        let id = Uuid::new_v4().to_string();
        let owner_role = if role == "admin" { "admin" } else { "user" };
        sqlx::query("INSERT INTO preference_rules (id,user_id,name,rule_version,conditions_json,exceptions_json,action,confidence_threshold,enabled,owner_role) VALUES (?,?,?,?,?,?,?,?,0,?)")
            .bind(&id).bind(user_id).bind(&input.name).bind(&input.rule_version)
            .bind(serde_json::to_string(&input.conditions)?).bind(serde_json::to_string(&input.exceptions)?).bind(&input.action)
            .bind(input.confidence_threshold).bind(owner_role).execute(&self.pool).await?;
        let rule = self
            .get_rule(user_id, &id)
            .await?
            .ok_or_else(|| anyhow!("created preference rule disappeared"))?;
        notify_preference_decision_worker();
        Ok(rule)
    }

    pub async fn update_rule(
        &self,
        user_id: &str,
        role: &str,
        id: &str,
        input: PreferenceRuleInput,
    ) -> Result<PreferenceRule> {
        validate_input(&input, role)?;
        let result = sqlx::query("UPDATE preference_rules SET name=?, rule_version=?, conditions_json=?, exceptions_json=?, action=?, confidence_threshold=?, updated_at=CURRENT_TIMESTAMP WHERE id=? AND user_id=?")
            .bind(&input.name).bind(&input.rule_version).bind(serde_json::to_string(&input.conditions)?).bind(serde_json::to_string(&input.exceptions)?).bind(&input.action).bind(input.confidence_threshold).bind(id).bind(user_id).execute(&self.pool).await?;
        if result.rows_affected() == 0 {
            return Err(anyhow!("preference rule not found"));
        }
        let rule = self
            .get_rule(user_id, id)
            .await?
            .ok_or_else(|| anyhow!("preference rule not found"))?;
        notify_preference_decision_worker();
        Ok(rule)
    }

    pub async fn set_enabled(
        &self,
        user_id: &str,
        role: &str,
        id: &str,
        enabled: bool,
    ) -> Result<()> {
        let rule = self
            .get_rule(user_id, id)
            .await?
            .ok_or_else(|| anyhow!("preference rule not found"))?;
        if enabled && rule.action == "auto_delete" && role != "admin" && rule.owner_role != "system"
        {
            return Err(anyhow!(
                "only administrators may enable automatic deletion rules"
            ));
        }
        sqlx::query("UPDATE preference_rules SET enabled=?, auto_paused=0, updated_at=CURRENT_TIMESTAMP WHERE id=? AND user_id=?").bind(enabled).bind(id).bind(user_id).execute(&self.pool).await?;
        notify_preference_decision_worker();
        Ok(())
    }

    pub async fn list_rules(&self, user_id: &str) -> Result<Vec<PreferenceRule>> {
        let rows = sqlx::query("SELECT id,user_id,name,rule_version,conditions_json,exceptions_json,action,confidence_threshold,enabled,owner_role,false_positive_count,auto_paused FROM preference_rules WHERE user_id=? OR owner_role='system' ORDER BY created_at DESC").bind(user_id).fetch_all(&self.pool).await?;
        rows.into_iter().map(rule_from_row).collect()
    }

    pub async fn get_rule(&self, user_id: &str, id: &str) -> Result<Option<PreferenceRule>> {
        sqlx::query("SELECT id,user_id,name,rule_version,conditions_json,exceptions_json,action,confidence_threshold,enabled,owner_role,false_positive_count,auto_paused FROM preference_rules WHERE id=? AND (user_id=? OR owner_role='system')").bind(id).bind(user_id).fetch_optional(&self.pool).await?.map(rule_from_row).transpose()
    }

    pub async fn evaluate_archive(
        &self,
        user_id: &str,
        archive_id: &str,
    ) -> Result<Vec<PreferenceRuleEvaluation>> {
        let row = sqlx::query("SELECT analysis.id, analysis.result_json FROM content_analyses analysis JOIN archives archive ON archive.id = analysis.archive_id WHERE analysis.archive_id=? AND analysis.content_fingerprint = archive.file_hash AND analysis.status='completed' ORDER BY analysis.created_at DESC LIMIT 1").bind(archive_id).fetch_optional(&self.pool).await?.ok_or_else(|| anyhow!("completed content analysis not found"))?;
        let analysis_id: String = row.get("id");
        let result: ContentAnalysisResult =
            serde_json::from_str(row.get::<String, _>("result_json").as_str())
                .context("invalid stored content analysis")?;
        let rules = self.list_rules(user_id).await?;
        let mut output = Vec::new();
        for rule in rules.into_iter().filter(|r| r.enabled && !r.auto_paused) {
            let (matched, evidence, detail, confidence) =
                evaluate_condition(&rule.conditions, &result)?;
            let excepted = !rule.exceptions.is_null()
                && rule.exceptions != json!({})
                && evaluate_condition(&rule.exceptions, &result)?.0;
            let eligible = matched
                && !excepted
                && (confidence as f64) >= rule.confidence_threshold
                && (rule.action != "auto_delete" || !evidence.is_empty());
            let decision = if !eligible {
                "no_match"
            } else {
                rule.action.as_str()
            };
            let key = format!(
                "analysis:{analysis_id}:rule:{}:{}",
                rule.id, rule.rule_version
            );
            let eval_id = Uuid::new_v4().to_string();
            let inserted = sqlx::query("INSERT OR IGNORE INTO preference_rule_evaluations (id,analysis_id,rule_id,rule_version,matched,matched_conditions_json,evidence_pages_json,decision,execution_status) VALUES (?,?,?,?,?,?,?,?,?)")
                .bind(&eval_id).bind(&analysis_id).bind(&rule.id).bind(&rule.rule_version).bind(eligible).bind(serde_json::to_string(&detail)?).bind(serde_json::to_string(&evidence)?).bind(decision).bind(if decision == "auto_delete" { "pending" } else { "recorded" }).execute(&self.pool).await?;
            let actual_id = if inserted.rows_affected() == 1 {
                eval_id
            } else {
                sqlx::query_scalar::<_,String>("SELECT id FROM preference_rule_evaluations WHERE analysis_id=? AND rule_id=? AND rule_version=?").bind(&analysis_id).bind(&rule.id).bind(&rule.rule_version).fetch_one(&self.pool).await?
            };
            let mut status = if decision == "auto_delete" {
                "pending".to_string()
            } else {
                "recorded".to_string()
            };
            let mut error = None;
            if decision == "auto_delete" {
                let restored: Option<i64> = sqlx::query_scalar("SELECT 1 FROM trash_entries WHERE archive_id=? AND rule_version=? AND status='restored' LIMIT 1").bind(archive_id).bind(&rule.rule_version).fetch_optional(&self.pool).await?;
                if restored.is_some() {
                    status = "skipped_correction".into();
                } else {
                    let model_confidence = confidence as f64;
                    let decision_result = AutoDeleteService::new(self.pool.clone())
                        .execute(AutoDeleteDecision {
                            archive_id: archive_id.into(),
                            user_id: user_id.into(),
                            reason: format!("preference rule {} matched", rule.name),
                            rule_version: rule.rule_version.clone(),
                            rule_id: rule.id.clone(),
                            evaluation_id: actual_id.clone(),
                            model_confidence,
                            evidence_pages: evidence.clone(),
                            decision_key: key,
                        })
                        .await;
                    match decision_result {
                        Ok(AutoDeleteResult::Applied | AutoDeleteResult::AlreadyCompleted) => {
                            status = "completed".into()
                        }
                        Err(e) => {
                            status = "retryable".into();
                            error = Some(e.to_string());
                        }
                    }
                }
                sqlx::query("UPDATE preference_rule_evaluations SET execution_status=?, error=?, next_attempt_at=CASE WHEN ? = 'retryable' THEN datetime('now', '+60 seconds') ELSE NULL END, updated_at=CURRENT_TIMESTAMP WHERE id=?").bind(&status).bind(&error).bind(&status).bind(&actual_id).execute(&self.pool).await?;
            }
            output.push(PreferenceRuleEvaluation {
                id: actual_id,
                analysis_id: analysis_id.clone(),
                rule_id: rule.id,
                rule_version: rule.rule_version,
                matched: eligible,
                matched_conditions: detail,
                evidence_pages: evidence,
                decision: decision.into(),
                execution_status: status,
                error,
            });
        }
        Ok(output)
    }

    pub async fn list_evaluations(
        &self,
        user_id: &str,
        archive_id: &str,
    ) -> Result<Vec<PreferenceRuleEvaluation>> {
        let rows = sqlx::query("SELECT e.id,e.analysis_id,e.rule_id,e.rule_version,e.matched,e.matched_conditions_json,e.evidence_pages_json,e.decision,e.execution_status,e.error FROM preference_rule_evaluations e JOIN content_analyses a ON a.id=e.analysis_id JOIN preference_rules r ON r.id=e.rule_id WHERE a.archive_id=? AND (r.user_id=? OR r.owner_role='system') ORDER BY e.created_at DESC").bind(archive_id).bind(user_id).fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|r| {
                Ok(PreferenceRuleEvaluation {
                    id: r.get("id"),
                    analysis_id: r.get("analysis_id"),
                    rule_id: r.get("rule_id"),
                    rule_version: r.get("rule_version"),
                    matched: r.get::<i64, _>("matched") != 0,
                    matched_conditions: serde_json::from_str(
                        r.get::<String, _>("matched_conditions_json").as_str(),
                    )?,
                    evidence_pages: serde_json::from_str(
                        r.get::<String, _>("evidence_pages_json").as_str(),
                    )?,
                    decision: r.get("decision"),
                    execution_status: r.get("execution_status"),
                    error: r.get("error"),
                })
            })
            .collect()
    }

    pub async fn rule_version_stats(
        &self,
        rule_id: &str,
        rule_version: &str,
    ) -> Result<PreferenceRuleVersionStats> {
        let mut windows = Vec::new();
        for days in [7_u16, 30, 90] {
            let interval = format!("-{days} days");
            let row = sqlx::query(
                "WITH evaluations AS (
                    SELECT e.id, e.matched, e.decision, e.execution_status, e.created_at, a.archive_id,
                           EXISTS(SELECT 1 FROM preference_rule_corrections c WHERE c.evaluation_id=e.id) AS restored_correction
                    FROM preference_rule_evaluations e
                    JOIN content_analyses a ON a.id=e.analysis_id
                    WHERE e.rule_id=? AND e.rule_version=? AND e.created_at >= datetime('now', ?)
                 )
                 SELECT
                    COALESCE(SUM(CASE WHEN matched=1 THEN 1 ELSE 0 END), 0) AS matched_count,
                    COUNT(DISTINCT CASE WHEN matched=1 THEN archive_id END) AS unique_archive_count,
                    COALESCE(SUM(CASE WHEN matched=1 AND decision='keep' THEN 1 ELSE 0 END), 0) AS keep_count,
                    COALESCE(SUM(CASE WHEN matched=1 AND decision='downrank' THEN 1 ELSE 0 END), 0) AS downrank_count,
                    COALESCE(SUM(CASE WHEN matched=1 AND decision='auto_delete' THEN 1 ELSE 0 END), 0) AS auto_delete_count,
                    COALESCE(SUM(CASE WHEN matched=1 AND decision='auto_delete' AND execution_status='completed' THEN 1 ELSE 0 END), 0) AS auto_delete_success_count,
                    COALESCE(SUM(CASE WHEN restored_correction=1 THEN 1 ELSE 0 END), 0) AS restore_correction_count,
                    MAX(CASE WHEN matched=1 THEN created_at END) AS last_matched_at
                 FROM evaluations",
            )
            .bind(rule_id)
            .bind(rule_version)
            .bind(interval)
            .fetch_one(&self.pool)
            .await?;
            let auto_delete_success_count: i64 = row.get("auto_delete_success_count");
            let restore_correction_count: i64 = row.get("restore_correction_count");
            windows.push(PreferenceRuleVersionWindowStats {
                days,
                matched_count: row.get("matched_count"),
                unique_archive_count: row.get("unique_archive_count"),
                keep_count: row.get("keep_count"),
                downrank_count: row.get("downrank_count"),
                auto_delete_count: row.get("auto_delete_count"),
                auto_delete_success_count,
                restore_correction_count,
                false_positive_rate: if auto_delete_success_count == 0 {
                    0.0
                } else {
                    restore_correction_count as f64 / auto_delete_success_count as f64
                },
                last_matched_at: row.get("last_matched_at"),
            });
        }
        Ok(PreferenceRuleVersionStats {
            rule_id: rule_id.to_string(),
            rule_version: rule_version.to_string(),
            windows,
        })
    }

    async fn process_completed_once(&self) -> Result<bool> {
        let rows = sqlx::query(&format!(
            "WITH eligible_retryable_evaluations AS ({ELIGIBLE_RETRYABLE_EVALUATIONS_SQL})
             SELECT DISTINCT a.archive_id, r.user_id FROM content_analyses a
             JOIN archives archive ON archive.id = a.archive_id
             JOIN preference_rules r ON r.enabled=1 AND r.auto_paused=0
             AND COALESCE(r.source, 'manual') <> 'learned_cold_start'
             WHERE a.status='completed' AND a.content_fingerprint = archive.file_hash AND NOT EXISTS
             (SELECT 1 FROM preference_rule_evaluations e
              WHERE e.analysis_id=a.id AND e.rule_id=r.id AND e.rule_version=r.rule_version
                AND e.execution_status <> 'retryable')
             AND NOT EXISTS
             (SELECT 1 FROM eligible_retryable_evaluations e
              WHERE e.analysis_id=a.id AND e.rule_id=r.id AND e.rule_version=r.rule_version
                AND e.next_attempt_at > CURRENT_TIMESTAMP)
             LIMIT 20",
        ))
        .fetch_all(&self.pool)
        .await?;
        let mut processed = false;
        for row in rows {
            let archive_id: String = row.get("archive_id");
            let user_id: String = row.get("user_id");
            if let Err(error) = self.evaluate_archive(&user_id, &archive_id).await {
                tracing::warn!(%archive_id, %user_id, %error, "preference rule evaluation failed");
            }
            processed = true;
        }
        Ok(processed)
    }

    async fn next_retry_delay(&self) -> Result<Option<StdDuration>> {
        let seconds: Option<f64> = sqlx::query_scalar(&format!(
            "WITH eligible_retryable_evaluations AS ({ELIGIBLE_RETRYABLE_EVALUATIONS_SQL})
             SELECT MIN(julianday(next_attempt_at) - julianday('now'))
             FROM eligible_retryable_evaluations",
        ))
        .fetch_one(&self.pool)
        .await?;
        Ok(seconds
            .map(|value| StdDuration::from_secs_f64((value.max(0.0) * 86_400.0).min(86_400.0))))
    }
}

pub fn spawn_preference_decision_worker(pool: Pool<Sqlite>) {
    let signal = preference_decision_signal().clone();
    tokio::spawn(async move {
        let service = PreferenceDecisionService::new(pool);
        loop {
            let notified = signal.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            match service.process_completed_once().await {
                Ok(true) => {}
                Ok(false) => match service.next_retry_delay().await {
                    Ok(Some(delay)) if delay.is_zero() => continue,
                    Ok(Some(delay)) => {
                        tokio::select! {
                            _ = notified => {}
                            _ = tokio::time::sleep(delay) => {}
                        }
                    }
                    Ok(None) => notified.await,
                    Err(error) => {
                        tracing::warn!(%error, "preference decision retry timer query failed");
                        tokio::select! {
                            _ = notified => {}
                            _ = tokio::time::sleep(StdDuration::from_secs(30)) => {}
                        }
                    }
                },
                Err(error) => {
                    tracing::warn!(%error, "preference decision worker iteration failed");
                    tokio::select! {
                        _ = notified => {}
                        _ = tokio::time::sleep(StdDuration::from_secs(30)) => {}
                    }
                }
            }
        }
    });
}

fn rule_from_row(r: sqlx::sqlite::SqliteRow) -> Result<PreferenceRule> {
    Ok(PreferenceRule {
        id: r.get("id"),
        user_id: r.get("user_id"),
        name: r.get("name"),
        rule_version: r.get("rule_version"),
        conditions: serde_json::from_str(r.get::<String, _>("conditions_json").as_str())?,
        exceptions: serde_json::from_str(r.get::<String, _>("exceptions_json").as_str())?,
        action: r.get("action"),
        confidence_threshold: r.get("confidence_threshold"),
        enabled: r.get::<i64, _>("enabled") != 0,
        owner_role: r.get("owner_role"),
        false_positive_count: r.get("false_positive_count"),
        auto_paused: r.get::<i64, _>("auto_paused") != 0,
    })
}

fn validate_input(input: &PreferenceRuleInput, role: &str) -> Result<()> {
    if input.name.trim().is_empty()
        || input.rule_version.trim().is_empty()
        || !matches!(input.action.as_str(), "keep" | "downrank" | "auto_delete")
    {
        return Err(anyhow!("invalid preference rule"));
    }
    if !(0.0..=1.0).contains(&input.confidence_threshold) {
        return Err(anyhow!("rule confidence threshold must be between 0 and 1"));
    }
    if input.action == "auto_delete" && role != "admin" {
        return Err(anyhow!(
            "only administrators may create automatic deletion rules"
        ));
    }
    Ok(())
}

fn evaluate_condition(
    condition: &Value,
    result: &ContentAnalysisResult,
) -> Result<(bool, Vec<i32>, Value, f32)> {
    if let Some(all) = condition.get("all").and_then(Value::as_array) {
        let mut pages = Vec::new();
        let mut details = Vec::new();
        let mut confidence: f32 = 1.0;
        for c in all {
            let (ok, p, d, conf) = evaluate_condition(c, result)?;
            if !ok {
                return Ok((false, p, json!({"all":details}), conf));
            }
            pages.extend(p);
            details.push(d);
            confidence = confidence.min(conf);
        }
        pages.sort_unstable();
        pages.dedup();
        return Ok((true, pages, json!({"all":details}), confidence));
    }
    if let Some(any) = condition.get("any").and_then(Value::as_array) {
        for c in any {
            let (ok, p, d, conf) = evaluate_condition(c, result)?;
            if ok {
                return Ok((true, p, json!({"any":[d]}), conf));
            }
        }
        return Ok((false, Vec::new(), json!({"any":[]} ), 0.0));
    }
    if let Some(not) = condition.get("not") {
        let (ok, _, _, _) = evaluate_condition(not, result)?;
        return Ok((!ok, Vec::new(), json!({"not":!ok}), 1.0));
    }
    if let Some(theme) = condition.get("theme").and_then(Value::as_str) {
        let ok = result.themes.iter().any(|t| t.eq_ignore_ascii_case(theme));
        return Ok((
            ok,
            Vec::new(),
            json!({"theme":theme,"matched":ok}),
            if ok { 1.0 } else { 0.0 },
        ));
    }
    if let Some(themes) = condition.get("themes").and_then(Value::as_array) {
        let ok = themes.iter().all(|t| {
            t.as_str()
                .is_some_and(|x| result.themes.iter().any(|v| v.eq_ignore_ascii_case(x)))
        });
        return Ok((
            ok,
            Vec::new(),
            json!({"themes":themes,"matched":ok}),
            if ok { 1.0 } else { 0.0 },
        ));
    }
    if let Some(concept) = condition.get("concept").and_then(Value::as_str) {
        let ok = result
            .themes
            .iter()
            .any(|theme| theme.eq_ignore_ascii_case(concept));
        return Ok((
            ok,
            Vec::new(),
            json!({"theme":concept,"matched":ok}),
            if ok { 1.0 } else { 0.0 },
        ));
    }
    Err(anyhow!("unsupported preference condition"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ContentAnalysisResult;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn retry_timer_test_pool() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("SQLite test database should connect");
        for statement in [
            "CREATE TABLE archives (id TEXT PRIMARY KEY, file_hash TEXT NOT NULL)",
            "CREATE TABLE content_analyses (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, content_fingerprint TEXT NOT NULL, status TEXT NOT NULL)",
            "CREATE TABLE preference_rules (id TEXT PRIMARY KEY, rule_version TEXT NOT NULL, enabled INTEGER NOT NULL, auto_paused INTEGER NOT NULL, source TEXT)",
            "CREATE TABLE preference_rule_evaluations (id TEXT PRIMARY KEY, analysis_id TEXT NOT NULL, rule_id TEXT NOT NULL, rule_version TEXT NOT NULL, execution_status TEXT NOT NULL, next_attempt_at DATETIME)",
        ] {
            sqlx::query(statement)
                .execute(&pool)
                .await
                .expect("retry timer test schema should be created");
        }
        pool
    }

    #[tokio::test]
    async fn retry_timer_ignores_ineligible_evaluations() {
        let pool = retry_timer_test_pool().await;
        sqlx::query(
            "INSERT INTO archives (id, file_hash) VALUES
             ('archive-disabled', 'hash-disabled'),
             ('archive-paused', 'hash-paused'),
             ('archive-version', 'hash-version'),
             ('archive-fingerprint', 'hash-fingerprint-current'),
             ('archive-pending', 'hash-pending'),
             ('archive-learned', 'hash-learned')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO content_analyses (id, archive_id, content_fingerprint, status) VALUES
             ('analysis-disabled', 'archive-disabled', 'hash-disabled', 'completed'),
             ('analysis-paused', 'archive-paused', 'hash-paused', 'completed'),
             ('analysis-version', 'archive-version', 'hash-version', 'completed'),
             ('analysis-fingerprint', 'archive-fingerprint', 'hash-fingerprint-old', 'completed'),
             ('analysis-pending', 'archive-pending', 'hash-pending', 'processing'),
             ('analysis-learned', 'archive-learned', 'hash-learned', 'completed')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO preference_rules (id, rule_version, enabled, auto_paused, source) VALUES
             ('rule-disabled', 'v1', 0, 0, 'manual'),
             ('rule-paused', 'v1', 1, 1, 'manual'),
             ('rule-version', 'v2', 1, 0, 'manual'),
             ('rule-fingerprint', 'v1', 1, 0, 'manual'),
             ('rule-pending', 'v1', 1, 0, 'manual'),
             ('rule-learned', 'v1', 1, 0, 'learned_cold_start')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO preference_rule_evaluations
             (id, analysis_id, rule_id, rule_version, execution_status, next_attempt_at) VALUES
             ('eval-disabled', 'analysis-disabled', 'rule-disabled', 'v1', 'retryable', datetime('now', '-1 minute')),
             ('eval-paused', 'analysis-paused', 'rule-paused', 'v1', 'retryable', datetime('now', '-1 minute')),
             ('eval-version', 'analysis-version', 'rule-version', 'v1', 'retryable', datetime('now', '-1 minute')),
             ('eval-fingerprint', 'analysis-fingerprint', 'rule-fingerprint', 'v1', 'retryable', datetime('now', '-1 minute')),
             ('eval-pending', 'analysis-pending', 'rule-pending', 'v1', 'retryable', datetime('now', '-1 minute')),
             ('eval-learned', 'analysis-learned', 'rule-learned', 'v1', 'retryable', datetime('now', '-1 minute'))",
        )
        .execute(&pool)
        .await
        .unwrap();

        let service = PreferenceDecisionService::new(pool);
        assert!(service.next_retry_delay().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn retry_timer_keeps_future_and_due_eligible_evaluations() {
        let pool = retry_timer_test_pool().await;
        sqlx::query(
            "INSERT INTO archives (id, file_hash) VALUES ('archive-eligible', 'hash-eligible')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO content_analyses (id, archive_id, content_fingerprint, status)
             VALUES ('analysis-eligible', 'archive-eligible', 'hash-eligible', 'completed')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO preference_rules (id, rule_version, enabled, auto_paused, source)
             VALUES ('rule-eligible', 'v1', 1, 0, 'manual')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO preference_rule_evaluations
             (id, analysis_id, rule_id, rule_version, execution_status, next_attempt_at)
             VALUES ('eval-eligible', 'analysis-eligible', 'rule-eligible', 'v1', 'retryable', datetime('now', '+1 hour'))",
        )
        .execute(&pool)
        .await
        .unwrap();

        let service = PreferenceDecisionService::new(pool.clone());
        let future_delay = service
            .next_retry_delay()
            .await
            .unwrap()
            .expect("future eligible retry should be scheduled");
        assert!(future_delay > StdDuration::from_secs(3_000));
        assert!(future_delay <= StdDuration::from_secs(3_600));

        sqlx::query(
            "UPDATE preference_rule_evaluations
             SET next_attempt_at = datetime('now', '-1 minute')
             WHERE id = 'eval-eligible'",
        )
        .execute(&pool)
        .await
        .unwrap();
        let due_delay = service.next_retry_delay().await.unwrap();
        assert_eq!(due_delay, Some(StdDuration::ZERO));
    }

    #[test]
    fn evaluates_combinations() {
        let r = ContentAnalysisResult {
            themes: vec!["drama".into(), "betrayal".into()],
            selected_tags: vec![],
        };
        let (ok, p, _, c) =
            evaluate_condition(&json!({"all":[{"theme":"drama"},{"theme":"betrayal"}]}), &r)
                .unwrap();
        assert!(ok);
        assert!(p.is_empty());
        assert_eq!(c, 1.0);
    }
}
