use axum::{
    extract::{Query, State, Path},
    http::StatusCode,
    Json,
};
use sqlx::{Pool, Sqlite};
use serde::Deserialize;
use crate::models::{Archive, PaginatedResponse, SearchRequest, Tag};
use crate::services::SearchService;

#[derive(Deserialize)]
pub struct TagSearchQuery {
    pub query: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct PopularTagsQuery {
    pub limit: Option<u32>,
}

pub async fn search_archives(
    State(pool): State<Pool<Sqlite>>,
    Query(params): Query<SearchRequest>,
) -> Result<Json<PaginatedResponse<Archive>>, StatusCode> {
    let search_service = SearchService::new(pool);
    
    match search_service.search_archives(params).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => {
            tracing::error!("Search error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_tags(
    State(pool): State<Pool<Sqlite>>,
) -> Result<Json<Vec<Tag>>, StatusCode> {
    let search_service = SearchService::new(pool);
    
    match search_service.get_all_tags().await {
        Ok(tags) => Ok(Json(tags)),
        Err(e) => {
            tracing::error!("Failed to fetch tags: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn search_tags(
    State(pool): State<Pool<Sqlite>>,
    Query(params): Query<TagSearchQuery>,
) -> Result<Json<Vec<Tag>>, StatusCode> {
    let query = params.query.as_deref().unwrap_or("");
    
    if query.is_empty() {
        return get_tags(State(pool)).await;
    }
    
    let search_service = SearchService::new(pool);
    
    match search_service.search_tags(query, params.limit).await {
        Ok(tags) => Ok(Json(tags)),
        Err(e) => {
            tracing::error!("Failed to search tags: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_popular_tags(
    State(pool): State<Pool<Sqlite>>,
    Query(params): Query<PopularTagsQuery>,
) -> Result<Json<Vec<(Tag, u32)>>, StatusCode> {
    let search_service = SearchService::new(pool);
    
    match search_service.get_popular_tags(params.limit).await {
        Ok(tags) => Ok(Json(tags)),
        Err(e) => {
            tracing::error!("Failed to fetch popular tags: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_archive_tags(
    State(pool): State<Pool<Sqlite>>,
    Path(archive_id): Path<String>,
) -> Result<Json<Vec<Tag>>, StatusCode> {
    let search_service = SearchService::new(pool);
    
    match search_service.get_tags_by_archive(&archive_id).await {
        Ok(tags) => Ok(Json(tags)),
        Err(e) => {
            tracing::error!("Failed to fetch archive tags: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}