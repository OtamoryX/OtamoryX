use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::{Pool, Row, Sqlite};
use std::collections::BTreeSet;
use std::time::Instant;
use uuid::Uuid;

use crate::models::{
    ContentAnalysisEvidence, ContentAnalysisResponse, ContentAnalysisResult, ModelContentAnalysis,
};
use crate::services::{load_ai_settings, run_chat_completion};
use crate::utils::extractor::ArchiveExtractor;

pub const CONTENT_ANALYSIS_PROMPT_VERSION: &str = "content-v1";
const MAX_SAMPLE_PAGES: usize = 20;
const MAX_RETRIES: i32 = 5;

#[derive(Debug, Clone)]
struct ClaimedAnalysis {
    id: String,
    archive_id: String,
    fingerprint: String,
    attempts: i32,
}

/// Deterministic cover/opening/middle/ending sampling. Page numbers are 1-based for the API.
pub fn sample_pages(page_count: i32) -> Vec<i32> {
    if page_count <= 0 {
        return Vec::new();
    }
    let target = (page_count as usize)
        .clamp(8, MAX_SAMPLE_PAGES)
        .min(page_count as usize);
    if target == 1 {
        return vec![1];
    }
    let mut pages = BTreeSet::new();
    pages.insert(1);
    pages.insert(page_count);
    pages.insert((page_count + 1) / 2);
    pages.insert((page_count + 2) / 3);
    pages.insert(((page_count * 2) + 2) / 3);
    for i in 0..target {
        pages.insert(1 + ((page_count - 1) as usize * i / (target - 1)) as i32);
    }
    let required = [
        1,
        page_count,
        (page_count + 1) / 2,
        (page_count + 2) / 3,
        ((page_count * 2) + 2) / 3,
    ];
    while pages.len() > target {
        if let Some(candidate) = pages.iter().copied().find(|page| !required.contains(page)) {
            pages.remove(&candidate);
        } else {
            break;
        }
    }
    pages.into_iter().collect()
}

pub fn page_role(page: i32, page_count: i32) -> &'static str {
    if page == 1 {
        "cover"
    } else if page <= (page_count + 2) / 3 {
        "opening"
    } else if page >= ((page_count * 2) + 2) / 3 {
        "ending"
    } else {
        "middle"
    }
}

