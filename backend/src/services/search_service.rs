use std::collections::HashMap;
use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite, Row};
use tracing::debug;
use chrono::{DateTime, Utc};

use crate::models::{Archive, PaginatedResponse, SearchRequest, Tag};

pub struct SearchService {
    db: Pool<Sqlite>,
}

impl SearchService {
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    pub async fn search_archives(&self, params: SearchRequest) -> Result<PaginatedResponse<Archive>> {
        let limit = params.limit.unwrap_or(20).min(100) as i64;
        let offset = ((params.page.unwrap_or(1) - 1) * limit as u32) as i64;
        
        debug!("Searching archives with params: {:?}", params);

        let (where_clause, mut bind_values) = self.build_where_clause(&params)?;
        let order_clause = self.build_order_clause(&params);

        let count_query = format!(
            "SELECT COUNT(DISTINCT a.id) as total FROM archives a {} {}",
            self.get_joins(&params),
            where_clause
        );

        let search_query = format!(
            r#"
            SELECT DISTINCT a.id, a.title, a.path, a.file_size, 
                   COALESCE(a.page_count, 0) as page_count, a.file_hash, 
                   a.created_at, a.updated_at
            FROM archives a
            {}
            {}
            {}
            LIMIT ? OFFSET ?
            "#,
            self.get_joins(&params),
            where_clause,
            order_clause
        );

        let total_count = self.execute_count_query(&count_query, &bind_values).await?;
        
        bind_values.push(limit.to_string());
        bind_values.push(offset.to_string());
        
        let archives = self.execute_search_query(&search_query, &bind_values).await?;

        let has_next = offset + limit < total_count as i64;

        Ok(PaginatedResponse {
            data: archives,
            page: params.page.unwrap_or(1),
            limit: limit as u32,
            total: total_count,
            has_next,
        })
    }

    pub async fn get_all_tags(&self) -> Result<Vec<Tag>> {
        let tags = sqlx::query("SELECT id, name, namespace FROM tags ORDER BY namespace, name")
            .fetch_all(&self.db)
            .await
            .context("Failed to fetch tags")?;

        let result = tags.into_iter().map(|row| {
            Tag {
                id: row.get::<String, _>("id").parse().unwrap_or(0),
                name: row.get("name"),
                namespace: row.get("namespace"),
            }
        }).collect();

        Ok(result)
    }

    pub async fn get_tags_by_archive(&self, archive_id: &str) -> Result<Vec<Tag>> {
        let tags = sqlx::query(
            r#"
            SELECT t.id, t.name, t.namespace 
            FROM tags t
            INNER JOIN archive_tags at ON t.id = at.tag_id
            WHERE at.archive_id = ?
            ORDER BY t.namespace, t.name
            "#
        )
        .bind(archive_id)
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch archive tags")?;

        let result = tags.into_iter().map(|row| {
            Tag {
                id: row.get::<String, _>("id").parse().unwrap_or(0),
                name: row.get("name"),
                namespace: row.get("namespace"),
            }
        }).collect();

        Ok(result)
    }

    pub async fn search_tags(&self, query: &str, limit: Option<u32>) -> Result<Vec<Tag>> {
        let limit = limit.unwrap_or(50).min(100) as i64;
        
        let tags = sqlx::query(
            r#"
            SELECT id, name, namespace 
            FROM tags 
            WHERE name LIKE ? OR namespace LIKE ?
            ORDER BY 
                CASE WHEN name = ? THEN 0 ELSE 1 END,
                namespace, name
            LIMIT ?
            "#
        )
        .bind(format!("%{}%", query))
        .bind(format!("%{}%", query))
        .bind(query)
        .bind(limit)
        .fetch_all(&self.db)
        .await
        .context("Failed to search tags")?;

        let result = tags.into_iter().map(|row| {
            Tag {
                id: row.get::<String, _>("id").parse().unwrap_or(0),
                name: row.get("name"),
                namespace: row.get("namespace"),
            }
        }).collect();

        Ok(result)
    }

    pub async fn get_popular_tags(&self, limit: Option<u32>) -> Result<Vec<(Tag, u32)>> {
        let limit = limit.unwrap_or(20).min(50) as i64;
        
        let tags = sqlx::query(
            r#"
            SELECT t.id, t.name, t.namespace, COUNT(at.archive_id) as usage_count
            FROM tags t
            INNER JOIN archive_tags at ON t.id = at.tag_id
            GROUP BY t.id, t.name, t.namespace
            ORDER BY usage_count DESC, t.name
            LIMIT ?
            "#
        )
        .bind(limit)
        .fetch_all(&self.db)
        .await
        .context("Failed to fetch popular tags")?;

        let result = tags.into_iter().map(|row| {
            let tag = Tag {
                id: row.get::<String, _>("id").parse().unwrap_or(0),
                name: row.get("name"),
                namespace: row.get("namespace"),
            };
            let count: i64 = row.get("usage_count");
            (tag, count as u32)
        }).collect();

        Ok(result)
    }

