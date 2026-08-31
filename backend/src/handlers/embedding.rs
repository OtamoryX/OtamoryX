use axum::{extract::State, http::StatusCode, response::Json};
use serde::Serialize;
use sqlx::{Pool, Sqlite};

use crate::{
    models::EmbeddingSettings,
    services::{
        embedding_settings_for_connection_test, load_embedding_settings, save_embedding_settings,
        test_embedding_connection,
    },
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingConnectionTestResponse {
    pub success: bool,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EmbeddingErrorResponse {
    pub message: String,
}

type EmbeddingApiError = (StatusCode, Json<EmbeddingErrorResponse>);

fn embedding_api_error(status: StatusCode, error: impl std::fmt::Display) -> EmbeddingApiError {
    (
        status,
        Json(EmbeddingErrorResponse {
            message: error.to_string(),
        }),
    )
}

pub async fn get_embedding_settings(
    State(pool): State<Pool<Sqlite>>,
) -> Result<Json<EmbeddingSettings>, EmbeddingApiError> {
    let settings = load_embedding_settings(&pool).await.map_err(|error| {
        tracing::error!("Failed to load embedding settings: {error:#}");
        embedding_api_error(StatusCode::INTERNAL_SERVER_ERROR, error)
    })?;
    Ok(Json(settings))
}

pub async fn update_embedding_settings(
    State(pool): State<Pool<Sqlite>>,
    Json(settings): Json<EmbeddingSettings>,
) -> Result<StatusCode, EmbeddingApiError> {
    save_embedding_settings(&pool, settings)
        .await
        .map_err(|error| {
            tracing::warn!("Rejected embedding settings update: {error:#}");
            embedding_api_error(StatusCode::BAD_REQUEST, error)
        })?;
    Ok(StatusCode::OK)
}

pub async fn test_embedding_connection_handler(
    State(pool): State<Pool<Sqlite>>,
    Json(settings): Json<EmbeddingSettings>,
) -> Json<EmbeddingConnectionTestResponse> {
    let stored = match load_embedding_settings(&pool).await {
        Ok(settings) => settings,
        Err(error) => {
            return Json(EmbeddingConnectionTestResponse {
                success: false,
                message: Some(format!("无法读取 embedding 设置: {error}")),
            });
        }
    };
    let effective = match embedding_settings_for_connection_test(&stored, settings) {
        Ok(settings) => settings,
        Err(error) => {
            return Json(EmbeddingConnectionTestResponse {
                success: false,
                message: Some(error.to_string()),
            });
        }
    };
    match test_embedding_connection(&effective).await {
        Ok(()) => Json(EmbeddingConnectionTestResponse {
            success: true,
            message: None,
        }),
        Err(error) => {
            tracing::warn!("Embedding connection test failed: {error:#}");
            Json(EmbeddingConnectionTestResponse {
                success: false,
                message: Some(error.to_string()),
            })
        }
    }
}
