use anyhow::Result;
use serde::Deserialize;
use sqlx::{Pool, Sqlite};
use tracing::debug;

use crate::models::{deserialize_comma_separated, Archive};
use crate::services::{ArchiveFilters, ArchiveQueryService, PaginationParams, QueryOptions};

#[derive(Debug, Clone, Deserialize)]
pub struct RandomArchiveParams {
    pub count: Option<u32>,
    #[serde(default, deserialize_with = "deserialize_comma_separated")]
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
    pub exclude_new: Option<bool>,
    pub category_id: Option<String>,
    pub query: Option<String>,
}

pub struct RandomService {
    query_service: ArchiveQueryService,
}

impl RandomService {
    pub fn new(db: Pool<Sqlite>) -> Self {
        let query_service = ArchiveQueryService::new(db);
        Self { query_service }
    }

    pub async fn get_random_archives(&self, params: RandomArchiveParams) -> Result<Vec<Archive>> {
        debug!("Getting random archives with filters: {:?}", params);

        let mut filters = ArchiveFilters::from_random_params(&params);

        // 如果指定了分类，先获取该分类下的 archive_ids
        if let Some(ref category_id) = params.category_id {
            let archive_ids: Vec<String> = sqlx::query_scalar(
                "SELECT archive_id FROM category_archives WHERE category_id = ?",
            )
            .bind(category_id)
            .fetch_all(self.query_service.db())
            .await?;

            if archive_ids.is_empty() {
                return Ok(vec![]);
            }
            filters.archive_ids = Some(archive_ids);
        }

        let pagination = PaginationParams::from_random_params(params.count);
        let options = QueryOptions {
            random: true,
            include_tags: true,
            user_id: None,
        };

        let response = self
            .query_service
            .query_archives(filters, pagination, options)
            .await?;
        Ok(response.data)
    }

    pub async fn get_random_archive_by_tag(&self, tag_name: &str) -> Result<Option<Archive>> {
        debug!("Getting random archive with tag: {}", tag_name);

        let filters = ArchiveFilters {
            tags: Some(vec![tag_name.to_string()]),
            ..Default::default()
        };
        let pagination = PaginationParams::from_random_params(Some(1));
        let options = QueryOptions {
            random: true,
            include_tags: true,
            user_id: None,
        };

        let response = self
            .query_service
            .query_archives(filters, pagination, options)
            .await?;
        Ok(response.data.into_iter().next())
    }

    pub async fn get_unread_random_archives(&self, count: Option<u32>) -> Result<Vec<Archive>> {
        debug!("Getting random unread archives");

        let filters = ArchiveFilters {
            unread_only: Some(true),
            ..Default::default()
        };
        let pagination = PaginationParams::from_random_params(count);
        let options = QueryOptions {
            random: true,
            include_tags: true,
            user_id: None,
        };

        let response = self
            .query_service
            .query_archives(filters, pagination, options)
            .await?;
        Ok(response.data)
    }

    pub async fn get_random_archives_by_date_range(
        &self,
        start_date: &str,
        end_date: &str,
        count: Option<u32>,
    ) -> Result<Vec<Archive>> {
        debug!(
            "Getting random archives between {} and {}",
            start_date, end_date
        );

        let filters = ArchiveFilters {
            created_after: Some(start_date.to_string()),
            created_before: Some(end_date.to_string()),
            ..Default::default()
        };
        let pagination = PaginationParams::from_random_params(count);
        let options = QueryOptions {
            random: true,
            include_tags: true,
            user_id: None,
        };

        let response = self
            .query_service
            .query_archives(filters, pagination, options)
            .await?;
        Ok(response.data)
    }

    pub async fn get_random_archives_with_minimum_pages(
        &self,
        min_pages: i32,
        count: Option<u32>,
    ) -> Result<Vec<Archive>> {
        debug!("Getting random archives with at least {} pages", min_pages);

        let filters = ArchiveFilters {
            min_pages: Some(min_pages),
            ..Default::default()
        };
        let pagination = PaginationParams::from_random_params(count);
        let options = QueryOptions {
            random: true,
            include_tags: true,
            user_id: None,
        };

        let response = self
            .query_service
            .query_archives(filters, pagination, options)
            .await?;
        Ok(response.data)
    }
}
