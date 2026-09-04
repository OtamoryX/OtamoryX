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
use std::path::Path;
use std::{
    collections::{BTreeSet, HashSet},
    sync::{Arc, OnceLock},
    time::Duration as StdDuration,
};
use tokio::sync::Notify;
use uuid::Uuid;

use crate::models::{
    ArchiveContentProfileDocument, ContentProfileFeature, CANONICAL_THEME_FEATURE_KIND,
};
use crate::utils::ArchiveExtractor;

pub const CONTENT_PROFILE_VERSION: &str = "profile-v1";

const MAX_SAMPLE_PAGES: usize = 64;
const LOW_RESOLUTION: u32 = 32;
const MIN_SECTION_BOUNDARY_DISTANCE: f64 = 0.20;
const SECTION_BOUNDARY_MARGIN: f64 = 0.05;
const MAX_RETRIES: i64 = 4;

static CONTENT_PROFILE_SIGNAL: OnceLock<Arc<Notify>> = OnceLock::new();

fn content_profile_signal() -> &'static Arc<Notify> {
    CONTENT_PROFILE_SIGNAL.get_or_init(|| Arc::new(Notify::new()))
}

pub fn notify_content_profile_worker() {
    content_profile_signal().notify_waiters();
}

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
        let queued = inserted.rows_affected() == 1;
        if queued {
            notify_content_profile_worker();
        }
        Ok(queued)
    }

    pub async fn process_next(&self) -> Result<bool> {
        self.release_expired().await?;
        self.process_next_queued().await
    }

    async fn process_next_queued(&self) -> Result<bool> {
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
                // Wake only behavior events that depend on this profile after the durable state
                // change. The learning worker does not poll dormant waiting events.
                if let Err(error) =
                    crate::services::PreferenceLearningService::new(self.pool.clone())
                        .wake_waiting_for_archive(&archive_id)
                        .await
                {
                    tracing::warn!(%archive_id, %error, "preference learning events were not woken after profile completion");
                }
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

    async fn release_expired(&self) -> Result<u64> {
        let updated = sqlx::query(
            "UPDATE content_profile_jobs
             SET status = 'retryable', next_attempt_at = CURRENT_TIMESTAMP,
                 last_error = 'expired worker lease', updated_at = CURRENT_TIMESTAMP
             WHERE status = 'running' AND updated_at < datetime('now', '-10 minutes')",
        )
        .execute(&self.pool)
        .await?;
        Ok(updated.rows_affected())
    }

    async fn next_retry_delay(&self) -> Result<Option<StdDuration>> {
        let seconds: Option<f64> = sqlx::query_scalar(
            "SELECT MIN(julianday(next_attempt_at) - julianday('now'))
             FROM content_profile_jobs
             WHERE status = 'retryable' AND next_attempt_at IS NOT NULL",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(seconds
            .map(|value| StdDuration::from_secs_f64((value.max(0.0) * 86_400.0).min(86_400.0))))
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
    let metadata_namespaces =
        crate::services::recommendations::namespace_policy::load_metadata_namespace_set(pool)
            .await?;
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
        append_tag_feature(document, &mut seen, &metadata_namespaces, &namespace, &name);
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
                    append_tag_feature(document, &mut seen, &metadata_namespaces, namespace, name);
                }
            }
        }
    }
    let canonical_theme_ids = load_canonical_theme_ids_for_archive(pool, archive_id).await?;
    append_canonical_theme_features(document, &mut seen, &canonical_theme_ids);
    Ok(())
}

async fn load_canonical_theme_ids_for_archive(
    pool: &Pool<Sqlite>,
    archive_id: &str,
) -> Result<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT themes.theme_tag_id
         FROM content_analyses analysis
         JOIN archives current_archive ON current_archive.id = analysis.archive_id
         JOIN content_analysis_themes themes ON themes.analysis_id = analysis.id
         JOIN tags ON tags.id = themes.theme_tag_id
         WHERE analysis.archive_id = ?
           AND analysis.content_fingerprint = current_archive.file_hash
           AND analysis.status = 'completed'
           AND analysis.canonicalization_status = 'completed'
           AND themes.canonicalization_status = 'completed'
           AND themes.theme_tag_id IS NOT NULL
           AND lower(trim(tags.namespace)) = 'theme'
           AND analysis.id = (SELECT latest.id FROM content_analyses latest
                              WHERE latest.archive_id = analysis.archive_id
                                AND latest.content_fingerprint = current_archive.file_hash
                                AND latest.status = 'completed'
                                AND latest.canonicalization_status = 'completed'
                              ORDER BY latest.created_at DESC, latest.id DESC LIMIT 1)
         ORDER BY themes.ordinal",
    )
    .bind(archive_id)
    .fetch_all(pool)
    .await?)
}

async fn load_canonical_theme_ids_for_analysis(
    pool: &Pool<Sqlite>,
    analysis_id: &str,
) -> Result<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT themes.theme_tag_id
         FROM content_analyses analysis
         JOIN content_analysis_themes themes ON themes.analysis_id = analysis.id
         JOIN tags ON tags.id = themes.theme_tag_id
         WHERE analysis.id = ?
           AND analysis.status = 'completed'
           AND analysis.canonicalization_status = 'completed'
           AND themes.canonicalization_status = 'completed'
           AND themes.theme_tag_id IS NOT NULL
           AND lower(trim(tags.namespace)) = 'theme'
         ORDER BY themes.ordinal",
    )
    .bind(analysis_id)
    .fetch_all(pool)
    .await?)
}

