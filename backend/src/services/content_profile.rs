//! Deterministic content measurements used by the incremental preference learner.
//!
//! This module intentionally does not classify semantic subjects. It measures
//! properties that can be recovered from the archive bytes and keeps original
//! tag values as data features without maintaining a vocabulary in code.

use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Utc};
use image::{imageops::FilterType, GenericImageView};
use serde_json::{json, Value};
use sqlx::{Pool, Row, Sqlite};
use std::collections::BTreeSet;
use std::path::Path;
use uuid::Uuid;

use crate::models::{ArchiveContentProfileDocument, ContentProfileFeature};
use crate::utils::ArchiveExtractor;

pub const CONTENT_PROFILE_VERSION: &str = "profile-v1";

const MAX_SAMPLE_PAGES: usize = 64;
const LOW_RESOLUTION: u32 = 32;
const MIN_SECTION_BOUNDARY_DISTANCE: f64 = 0.20;
const SECTION_BOUNDARY_MARGIN: f64 = 0.05;
const MAX_RETRIES: i64 = 4;

#[derive(Clone)]
pub struct ContentProfileService {
    pool: Pool<Sqlite>,
}

#[derive(Debug, Clone)]
struct PageMeasurement {
    page_index: usize,
    luma: Vec<f64>,
    structure: Vec<bool>,
    color_fraction: f64,
    gray_fraction: f64,
    chroma: f64,
    aspect_ratio: f64,
}

#[derive(Debug, Clone)]
struct ProfileBuildResult {
    document: ArchiveContentProfileDocument,
    status: &'static str,
    method: Value,
}

impl ContentProfileService {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Queue a profile for an archive that has just entered the library.
    pub async fn enqueue_for_new_archive(&self, archive_id: &str) -> Result<bool> {
        self.enqueue_for_archive(archive_id, "new_archive").await
    }

    /// Queue an existing archive only when a real user/recommendation event has
    /// touched it. This is the cold-start boundary: there is no library scan.
    pub async fn enqueue_for_trigger(&self, archive_id: &str, trigger: &str) -> Result<bool> {
        if !matches!(
            trigger,
            "recommendation"
                | "visible"
                | "open"
                | "page_turn"
                | "exit"
                | "continue_reading"
                | "repeat_open"
                | "manual_delete"
                | "restore"
        ) {
            return Ok(false);
        }
        self.enqueue_for_archive(archive_id, trigger).await
    }

    pub async fn enqueue_for_archives(
        &self,
        archive_ids: Vec<String>,
        trigger: &str,
    ) -> Result<u64> {
        let mut queued = 0;
        for archive_id in archive_ids {
            if self.enqueue_for_trigger(&archive_id, trigger).await? {
                queued += 1;
            }
        }
        Ok(queued)
    }

    async fn enqueue_for_archive(&self, archive_id: &str, trigger: &str) -> Result<bool> {
        let archive = sqlx::query("SELECT file_hash, page_count FROM archives WHERE id = ?")
            .bind(archive_id)
            .fetch_optional(&self.pool)
            .await?;
        let (fingerprint, expected_page_count) = if let Some(row) = archive {
            (
                row.get::<String, _>("file_hash"),
                row.get::<i32, _>("page_count"),
            )
        } else {
            // Manual deletion records feedback after moving the archive. Keep
            // the cold-start profile job usable while its trash snapshot is
            // still recoverable.
            let row = sqlx::query(
                "SELECT metadata_json FROM trash_entries
                 WHERE archive_id = ? AND status = 'active' AND trash_path IS NOT NULL
                 ORDER BY deleted_at DESC LIMIT 1",
            )
            .bind(archive_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow!("archive `{archive_id}` not found"))?;
            let snapshot: Value =
                serde_json::from_str(row.get::<String, _>("metadata_json").as_str())
                    .context("invalid trash snapshot while queuing content profile")?;
            let fingerprint = snapshot
                .get("file_hash")
                .or_else(|| snapshot.get("fileHash"))
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow!("trash snapshot has no content fingerprint"))?
                .to_string();
            let page_count = snapshot
                .get("page_count")
                .or_else(|| snapshot.get("pageCount"))
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .clamp(0, i64::from(i32::MAX)) as i32;
            (fingerprint, page_count)
        };

