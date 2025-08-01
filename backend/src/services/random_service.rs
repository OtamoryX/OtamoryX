use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, Row};
use tracing::debug;
use serde::Deserialize;

use crate::models::Archive;
use crate::services::SearchService;

#[derive(Debug, Clone, Deserialize)]
pub struct RandomArchiveParams {
    pub count: Option<u32>,
    pub tags: Option<Vec<String>>,
    pub min_pages: Option<i32>,
    pub max_pages: Option<i32>,
    pub min_file_size: Option<i64>,
    pub max_file_size: Option<i64>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
    pub exclude_new: Option<bool>,
}

pub struct RandomService {
    db: Pool<Sqlite>,
    search_service: SearchService,
}

impl RandomService {
    pub fn new(db: Pool<Sqlite>) -> Self {
        let search_service = SearchService::new(db.clone());
        Self { db, search_service }
    }

    pub async fn get_random_archives(&self, params: RandomArchiveParams) -> Result<Vec<Archive>> {
        let count = params.count.unwrap_or(20).min(100) as i64;
        debug!("Getting {} random archives with filters", count);

        if self.has_filters(&params) {
            self.get_filtered_random_archives(&params, count).await
        } else {
            self.get_simple_random_archives(count).await
        }
    }

    pub async fn get_random_archive_by_tag(&self, tag_name: &str) -> Result<Option<Archive>> {
        debug!("Getting random archive with tag: {}", tag_name);

        let rows = sqlx::query(
            r#"
            SELECT DISTINCT a.id, a.title, a.path, a.file_size, 
                   COALESCE(a.page_count, 0) as page_count, a.file_hash, 
                   a.created_at, a.updated_at
            FROM archives a
            INNER JOIN archive_tags at ON a.id = at.archive_id
            INNER JOIN tags t ON at.tag_id = t.id
            WHERE t.name = ?
            ORDER BY RANDOM()
            LIMIT 1
            "#
        )
        .bind(tag_name)
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch random archive by tag")?;

        match rows {
            Some(row) => {
                let mut archive = self.row_to_archive(row)?;
                let tags = self.search_service.get_tags_by_archive(&archive.id).await?;
                archive.tags = tags;
                Ok(Some(archive))
            }
            None => Ok(None),
        }
    }

    pub async fn get_unread_random_archives(&self, count: Option<u32>) -> Result<Vec<Archive>> {
        let count = count.unwrap_or(10).min(50) as i64;
        debug!("Getting {} random unread archives", count);

        let rows = sqlx::query(
            r#"
            SELECT DISTINCT a.id, a.title, a.path, a.file_size, 
                   COALESCE(a.page_count, 0) as page_count, a.file_hash, 
                   a.created_at, a.updated_at
            FROM archives a
            LEFT JOIN reading_progress rp ON a.id = rp.archive_id
            WHERE rp.archive_id IS NULL OR rp.current_page = 0
            ORDER BY RANDOM()
            LIMIT ?
            "#
        )
        .bind(count)
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch unread random archives")?;

        let mut archives = Vec::new();
        for row in rows {
            let mut archive = self.row_to_archive(row)?;
            let tags = self.search_service.get_tags_by_archive(&archive.id).await?;
            archive.tags = tags;
            archives.push(archive);
        }

        Ok(archives)
    }

    pub async fn get_random_archives_by_date_range(
        &self, 
        start_date: &str, 
        end_date: &str, 
        count: Option<u32>
    ) -> Result<Vec<Archive>> {
        let count = count.unwrap_or(10).min(50) as i64;
        debug!("Getting {} random archives between {} and {}", count, start_date, end_date);

        let rows = sqlx::query(
            r#"
            SELECT DISTINCT a.id, a.title, a.path, a.file_size, 
                   COALESCE(a.page_count, 0) as page_count, a.file_hash, 
                   a.created_at, a.updated_at
            FROM archives a
            WHERE a.created_at >= ? AND a.created_at <= ?
            ORDER BY RANDOM()
            LIMIT ?
            "#
        )
        .bind(start_date)
        .bind(end_date)
        .bind(count)
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch random archives by date range")?;

        let mut archives = Vec::new();
        for row in rows {
            let mut archive = self.row_to_archive(row)?;
            let tags = self.search_service.get_tags_by_archive(&archive.id).await?;
            archive.tags = tags;
            archives.push(archive);
        }

        Ok(archives)
    }

    pub async fn get_random_archives_with_minimum_pages(&self, min_pages: i32, count: Option<u32>) -> Result<Vec<Archive>> {
        let count = count.unwrap_or(10).min(50) as i64;
        debug!("Getting {} random archives with at least {} pages", count, min_pages);

        let rows = sqlx::query(
            r#"
            SELECT DISTINCT a.id, a.title, a.path, a.file_size, 
                   COALESCE(a.page_count, 0) as page_count, a.file_hash, 
                   a.created_at, a.updated_at
            FROM archives a
            WHERE COALESCE(a.page_count, 0) >= ?
            ORDER BY RANDOM()
            LIMIT ?
            "#
        )
        .bind(min_pages)
        .bind(count)
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch random archives with minimum pages")?;

        let mut archives = Vec::new();
        for row in rows {
            let mut archive = self.row_to_archive(row)?;
            let tags = self.search_service.get_tags_by_archive(&archive.id).await?;
            archive.tags = tags;
            archives.push(archive);
        }

        Ok(archives)
    }

