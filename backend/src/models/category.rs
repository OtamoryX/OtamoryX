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
    #[serde(rename = "themeIds")]
    pub theme_ids: Option<Vec<String>>,
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
    pub fn has_filter_criteria(&self) -> bool {
        self.query
            .as_ref()
            .is_some_and(|query| !query.trim().is_empty())
            || self.tags.as_ref().is_some_and(|tags| !tags.is_empty())
            || self
                .theme_ids
                .as_ref()
                .is_some_and(|theme_ids| !theme_ids.is_empty())
            || self.min_pages.is_some()
            || self.max_pages.is_some()
            || self.min_file_size.is_some()
            || self.max_file_size.is_some()
            || self
                .created_after
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty())
            || self
                .created_before
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty())
            || self
                .last_read_after
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty())
            || self
                .last_read_before
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty())
    }

    pub fn into_search_request(
        self,
        page_numb: Option<u64>,
        page_size: Option<u64>,
    ) -> SearchRequest {
        SearchRequest {
            query: self.query,
            tags: self.tags,
            theme_ids: self.theme_ids,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_theme_ids_when_building_a_dynamic_search_request() {
        let params: CategorySearchParams =
            serde_json::from_str(r#"{"themeIds":["theme-space"],"query":null}"#)
                .expect("dynamic category search params should deserialize theme ids");
        assert!(params.has_filter_criteria());
        let request = params.into_search_request(Some(1), Some(48));
        assert_eq!(request.theme_ids, Some(vec!["theme-space".to_string()]));
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryDeletePreview {
    #[serde(rename = "categoryType")]
    pub category_type: String,
    pub matched: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryBatchDeleteResult {
    #[serde(rename = "categoryType")]
    pub category_type: String,
    pub matched: u64,
    pub deleted: u64,
    pub failed: u64,
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
