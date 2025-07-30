use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "isStatic")]
    pub is_static: bool, // true为静态分类，false为动态分类
    #[serde(rename = "archiveCount")]
    pub archive_count: i32,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DynamicCategory {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "searchParams")]
    pub search_params: String, // JSON格式的搜索参数
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateDynamicCategoryRequest {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "searchParams")]
    pub search_params: CategorySearchParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySearchParams {
    pub query: Option<String>,
    pub tags: Option<Vec<String>>,
    #[serde(rename = "minPages")]
    pub min_pages: Option<i32>,
    #[serde(rename = "maxPages")]
    pub max_pages: Option<i32>,
    #[serde(rename = "minFileSize")]
    pub min_file_size: Option<i64>,
    #[serde(rename = "maxFileSize")]
    pub max_file_size: Option<i64>,
    #[serde(rename = "sortBy")]
    pub sort_by: Option<String>, // "title", "created_at", "updated_at", "file_size", "page_count"
    #[serde(rename = "sortOrder")]
    pub sort_order: Option<String>, // "asc", "desc"
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddArchivesToCategoryRequest {
    #[serde(rename = "archiveIds")]
    pub archive_ids: Vec<String>,
}