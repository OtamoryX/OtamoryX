use crate::middleware::{auth::AuthInfo, path_permission};
use crate::models::{
    AddCollectionMemberRequest, CollectionReviewAction, CreateCollectionRequest,
    UpdateCollectionRequest, VersionCleanupRequest,
};
use crate::services::collection_service;
use crate::services::ArchiveCacheService;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct CollectionListQuery {
    pub query: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VersionGroupListQuery {
    pub query: Option<String>,
    pub status: Option<String>,
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
    for mut collection in collections {
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
                collection.progress_percentage =
                    collection_service::collection_progress(&pool, &collection.id, &auth.user_id)
                        .await
                        .map_err(internal_error)?;
                visible.push(collection);
            }
        }
    }
    Ok(Json(visible))
}

pub async fn list_version_groups(
    State(pool): State<Pool<Sqlite>>,
    Query(query): Query<VersionGroupListQuery>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
) -> Result<Json<Vec<crate::models::VersionGroup>>, StatusCode> {
    let groups = collection_service::list_version_groups(&pool, query.query.as_deref())
        .await
        .map_err(internal_error)?;
    let mut visible = Vec::new();
    for mut group in groups {
        if query
            .status
            .as_deref()
            .is_some_and(|status| status != group.status)
        {
            continue;
        }
        let mut members = Vec::with_capacity(group.members.len());
        for member in group.members {
            if path_permission::has_path_permission(&pool, &auth, &member.archive.path).await? {
                members.push(member);
            }
        }
        group.members = members;
        if group.members.len() < 2 {
            continue;
        }
        if group
            .recommended_archive_id
            .as_ref()
            .is_some_and(|id| !group.members.iter().any(|member| &member.archive.id == id))
        {
            group.recommended_archive_id = None;
            group.reclaimable_size = 0;
            for member in &mut group.members {
                member.is_recommended = false;
                member.recommendation_reasons.clear();
            }
        }
        visible.push(group);
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
    detail.collection.progress_percentage =
        collection_service::collection_progress(&pool, &detail.collection.id, &auth.user_id)
            .await
            .map_err(internal_error)?;
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

pub async fn delete_all_collections(
    State(pool): State<Pool<Sqlite>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let deleted = collection_service::delete_all_collections(&pool)
        .await
        .map_err(internal_error)?;
    Ok(Json(serde_json::json!({ "deleted": deleted })))
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
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    Json(request): Json<UpdateCollectionRequest>,
) -> Result<StatusCode, StatusCode> {
    let detail = collection_service::get_collection(&pool, &id)
        .await
        .map_err(internal_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut has_access = false;
    for member in &detail.members {
        if path_permission::has_path_permission(&pool, &auth, &member.archive.path).await? {
            has_access = true;
            break;
        }
    }
    if !has_access {
        return Err(StatusCode::FORBIDDEN);
    }
    collection_service::update_collection(
        &pool,
        &id,
        request.display_title.as_deref(),
        request.subtitle.as_deref(),
        request.is_manual_locked,
    )
    .await
    .map(|_| StatusCode::OK)
    .map_err(internal_error)
}

pub async fn keep_all_versions(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    collection_service::keep_all_versions(&pool, &id)
        .await
        .map(|_| StatusCode::OK)
        .map_err(internal_error)
}

pub async fn cleanup_versions(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
    axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
    Json(request): Json<VersionCleanupRequest>,
) -> Result<Json<crate::models::VersionCleanupResponse>, StatusCode> {
    collection_service::cleanup_versions(
        &pool,
        &archive_cache,
        &id,
        &request.keep_archive_id,
        &request.delete_archive_ids,
    )
    .await
    .map(Json)
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
