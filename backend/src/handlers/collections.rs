use crate::middleware::{auth::AuthInfo, path_permission};
use crate::models::{
    AddCollectionMemberRequest, CollectionReviewAction, CreateCollectionRequest,
    UpdateCollectionRequest,
};
use crate::services::collection_service;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use sqlx::{Pool, Sqlite};

#[derive(Debug, Deserialize)]
pub struct CollectionListQuery {
    pub query: Option<String>,
}

pub async fn list_collections(
    State(pool): State<Pool<Sqlite>>,
    Query(query): Query<CollectionListQuery>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
) -> Result<Json<Vec<crate::models::CollectionSummary>>, StatusCode> {
    let collections = collection_service::list_collections(&pool, query.query.as_deref())
        .await
        .map_err(internal_error)?;
    let mut visible = Vec::with_capacity(collections.len());
    for collection in collections {
        let Some(cover_id) = collection.cover_archive_id.as_deref() else {
            visible.push(collection);
            continue;
        };
        let path = sqlx::query_scalar::<_, String>("SELECT path FROM archives WHERE id = ?")
            .bind(cover_id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if let Some(path) = path {
            if path_permission::has_path_permission(&pool, &auth, &path).await? {
                visible.push(collection);
            }
        }
    }
    Ok(Json(visible))
}

pub async fn get_collection(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
) -> Result<Json<crate::models::CollectionDetail>, StatusCode> {
    let mut detail = collection_service::get_collection(&pool, &id)
        .await
        .map_err(internal_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut visible_members = Vec::with_capacity(detail.members.len());
    for member in detail.members {
        if path_permission::has_path_permission(&pool, &auth, &member.archive.path).await? {
            visible_members.push(member);
        }
    }
    detail.members = visible_members;
    if detail.members.is_empty() {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(Json(detail))
}

pub async fn list_review_items(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
) -> Result<Json<Vec<crate::models::CollectionReviewItem>>, StatusCode> {
    let items = collection_service::list_review_items(&pool)
        .await
        .map_err(internal_error)?;
    let mut visible = Vec::new();
    for item in items {
        if path_permission::has_path_permission(&pool, &auth, &item.archive.path).await? {
            visible.push(item);
        }
    }
    Ok(Json(visible))
}

pub async fn rebuild_collections(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(_auth): axum::extract::Extension<AuthInfo>,
) -> Result<Json<crate::models::CollectionRebuildResponse>, StatusCode> {
    collection_service::rebuild_collections(&pool)
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn apply_review(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    Json(request): Json<CollectionReviewAction>,
) -> Result<StatusCode, StatusCode> {
    let archive_id = sqlx::query_scalar::<_, String>(
        "SELECT archive_id FROM collection_review_items WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;
    path_permission::authorize_archive_access(&pool, &auth, &archive_id).await?;
    collection_service::apply_review(&pool, &id, &request.action)
        .await
        .map(|_| StatusCode::OK)
        .map_err(internal_error)
}

pub async fn create_collection(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    Json(request): Json<CreateCollectionRequest>,
) -> Result<Json<crate::models::CollectionSummary>, StatusCode> {
    if request.display_title.trim().is_empty() || request.archive_ids.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    for archive_id in &request.archive_ids {
        path_permission::authorize_archive_access(&pool, &auth, archive_id).await?;
    }
    let id = uuid::Uuid::new_v4().to_string();
    let normalized_key = format!("manual:{}", id);
    sqlx::query("INSERT INTO collections (id, display_title, normalized_key, status, is_manual_locked) VALUES (?, ?, ?, 'manual', TRUE)")
        .bind(&id)
        .bind(request.display_title.trim())
        .bind(normalized_key)
        .execute(&pool)
        .await
        .map_err(internal_error)?;
    for archive_id in request.archive_ids {
        collection_service::add_member(&pool, &id, &archive_id)
            .await
            .map_err(internal_error)?;
    }
    let summary = collection_service::list_collections(&pool, None)
        .await
        .map_err(internal_error)?
        .into_iter()
        .find(|collection| collection.id == id)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(summary))
}

pub async fn update_collection(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
    Json(request): Json<UpdateCollectionRequest>,
) -> Result<StatusCode, StatusCode> {
    collection_service::update_collection(
        &pool,
        &id,
        request.display_title.as_deref(),
        request.is_manual_locked,
    )
    .await
    .map(|_| StatusCode::OK)
    .map_err(internal_error)
}

pub async fn add_member(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    Json(request): Json<AddCollectionMemberRequest>,
) -> Result<StatusCode, StatusCode> {
    path_permission::authorize_archive_access(&pool, &auth, &request.archive_id).await?;
    collection_service::add_member(&pool, &id, &request.archive_id)
        .await
        .map(|_| StatusCode::OK)
        .map_err(internal_error)
}

pub async fn remove_member(
    State(pool): State<Pool<Sqlite>>,
    Path(archive_id): Path<String>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
) -> Result<StatusCode, StatusCode> {
    path_permission::authorize_archive_access(&pool, &auth, &archive_id).await?;
    collection_service::remove_member(&pool, &archive_id)
        .await
        .map(|_| StatusCode::OK)
        .map_err(internal_error)
}

fn internal_error(error: impl std::fmt::Display) -> StatusCode {
    tracing::error!("Collection operation failed: {error}");
    StatusCode::INTERNAL_SERVER_ERROR
}
