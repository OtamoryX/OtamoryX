use anyhow::{Context, Result};
use sqlx::{Pool, Row, Sqlite};
use tracing::debug;

use crate::models::{Archive, PaginatedResponse, SearchRequest, TagModel};
use crate::services::{ArchiveFilters, ArchiveQueryService, PaginationParams, QueryOptions};

pub struct SearchService {
    query_service: ArchiveQueryService,
}

impl SearchService {
    pub fn new(db: Pool<Sqlite>) -> Self {
        let query_service = ArchiveQueryService::new(db.clone());
        Self { query_service }
    }

    /// 搜索档案 - 现在使用统一的查询服务
    pub async fn search_archives(
        &self,
        params: SearchRequest,
        user_id: &str,
    ) -> Result<PaginatedResponse<Archive>> {
        debug!("Searching archives with params: {:?}", params);

        let filters = ArchiveFilters::from_search_request(&params);
        let pagination = PaginationParams::from_search_request(&params);
        let options = QueryOptions {
            random: false,
            include_tags: true,
            user_id: Some(user_id.to_string()),
        };

        self.query_service
            .query_archives(filters, pagination, options)
            .await
    }

    /// 获取所有标签
    pub async fn get_all_tags(&self) -> Result<Vec<TagModel>> {
        let tags = sqlx::query(
            "SELECT t.id, t.name, t.namespace, l.name AS localized_name \
             FROM tags t LEFT JOIN tag_localizations l \
             ON l.tag_id = t.id AND l.locale = 'zh-Hans' AND l.status = 'completed' \
             ORDER BY t.namespace, t.name",
        )
        .fetch_all(self.query_service.db())
        .await
        .context("Failed to fetch tags")?;

        let result = tags
            .into_iter()
            .map(|row| TagModel {
                id: row.get::<String, _>("id"),
                name: row.get("name"),
                namespace: row.get("namespace"),
                localized_name: row.get("localized_name"),
            })
            .collect();

        Ok(result)
    }

    /// 获取指定档案的标签
    pub async fn get_tags_by_archive(&self, archive_id: &str) -> Result<Vec<TagModel>> {
        let tags = sqlx::query(
            r#"
            SELECT t.id, t.name, t.namespace, l.name AS localized_name
            FROM tags t LEFT JOIN tag_localizations l
            ON l.tag_id = t.id AND l.locale = 'zh-Hans' AND l.status = 'completed'
            INNER JOIN archive_tags at ON t.id = at.tag_id
            WHERE at.archive_id = ?
            ORDER BY t.namespace, t.name
            "#,
        )
        .bind(archive_id)
        .fetch_all(self.query_service.db())
        .await
        .context("Failed to fetch archive tags")?;

        let result = tags
            .into_iter()
            .map(|row| TagModel {
                id: row.get::<String, _>("id"),
                name: row.get("name"),
                namespace: row.get("namespace"),
                localized_name: row.get("localized_name"),
            })
            .collect();

        Ok(result)
    }

    /// 搜索标签
    pub async fn search_tags(&self, query: &str, limit: Option<u32>) -> Result<Vec<TagModel>> {
        let limit = limit.unwrap_or(50).min(100) as i64;

        let tags = sqlx::query(
            r#"
            SELECT t.id, t.name, t.namespace, l.name AS localized_name
            FROM tags t LEFT JOIN tag_localizations l
            ON l.tag_id = t.id AND l.locale = 'zh-Hans' AND l.status = 'completed'
            WHERE t.name LIKE ? OR t.namespace LIKE ? OR l.name LIKE ?
            ORDER BY 
                CASE WHEN t.name = ? OR l.name = ? THEN 0 ELSE 1 END,
                t.namespace, t.name
            LIMIT ?
            "#,
        )
        .bind(format!("%{}%", query))
        .bind(format!("%{}%", query))
        .bind(format!("%{}%", query))
        .bind(query)
        .bind(query)
        .bind(limit)
        .fetch_all(self.query_service.db())
        .await
        .context("Failed to search tags")?;

        let result = tags
            .into_iter()
            .map(|row| TagModel {
                id: row.get::<String, _>("id"),
                name: row.get("name"),
                namespace: row.get("namespace"),
                localized_name: row.get("localized_name"),
            })
            .collect();

        Ok(result)
    }

    /// 获取热门标签
    pub async fn get_popular_tags(&self, limit: Option<u32>) -> Result<Vec<(TagModel, u32)>> {
        let limit = limit.unwrap_or(20).min(50) as i64;

        let tags = sqlx::query(
            r#"
            SELECT t.id, t.name, t.namespace, l.name AS localized_name, COUNT(at.archive_id) as usage_count
            FROM tags t LEFT JOIN tag_localizations l
            ON l.tag_id = t.id AND l.locale = 'zh-Hans' AND l.status = 'completed'
            INNER JOIN archive_tags at ON t.id = at.tag_id
            GROUP BY t.id, t.name, t.namespace
            ORDER BY usage_count DESC, t.name
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(self.query_service.db())
        .await
        .context("Failed to fetch popular tags")?;

        let result = tags
            .into_iter()
            .map(|row| {
                let tag = TagModel {
                    id: row.get::<String, _>("id"),
                    name: row.get("name"),
                    namespace: row.get("namespace"),
                    localized_name: row.get("localized_name"),
                };
                let count: i64 = row.get("usage_count");
                (tag, count as u32)
            })
            .collect();

        Ok(result)
    }

    /// 批量填充档案标签（委托给统一的查询服务）
    pub async fn populate_archive_tags(
        &self,
        archives: &mut [Archive],
        archive_ids: &[String],
    ) -> Result<()> {
        self.query_service
            .populate_archive_tags(archives, archive_ids)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        for statement in [
            "CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL)",
            "CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL)",
            "CREATE TABLE tag_localizations (tag_id TEXT NOT NULL, locale TEXT NOT NULL, name TEXT, status TEXT NOT NULL, PRIMARY KEY(tag_id, locale))",
        ] {
            sqlx::query(statement).execute(&pool).await.unwrap();
        }
        pool
    }

    #[tokio::test]
    async fn exposes_and_searches_simplified_chinese_tag_labels() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES ('tag-1', 'big breasts', 'general')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO tag_localizations (tag_id, locale, name, status) VALUES ('tag-1', 'zh-Hans', '巨乳', 'completed')")
            .execute(&pool)
            .await
            .unwrap();

        let service = SearchService::new(pool);
        let tags = service.get_all_tags().await.unwrap();
        assert_eq!(tags[0].name, "big breasts");
        assert_eq!(tags[0].localized_name.as_deref(), Some("巨乳"));

        let matched = service.search_tags("巨乳", None).await.unwrap();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].id, "tag-1");
    }
}
