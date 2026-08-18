use crate::middleware::{auth::AuthInfo, path_permission};
use crate::models::{
    AddCollectionMemberRequest, CategorySearchParams, CollectionDeletionResponse,
    CollectionReviewAction, CreateCollectionRequest, SearchRequest, UpdateCollectionRequest,
    VersionCleanupRequest,
};
use crate::services::{
    collection_service, ArchiveCacheService, ArchiveDeletionService, ArchiveFilters,
    ArchiveQueryService, QueryOptions,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use sqlx::{Pool, Row, Sqlite};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct CollectionListQuery {
    #[serde(flatten)]
    pub filters: SearchRequest,
    #[serde(rename = "categoryId")]
    pub category_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VersionGroupListQuery {
    #[serde(flatten)]
    pub filters: SearchRequest,
    #[serde(rename = "categoryId")]
    pub category_id: Option<String>,
    pub status: Option<String>,
}

fn has_member_filter(filters: &SearchRequest, category_id: Option<&str>) -> bool {
    category_id.is_some()
        || filters
            .query
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
        || filters
            .tags
            .as_ref()
            .is_some_and(|values| !values.is_empty())
        || filters.min_pages.is_some()
        || filters.max_pages.is_some()
        || filters.min_file_size.is_some()
        || filters.max_file_size.is_some()
        || filters
            .created_after
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
        || filters
            .created_before
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
        || filters
            .last_read_after
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
        || filters
            .last_read_before
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
}

async fn category_archive_ids(
    pool: &Pool<Sqlite>,
    category_id: &str,
    user_id: &str,
) -> Result<Vec<String>, StatusCode> {
    let category = sqlx::query!(
        "SELECT category_type, search_criteria FROM categories WHERE id = ?",
        category_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some(category) = category else {
        return Ok(Vec::new());
    };
    if category.category_type == "static" {
        return sqlx::query_scalar::<_, String>(
            "SELECT archive_id FROM category_archives WHERE category_id = ?",
        )
        .bind(category_id)
        .fetch_all(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    }
    let params = serde_json::from_str::<CategorySearchParams>(
        category
            .search_criteria
            .as_deref()
            .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
    )
    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
    let targets = ArchiveQueryService::new(pool.clone())
        .query_delete_targets(
            ArchiveFilters::from_search_request(&params.into_search_request(None, None)),
            QueryOptions {
                random: false,
                include_tags: false,
                user_id: Some(user_id.to_string()),
            },
        )
        .await
        .map_err(internal_error)?;
    Ok(targets.into_iter().map(|target| target.id).collect())
}

async fn matching_archive_ids(
    pool: &Pool<Sqlite>,
    auth: &AuthInfo,
    filters: &SearchRequest,
    category_id: Option<&str>,
) -> Result<Option<Vec<String>>, StatusCode> {
    if !has_member_filter(filters, category_id) {
        return Ok(None);
    }
    let category_ids = match category_id {
        Some(id) => Some(category_archive_ids(pool, id, &auth.user_id).await?),
        None => None,
    };
    if category_ids.as_ref().is_some_and(Vec::is_empty) {
        return Ok(Some(Vec::new()));
    }
    let mut archive_filters = ArchiveFilters::from_search_request(filters);
    archive_filters.archive_ids = category_ids;
    let targets = ArchiveQueryService::new(pool.clone())
        .query_delete_targets(
            archive_filters,
            QueryOptions {
                random: false,
                include_tags: false,
                user_id: Some(auth.user_id.clone()),
            },
        )
        .await
        .map_err(internal_error)?;
    let paths = if auth.role == "admin" {
        Vec::new()
    } else {
        path_permission::get_user_paths(pool, &auth.user_id).await?
    };
    Ok(Some(
        targets
            .into_iter()
            .filter(|target| {
                path_permission::has_path_permission_with_paths(&auth.role, &paths, &target.path)
            })
            .map(|target| target.id)
            .collect(),
    ))
}

pub async fn list_collections(
    State(pool): State<Pool<Sqlite>>,
    Query(query): Query<CollectionListQuery>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
) -> Result<Json<Vec<crate::models::CollectionSummary>>, StatusCode> {
    let matching_ids =
        matching_archive_ids(&pool, &auth, &query.filters, query.category_id.as_deref()).await?;
    let collections = collection_service::list_collections(
        &pool,
        matching_ids.as_deref(),
        query.filters.sort_by.as_deref(),
        query.filters.sort_order.as_deref(),
    )
    .await
    .map_err(internal_error)?;
    if collections.is_empty() {
        return Ok(Json(Vec::new()));
    }
    let paths = if auth.role == "admin" {
        Vec::new()
    } else {
        path_permission::get_user_paths(&pool, &auth.user_id).await?
    };
    let ids = collections
        .iter()
        .map(|collection| collection.id.as_str())
        .collect::<Vec<_>>();
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let member_paths_sql = format!("SELECT cm.collection_id, a.path FROM collection_members cm JOIN archives a ON a.id = cm.archive_id WHERE cm.collection_id IN ({placeholders})");
    let mut member_paths_query = sqlx::query(&member_paths_sql);
    for id in ids {
        member_paths_query = member_paths_query.bind(id);
    }
    let member_paths = member_paths_query
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .into_iter()
        .fold(HashMap::<String, Vec<String>>::new(), |mut result, row| {
            result
                .entry(row.get("collection_id"))
                .or_default()
                .push(row.get("path"));
            result
        });
    let mut visible = Vec::with_capacity(collections.len());
    for mut collection in collections {
        if member_paths
            .get(&collection.id)
            .is_some_and(|paths_for_collection| {
                paths_for_collection.iter().any(|path| {
                    path_permission::has_path_permission_with_paths(&auth.role, &paths, path)
                })
            })
        {
            collection.progress_percentage =
                collection_service::collection_progress(&pool, &collection.id, &auth.user_id)
                    .await
                    .map_err(internal_error)?;
            visible.push(collection);
        }
    }
    Ok(Json(visible))
}

pub async fn list_version_groups(
    State(pool): State<Pool<Sqlite>>,
    Query(query): Query<VersionGroupListQuery>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
) -> Result<Json<Vec<crate::models::VersionGroup>>, StatusCode> {
    let matching_ids =
        matching_archive_ids(&pool, &auth, &query.filters, query.category_id.as_deref()).await?;
    let matching_set = matching_ids
        .as_ref()
        .map(|ids| ids.iter().cloned().collect::<HashSet<_>>());
    let groups = collection_service::list_version_groups(
        &pool,
        matching_set.as_ref(),
        query.filters.sort_by.as_deref(),
        query.filters.sort_order.as_deref(),
    )
    .await
    .map_err(internal_error)?;
    let paths = if auth.role == "admin" {
        Vec::new()
    } else {
        path_permission::get_user_paths(&pool, &auth.user_id).await?
    };
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
            if path_permission::has_path_permission_with_paths(
                &auth.role,
                &paths,
                &member.archive.path,
            ) {
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
    Query(query): Query<CollectionListQuery>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
) -> Result<Json<crate::models::CollectionDetail>, StatusCode> {
    let matching_ids =
        matching_archive_ids(&pool, &auth, &query.filters, query.category_id.as_deref()).await?;
    let matching_set = matching_ids
        .as_ref()
        .map(|ids| ids.iter().cloned().collect::<HashSet<_>>());
    let mut detail = collection_service::get_collection(&pool, &id)
        .await
        .map_err(internal_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let mut visible_members = Vec::with_capacity(detail.members.len());
    for mut member in detail.members {
        if path_permission::has_path_permission(&pool, &auth, &member.archive.path).await? {
            member.matches_filter = matching_set
                .as_ref()
                .is_some_and(|ids| ids.contains(&member.archive.id));
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
) -> Result<Json<crate::models::CollectionRebuildResponse>, StatusCode> {
    collection_service::rebuild_collections(&pool)
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn preview_collection_rebuild(
    State(pool): State<Pool<Sqlite>>,
) -> Result<Json<crate::models::CollectionRebuildPreview>, StatusCode> {
    collection_service::preview_collection_rebuild(&pool)
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn delete_collection_with_members(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
) -> Result<Json<CollectionDeletionResponse>, StatusCode> {
    let targets = collection_service::collection_member_delete_targets(&pool, &id)
        .await
        .map_err(|error| {
            if error.to_string() == "collection not found" {
                StatusCode::NOT_FOUND
            } else {
                internal_error(error)
            }
        })?;

    // Validate every member before deleting anything in this collection.
    for target in &targets {
        path_permission::authorize_archive_access(&pool, &auth, &target.id).await?;
    }

    let summary = ArchiveDeletionService::new(pool.clone(), archive_cache)
        .delete_targets(
            &auth.user_id,
            targets,
            "user deleted collection with members",
            "collection_delete_with_members",
        )
        .await
        .map_err(internal_error)?;
    if summary.failed > 0 {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    collection_service::delete_collection(&pool, &id)
        .await
        .map_err(internal_error)?;
    Ok(Json(CollectionDeletionResponse {
        collection_id: id,
        deleted_archives: summary.deleted,
    }))
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
    let summary = collection_service::list_collections(&pool, None, None, None)
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

pub async fn restore_version_group(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    collection_service::restore_version_group(&pool, &id)
        .await
        .map(|_| StatusCode::OK)
        .map_err(internal_error)
}

pub async fn cleanup_versions(
    State(pool): State<Pool<Sqlite>>,
    Path(id): Path<String>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
    Json(request): Json<VersionCleanupRequest>,
) -> Result<Json<crate::models::VersionCleanupResponse>, StatusCode> {
    collection_service::cleanup_versions(
        &pool,
        &archive_cache,
        &auth.user_id,
        &id,
        &request.keep_archive_id,
        &request.delete_archive_ids,
        request.idempotency_key.as_deref(),
    )
    .await
    .map(Json)
    .map_err(|error| {
        let message = error.to_string();
        if message.contains("idempotency key")
            || message.contains("already in progress")
            || message.contains("no longer active")
        {
            tracing::warn!("Version cleanup conflict: {error}");
            StatusCode::CONFLICT
        } else {
            internal_error(error)
        }
    })
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
