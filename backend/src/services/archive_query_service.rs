use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{Pool, Row, Sqlite};
use std::collections::HashMap;
use tracing::debug;

/// 支持多类型的绑定值，避免数值被当作字符串绑定到 SQLite 导致字典序比较
#[derive(Debug, Clone)]
pub enum BindValue {
    String(String),
    Int(i64),
}

use crate::models::{Archive, PaginatedResponse, TagModel};

#[derive(Debug, Clone, Default)]
pub struct ArchiveFilters {
    pub query: Option<String>,
    pub tags: Option<Vec<String>>,
    pub min_pages: Option<i32>,
    pub max_pages: Option<i32>,
    pub min_file_size: Option<i64>,
    pub max_file_size: Option<i64>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
    pub last_read_after: Option<String>,
    pub last_read_before: Option<String>,
    pub archive_ids: Option<Vec<String>>,         // 用于分类过滤
    pub exclude_archive_ids: Option<Vec<String>>, // 排除特定档案
    pub unread_only: Option<bool>,                // 只查询未读档案
}

#[derive(Debug, Clone)]
pub struct PaginationParams {
    pub page_numb: u64,
    pub page_size: u64,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    pub random: bool,
    pub include_tags: bool,
    pub user_id: Option<String>, // 用于权限检查
}

/// 统一的档案查询服务，消除重复代码
pub struct ArchiveQueryService {
    db: Pool<Sqlite>,
}

#[derive(Debug, Clone)]
pub struct ArchiveDeleteTarget {
    pub id: String,
    pub path: String,
}

impl ArchiveQueryService {
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &Pool<Sqlite> {
        &self.db
    }

    /// 统一的档案查询接口，支持过滤、分页、排序
    pub async fn query_archives(
        &self,
        filters: ArchiveFilters,
        pagination: PaginationParams,
        options: QueryOptions,
    ) -> Result<PaginatedResponse<Archive>> {
        let limit = pagination.page_size as i64;
        let offset = ((pagination.page_numb - 1) * pagination.page_size) as i64;

        debug!(
            "Querying archives with filters: {:?}, pagination: {:?}, options: {:?}",
            filters, pagination, options
        );

        let (where_clause, bind_values) = self.build_where_clause(&filters, &options)?;
        let order_clause = self.build_order_clause(&pagination, options.random);
        let joins = self.get_joins(&filters);

        // 构建计数查询
        let count_query = format!(
            "SELECT COUNT(DISTINCT a.id) as total FROM archives a {} {}",
            joins, where_clause
        );

        // 构建数据查询
        let data_query = format!(
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
            joins, where_clause, order_clause
        );

        // 执行计数查询
        let total = self.execute_count_query(&count_query, &bind_values).await?;

        // 执行数据查询
        let mut bind_values_with_pagination = bind_values;
        bind_values_with_pagination.push(BindValue::Int(limit));
        bind_values_with_pagination.push(BindValue::Int(offset));

        let mut archives = self
            .execute_data_query(&data_query, &bind_values_with_pagination)
            .await?;

        // 填充标签信息
        if options.include_tags && !archives.is_empty() {
            let archive_ids: Vec<String> = archives.iter().map(|a| a.id.clone()).collect();
            self.populate_archive_tags(&mut archives, &archive_ids)
                .await?;
        }

        let has_next = offset + limit < total as i64;

        Ok(PaginatedResponse {
            data: archives,
            page_numb: pagination.page_numb,
            page_size: pagination.page_size,
            total,
            has_next,
        })
    }

    /// Resolve every archive matching the filters without pagination or tag hydration.
    pub async fn query_delete_targets(
        &self,
        filters: ArchiveFilters,
        options: QueryOptions,
    ) -> Result<Vec<ArchiveDeleteTarget>> {
        let (where_clause, bind_values) = self.build_where_clause(&filters, &options)?;
        let joins = self.get_joins(&filters);
        let query = format!(
            "SELECT DISTINCT a.id, a.path FROM archives a {} {}",
            joins, where_clause
        );

        let mut sqlx_query = sqlx::query(&query);
        for value in &bind_values {
            sqlx_query = match value {
                BindValue::String(value) => sqlx_query.bind(value),
                BindValue::Int(value) => sqlx_query.bind(value),
            };
        }

        let rows = sqlx_query
            .fetch_all(&self.db)
            .await
            .context("Failed to resolve archive delete targets")?;

        Ok(rows
            .into_iter()
            .map(|row| ArchiveDeleteTarget {
                id: row.get("id"),
                path: row.get("path"),
            })
            .collect())
    }