pub fn parse_model_result(
    raw: &str,
    sampled_pages: &[i32],
) -> Result<(ContentAnalysisResult, Vec<ContentAnalysisEvidence>)> {
    let model: ModelContentAnalysis =
        serde_json::from_str(raw).map_err(|e| anyhow!("invalid content analysis JSON: {e}"))?;
    if model.themes.iter().any(|v| v.trim().is_empty())
        || model.concepts.is_empty()
        || model.evidence.is_empty()
    {
        return Err(anyhow!(
            "content analysis response is missing themes, concepts, or evidence"
        ));
    }
    let allowed: BTreeSet<i32> = sampled_pages.iter().copied().collect();
    let mut evidence = Vec::with_capacity(model.evidence.len());
    for item in model.evidence {
        if !allowed.contains(&item.page)
            || item.summary.trim().is_empty()
            || item.concepts.is_empty()
        {
            return Err(anyhow!(
                "content analysis evidence references an invalid page"
            ));
        }
        if item.confidence.is_some_and(|v| !(0.0..=1.0).contains(&v)) {
            return Err(anyhow!(
                "content analysis evidence confidence is out of range"
            ));
        }
        evidence.push(ContentAnalysisEvidence {
            page_number: item.page,
            page_role: item.role,
            concepts: item.concepts,
            confidence: item.confidence,
            summary: item.summary,
        });
    }
    let concepts = model
        .concepts
        .into_iter()
        .map(|item| {
            if item.name.trim().is_empty()
                || !(0.0..=1.0).contains(&item.confidence)
                || item.evidence_pages.is_empty()
                || item.evidence_pages.iter().any(|p| !allowed.contains(p))
            {
                return Err(anyhow!(
                    "content analysis concept has invalid confidence or evidence"
                ));
            }
            Ok(crate::models::ContentConcept {
                name: item.name,
                confidence: item.confidence,
                evidence_pages: item.evidence_pages,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((
        ContentAnalysisResult {
            themes: model.themes,
            concepts,
        },
        evidence,
    ))
}

pub struct ContentAnalysisService {
    pool: Pool<Sqlite>,
}

impl ContentAnalysisService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn enqueue_for_archive(&self, archive_id: &str) -> Result<bool> {
        let row = sqlx::query("SELECT file_hash FROM archives WHERE id = ?")
            .bind(archive_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow!("archive `{archive_id}` not found"))?;
        let fingerprint: String = row.get("file_hash");
        let inserted = sqlx::query("INSERT OR IGNORE INTO content_analyses (id, archive_id, content_fingerprint, prompt_version, status) VALUES (?, ?, ?, ?, 'pending')")
            .bind(Uuid::new_v4().to_string()).bind(archive_id).bind(fingerprint).bind(CONTENT_ANALYSIS_PROMPT_VERSION).execute(&self.pool).await?;
        Ok(inserted.rows_affected() == 1)
    }

    pub async fn process_next(&self) -> Result<bool> {
        self.release_expired().await?;
        let Some(job) = self.claim_next().await? else {
            return Ok(false);
        };
        let started = Instant::now();
        let settings = load_ai_settings(&self.pool).await?;
        let outcome = self.analyze(&settings, &job).await;
        match outcome {
            Ok((result, evidence)) => {
                self.complete(
                    &job,
                    &settings.connection.provider,
                    &settings.connection.model,
                    result,
                    evidence,
                )
                .await?
            }
            Err(error) => self.fail(&job, &error.to_string()).await?,
        }
        tracing::info!(analysis_id=%job.id, archive_id=%job.archive_id, elapsed_ms=started.elapsed().as_millis(), "content analysis job finished");
        Ok(true)
    }

    async fn analyze(
        &self,
        settings: &crate::models::AISettings,
        job: &ClaimedAnalysis,
    ) -> Result<(ContentAnalysisResult, Vec<ContentAnalysisEvidence>)> {
        let row = sqlx::query("SELECT path, page_count FROM archives WHERE id = ?")
            .bind(&job.archive_id)
            .fetch_one(&self.pool)
            .await?;
        let path: String = row.get("path");
        let count: i32 = row.get("page_count");
        let pages = sample_pages(count);
        let mut page_info = Vec::new();
        for page in &pages {
            let name = ArchiveExtractor::extract_single_page(&path, (*page - 1) as usize)
                .map(|p| p.name)
                .unwrap_or_else(|_| format!("page-{page}"));
            page_info.push(json!({"page":page,"role":page_role(*page,count),"file":name}));
        }
        let prompt = format!("Archive fingerprint: {}\nSampled pages: {}\nReturn JSON with themes, concepts (name, confidence 0..1, evidencePages), and evidence (page, role, concepts, confidence, summary). Every concept must cite sampled pages.", job.fingerprint, serde_json::to_string(&page_info)?);
        let raw = run_chat_completion(settings, "You analyze comic content. Do not make deletion decisions. Return only the requested JSON.", &prompt, 1800).await?;
        parse_model_result(&raw, &pages)
    }

    async fn claim_next(&self) -> Result<Option<ClaimedAnalysis>> {
        let row = sqlx::query("SELECT id, archive_id, content_fingerprint, attempts FROM content_analyses WHERE status IN ('pending','retryable') ORDER BY updated_at ASC LIMIT 1").fetch_optional(&self.pool).await?;
        let Some(row) = row else { return Ok(None) };
        let job = ClaimedAnalysis {
            id: row.get("id"),
            archive_id: row.get("archive_id"),
            fingerprint: row.get("content_fingerprint"),
            attempts: row.get("attempts"),
        };
        let lease = Utc::now() + Duration::minutes(10);
        let updated = sqlx::query("UPDATE content_analyses SET status='running', attempts=attempts+1, started_at=?, lease_expires_at=?, updated_at=? WHERE id=? AND status IN ('pending','retryable')")
            .bind(Utc::now()).bind(lease).bind(Utc::now()).bind(&job.id).execute(&self.pool).await?;
        Ok((updated.rows_affected() == 1).then_some(job))
    }

    async fn release_expired(&self) -> Result<()> {
        sqlx::query("UPDATE content_analyses SET status='retryable', lease_expires_at=NULL, updated_at=? WHERE status='running' AND lease_expires_at < ?").bind(Utc::now()).bind(Utc::now()).execute(&self.pool).await?;
        Ok(())
    }
    async fn complete(
        &self,
        job: &ClaimedAnalysis,
        provider: &str,
        model: &str,
        result: ContentAnalysisResult,
        evidence: Vec<ContentAnalysisEvidence>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();
        sqlx::query("UPDATE content_analyses SET status='completed', provider=?, model=?, result_json=?, completed_at=?, updated_at=?, lease_expires_at=NULL, last_error=NULL WHERE id=?")
            .bind(provider).bind(model).bind(serde_json::to_string(&result)?).bind(now).bind(now).bind(&job.id).execute(&mut *tx).await?;
        for item in evidence {
            sqlx::query("INSERT INTO content_analysis_evidence (id, analysis_id, page_number, page_role, concepts_json, confidence, summary) VALUES (?, ?, ?, ?, ?, ?, ?)").bind(Uuid::new_v4().to_string()).bind(&job.id).bind(item.page_number).bind(item.page_role).bind(serde_json::to_string(&item.concepts)?).bind(item.confidence).bind(item.summary).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
    async fn fail(&self, job: &ClaimedAnalysis, error: &str) -> Result<()> {
        let status = if job.attempts + 1 >= MAX_RETRIES {
            "failed"
        } else {
            "retryable"
        };
        let delay = 2_i64.pow((job.attempts as u32).min(5));
        sqlx::query("UPDATE content_analyses SET status=?, last_error=?, updated_at=?, lease_expires_at=NULL, completed_at=CASE WHEN ?='failed' THEN ? ELSE completed_at END WHERE id=?")
            .bind(status).bind(error).bind(Utc::now() + Duration::seconds(delay)).bind(status).bind(Utc::now()).bind(&job.id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get(&self, archive_id: &str) -> Result<Option<ContentAnalysisResponse>> {
        let row = sqlx::query("SELECT id, archive_id, content_fingerprint, status, provider, model, prompt_version, result_json, attempts, last_error FROM content_analyses WHERE archive_id=? ORDER BY created_at DESC LIMIT 1").bind(archive_id).fetch_optional(&self.pool).await?;
        let Some(row) = row else { return Ok(None) };
        let id: String = row.get("id");
        let evidence_rows = sqlx::query("SELECT page_number, page_role, concepts_json, confidence, summary FROM content_analysis_evidence WHERE analysis_id=? ORDER BY page_number").bind(&id).fetch_all(&self.pool).await?;
        let evidence = evidence_rows
            .into_iter()
            .map(|r| {
                Ok(ContentAnalysisEvidence {
                    page_number: r.get("page_number"),
                    page_role: r.get("page_role"),
                    concepts: serde_json::from_str(r.get::<String, _>("concepts_json").as_str())?,
                    confidence: r.get("confidence"),
                    summary: r.get("summary"),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Some(ContentAnalysisResponse {
            id,
            archive_id: row.get("archive_id"),
            content_fingerprint: row.get("content_fingerprint"),
            status: row.get("status"),
            provider: row.get("provider"),
            model: row.get("model"),
            prompt_version: row.get("prompt_version"),
            result: row
                .try_get::<Option<String>, _>("result_json")?
                .and_then(|v| serde_json::from_str(&v).ok()),
            attempts: row.get("attempts"),
            last_error: row.get("last_error"),
            evidence,
        }))
    }
}

pub fn spawn_content_analysis_worker(pool: Pool<Sqlite>) {
    tokio::spawn(async move {
        let service = ContentAnalysisService::new(pool);
        loop {
            match service.process_next().await {
                Ok(true) => {}
                Ok(false) => tokio::time::sleep(std::time::Duration::from_secs(3)).await,
                Err(err) => {
                    tracing::warn!(error=%err, "content analysis worker iteration failed");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn samples_are_stable_and_unique() {
        let a = sample_pages(100);
        assert_eq!(a, sample_pages(100));
        assert_eq!(a.len(), 20);
        assert_eq!(a.iter().collect::<BTreeSet<_>>().len(), 20);
        assert_eq!(a[0], 1);
        assert_eq!(*a.last().unwrap(), 100);
    }
    #[test]
    fn short_samples_shrink() {
        assert_eq!(sample_pages(3), vec![1, 2, 3]);
        assert_eq!(sample_pages(7).len(), 7);
    }
    #[test]
    fn invalid_model_response_rejected() {
        assert!(parse_model_result("{}", &[1, 2]).is_err());
    }
}
