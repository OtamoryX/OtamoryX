use crate::middleware::{auth::AuthInfo, path_permission};
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;

use crate::services::content_analysis::service::{
    validate_ocr_experiment_sample_pages, ContentAnalysisService, MAX_OCR_EXPERIMENT_ARCHIVES,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrSamplingExperimentRequest {
    pub archive_ids: Vec<String>,
    pub sample_pages: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrSamplingExperimentResponse {
    pub attempted: usize,
    pub queued: usize,
    pub skipped: usize,
    pub failed: usize,
    pub sample_pages: usize,
}

pub async fn get_content_analysis(
    State(pool): State<Pool<Sqlite>>,
    Path(archive_id): Path<String>,
    Extension(auth): Extension<AuthInfo>,
) -> Result<Json<crate::models::ContentAnalysisResponse>, StatusCode> {
    let archive_path = sqlx::query_scalar::<_, String>("SELECT path FROM archives WHERE id = ?")
        .bind(&archive_id)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if !path_permission::has_path_permission(&pool, &auth, &archive_path).await? {
        return Err(StatusCode::FORBIDDEN);
    }
    ContentAnalysisService::new(pool)
        .get(&archive_id)
        .await
        .map_err(|err| {
            tracing::warn!(archive_id, "content analysis lookup failed: {err:#}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Queues a small, explicit OCR sampling experiment without changing the normal intake default.
/// This endpoint is admin-only through the router it is registered on.
pub async fn enqueue_ocr_sampling_experiment(
    State(pool): State<Pool<Sqlite>>,
    Json(request): Json<OcrSamplingExperimentRequest>,
) -> Result<Json<OcrSamplingExperimentResponse>, StatusCode> {
    let sample_pages = validate_ocr_experiment_sample_pages(request.sample_pages)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut seen = HashSet::new();
    let archive_ids = request
        .archive_ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .filter(|id| seen.insert(id.clone()))
        .collect::<Vec<_>>();
    if archive_ids.is_empty() || archive_ids.len() > MAX_OCR_EXPERIMENT_ARCHIVES {
        return Err(StatusCode::BAD_REQUEST);
    }

    let workflow = ContentAnalysisService::new(pool);
    let mut response = OcrSamplingExperimentResponse {
        attempted: archive_ids.len(),
        queued: 0,
        skipped: 0,
        failed: 0,
        sample_pages,
    };
    for archive_id in archive_ids {
        match workflow
            .enqueue_ocr_sampling_experiment(&archive_id, sample_pages)
            .await
        {
            Ok(true) => response.queued += 1,
            Ok(false) => response.skipped += 1,
            Err(error) => {
                response.failed += 1;
                tracing::warn!(
                    archive_id,
                    sample_pages,
                    error = %error,
                    "failed to enqueue OCR sampling experiment"
                );
            }
        }
    }
    Ok(Json(response))
}