        let complete: Option<(String, f64)> = sqlx::query_as(
            "SELECT status, coverage FROM archive_content_profiles
             WHERE archive_id = ? AND content_fingerprint = ? AND profile_version = ?
             ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(archive_id)
        .bind(&fingerprint)
        .bind(CONTENT_PROFILE_VERSION)
        .fetch_optional(&self.pool)
        .await?;
        if complete.is_some_and(|(status, coverage)| {
            matches!(status.as_str(), "completed" | "partial") && coverage >= 0.60
        }) {
            return Ok(false);
        }

        sqlx::query(
            "INSERT OR IGNORE INTO archive_content_profiles
             (id, archive_id, content_fingerprint, profile_version, status,
              expected_page_count, created_at, updated_at)
             VALUES (?, ?, ?, ?, 'pending', ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(archive_id)
        .bind(&fingerprint)
        .bind(CONTENT_PROFILE_VERSION)
        .bind(expected_page_count)
        .execute(&self.pool)
        .await?;

        let inserted = sqlx::query(
            "INSERT OR IGNORE INTO content_profile_jobs
             (id, archive_id, content_fingerprint, profile_version, trigger_source,
              status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, 'pending', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(archive_id)
        .bind(&fingerprint)
        .bind(CONTENT_PROFILE_VERSION)
        .bind(trigger)
        .execute(&self.pool)
        .await?;
        Ok(inserted.rows_affected() == 1)
    }

    pub async fn process_next(&self) -> Result<bool> {
        self.release_expired().await?;
        let Some(row) = sqlx::query(
            "SELECT id, archive_id, content_fingerprint, attempts
             FROM content_profile_jobs
             WHERE status IN ('pending', 'retryable')
               AND (next_attempt_at IS NULL OR next_attempt_at <= CURRENT_TIMESTAMP)
             ORDER BY updated_at ASC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(false);
        };

        let job_id: String = row.get("id");
        let archive_id: String = row.get("archive_id");
        let fingerprint: String = row.get("content_fingerprint");
        let attempts: i64 = row.get("attempts");
        let claimed = sqlx::query(
            "UPDATE content_profile_jobs
             SET status = 'running', attempts = attempts + 1,
                 last_error = NULL, updated_at = CURRENT_TIMESTAMP
             WHERE id = ? AND status IN ('pending', 'retryable')
               AND (next_attempt_at IS NULL OR next_attempt_at <= CURRENT_TIMESTAMP)",
        )
        .bind(&job_id)
        .execute(&self.pool)
        .await?;
        if claimed.rows_affected() != 1 {
            return Ok(true);
        }

        let source = self.profile_source(&archive_id, &fingerprint).await?;
        let result = match source {
            Some((path, expected_page_count)) => {
                let fingerprint_for_task = fingerprint.clone();
                let build = tokio::task::spawn_blocking(move || {
                    build_profile(Path::new(&path), expected_page_count, &fingerprint_for_task)
                })
                .await
                .map_err(|error| anyhow!("content profile task failed: {error}"))?;
                match build {
                    Ok(mut profile) => {
                        append_tag_features(&self.pool, &archive_id, &mut profile.document).await?;
                        self.store_profile(&archive_id, &fingerprint, &profile)
                            .await
                    }
                    Err(error) => Err(error),
                }
            }
            None => Err(anyhow!("archive `{archive_id}` no longer exists")),
        };

        match result {
            Ok(()) => {
                sqlx::query(
                    "UPDATE content_profile_jobs
                     SET status = 'completed', last_error = NULL, next_attempt_at = NULL,
                         updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                )
                .bind(&job_id)
                .execute(&self.pool)
                .await?;
                // A profile may complete after its behavior event was queued.
                // Rebuild only from already-applied cold-start aggregates.
                crate::services::PreferenceLearningService::new(self.pool.clone())
                    .rebuild_for_archive(&archive_id)
                    .await?;
            }
            Err(error) => {
                let status = if attempts + 1 >= MAX_RETRIES {
                    "failed"
                } else {
                    "retryable"
                };
                let delay = 2_i64.pow((attempts as u32).min(5));
                sqlx::query(
                    "UPDATE content_profile_jobs
                     SET status = ?, last_error = ?, next_attempt_at = ?,
                         updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                )
                .bind(status)
                .bind(error.to_string())
                .bind(Utc::now() + Duration::seconds(delay))
                .bind(&job_id)
                .execute(&self.pool)
                .await?;
                sqlx::query(
                    "UPDATE archive_content_profiles
                     SET status = ?, last_error = ?, updated_at = CURRENT_TIMESTAMP
                     WHERE archive_id = ? AND content_fingerprint = ? AND profile_version = ?",
                )
                .bind(status)
                .bind(error.to_string())
                .bind(&archive_id)
                .bind(&fingerprint)
                .bind(CONTENT_PROFILE_VERSION)
                .execute(&self.pool)
                .await?;
                tracing::warn!(%archive_id, %error, "content profile job failed");
            }
        }
        Ok(true)
    }

    async fn profile_source(
        &self,
        archive_id: &str,
        fingerprint: &str,
    ) -> Result<Option<(String, i32)>> {
        if let Some(row) =
            sqlx::query("SELECT path, page_count, file_hash FROM archives WHERE id = ?")
                .bind(archive_id)
                .fetch_optional(&self.pool)
                .await?
        {
            let current_fingerprint: String = row.get("file_hash");
            if current_fingerprint == fingerprint {
                return Ok(Some((row.get("path"), row.get("page_count"))));
            }
            return Ok(None);
        }

        let Some(row) = sqlx::query(
            "SELECT trash_path, metadata_json FROM trash_entries
             WHERE archive_id = ? AND status = 'active' AND trash_path IS NOT NULL
             ORDER BY deleted_at DESC LIMIT 1",
        )
        .bind(archive_id)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };
        let path: Option<String> = row.try_get("trash_path")?;
        let snapshot: Value = serde_json::from_str(row.get::<String, _>("metadata_json").as_str())
            .context("invalid trash snapshot while processing content profile")?;
        let snapshot_fingerprint = snapshot
            .get("file_hash")
            .or_else(|| snapshot.get("fileHash"))
            .and_then(Value::as_str);
        if snapshot_fingerprint.is_some_and(|value| value != fingerprint) {
            return Ok(None);
        }
        let Some(path) = path else {
            return Ok(None);
        };
        let expected_page_count = snapshot
            .get("page_count")
            .or_else(|| snapshot.get("pageCount"))
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .clamp(0, i64::from(i32::MAX)) as i32;
        Ok(Some((path, expected_page_count)))
    }

    async fn release_expired(&self) -> Result<()> {
        sqlx::query(
            "UPDATE content_profile_jobs
             SET status = 'retryable', next_attempt_at = CURRENT_TIMESTAMP,
                 last_error = 'expired worker lease', updated_at = CURRENT_TIMESTAMP
             WHERE status = 'running' AND updated_at < datetime('now', '-10 minutes')",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn store_profile(
        &self,
        archive_id: &str,
        fingerprint: &str,
        profile: &ProfileBuildResult,
    ) -> Result<()> {
        let document_json = serde_json::to_string(&profile.document)?;
        let method_json = serde_json::to_string(&profile.method)?;
        sqlx::query(
            "INSERT INTO archive_content_profiles
             (id, archive_id, content_fingerprint, profile_version, status, profile_json,
              expected_page_count, actual_page_count, sampled_page_count, decoded_page_count,
              coverage, method_json, last_error, attempts, completed_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(archive_id, content_fingerprint, profile_version) DO UPDATE SET
               status = excluded.status, profile_json = excluded.profile_json,
               expected_page_count = excluded.expected_page_count,
               actual_page_count = excluded.actual_page_count,
               sampled_page_count = excluded.sampled_page_count,
               decoded_page_count = excluded.decoded_page_count,
               coverage = excluded.coverage, method_json = excluded.method_json,
               last_error = NULL, attempts = archive_content_profiles.attempts + 1,
               completed_at = excluded.completed_at, updated_at = excluded.updated_at",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(archive_id)
        .bind(fingerprint)
        .bind(CONTENT_PROFILE_VERSION)
        .bind(profile.status)
        .bind(document_json)
        .bind(profile.document.expected_page_count)
        .bind(profile.document.actual_page_count)
        .bind(profile.document.sampled_page_count)
        .bind(profile.document.decoded_page_count)
        .bind(profile.document.coverage)
        .bind(method_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

async fn append_tag_features(
    pool: &Pool<Sqlite>,
    archive_id: &str,
    document: &mut ArchiveContentProfileDocument,
) -> Result<()> {
    let rows = sqlx::query(
        "SELECT t.namespace, t.name
         FROM tags t JOIN archive_tags at ON at.tag_id = t.id
         WHERE at.archive_id = ? ORDER BY t.namespace, t.name",
    )
    .bind(archive_id)
    .fetch_all(pool)
    .await?;
    let mut seen = BTreeSet::new();
    for row in rows {
        let namespace: String = row.get("namespace");
        let name: String = row.get("name");
        append_tag_feature(document, &mut seen, &namespace, &name);
    }
    if seen.is_empty() {
        if let Some(row) = sqlx::query(
            "SELECT metadata_json FROM trash_entries
             WHERE archive_id = ? AND status = 'active'
             ORDER BY deleted_at DESC LIMIT 1",
        )
        .bind(archive_id)
        .fetch_optional(pool)
        .await?
        {
            let snapshot: Value =
                serde_json::from_str(row.get::<String, _>("metadata_json").as_str())
                    .context("invalid trash snapshot while reading content tags")?;
            if let Some(tags) = snapshot.get("tags").and_then(Value::as_array) {
                for tag in tags {
                    let Some(namespace) = tag.get("namespace").and_then(Value::as_str) else {
                        continue;
                    };
                    let Some(name) = tag.get("name").and_then(Value::as_str) else {
                        continue;
                    };
                    append_tag_feature(document, &mut seen, namespace, name);
                }
            }
        }
    }
    Ok(())
}

fn append_tag_feature(
    document: &mut ArchiveContentProfileDocument,
    seen: &mut BTreeSet<String>,
    namespace: &str,
    name: &str,
) {
    let namespace = normalize_tag_value(namespace);
    let name = normalize_tag_value(name);
    if namespace.is_empty() || name.is_empty() {
        return;
    }
    let key = format!("tag:{namespace}:{name}");
    if seen.insert(key.clone()) {
        document.features.push(ContentProfileFeature {
            key,
            value: 1.0,
            kind: "binary".to_string(),
        });
    }
}

fn normalize_tag_value(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_ascii_lowercase()
}

fn build_profile(
    path: &Path,
    expected_page_count: i32,
    fingerprint: &str,
) -> Result<ProfileBuildResult> {
    let (actual_page_count, _) = ArchiveExtractor::get_page_count(path)
        .with_context(|| format!("failed to enumerate archive pages: {}", path.display()))?;
    if actual_page_count == 0 {
        return Err(anyhow!("archive contains no image pages"));
    }
    let indices = sample_page_indices(actual_page_count, MAX_SAMPLE_PAGES);
    let mut measurements = Vec::with_capacity(indices.len());
    let mut page_errors = Vec::new();
    for index in &indices {
        match ArchiveExtractor::extract_single_page(path, *index)
            .and_then(|page| analyze_page(&page.data, *index))
        {
            Ok(measurement) => measurements.push(measurement),
            Err(error) => page_errors.push(json!({
                "page": index + 1,
                "error": error.to_string(),
            })),
        }
    }
    if measurements.is_empty() {
        return Err(anyhow!("no sampled pages could be decoded"));
    }

    let expected = expected_page_count.max(0) as usize;
    let actual = actual_page_count;
    let coverage = measurements.len() as f64 / indices.len().max(1) as f64;
    let (features, summary) = aggregate_measurements(&measurements, actual as f64);
    let status = if coverage >= 0.999 && (expected == 0 || expected == actual) {
        "completed"
    } else {
        "partial"
    };
    let document = ArchiveContentProfileDocument {
        profile_version: CONTENT_PROFILE_VERSION.to_string(),
        content_fingerprint: fingerprint.to_string(),
        expected_page_count,
        actual_page_count: actual as i32,
        sampled_page_count: indices.len() as i32,
        decoded_page_count: measurements.len() as i32,
        coverage,
        features,
        measurements: json!({
            "pageCountMatch": expected == 0 || expected == actual,
            "sampledPageNumbers": indices.iter().map(|index| index + 1).collect::<Vec<_>>(),
            "decodeErrors": page_errors,
            "summary": summary,
        }),
    };
    Ok(ProfileBuildResult {
        document,
        status,
        method: json!({
            "name": "pixel-and-low-resolution-structure",
            "version": CONTENT_PROFILE_VERSION,
            "sampleLimit": MAX_SAMPLE_PAGES,
            "resolution": LOW_RESOLUTION,
            "distance": "normalized-luma-l1-and-ink-mask-hamming",
        }),
    })
}

fn analyze_page(data: &[u8], page_index: usize) -> Result<PageMeasurement> {
    let decoded = image::load_from_memory(data).context("failed to decode page image")?;
    let (width, height) = decoded.dimensions();
    if width == 0 || height == 0 {
        return Err(anyhow!("page image has zero dimensions"));
    }
    let rgb = decoded.to_rgb8();
    let mut gray_pixels = 0_u64;
    let mut chroma_sum = 0.0;
    for pixel in rgb.pixels() {
        let [red, green, blue] = pixel.0;
        let max = red.max(green).max(blue);
        let min = red.min(green).min(blue);
        let delta = f64::from(max - min) / 255.0;
        if max - min <= 10 {
            gray_pixels += 1;
        }
        chroma_sum += delta;
    }
    let pixel_count = u64::from(width) * u64::from(height);
    let gray_fraction = gray_pixels as f64 / pixel_count.max(1) as f64;
    let color_fraction = 1.0 - gray_fraction;
    let chroma = chroma_sum / pixel_count.max(1) as f64;

    let small = decoded.resize_exact(LOW_RESOLUTION, LOW_RESOLUTION, FilterType::Triangle);
    let luma = small.to_luma8();
    let luma_values: Vec<f64> = luma
        .pixels()
        .map(|pixel| f64::from(pixel[0]) / 255.0)
        .collect();
    let structure = luma_values.iter().map(|value| *value < 0.88).collect();
    Ok(PageMeasurement {
        page_index,
        luma: luma_values,
        structure,
        color_fraction,
        gray_fraction,
        chroma,
        aspect_ratio: width as f64 / height as f64,
    })
}

fn aggregate_measurements(
    pages: &[PageMeasurement],
    actual_page_count: f64,
) -> (Vec<ContentProfileFeature>, Value) {
    let count = pages.len().max(1) as f64;
    let color_fraction = pages.iter().map(|page| page.color_fraction).sum::<f64>() / count;
    let gray_fraction = pages.iter().map(|page| page.gray_fraction).sum::<f64>() / count;
    let chroma = pages.iter().map(|page| page.chroma).sum::<f64>() / count;
    let aspect_ratio = pages.iter().map(|page| page.aspect_ratio).sum::<f64>() / count;
    let aspect_variance = pages
        .iter()
        .map(|page| (page.aspect_ratio - aspect_ratio).powi(2))
        .sum::<f64>()
        / count;

    let pairs: Vec<(f64, f64)> = pages
        .windows(2)
        .map(|pair| {
            (
                visual_distance(&pair[0].luma, &pair[1].luma),
                structure_distance(&pair[0].structure, &pair[1].structure),
            )
        })
        .collect();
    let visual_distances: Vec<f64> = pairs.iter().map(|(visual, _)| *visual).collect();
    let structure_distances: Vec<f64> = pairs.iter().map(|(_, structure)| *structure).collect();
    let visual_change = mean(&visual_distances);
    let layout_stability = 1.0 - mean(&structure_distances);
    let visual_p50 = percentile(&visual_distances, 0.50);
    let visual_p90 = percentile(&visual_distances, 0.90);
    let similarity_p50 = 1.0 - visual_p50;
    let similarity_p90 = 1.0 - visual_p90;
    let duplicate_page_ratio = if visual_distances.is_empty() {
        0.0
    } else {
        visual_distances
            .iter()
            .filter(|distance| **distance <= 0.03)
            .count() as f64
            / visual_distances.len() as f64
    };
    let visual_mad = median_absolute_deviation(&visual_distances, visual_p50);
    let section_boundary_threshold = (visual_p50 + 3.0 * visual_mad + SECTION_BOUNDARY_MARGIN)
        .max(MIN_SECTION_BOUNDARY_DISTANCE)
        .min(0.95);
    let section_boundary_score = if visual_distances.is_empty() {
        0.0
    } else {
        visual_distances
            .iter()
            .filter(|distance| **distance >= section_boundary_threshold)
            .count() as f64
            / visual_distances.len() as f64
    };

    let numeric = [
        ("page_count", actual_page_count),
        ("color_fraction", color_fraction),
        ("gray_fraction", gray_fraction),
        ("average_chroma", chroma),
        ("page_similarity_p50", similarity_p50),
        ("page_similarity_p90", similarity_p90),
        ("duplicate_page_ratio", duplicate_page_ratio),
        ("visual_change_score", visual_change),
        ("layout_stability_score", layout_stability),
        ("section_boundary_score", section_boundary_score),
        ("page_aspect_ratio_mean", aspect_ratio),
        ("page_aspect_ratio_variance", aspect_variance),
    ];
    let features = numeric
        .into_iter()
        .map(|(key, value)| ContentProfileFeature {
            key: key.to_string(),
            value: value.clamp(0.0, 1.0e6),
            kind: "numeric".to_string(),
        })
        .collect();
    let summary = json!({
        "pageCount": actual_page_count,
        "sampledPageCount": pages.len(),
        "colorFractionMean": color_fraction,
        "grayFractionMean": gray_fraction,
        "averageChroma": chroma,
        "visualDistanceMean": visual_change,
        "visualDistanceP50": visual_p50,
        "visualDistanceP90": visual_p90,
        "structureDistanceMean": mean(&structure_distances),
        "layoutStability": layout_stability,
        "sectionBoundaryThreshold": section_boundary_threshold,
        "pageIndexes": pages.iter().map(|page| page.page_index + 1).collect::<Vec<_>>(),
    });
    (features, summary)
}

fn median_absolute_deviation(values: &[f64], median: f64) -> f64 {
    let deviations: Vec<f64> = values.iter().map(|value| (value - median).abs()).collect();
    percentile(&deviations, 0.50)
}

fn visual_distance(left: &[f64], right: &[f64]) -> f64 {
    if left.is_empty() || left.len() != right.len() {
        return 1.0;
    }
    left.iter()
        .zip(right)
        .map(|(a, b)| (a - b).abs())
        .sum::<f64>()
        / left.len() as f64
}

fn structure_distance(left: &[bool], right: &[bool]) -> f64 {
    if left.is_empty() || left.len() != right.len() {
        return 1.0;
    }
    left.iter().zip(right).filter(|(a, b)| a != b).count() as f64 / left.len() as f64
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn percentile(values: &[f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * percentile.clamp(0.0, 1.0)).round() as usize;
    sorted[index]
}

fn sample_page_indices(page_count: usize, limit: usize) -> Vec<usize> {
    if page_count == 0 || limit == 0 {
        return Vec::new();
    }
    if limit == 1 {
        return vec![0];
    }
    if page_count <= limit {
        return (0..page_count).collect();
    }
    let mut pages = BTreeSet::new();
    for index in 0..limit {
        pages.insert(index * (page_count - 1) / (limit - 1));
    }
    pages.into_iter().collect()
}

pub fn spawn_content_profile_worker(pool: Pool<Sqlite>) {
    tokio::spawn(async move {
        let service = ContentProfileService::new(pool);
        loop {
            match service.process_next().await {
                Ok(true) => {}
                Ok(false) => tokio::time::sleep(std::time::Duration::from_secs(5)).await,
                Err(error) => {
                    tracing::warn!(%error, "content profile worker iteration failed");
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_sampling_is_stable_and_covers_endpoints() {
        assert_eq!(sample_page_indices(3, 64), vec![0, 1, 2]);
        let pages = sample_page_indices(100, 10);
        assert_eq!(pages.len(), 10);
        assert_eq!(pages[0], 0);
        assert_eq!(*pages.last().unwrap(), 99);
        assert_eq!(pages, sample_page_indices(100, 10));
    }

    #[test]
    fn visual_distance_is_normalized() {
        assert_eq!(visual_distance(&[0.0, 1.0], &[0.0, 1.0]), 0.0);
        assert_eq!(visual_distance(&[0.0, 1.0], &[1.0, 0.0]), 1.0);
        assert_eq!(structure_distance(&[true, false], &[false, false]), 0.5);
    }

    #[test]
    fn numeric_profile_features_include_actual_page_count() {
        let page = PageMeasurement {
            page_index: 0,
            luma: vec![0.0, 1.0],
            structure: vec![true, false],
            color_fraction: 0.25,
            gray_fraction: 0.75,
            chroma: 0.10,
            aspect_ratio: 0.75,
        };
        let (features, summary) = aggregate_measurements(&[page], 60.0);
        let page_count = features
            .iter()
            .find(|feature| feature.key == "page_count")
            .expect("page count should be a learnable feature");
        assert_eq!(page_count.value, 60.0);
        assert_eq!(summary["pageCount"], 60.0);
        assert_eq!(summary["sampledPageCount"], 1);
    }

    #[test]
    fn section_boundary_threshold_does_not_mark_uniform_changes_as_boundaries() {
        let pages = (0..4)
            .map(|page_index| PageMeasurement {
                page_index,
                luma: vec![0.0, 0.5, 1.0],
                structure: vec![true, false, false],
                color_fraction: 0.0,
                gray_fraction: 1.0,
                chroma: 0.0,
                aspect_ratio: 0.75,
            })
            .collect::<Vec<_>>();
        let (features, _) = aggregate_measurements(&pages, 4.0);
        let boundary_score = features
            .iter()
            .find(|feature| feature.key == "section_boundary_score")
            .expect("section boundary feature should exist")
            .value;
        assert_eq!(boundary_score, 0.0);
    }

    #[test]
    fn tag_values_are_normalized_without_a_concept_vocabulary() {
        assert_eq!(normalize_tag_value("  Some   Value "), "some value");
    }
}