    fn build_where_clause(&self, params: &SearchRequest) -> Result<(String, Vec<String>)> {
        let mut conditions = Vec::new();
        let mut bind_values = Vec::new();

        if let Some(query) = &params.query {
            if !query.trim().is_empty() {
                conditions.push("a.title LIKE ?".to_string());
                bind_values.push(format!("%{}%", query));
            }
        }

        if let Some(tags) = &params.tags {
            if !tags.is_empty() {
                let tag_placeholders = tags.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                conditions.push(format!(
                    "a.id IN (SELECT DISTINCT at.archive_id FROM archive_tags at INNER JOIN tags t ON at.tag_id = t.id WHERE t.name IN ({}))",
                    tag_placeholders
                ));
                for tag in tags {
                    bind_values.push(tag.clone());
                }
            }
        }

        if let Some(min_pages) = params.min_pages {
            conditions.push("COALESCE(a.page_count, 0) >= ?".to_string());
            bind_values.push(min_pages.to_string());
        }

        if let Some(max_pages) = params.max_pages {
            conditions.push("COALESCE(a.page_count, 0) <= ?".to_string());
            bind_values.push(max_pages.to_string());
        }

        if let Some(min_size) = params.min_file_size {
            conditions.push("a.file_size >= ?".to_string());
            bind_values.push(min_size.to_string());
        }

        if let Some(max_size) = params.max_file_size {
            conditions.push("a.file_size <= ?".to_string());
            bind_values.push(max_size.to_string());
        }

        if let Some(created_after) = &params.created_after {
            conditions.push("a.created_at >= ?".to_string());
            bind_values.push(created_after.clone());
        }

        if let Some(created_before) = &params.created_before {
            conditions.push("a.created_at <= ?".to_string());
            bind_values.push(created_before.clone());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        Ok((where_clause, bind_values))
    }

    fn build_order_clause(&self, params: &SearchRequest) -> String {
        let sort_by = params.sort_by.as_deref().unwrap_or("created_at");
        let sort_order = params.sort_order.as_deref().unwrap_or("desc");

        let valid_sort_by = match sort_by {
            "title" => "a.title",
            "file_size" => "a.file_size",
            "page_count" => "COALESCE(a.page_count, 0)",
            "updated_at" => "a.updated_at",
            _ => "a.created_at",
        };

        let valid_sort_order = match sort_order.to_lowercase().as_str() {
            "asc" => "ASC",
            _ => "DESC",
        };

        format!("ORDER BY {} {}", valid_sort_by, valid_sort_order)
    }

    fn get_joins(&self, params: &SearchRequest) -> String {
        let mut joins = Vec::new();

        if params.tags.is_some() {
            joins.push("LEFT JOIN archive_tags at ON a.id = at.archive_id".to_string());
            joins.push("LEFT JOIN tags t ON at.tag_id = t.id".to_string());
        }

        joins.join(" ")
    }

    async fn execute_count_query(&self, query: &str, bind_values: &[String]) -> Result<u32> {
        let mut sqlx_query = sqlx::query(query);
        for value in bind_values {
            sqlx_query = sqlx_query.bind(value);
        }

        let row = sqlx_query
            .fetch_one(&self.db)
            .await
            .context("Failed to execute count query")?;

        let total: i64 = row.get("total");
        Ok(total as u32)
    }

    async fn execute_search_query(&self, query: &str, bind_values: &[String]) -> Result<Vec<Archive>> {
        let mut sqlx_query = sqlx::query(query);
        for value in bind_values {
            sqlx_query = sqlx_query.bind(value);
        }

        let rows = sqlx_query
            .fetch_all(&self.db)
            .await
            .context("Failed to execute search query")?;

        let mut archives = Vec::new();
        let mut archive_ids_for_tags = Vec::new();

        for row in rows {
            let id: String = row.get("id");
            let title: String = row.get("title");
            let path: String = row.get("path");
            let file_size: i64 = row.get("file_size");
            let page_count: i32 = row.get("page_count");
            let hash: String = row.get("file_hash");
            let created_at: DateTime<Utc> = row.get("created_at");
            let updated_at: DateTime<Utc> = row.get("updated_at");

            archive_ids_for_tags.push(id.clone());

            archives.push(Archive {
                id,
                title,
                path,
                file_size,
                page_count,
                hash,
                created_at,
                updated_at,
                tags: vec![],
            });
        }

        self.populate_archive_tags(&mut archives, &archive_ids_for_tags).await?;

        Ok(archives)
    }

    async fn populate_archive_tags(&self, archives: &mut [Archive], archive_ids: &[String]) -> Result<()> {
        if archive_ids.is_empty() {
            return Ok(());
        }

        let placeholders = archive_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let tags_query = format!(
            r#"
            SELECT at.archive_id, t.id, t.name, t.namespace
            FROM archive_tags at
            INNER JOIN tags t ON at.tag_id = t.id
            WHERE at.archive_id IN ({})
            ORDER BY t.namespace, t.name
            "#,
            placeholders
        );

        let mut sqlx_query = sqlx::query(&tags_query);
        for archive_id in archive_ids {
            sqlx_query = sqlx_query.bind(archive_id);
        }

        let tag_rows = sqlx_query
            .fetch_all(&self.db)
            .await
            .context("Failed to fetch archive tags")?;

        let mut tags_by_archive: HashMap<String, Vec<Tag>> = HashMap::new();
        for row in tag_rows {
            let archive_id: String = row.get("archive_id");
            let tag = Tag {
                id: row.get::<String, _>("id").parse().unwrap_or(0),
                name: row.get("name"),
                namespace: row.get("namespace"),
            };

            tags_by_archive.entry(archive_id).or_default().push(tag);
        }

        for archive in archives {
            if let Some(tags) = tags_by_archive.remove(&archive.id) {
                archive.tags = tags;
            }
        }

        Ok(())
    }
}