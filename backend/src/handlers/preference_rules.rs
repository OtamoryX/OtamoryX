use crate::middleware::auth::AuthInfo;
use crate::models::PreferenceRuleInput;
use crate::services::PreferenceDecisionService;
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::Json,
    Json as AxumJson,
};
use sqlx::{Pool, Sqlite};

pub async fn list_rules(
    State(pool): State<Pool<Sqlite>>,
    Extension(auth): Extension<AuthInfo>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    PreferenceDecisionService::new(pool)
        .list_rules(&auth.user_id)
        .await
        .map(|v| Json(serde_json::json!(v)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
pub async fn create_rule(
    State(pool): State<Pool<Sqlite>>,
    Extension(auth): Extension<AuthInfo>,
    AxumJson(input): AxumJson<PreferenceRuleInput>,
) -> Result<(StatusCode, Json<crate::models::PreferenceRule>), StatusCode> {
    PreferenceDecisionService::new(pool)
        .create_rule(&auth.user_id, &auth.role, input)
        .await
        .map(|v| (StatusCode::CREATED, Json(v)))
        .map_err(|e| {
            if e.to_string().contains("only administrators") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::BAD_REQUEST
            }
        })
}
pub async fn update_rule(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
    Extension(auth): Extension<AuthInfo>,
    AxumJson(input): AxumJson<PreferenceRuleInput>,
) -> Result<Json<crate::models::PreferenceRule>, StatusCode> {
    PreferenceDecisionService::new(pool)
        .update_rule(&auth.user_id, &auth.role, &id, input)
        .await
        .map(Json)
        .map_err(|e| {
            if e.to_string().contains("only administrators") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::BAD_REQUEST
            }
        })
}
pub async fn enable_rule(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
    Extension(auth): Extension<AuthInfo>,
) -> Result<StatusCode, StatusCode> {
    PreferenceDecisionService::new(pool)
        .set_enabled(&auth.user_id, &auth.role, &id, true)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| {
            if e.to_string().contains("only administrators") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::BAD_REQUEST
            }
        })
}
pub async fn disable_rule(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
    Extension(auth): Extension<AuthInfo>,
) -> Result<StatusCode, StatusCode> {
    PreferenceDecisionService::new(pool)
        .set_enabled(&auth.user_id, &auth.role, &id, false)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|_| StatusCode::BAD_REQUEST)
}
pub async fn evaluate_archive(
    State(pool): State<Pool<Sqlite>>,
    Path(archive_id): Path<String>,
    Extension(auth): Extension<AuthInfo>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    PreferenceDecisionService::new(pool)
        .evaluate_archive(&auth.user_id, &archive_id)
        .await
        .map(|v| Json(serde_json::json!(v)))
        .map_err(|e| {
            tracing::warn!(archive_id, error=%e, "preference evaluation failed");
            StatusCode::BAD_REQUEST
        })
}
pub async fn list_evaluations(
    State(pool): State<Pool<Sqlite>>,
    Path(archive_id): Path<String>,
    Extension(auth): Extension<AuthInfo>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    PreferenceDecisionService::new(pool)
        .list_evaluations(&auth.user_id, &archive_id)
        .await
        .map(|v| Json(serde_json::json!(v)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
