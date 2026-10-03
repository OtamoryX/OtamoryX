use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use sqlx::{Pool, Sqlite};

use crate::services::recommendations::weighted_graph::{
    self, ScoredWeightedTagRelationEdge, WeightedGraphPolicy, WeightedTagGraphStatus,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateWeightedGraphPolicyRequest {
    pub expected_version: i64,
    pub global_gain: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewWeightedRelationRequest {
    pub expected_revision: i64,
    pub expected_input_hash: String,
    pub expected_scorer_version: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeightedRelationListQuery {
    pub status: Option<String>,
    pub limit: Option<usize>,
}

pub async fn get_weighted_graph_policy(
    State(pool): State<Pool<Sqlite>>,
) -> Result<Json<WeightedGraphPolicy>, StatusCode> {
    weighted_graph::load_weighted_graph_policy(&pool)
        .await
        .map(Json)
        .map_err(|error| {
            tracing::error!(%error, "failed to load weighted tag graph policy");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn get_weighted_graph_status(
    State(pool): State<Pool<Sqlite>>,
) -> Result<Json<WeightedTagGraphStatus>, StatusCode> {
    weighted_graph::load_weighted_tag_graph_status(&pool)
        .await
        .map(Json)
        .map_err(|error| {
            tracing::error!(%error, "failed to load weighted tag graph status");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn update_weighted_graph_policy(
    State(pool): State<Pool<Sqlite>>,
    Json(request): Json<UpdateWeightedGraphPolicyRequest>,
) -> Result<Json<WeightedGraphPolicy>, StatusCode> {
    if request.expected_version < 0
        || !request.global_gain.is_finite()
        || !(0.0..=1.0).contains(&request.global_gain)
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let changed = weighted_graph::update_weighted_graph_policy(
        &pool,
        request.expected_version,
        request.global_gain,
    )
    .await
    .map_err(|error| {
        tracing::error!(%error, "failed to update weighted tag graph policy");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    if !changed {
        return Err(StatusCode::CONFLICT);
    }
    get_weighted_graph_policy(State(pool)).await
}

pub async fn list_weighted_relation_edges(
    State(pool): State<Pool<Sqlite>>,
    Query(query): Query<WeightedRelationListQuery>,
) -> Result<Json<Vec<ScoredWeightedTagRelationEdge>>, StatusCode> {
    let status = query.status.as_deref().unwrap_or("observing");
    let limit = query.limit.unwrap_or(100);
    if !matches!(status, "observing" | "active" | "rejected") || !(1..=500).contains(&limit) {
        return Err(StatusCode::BAD_REQUEST);
    }
    weighted_graph::list_scored_relation_weights(&pool, status, limit)
        .await
        .map(Json)
        .map_err(|error| {
            tracing::error!(%error, "failed to list scored weighted tag relations");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn review_weighted_relation_edge(
    State(pool): State<Pool<Sqlite>>,
    Path((tag_a_id, tag_b_id)): Path<(String, String)>,
    Json(request): Json<ReviewWeightedRelationRequest>,
) -> Result<StatusCode, StatusCode> {
    if !matches!(request.status.as_str(), "active" | "rejected")
        || request.expected_revision < 1
        || request.expected_input_hash.trim().is_empty()
        || request.expected_scorer_version.trim().is_empty()
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    match weighted_graph::review_weighted_relation_edge(
        &pool,
        &tag_a_id,
        &tag_b_id,
        request.expected_revision,
        &request.expected_input_hash,
        &request.expected_scorer_version,
        &request.status,
    )
    .await
    {
        Ok(true) => Ok(StatusCode::OK),
        Ok(false) => Err(StatusCode::CONFLICT),
        Err(error) if error.to_string().contains("metadata tag relations") => {
            Err(StatusCode::BAD_REQUEST)
        }
        Err(error) => {
            tracing::error!(%error, "failed to review weighted tag relation edge");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