fn append_canonical_theme_features(
    document: &mut ArchiveContentProfileDocument,
    seen: &mut BTreeSet<String>,
    theme_ids: &[String],
) {
    for theme_id in theme_ids {
        let theme_id = theme_id.trim();
        if theme_id.is_empty() {
            continue;
        }
        let key = format!("theme:{theme_id}");
        if seen.insert(key.clone()) {
            document.features.push(ContentProfileFeature {
                key,
                value: 1.0,
                kind: CANONICAL_THEME_FEATURE_KIND.to_string(),
            });
        }
    }
}

/// A profile can finish before the separate canonicalization job. Replace only the canonical
/// theme features after that job commits so deterministic visual/tag measurements remain intact.
pub(crate) async fn refresh_canonical_theme_features(
    pool: &Pool<Sqlite>,
    analysis_id: &str,
    archive_id: &str,
    fingerprint: &str,
) -> Result<()> {
    let Some(row) = sqlx::query(
        "SELECT id, profile_json
         FROM archive_content_profiles
         WHERE archive_id = ? AND content_fingerprint = ? AND profile_version = ?
           AND status IN ('completed', 'partial')
         LIMIT 1",
    )
    .bind(archive_id)
    .bind(fingerprint)
    .bind(CONTENT_PROFILE_VERSION)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(());
    };
    let profile_id: String = row.get("id");
    let profile_json: String = row.get("profile_json");
    let mut document: ArchiveContentProfileDocument =
        serde_json::from_str(&profile_json).context("invalid content profile")?;
    document
        .features
        .retain(|feature| !feature.key.starts_with("theme:"));
    let mut seen = document
        .features
        .iter()
        .map(|feature| feature.key.clone())
        .collect::<BTreeSet<_>>();
    let theme_ids = load_canonical_theme_ids_for_analysis(pool, analysis_id).await?;
    append_canonical_theme_features(&mut document, &mut seen, &theme_ids);
    sqlx::query(
        "UPDATE archive_content_profiles
         SET profile_json = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(serde_json::to_string(&document)?)
    .bind(profile_id)
    .execute(pool)
    .await?;
    Ok(())
}

fn append_tag_feature(
    document: &mut ArchiveContentProfileDocument,
    seen: &mut BTreeSet<String>,
    metadata_namespaces: &HashSet<String>,
    namespace: &str,
    name: &str,
) {
    let namespace = normalize_tag_value(namespace);
    let name = normalize_tag_value(name);
    if namespace.is_empty()
        || name.is_empty()
        || namespace == "theme"
        || metadata_namespaces.contains(&namespace)
    {
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
    let reaper_pool = pool.clone();
    tokio::spawn(async move {
        let service = ContentProfileService::new(reaper_pool);
        loop {
            match service.release_expired().await {
                Ok(released) if released > 0 => notify_content_profile_worker(),
                Ok(_) => {}
                Err(error) => tracing::warn!(%error, "content profile lease recovery failed"),
            }
            tokio::time::sleep(StdDuration::from_secs(10 * 60)).await;
        }
    });

    let signal = content_profile_signal().clone();
    tokio::spawn(async move {
        let service = ContentProfileService::new(pool);
        loop {
            let notified = signal.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            match service.process_next_queued().await {
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
                        tracing::warn!(%error, "content profile retry timer query failed");
                        tokio::select! {
                            _ = notified => {}
                            _ = tokio::time::sleep(StdDuration::from_secs(30)) => {}
                        }
                    }
                },
                Err(error) => {
                    tracing::warn!(%error, "content profile worker iteration failed");
                    tokio::select! {
                        _ = notified => {}
                        _ = tokio::time::sleep(StdDuration::from_secs(30)) => {}
                    }
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

    #[test]
    fn metadata_and_canonical_theme_tags_do_not_enter_profiles() {
        let mut document = ArchiveContentProfileDocument {
            profile_version: CONTENT_PROFILE_VERSION.to_string(),
            content_fingerprint: "fingerprint".to_string(),
            expected_page_count: 1,
            actual_page_count: 1,
            sampled_page_count: 1,
            decoded_page_count: 1,
            coverage: 1.0,
            features: Vec::new(),
            measurements: Value::Null,
        };
        let mut seen = BTreeSet::new();
        let metadata_namespaces = HashSet::from(["artist".to_string()]);
        append_tag_feature(
            &mut document,
            &mut seen,
            &metadata_namespaces,
            "ARTIST",
            "creator",
        );
        append_tag_feature(
            &mut document,
            &mut seen,
            &metadata_namespaces,
            "theme",
            "Space Opera",
        );
        append_tag_feature(
            &mut document,
            &mut seen,
            &metadata_namespaces,
            "general",
            "useful signal",
        );

        assert_eq!(
            document
                .features
                .iter()
                .map(|feature| feature.key.as_str())
                .collect::<Vec<_>>(),
            vec!["tag:general:useful signal"]
        );
    }

    #[test]
    fn canonical_theme_features_use_stable_ids_and_deduplicate_associations() {
        let mut document = ArchiveContentProfileDocument {
            profile_version: CONTENT_PROFILE_VERSION.to_string(),
            content_fingerprint: "fingerprint".to_string(),
            expected_page_count: 1,
            actual_page_count: 1,
            sampled_page_count: 1,
            decoded_page_count: 1,
            coverage: 1.0,
            features: Vec::new(),
            measurements: Value::Null,
        };
        let mut seen = BTreeSet::new();
        append_canonical_theme_features(
            &mut document,
            &mut seen,
            &["theme-id".to_string(), "theme-id".to_string()],
        );

        assert_eq!(document.features.len(), 1);
        assert_eq!(document.features[0].key, "theme:theme-id");
        assert_eq!(document.features[0].kind, CANONICAL_THEME_FEATURE_KIND);
    }
}
