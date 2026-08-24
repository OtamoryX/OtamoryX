use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::{Pool, Sqlite};

use crate::models::{OcrOperationResponse, OcrSettingsResponse, OcrSettingsUpdate};
use crate::services::{load_ocr_settings, ocr_manager};

pub async fn get_ocr_settings(
    State(pool): State<Pool<Sqlite>>,
) -> Result<Json<OcrSettingsResponse>, StatusCode> {
    let settings = load_ocr_settings(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ocr_manager().response(&settings)))
}

pub async fn update_ocr_settings(
    State(pool): State<Pool<Sqlite>>,
    Json(update): Json<OcrSettingsUpdate>,
) -> Result<StatusCode, StatusCode> {
    ocr_manager()
        .update_settings(&pool, update)
        .await
        .map_err(|error| {
            tracing::warn!(%error, "failed to update OCR settings");
            StatusCode::BAD_REQUEST
        })?;
    Ok(StatusCode::OK)
}

pub async fn download_ocr_model(
    Path(model_id): Path<String>,
) -> Result<(StatusCode, Json<OcrOperationResponse>), StatusCode> {
    let manager = ocr_manager();
    if !crate::services::OCR_MODELS
        .iter()
        .any(|model| model.id == model_id)
    {
        return Err(StatusCode::NOT_FOUND);
    }
    tokio::spawn(async move {
        if let Err(error) = manager.download_model(&model_id).await {
            tracing::warn!(model=%model_id, %error, "OCR model download failed");
        }
    });
    Ok((
        StatusCode::ACCEPTED,
        Json(OcrOperationResponse {
            accepted: true,
            message: "OCR 模型下载已加入后台任务".to_string(),
        }),
    ))
}

pub async fn activate_ocr_model(
    State(pool): State<Pool<Sqlite>>,
    Path(model_id): Path<String>,
) -> Result<(StatusCode, Json<OcrOperationResponse>), StatusCode> {
    if !crate::services::OCR_MODELS
        .iter()
        .any(|model| model.id == model_id)
    {
        return Err(StatusCode::NOT_FOUND);
    }
    let manager = ocr_manager();
    tokio::spawn(async move {
        if let Err(error) = manager.activate_model(&pool, &model_id).await {
            tracing::warn!(model=%model_id, %error, "failed to activate OCR model");
        }
    });
    Ok((
        StatusCode::ACCEPTED,
        Json(OcrOperationResponse {
            accepted: true,
            message: "OCR 模型切换已加入后台任务，旧任务将按新模型重新执行".to_string(),
        }),
    ))
}
