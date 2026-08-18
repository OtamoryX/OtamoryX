use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, ImageReader, Limits};
use serde_json::json;
use sqlx::{Pool, Row, Sqlite};
use std::collections::BTreeSet;
use std::io::{BufReader, Cursor};
use std::time::Instant;
use uuid::Uuid;

use crate::models::{
    ContentAnalysisEvidence, ContentAnalysisResponse, ContentAnalysisResult, ModelContentAnalysis,
};
use crate::services::{load_ai_settings, ocr_manager, run_vision_chat_completion, VisionImage};
use crate::utils::extractor::ArchiveExtractor;

pub const CONTENT_ANALYSIS_PROMPT_VERSION: &str = "content-v1";
const MAX_SAMPLE_PAGES: usize = 20;
const MAX_RETRIES: i32 = 5;
const MAX_DECODED_PAGE_DIMENSION: u32 = 10_000;
const MAX_DECODED_PAGE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_VISION_PAGE_DIMENSION: u32 = 1_280;
const MAX_VISION_PAGE_BYTES: usize = 512 * 1024;
const VISION_JPEG_QUALITY: u8 = 80;
const FALLBACK_VISION_JPEG_QUALITY: u8 = 68;
const FALLBACK_VISION_PAGE_DIMENSION: u32 = 960;

#[derive(Debug, Clone)]
struct ClaimedAnalysis {
    id: String,
    archive_id: String,
    fingerprint: String,
    attempts: i32,
}

#[derive(Debug)]
struct PreparedPage {
    page_number: i32,
    page_role: &'static str,
    image: VisionImage,
}

fn encode_jpeg(image: &image::DynamicImage, quality: u8) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    JpegEncoder::new_with_quality(&mut data, quality)
        .encode_image(&image.to_rgb8())
        .map_err(|error| anyhow!("failed to encode normalized page as JPEG: {error}"))?;
    Ok(data)
}

fn prepare_page_image(data: &[u8]) -> Result<VisionImage> {
    let mut reader = ImageReader::new(BufReader::new(Cursor::new(data)));
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DECODED_PAGE_DIMENSION);
    limits.max_image_height = Some(MAX_DECODED_PAGE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODED_PAGE_BYTES);
    reader.limits(limits);
    let decoded = reader
        .with_guessed_format()
        .map_err(|error| anyhow!("failed to identify page image format: {error}"))?
        .decode()
        .map_err(|error| anyhow!("failed to decode page image: {error}"))?;
    let normalized = decoded.resize(
        MAX_VISION_PAGE_DIMENSION,
        MAX_VISION_PAGE_DIMENSION,
        FilterType::Lanczos3,
    );
    let mut encoded = encode_jpeg(&normalized, VISION_JPEG_QUALITY)?;
    if encoded.len() > MAX_VISION_PAGE_BYTES {
        let fallback = normalized.resize(
            FALLBACK_VISION_PAGE_DIMENSION,
            FALLBACK_VISION_PAGE_DIMENSION,
            FilterType::Lanczos3,
        );
        encoded = encode_jpeg(&fallback, FALLBACK_VISION_JPEG_QUALITY)?;
    }
    if encoded.len() > MAX_VISION_PAGE_BYTES {
        return Err(anyhow!(
            "normalized page image exceeds {} KiB",
            MAX_VISION_PAGE_BYTES / 1024
        ));
    }
    Ok(VisionImage::jpeg(encoded))
}