    pub async fn count_matching_archives(
        &self,
        filters: ArchiveFilters,
        options: QueryOptions,
    ) -> Result<u64> {
        let (where_clause, bind_values) = self.build_where_clause(&filters, &options)?;
        let joins = self.get_joins(&filters);
        let query = format!(
            "SELECT COUNT(DISTINCT a.id) as total FROM archives a {} {}",
            joins, where_clause
        );

        self.execute_count_query(&query, &bind_values).await
    }

    /// 获取单个档案及其标签
    pub async fn get_archive_with_tags(&self, archive_id: &str) -> Result<Option<Archive>> {
        let archive_opt = sqlx::query(
            r#"
            SELECT id, title, path, file_size, 
                   COALESCE(page_count, 0) as page_count, file_hash, 
                   created_at, updated_at
            FROM archives 
            WHERE id = ?
            "#,
        )
        .bind(archive_id)
        .fetch_optional(&self.db)
        .await
        .context("Failed to fetch archive")?;

        if let Some(row) = archive_opt {
            let archive = Self::row_to_archive(row)?;

            // 填充标签
            let archive_ids = vec![archive.id.clone()];
            let mut archives = vec![archive];
            self.populate_archive_tags(&mut archives, &archive_ids)
                .await?;

            Ok(archives.into_iter().next())
        } else {
            Ok(None)
        }
    }

    /// 批量填充档案标签
    pub async fn populate_archive_tags(
        &self,
        archives: &mut [Archive],
        archive_ids: &[String],
    ) -> Result<()> {
        if archive_ids.is_empty() {
            return Ok(());
        }

        let placeholders = archive_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

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

        let mut query = sqlx::query(&tags_query);
        for archive_id in archive_ids {
            query = query.bind(archive_id);
        }

        let tag_rows = query
            .fetch_all(&self.db)
            .await
            .context("Failed to fetch archive tags")?;

        // 按档案ID分组标签
        let mut tags_by_archive: HashMap<String, Vec<TagModel>> = HashMap::new();
        for row in tag_rows {
            let archive_id: String = row.get("archive_id");
            let tag = TagModel {
                id: row.get::<String, _>("id"),
                name: row.get("name"),
                namespace: row.get("namespace"),
            };
            tags_by_archive.entry(archive_id).or_default().push(tag);
        }

        // 为每个档案分配标签
        for archive in archives {
            if let Some(tags) = tags_by_archive.remove(&archive.id) {
                archive.tags = tags;
            }
        }

        Ok(())
    }