    fn has_filters(&self, params: &RandomArchiveParams) -> bool {
        params.tags.is_some() 
            || params.min_pages.is_some() 
            || params.max_pages.is_some()
            || params.min_file_size.is_some()
            || params.max_file_size.is_some()
            || params.created_after.is_some()
            || params.created_before.is_some()
            || params.exclude_new.unwrap_or(false)
    }

    async fn get_filtered_random_archives(&self, params: &RandomArchiveParams, count: i64) -> Result<Vec<Archive>> {
        let mut where_conditions = Vec::new();
        let mut bind_values = Vec::new();

        if let Some(tags) = &params.tags {
            if !tags.is_empty() {
                let tag_placeholders = tags.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                where_conditions.push(format!(
                    "a.id IN (SELECT DISTINCT at.archive_id FROM archive_tags at INNER JOIN tags t ON at.tag_id = t.id WHERE t.name IN ({}))",
                    tag_placeholders
                ));
                for tag in tags {
                    bind_values.push(tag.clone());
                }
            }
        }

        if let Some(min_pages) = params.min_pages {
            where_conditions.push("COALESCE(a.page_count, 0) >= ?".to_string());
            bind_values.push(min_pages.to_string());
        }

        if let Some(max_pages) = params.max_pages {
            where_conditions.push("COALESCE(a.page_count, 0) <= ?".to_string());
            bind_values.push(max_pages.to_string());
        }

        if let Some(min_size) = params.min_file_size {
            where_conditions.push("a.file_size >= ?".to_string());
            bind_values.push(min_size.to_string());
        }

        if let Some(max_size) = params.max_file_size {
            where_conditions.push("a.file_size <= ?".to_string());
            bind_values.push(max_size.to_string());
        }

        if let Some(created_after) = &params.created_after {
            where_conditions.push("a.created_at >= ?".to_string());
            bind_values.push(created_after.clone());
        }

        if let Some(created_before) = &params.created_before {
            where_conditions.push("a.created_at <= ?".to_string());
            bind_values.push(created_before.clone());
        }

        if params.exclude_new.unwrap_or(false) {
            where_conditions.push(
                "a.id NOT IN (SELECT DISTINCT at.archive_id FROM archive_tags at INNER JOIN tags t ON at.tag_id = t.id WHERE t.name = 'new' AND t.namespace = 'system')"
                .to_string()
            );
        }

        let where_clause = if where_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_conditions.join(" AND "))
        };

        let query = format!(
            r#"
            SELECT DISTINCT a.id, a.title, a.path, a.file_size, 
                   COALESCE(a.page_count, 0) as page_count, a.file_hash, 
                   a.created_at, a.updated_at
            FROM archives a
            {}
            ORDER BY RANDOM()
            LIMIT ?
            "#,
            where_clause
        );

        bind_values.push(count.to_string());

        let mut sqlx_query = sqlx::query(&query);
        for value in &bind_values {
            sqlx_query = sqlx_query.bind(value);
        }

        let rows = sqlx_query
            .fetch_all(&self.db)
            .await
            .context("Failed to fetch filtered random archives")?;

        let mut archives = Vec::new();
        for row in rows {
            let mut archive = self.row_to_archive(row)?;
            let tags = self.search_service.get_tags_by_archive(&archive.id).await?;
            archive.tags = tags;
            archives.push(archive);
        }

        Ok(archives)
    }

    async fn get_simple_random_archives(&self, count: i64) -> Result<Vec<Archive>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT a.id, a.title, a.path, a.file_size, 
                   COALESCE(a.page_count, 0) as page_count, a.file_hash, 
                   a.created_at, a.updated_at
            FROM archives a
            ORDER BY RANDOM()
            LIMIT ?
            "#
        )
        .bind(count)
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch random archives")?;

        let mut archives = Vec::new();
        for row in rows {
            let mut archive = self.row_to_archive(row)?;
            let tags = self.search_service.get_tags_by_archive(&archive.id).await?;
            archive.tags = tags;
            archives.push(archive);
        }

        Ok(archives)
    }

    fn row_to_archive(&self, row: sqlx::sqlite::SqliteRow) -> Result<Archive> {
        Ok(Archive {
            id: row.get("id"),
            title: row.get("title"),
            path: row.get("path"),
            file_size: row.get("file_size"),
            page_count: row.get("page_count"),
            hash: row.get("file_hash"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            tags: vec![], // Will be populated by caller
        })
    }
}