fn prepare_pages(path: &str, page_count: i32, pages: &[i32]) -> Result<Vec<PreparedPage>> {
    pages
        .iter()
        .map(|page| {
            let extracted = ArchiveExtractor::extract_single_page(path, (*page - 1) as usize)
                .map_err(|error| anyhow!("failed to extract page {page}: {error}"))?;
            let image = prepare_page_image(&extracted.data)
                .map_err(|error| anyhow!("failed to prepare page {page}: {error}"))?;
            Ok(PreparedPage {
                page_number: *page,
                page_role: page_role(*page, page_count),
                image,
            })
        })
        .collect()
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
        let page_path = path.clone();
        let pages_for_extraction = pages.clone();
        let prepared_pages = tokio::task::spawn_blocking(move || {
            prepare_pages(&page_path, count, &pages_for_extraction)
        })
        .await
        .map_err(|error| anyhow!("content analysis page preparation task failed: {error}"))??;
        let page_info = prepared_pages
            .iter()
            .map(|page| json!({"page": page.page_number, "role": page.page_role}))
            .collect::<Vec<_>>();
        let manager = ocr_manager();
        let ocr_context = manager.prepare_analysis(&self.pool).await?;
        let mut ocr_info = Vec::new();
        if ocr_context.is_some() {
            for page in &prepared_pages {
                match manager
                    .recognize_page(&self.pool, page.image.data().to_vec())
                    .await
                {
                    Ok(Some(text)) if !text.trim().is_empty() => {
                        ocr_info.push(json!({
                            "page": page.page_number,
                            "text": text,
                        }));
                    }
                    Ok(_) => {}
                    Err(error) if error.to_string().contains("model changed") => {
                        return Err(error);
                    }
                    Err(error) => {
                        tracing::warn!(
                            archive_id=%job.archive_id,
                            page=page.page_number,
                            %error,
                            "OCR failed; continuing with vision analysis"
                        );
                    }
                }
            }
            if let Some((model_id, generation)) = ocr_context {
                manager
                    .validate_analysis_generation(&self.pool, &model_id, generation)
                    .await?;
            }
        }
        let images = prepared_pages
            .into_iter()
            .map(|page| page.image)
            .collect::<Vec<_>>();
        let prompt = format!("Archive fingerprint: {}\nThe following sampled page descriptors correspond to the attached images in exactly the same order: {}\nOCR text extracted from the same pages (may be empty or imperfect): {}\nAnalyze the actual pixels and visible text in these comic pages. Use OCR only as auxiliary evidence; the attached images are authoritative. Return JSON with themes, concepts (name, confidence 0..1, evidencePages), and evidence (page, role, concepts, confidence, summary). Every concept must cite sampled pages.", job.fingerprint, serde_json::to_string(&page_info)?, serde_json::to_string(&ocr_info)?);
        let raw = run_vision_chat_completion(settings, "You analyze comic content. Do not make deletion decisions. Return only the requested JSON.", &prompt, &images, 1800).await?;
        parse_model_result(&raw, &pages)
    }

    async fn claim_next(&self) -> Result<Option<ClaimedAnalysis>> {
        let row = sqlx::query("SELECT id, archive_id, content_fingerprint, attempts FROM content_analyses WHERE status IN ('pending','retryable') AND (next_attempt_at IS NULL OR next_attempt_at <= CURRENT_TIMESTAMP) ORDER BY updated_at ASC LIMIT 1").fetch_optional(&self.pool).await?;
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
        sqlx::query("UPDATE content_analyses SET status='retryable', lease_expires_at=NULL, next_attempt_at=NULL, updated_at=? WHERE status='running' AND lease_expires_at < ?").bind(Utc::now()).bind(Utc::now()).execute(&self.pool).await?;
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
        sqlx::query("UPDATE content_analyses SET status='completed', provider=?, model=?, result_json=?, completed_at=?, updated_at=?, lease_expires_at=NULL, next_attempt_at=NULL, last_error=NULL WHERE id=?")
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
        sqlx::query("UPDATE content_analyses SET status=?, last_error=?, updated_at=?, next_attempt_at=?, lease_expires_at=NULL, completed_at=CASE WHEN ?='failed' THEN ? ELSE completed_at END WHERE id=?")
            .bind(status).bind(error).bind(Utc::now()).bind(Utc::now() + Duration::seconds(delay)).bind(status).bind(Utc::now()).bind(&job.id).execute(&self.pool).await?;
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

    #[test]
    fn page_images_are_decoded_scaled_and_normalized_for_vision() {
        let mut source = Vec::new();
        image::DynamicImage::new_rgb8(2_000, 1_000)
            .write_to(&mut Cursor::new(&mut source), image::ImageFormat::Png)
            .unwrap();

        let prepared = prepare_page_image(&source).unwrap();
        let decoded = image::load_from_memory(prepared.data()).unwrap();
        assert_eq!(prepared.media_type(), "image/jpeg");
        assert_eq!((decoded.width(), decoded.height()), (1_280, 640));
        assert!(prepared.data().len() <= MAX_VISION_PAGE_BYTES);
    }

    #[test]
    fn invalid_page_images_are_rejected_instead_of_becoming_filename_only_input() {
        assert!(prepare_page_image(b"not an image").is_err());
    }

    #[tokio::test]
    async fn retry_backoff_blocks_claim_until_due() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE content_analyses (id TEXT PRIMARY KEY, archive_id TEXT, content_fingerprint TEXT, status TEXT, attempts INTEGER, updated_at DATETIME, next_attempt_at DATETIME, lease_expires_at DATETIME, started_at DATETIME, last_error TEXT, completed_at DATETIME)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO content_analyses (id,archive_id,content_fingerprint,status,attempts,updated_at,next_attempt_at) VALUES ('a','archive','hash','retryable',1,CURRENT_TIMESTAMP,datetime('now','+1 hour'))")
            .execute(&pool)
            .await
            .unwrap();
        let service = ContentAnalysisService::new(pool.clone());
        assert!(service.claim_next().await.unwrap().is_none());
        sqlx::query("UPDATE content_analyses SET next_attempt_at=datetime('now','-1 second')")
            .execute(&pool)
            .await
            .unwrap();
        assert!(service.claim_next().await.unwrap().is_some());
    }
}