    /// 构建 WHERE 子句
    fn build_where_clause(
        &self,
        filters: &ArchiveFilters,
        options: &QueryOptions,
    ) -> Result<(String, Vec<BindValue>)> {
        let mut conditions = Vec::new();
        let mut bind_values: Vec<BindValue> = Vec::new();

        // 标题搜索
        if let Some(query) = &filters.query {
            if !query.trim().is_empty() {
                conditions.push("a.title LIKE ?".to_string());
                bind_values.push(BindValue::String(format!("%{}%", query)));
            }
        }

        // 标签过滤 (取交集: archive 必须同时拥有所有指定的 tags)
        if let Some(tags) = &filters.tags {
            if !tags.is_empty() {
                let tag_placeholders = tags.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                let tag_count = tags.len();
                conditions.push(format!(
                    "a.id IN (SELECT at.archive_id FROM archive_tags at INNER JOIN tags t ON at.tag_id = t.id WHERE (t.namespace || ':' || t.name) IN ({}) GROUP BY at.archive_id HAVING COUNT(DISTINCT t.id) = {})",
                    tag_placeholders, tag_count
                ));
                for tag in tags {
                    bind_values.push(BindValue::String(tag.clone()));
                }
            }
        }

        // 档案ID过滤（用于分类）
        if let Some(archive_ids) = &filters.archive_ids {
            if !archive_ids.is_empty() {
                let id_placeholders = archive_ids
                    .iter()
                    .map(|_| "?")
                    .collect::<Vec<_>>()
                    .join(",");
                conditions.push(format!("a.id IN ({})", id_placeholders));
                for id in archive_ids {
                    bind_values.push(BindValue::String(id.clone()));
                }
            }
        }

        // 排除档案ID
        if let Some(exclude_ids) = &filters.exclude_archive_ids {
            if !exclude_ids.is_empty() {
                let id_placeholders = exclude_ids
                    .iter()
                    .map(|_| "?")
                    .collect::<Vec<_>>()
                    .join(",");
                conditions.push(format!("a.id NOT IN ({})", id_placeholders));
                for id in exclude_ids {
                    bind_values.push(BindValue::String(id.clone()));
                }
            }
        }

        // 页数过滤（使用整数绑定）
        if let Some(min_pages) = filters.min_pages {
            conditions.push("COALESCE(a.page_count, 0) >= ?".to_string());
            bind_values.push(BindValue::Int(min_pages as i64));
        }

        if let Some(max_pages) = filters.max_pages {
            conditions.push("COALESCE(a.page_count, 0) <= ?".to_string());
            bind_values.push(BindValue::Int(max_pages as i64));
        }

        // 文件大小过滤（使用整数绑定）
        if let Some(min_size) = filters.min_file_size {
            conditions.push("a.file_size >= ?".to_string());
            bind_values.push(BindValue::Int(min_size));
        }

        if let Some(max_size) = filters.max_file_size {
            conditions.push("a.file_size <= ?".to_string());
            bind_values.push(BindValue::Int(max_size));
        }

        // 创建时间过滤 (日期字符串比较，需要处理时间部分以确保包含首尾日期)
        if let Some(created_after) = &filters.created_after {
            conditions.push("date(a.created_at) >= ?".to_string());
            bind_values.push(BindValue::String(created_after.clone()));
        }

        if let Some(created_before) = &filters.created_before {
            conditions.push("date(a.created_at) <= ?".to_string());
            bind_values.push(BindValue::String(created_before.clone()));
        }

        // 阅读进度过滤（需要用户ID）
        let needs_progress_join = filters.last_read_after.is_some()
            || filters.last_read_before.is_some()
            || filters.unread_only.unwrap_or(false);

        if needs_progress_join {
            if let Some(user_id) = &options.user_id {
                conditions.push("rp.user_id = ?".to_string());
                bind_values.push(BindValue::String(user_id.clone()));
            }
        }

        // 未读档案过滤
        if filters.unread_only.unwrap_or(false) {
            conditions.push("(rp.archive_id IS NULL OR rp.current_page = 0)".to_string());
        }

        if let Some(last_read_after) = &filters.last_read_after {
            conditions.push("rp.last_read_at >= ?".to_string());
            bind_values.push(BindValue::String(last_read_after.clone()));
        }

        if let Some(last_read_before) = &filters.last_read_before {
            conditions.push("rp.last_read_at <= ?".to_string());
            bind_values.push(BindValue::String(last_read_before.clone()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        Ok((where_clause, bind_values))
    }

    /// 构建 ORDER 子句
    fn build_order_clause(&self, pagination: &PaginationParams, random: bool) -> String {
        if random {
            return "ORDER BY RANDOM()".to_string();
        }

        let sort_by = pagination.sort_by.as_deref().unwrap_or("created_at");
        let sort_order = pagination.sort_order.as_deref().unwrap_or("desc");

        let valid_sort_by = match sort_by {
            "title" => "a.title",
            "fileSize" | "file_size" => "a.file_size",
            "pageCount" | "page_count" => "COALESCE(a.page_count, 0)",
            "updatedAt" | "updated_at" => "a.updated_at",
            "createdAt" | "created_at" => "a.created_at",
            "lastReadAt" => "COALESCE(rp.last_read_at, '1900-01-01 00:00:00')",
            _ => "a.created_at",
        };

        let valid_sort_order = match sort_order.to_lowercase().as_str() {
            "asc" => "ASC",
            _ => "DESC",
        };

        format!("ORDER BY {} {}", valid_sort_by, valid_sort_order)
    }

    /// 构建 JOIN 子句
    fn get_joins(&self, filters: &ArchiveFilters) -> String {
        let mut joins = Vec::new();

        // 注意: tags 过滤通过 IN 子查询实现，不需要在主查询中 JOIN
        // 这样避免因为一个 archive 有多个 tag 而产生重复行

        let needs_progress_join = filters.last_read_after.is_some()
            || filters.last_read_before.is_some()
            || filters.unread_only.unwrap_or(false);

        if needs_progress_join {
            joins.push("LEFT JOIN reading_progress rp ON a.id = rp.archive_id".to_string());
        }

        joins.join(" ")
    }

    /// 执行计数查询
    async fn execute_count_query(&self, query: &str, bind_values: &[BindValue]) -> Result<u64> {
        let mut sqlx_query = sqlx::query(query);
        for value in bind_values {
            sqlx_query = match value {
                BindValue::String(s) => sqlx_query.bind(s),
                BindValue::Int(i) => sqlx_query.bind(i),
            };
        }

        let row = sqlx_query
            .fetch_one(&self.db)
            .await
            .context("Failed to execute count query")?;

        let total: i64 = row.get("total");
        Ok(total as u64)
    }

    /// 执行数据查询
    async fn execute_data_query(
        &self,
        query: &str,
        bind_values: &[BindValue],
    ) -> Result<Vec<Archive>> {
        let mut sqlx_query = sqlx::query(query);
        for value in bind_values {
            sqlx_query = match value {
                BindValue::String(s) => sqlx_query.bind(s),
                BindValue::Int(i) => sqlx_query.bind(i),
            };
        }

        let rows = sqlx_query
            .fetch_all(&self.db)
            .await
            .context("Failed to execute data query")?;

        let mut archives = Vec::new();
        for row in rows {
            archives.push(Self::row_to_archive(row)?);
        }

        Ok(archives)
    }

    /// 将数据库行转换为 Archive 结构
    fn row_to_archive(row: sqlx::sqlite::SqliteRow) -> Result<Archive> {
        Ok(Archive {
            id: row.get::<String, _>("id"),
            title: row.get("title"),
            path: row.get("path"),
            file_size: row.get("file_size"),
            page_count: row.get::<i32, _>("page_count"),
            hash: row.get("file_hash"),
            created_at: {
                let dt: DateTime<Utc> = row.get("created_at");
                dt
            },
            updated_at: {
                let dt: DateTime<Utc> = row.get("updated_at");
                dt
            },
            tags: vec![], // 标签单独填充
        })
    }
}

// 便利转换函数
impl ArchiveFilters {
    pub fn from_search_request(req: &crate::models::SearchRequest) -> Self {
        Self {
            query: req.query.clone(),
            tags: req.tags.clone(),
            min_pages: req.min_pages,
            max_pages: req.max_pages,
            min_file_size: req.min_file_size,
            max_file_size: req.max_file_size,
            created_after: req.created_after.clone(),
            created_before: req.created_before.clone(),
            last_read_after: req.last_read_after.clone(),
            last_read_before: req.last_read_before.clone(),
            archive_ids: None,
            exclude_archive_ids: None,
            unread_only: None,
        }
    }
}

impl PaginationParams {
    pub fn from_search_request(req: &crate::models::SearchRequest) -> Self {
        Self {
            page_numb: req.page_numb.unwrap_or(1),
            page_size: req.page_size.unwrap_or(20),
            sort_by: req.sort_by.clone(),
            sort_order: req.sort_order.clone(),
        }
    }

    pub fn from_random_params(count: Option<u32>) -> Self {
        Self {
            page_numb: 1,
            page_size: count.unwrap_or(20).min(100) as u64,
            sort_by: None,
            sort_order: None,
        }
    }
}

impl ArchiveFilters {
    pub fn from_random_params(params: &super::random_service::RandomArchiveParams) -> Self {
        Self {
            query: params.query.clone(),
            tags: params.tags.clone(),
            min_pages: params.min_pages,
            max_pages: params.max_pages,
            min_file_size: params.min_file_size,
            max_file_size: params.max_file_size,
            created_after: params.created_after.clone(),
            created_before: params.created_before.clone(),
            last_read_after: None,
            last_read_before: None,
            archive_ids: None,
            exclude_archive_ids: if params.exclude_new.unwrap_or(false) {
                // 这里可以预先查询 'new' 标签的档案 ID，或者在查询时处理
                None
            } else {
                None
            },
            unread_only: None,
        }
    }
}
