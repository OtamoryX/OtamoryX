use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::SearchRequest;

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
    #[serde(rename = "createdAfter")]
    pub created_after: Option<String>,
    #[serde(rename = "createdBefore")]
    pub created_before: Option<String>,
    #[serde(rename = "lastReadAfter")]
    pub last_read_after: Option<String>,
    #[serde(rename = "lastReadBefore")]
    pub last_read_before: Option<String>,
    #[serde(rename = "sortBy")]
    pub sort_by: Option<String>, // "title", "created_at", "updated_at", "file_size", "page_count"
    #[serde(rename = "sortOrder")]
    pub sort_order: Option<String>, // "asc", "desc"
}

impl CategorySearchParams {
    pub fn into_search_request(
        self,
        page_numb: Option<u64>,
        page_size: Option<u64>,
    ) -> SearchRequest {
        SearchRequest {
            query: self.query,
            tags: self.tags,
            min_pages: self.min_pages,
            max_pages: self.max_pages,
            min_file_size: self.min_file_size,
            max_file_size: self.max_file_size,
            created_after: self.created_after,
            created_before: self.created_before,
            last_read_after: self.last_read_after,
            last_read_before: self.last_read_before,
            sort_by: self.sort_by,
            sort_order: self.sort_order,
            page_numb,
            page_size,
        }
    }
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
