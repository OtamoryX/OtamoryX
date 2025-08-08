use super::TagModel;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Archive {
    pub id: String,
    pub title: String,
    pub path: String,
    #[serde(rename = "fileSize")]
    pub file_size: i64,
    #[serde(rename = "pageCount")]
    pub page_count: i32,
    pub hash: String,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub tags: Vec<TagModel>,
}

// Tag is defined in tag.rs and imported as TagModel

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveWithTags {
    #[serde(flatten)]
    pub archive: Archive,
    pub tags: Vec<TagModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    #[serde(rename = "pageNumb")]
    pub page_numb: u64,
    #[serde(rename = "pageSize")]
    pub page_size: u64,
    pub total: u64,
    #[serde(rename = "hasNext")]
    pub has_next: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateArchiveRequest {
    pub title: String,
    pub path: String,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchRequest {
    pub query: Option<String>,     // 标题关键词搜索
    pub tags: Option<Vec<String>>, // 标签搜索
    #[serde(rename = "minPages")]
    pub min_pages: Option<i32>, // 最小页数
    #[serde(rename = "maxPages")]
    pub max_pages: Option<i32>, // 最大页数
    #[serde(rename = "minFileSize")]
    pub min_file_size: Option<i64>, // 最小文件大小（字节）
    #[serde(rename = "maxFileSize")]
    pub max_file_size: Option<i64>, // 最大文件大小（字节）
    #[serde(rename = "createdAfter")]
    pub created_after: Option<String>, // 创建时间之后（ISO 8601格式）
    #[serde(rename = "createdBefore")]
    pub created_before: Option<String>, // 创建时间之前（ISO 8601格式）
    #[serde(rename = "lastReadAfter")]
    pub last_read_after: Option<String>, // 最后阅读时间之后（ISO 8601格式）
    #[serde(rename = "lastReadBefore")]
    pub last_read_before: Option<String>, // 最后阅读时间之前（ISO 8601格式）
    #[serde(rename = "sortBy")]
    pub sort_by: Option<String>, // 排序字段：title, created_at, updated_at, file_size, page_count
    #[serde(rename = "sortOrder")]
    pub sort_order: Option<String>, // 排序方向：asc, desc
    #[serde(rename = "pageNumb")]
    pub page_numb: Option<u64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<u64>,
}
