use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use sqlx::{Pool, Row, Sqlite};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use crate::middleware::auth::AuthInfo;
use crate::models::tag::{TagDirectoryItem, TagDirectoryResponse};
use crate::models::TagModel;
use crate::services::{
    is_system_managed_theme_namespace, ArchiveCacheService, ArchiveDeleteTarget,
    ArchiveDeletionService,
};

const DEFAULT_DIRECTORY_PAGE_SIZE: u64 = 48;
const MAX_DIRECTORY_PAGE_SIZE: u64 = 200;
const MAX_DIRECTORY_PAGE_NUMBER: u64 = 10_000_000;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TagDirectoryQuery {
    pub kind: Option<String>,
    pub query: Option<String>,
    pub namespace: Option<String>,
    pub sort: Option<String>,
    pub page_numb: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub namespace: String,
}

pub struct TagHandler;

fn internal_database_error(error: sqlx::Error) -> StatusCode {
    tracing::error!(error = %error, "tag directory database query failed");
    StatusCode::INTERNAL_SERVER_ERROR
}

async fn execute_count(
    pool: &Pool<Sqlite>,
    query: &str,
    values: &[String],
) -> Result<u64, StatusCode> {
    let mut request = sqlx::query(query);
    for value in values {
        request = request.bind(value);
    }
    let row = request
        .fetch_one(pool)
        .await
        .map_err(internal_database_error)?;
    Ok(row.get::<i64, _>("total") as u64)
}

impl TagHandler {
    /// POST /api/v1/tags - 创建标签（如已存在则返回现有记录）
    pub async fn create_tag(
        State(pool): State<Pool<Sqlite>>,
        Json(req): Json<CreateTagRequest>,
    ) -> Result<Json<TagModel>, StatusCode> {
        if is_system_managed_theme_namespace(&req.namespace) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query!(
            "INSERT OR IGNORE INTO tags (id, name, namespace) VALUES (?, ?, ?)",
            id,
            req.name,
            req.namespace
        )
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let tag = sqlx::query_as::<_, TagModel>(
            "SELECT t.id, t.name, t.namespace, l.name AS localized_name \
             FROM tags t LEFT JOIN tag_localizations l \
             ON l.tag_id = t.id AND l.locale = 'zh-Hans' AND l.status = 'completed' \
             WHERE t.name = ? AND t.namespace = ?",
        )
        .bind(&req.name)
        .bind(&req.namespace)
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Err(error) = crate::services::enqueue_tag_localization(&pool, &tag.id).await {
            tracing::warn!(tag_id = %tag.id, error = %error, "failed to queue tag localization");
        }

        Ok(Json(tag))
    }

    /// GET /api/v1/tags - 获取标签列表
    pub async fn list_tags(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<Json<Vec<TagModel>>, StatusCode> {
        let tags = sqlx::query_as::<_, TagModel>(
            "SELECT t.id, t.name, t.namespace, l.name AS localized_name \
             FROM tags t LEFT JOIN tag_localizations l \
             ON l.tag_id = t.id AND l.locale = 'zh-Hans' AND l.status = 'completed' \
             ORDER BY t.namespace, t.name",
        )
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Json(tags))
    }

    /// GET /api/v1/tags/directory - 普通标签和 canonical theme 的只读发现目录。
    pub async fn get_tag_directory(
        State(pool): State<Pool<Sqlite>>,
        Query(params): Query<TagDirectoryQuery>,
    ) -> Result<Json<TagDirectoryResponse>, StatusCode> {
        let kind = params.kind.as_deref().unwrap_or("tag");
        let sort = params.sort.as_deref().unwrap_or("usage");
        if !matches!(kind, "tag" | "theme") || !matches!(sort, "usage" | "name") {
            return Err(StatusCode::BAD_REQUEST);
        }

        let page_numb = params
            .page_numb
            .unwrap_or(1)
            .clamp(1, MAX_DIRECTORY_PAGE_NUMBER);
        let page_size = params
            .page_size
            .unwrap_or(DEFAULT_DIRECTORY_PAGE_SIZE)
            .clamp(1, MAX_DIRECTORY_PAGE_SIZE);
        let offset = (page_numb - 1) * page_size;
        let query = params
            .query
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let namespace = params
            .namespace
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let (data, total, namespaces) = if kind == "tag" {
            Self::get_tag_directory_page(&pool, query, namespace, sort, page_size, offset).await?
        } else {
            Self::get_theme_directory_page(&pool, query, namespace, sort, page_size, offset).await?
        };

        Ok(Json(TagDirectoryResponse {
            data,
            page_numb,
            page_size,
            total,
            has_next: offset + page_size < total,
            namespaces,
        }))
    }

    async fn get_tag_directory_page(
        pool: &Pool<Sqlite>,
        query: Option<&str>,
        namespace: Option<&str>,
        sort: &str,
        page_size: u64,
        offset: u64,
    ) -> Result<(Vec<TagDirectoryItem>, u64, Vec<String>), StatusCode> {
        let namespaces = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT t.namespace
             FROM tags t
             WHERE lower(trim(t.namespace)) NOT IN ('system', 'theme')
               AND EXISTS (SELECT 1 FROM archive_tags at WHERE at.tag_id = t.id)
             ORDER BY t.namespace COLLATE NOCASE",
        )
        .fetch_all(pool)
        .await
        .map_err(internal_database_error)?;

        let mut conditions = vec![
            "lower(trim(t.namespace)) NOT IN ('system', 'theme')".to_string(),
            "EXISTS (SELECT 1 FROM archive_tags eligible_at WHERE eligible_at.tag_id = t.id)"
                .to_string(),
        ];
        let mut filter_values = Vec::new();
        if let Some(query) = query {
            conditions.push(
                "(lower(t.name) LIKE ? OR lower(COALESCE(l.name, '')) LIKE ? OR lower(t.namespace) LIKE ?)"
                    .to_string(),
            );
            let pattern = format!("%{}%", query.to_lowercase());
            filter_values.extend([pattern.clone(), pattern.clone(), pattern]);
        }
        if let Some(namespace) = namespace {
            conditions.push("lower(trim(t.namespace)) = lower(trim(?))".to_string());
            filter_values.push(namespace.to_string());
        }
        let where_clause = format!("WHERE {}", conditions.join(" AND "));
        let base_from = "FROM tags t
             LEFT JOIN tag_localizations l
               ON l.tag_id = t.id AND l.locale = 'zh-Hans' AND l.status = 'completed'
             JOIN archive_tags at ON at.tag_id = t.id";

        let count_query =
            format!("SELECT COUNT(DISTINCT t.id) AS total {base_from} {where_clause}");
        let total = execute_count(pool, &count_query, &filter_values).await?;

        let order_clause = if sort == "name" {
            "ORDER BY lower(COALESCE(NULLIF(trim(l.name), ''), t.name)) ASC,
                      lower(t.name) ASC, lower(t.namespace) ASC, t.id ASC"
        } else {
            "ORDER BY archive_count DESC,
                      lower(COALESCE(NULLIF(trim(l.name), ''), t.name)) ASC,
                      lower(t.name) ASC, lower(t.namespace) ASC, t.id ASC"
        };
        let data_query = format!(
            "SELECT t.id, t.name, t.namespace, l.name AS localized_name,
                    COUNT(DISTINCT at.archive_id) AS archive_count
             {base_from}
             {where_clause}
             GROUP BY t.id, t.name, t.namespace, l.name
             {order_clause}
             LIMIT ? OFFSET ?"
        );
        let mut request = sqlx::query(&data_query);
        for value in &filter_values {
            request = request.bind(value);
        }
        let rows = request
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(pool)
            .await
            .map_err(internal_database_error)?;
        let data = rows
            .into_iter()
            .map(|row| TagDirectoryItem {
                id: row.get("id"),
                name: row.get("name"),
                namespace: row.get("namespace"),
                localized_name: row.get("localized_name"),
                archive_count: row.get("archive_count"),
                aliases: Vec::new(),
            })
            .collect();

        Ok((data, total, namespaces))
    }

    async fn get_theme_directory_page(
        pool: &Pool<Sqlite>,
        query: Option<&str>,
        namespace: Option<&str>,
        sort: &str,
        page_size: u64,
        offset: u64,
    ) -> Result<(Vec<TagDirectoryItem>, u64, Vec<String>), StatusCode> {
        let mut conditions = vec!["lower(trim(t.namespace)) = 'theme'".to_string()];
        let mut filter_values = Vec::new();
        if let Some(namespace) = namespace {
            conditions.push("lower(trim(t.namespace)) = lower(trim(?))".to_string());
            filter_values.push(namespace.to_string());
        }
        if let Some(query) = query {
            conditions.push(
                "(lower(names.normalized_name) LIKE ?
                  OR lower(t.name) LIKE ?
                  OR lower(COALESCE(l.name, '')) LIKE ?
                  OR EXISTS (
                      SELECT 1 FROM current_theme_relations search_rel
                      WHERE search_rel.theme_tag_id = names.theme_tag_id
                        AND lower(search_rel.generated_name) LIKE ?
                  ))"
                .to_string(),
            );
            let pattern = format!("%{}%", query.to_lowercase());
            filter_values.extend([pattern.clone(), pattern.clone(), pattern.clone(), pattern]);
        }
        let where_clause = format!("WHERE {}", conditions.join(" AND "));
        // A theme is discoverable only when it is attached to the latest completed analysis for
        // an archive whose file fingerprint is still current. Historical analyses must not
        // resurrect themes after a file has been replaced.
        let cte = "WITH completed_theme_relations AS (
                       SELECT themes.theme_tag_id, themes.generated_name, analysis.archive_id,
                              analysis.content_fingerprint
                       FROM content_analysis_themes themes
                       JOIN content_analyses analysis ON analysis.id = themes.analysis_id
                       JOIN tags theme_tag
                         ON theme_tag.id = themes.theme_tag_id
                        AND lower(trim(theme_tag.namespace)) = 'theme'
                       WHERE themes.canonicalization_status = 'completed'
                         AND analysis.status = 'completed'
                         AND analysis.canonicalization_status = 'completed'
                         AND analysis.id = (SELECT latest.id
                                            FROM content_analyses latest
                                            WHERE latest.archive_id = analysis.archive_id
                                              AND latest.content_fingerprint = analysis.content_fingerprint
                                              AND latest.status = 'completed'
                                              AND latest.canonicalization_status = 'completed'
                                            ORDER BY latest.created_at DESC, latest.id DESC
                                            LIMIT 1)
                   ),
                   current_theme_relations AS (
                       SELECT DISTINCT rel.theme_tag_id, rel.archive_id, rel.generated_name
                       FROM completed_theme_relations rel
                       JOIN archives current_archive ON current_archive.id = rel.archive_id
                                                    AND current_archive.file_hash = rel.content_fingerprint
                   )";
        let base_from = "FROM canonical_theme_names names
             JOIN tags t ON t.id = names.theme_tag_id
             JOIN current_theme_relations rel ON rel.theme_tag_id = names.theme_tag_id
             LEFT JOIN tag_localizations l
               ON l.tag_id = t.id AND l.locale = 'zh-Hans' AND l.status = 'completed'";

        let count_query = format!(
            "{cte}
             SELECT COUNT(DISTINCT names.theme_tag_id) AS total
             {base_from}
             {where_clause}"
        );
        let total = execute_count(pool, &count_query, &filter_values).await?;

        let order_clause = if sort == "name" {
            "ORDER BY lower(COALESCE(NULLIF(trim(l.name), ''), t.name)) ASC,
                      lower(names.normalized_name) ASC, names.theme_tag_id ASC"
        } else {
            "ORDER BY archive_count DESC,
                      lower(COALESCE(NULLIF(trim(l.name), ''), t.name)) ASC,
                      lower(names.normalized_name) ASC, names.theme_tag_id ASC"
        };
        let data_query = format!(
            "{cte}
             SELECT names.theme_tag_id AS id, t.name, t.namespace,
                    l.name AS localized_name, names.normalized_name AS canonical_normalized_name,
                    COUNT(DISTINCT rel.archive_id) AS archive_count
             {base_from}
             {where_clause}
             GROUP BY names.theme_tag_id, t.name, t.namespace, l.name, names.normalized_name
             {order_clause}
             LIMIT ? OFFSET ?"
        );
        let mut request = sqlx::query(&data_query);
        for value in &filter_values {
            request = request.bind(value);
        }
        let rows = request
            .bind(page_size as i64)
            .bind(offset as i64)
            .fetch_all(pool)
            .await
            .map_err(internal_database_error)?;

        let mut canonical_names = HashMap::new();
        let mut data = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = row.get("id");
            canonical_names.insert(
                id.clone(),
                row.get::<String, _>("canonical_normalized_name"),
            );
            data.push(TagDirectoryItem {
                id,
                name: row.get("name"),
                namespace: row.get("namespace"),
                localized_name: row.get("localized_name"),
                archive_count: row.get("archive_count"),
                aliases: Vec::new(),
            });
        }

        if !data.is_empty() {
            let placeholders = data.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let aliases_query = format!(
                "{cte}
                 SELECT rel.theme_tag_id, rel.generated_name
                 FROM current_theme_relations rel
                 WHERE rel.theme_tag_id IN ({placeholders})
                 ORDER BY rel.theme_tag_id, rel.generated_name COLLATE NOCASE"
            );
            let mut request = sqlx::query(&aliases_query);
            for item in &data {
                request = request.bind(&item.id);
            }
            let alias_rows = request
                .fetch_all(pool)
                .await
                .map_err(internal_database_error)?;
            let mut aliases_by_theme: HashMap<String, BTreeSet<String>> = HashMap::new();
            for row in alias_rows {
                let theme_id: String = row.get("theme_tag_id");
                let normalized = crate::services::content_analysis::normalize_theme_name(
                    row.get::<String, _>("generated_name").as_str(),
                );
                if !normalized.is_empty()
                    && canonical_names
                        .get(&theme_id)
                        .is_none_or(|canonical| canonical != &normalized)
                {
                    aliases_by_theme
                        .entry(theme_id)
                        .or_default()
                        .insert(normalized);
                }
            }
            for item in &mut data {
                item.aliases = aliases_by_theme
                    .remove(&item.id)
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
            }
        }

        Ok((data, total, vec!["theme".to_string()]))
    }

    /// DELETE /api/v1/tags/:id/archives/batch-delete - 批量删除标签下的漫画
    pub async fn batch_delete_tag_archives(
        State(pool): State<Pool<Sqlite>>,
        Path(tag_id): Path<String>,
        axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
        axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
    ) -> Result<StatusCode, StatusCode> {
        // Canonical themes have their own content-analysis relation and cannot be deleted through
        // the ordinary archive-tag batch workflow.
        let tag_namespace =
            sqlx::query_scalar::<_, String>("SELECT namespace FROM tags WHERE id = ?")
                .bind(&tag_id)
                .fetch_optional(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let Some(tag_namespace) = tag_namespace else {
            tracing::debug!(
                "Tag {} not found during batch delete, treating as no-op",
                tag_id
            );
            return Ok(StatusCode::OK);
        };
        if is_system_managed_theme_namespace(&tag_namespace) {
            return Err(StatusCode::BAD_REQUEST);
        }

        // 获取该标签关联的所有存档ID和文件路径
        let archive_rows = sqlx::query!(
            "SELECT a.id, a.path
             FROM archives a
             INNER JOIN archive_tags at ON a.id = at.archive_id
             WHERE at.tag_id = ?",
            tag_id
        )
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let targets = archive_rows
            .into_iter()
            .filter_map(|row| row.id.map(|id| ArchiveDeleteTarget { id, path: row.path }))
            .collect();
        let summary = ArchiveDeletionService::new(pool, archive_cache)
            .delete_targets(
                &auth.user_id,
                targets,
                "user initiated tag batch deletion",
                "tag_batch_delete",
            )
            .await
            .map_err(|e| {
                tracing::error!("Tag {} batch deletion failed: {}", tag_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if summary.failed > 0 {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        Ok(StatusCode::OK)
    }

    /// DELETE /api/v1/tags/prune - 清理未使用的标签
    pub async fn prune_unused_tags(
        State(pool): State<Pool<Sqlite>>,
    ) -> Result<StatusCode, StatusCode> {
        // 删除没有关联任何存档的标签
        sqlx::query(
            r#"
            DELETE FROM tags 
            WHERE id NOT IN (
                SELECT DISTINCT tag_id FROM archive_tags
            )
            AND name != 'new'  -- 保护"new"系统标签
            AND lower(trim(namespace)) != 'theme'  -- canonical themes are referenced outside archive_tags
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(StatusCode::OK)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::archive::ArchiveCacheConfig;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_tags_schema(pool: &Pool<Sqlite>) {
        sqlx::query(
            r#"
            CREATE TABLE tags (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                namespace TEXT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("create tags");

        sqlx::query(
            r#"
            CREATE TABLE archives (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("create archives");

        sqlx::query(
            r#"
            CREATE TABLE archive_tags (
                archive_id TEXT NOT NULL,
                tag_id TEXT NOT NULL,
                PRIMARY KEY (archive_id, tag_id)
            )
            "#,
        )
        .execute(pool)
        .await
        .expect("create archive_tags");
    }

    async fn setup_directory_schema(pool: &Pool<Sqlite>) {
        for statement in [
            "CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL)",
            "CREATE TABLE archives (id TEXT PRIMARY KEY, file_hash TEXT NOT NULL)",
            "CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL, PRIMARY KEY (archive_id, tag_id))",
            "CREATE TABLE tag_localizations (tag_id TEXT NOT NULL, locale TEXT NOT NULL, name TEXT, status TEXT NOT NULL, PRIMARY KEY (tag_id, locale))",
            "CREATE TABLE canonical_theme_names (normalized_name TEXT PRIMARY KEY, theme_tag_id TEXT NOT NULL UNIQUE)",
            "CREATE TABLE content_analyses (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, content_fingerprint TEXT NOT NULL, status TEXT NOT NULL, canonicalization_status TEXT NOT NULL, created_at TEXT NOT NULL)",
            "CREATE TABLE content_analysis_themes (analysis_id TEXT NOT NULL, theme_tag_id TEXT, ordinal INTEGER NOT NULL, generated_name TEXT NOT NULL, canonicalization_status TEXT NOT NULL, PRIMARY KEY (analysis_id, ordinal), UNIQUE (analysis_id, theme_tag_id))",
        ] {
            sqlx::query(statement)
                .execute(pool)
                .await
                .expect("create tag directory schema");
        }
    }

    #[tokio::test]
    async fn pruning_unused_tags_preserves_canonical_themes() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite pool");
        setup_tags_schema(&pool).await;
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES
             ('new-tag', 'new', 'system'),
             ('unused-tag', 'unused', 'general'),
             ('theme-tag', 'sample theme', 'theme'),
             ('used-tag', 'used', 'general')",
        )
        .execute(&pool)
        .await
        .expect("insert test tags");
        sqlx::query("INSERT INTO archives (id, path) VALUES ('archive-1', '/tmp/archive.cbz')")
            .execute(&pool)
            .await
            .expect("insert test archive");
        sqlx::query(
            "INSERT INTO archive_tags (archive_id, tag_id) VALUES ('archive-1', 'used-tag')",
        )
        .execute(&pool)
        .await
        .expect("associate used tag");

        TagHandler::prune_unused_tags(State(pool.clone()))
            .await
            .expect("prune should succeed");

        let remaining: Vec<(String, String)> =
            sqlx::query_as("SELECT name, namespace FROM tags ORDER BY id")
                .fetch_all(&pool)
                .await
                .expect("read remaining tags");
        assert_eq!(
            remaining,
            vec![
                ("new".to_string(), "system".to_string()),
                ("sample theme".to_string(), "theme".to_string()),
                ("used".to_string(), "general".to_string()),
            ]
        );
    }

    #[tokio::test]
    async fn create_tag_rejects_system_managed_theme_namespace() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite pool");
        setup_tags_schema(&pool).await;

        let result = TagHandler::create_tag(
            State(pool.clone()),
            Json(CreateTagRequest {
                name: "Space Opera".to_string(),
                namespace: " THEME ".to_string(),
            }),
        )
        .await;

        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags")
            .fetch_one(&pool)
            .await
            .expect("count tags");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn directory_paginates_and_counts_distinct_regular_tag_archives() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite memory pool");
        setup_directory_schema(&pool).await;
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES
             ('tag-general', 'hero', 'general'),
             ('tag-artist', 'alice', 'artist'),
             ('tag-system', 'new', 'system'),
             ('tag-theme', 'space opera', 'theme'),
             ('tag-unused', 'unused', 'general')",
        )
        .execute(&pool)
        .await
        .expect("insert tags");
        sqlx::query(
            "INSERT INTO archives (id, file_hash) VALUES
             ('archive-1', 'hash-1'), ('archive-2', 'hash-2')",
        )
        .execute(&pool)
        .await
        .expect("insert archives");
        sqlx::query(
            "INSERT INTO archive_tags (archive_id, tag_id) VALUES
             ('archive-1', 'tag-general'), ('archive-2', 'tag-general'),
             ('archive-1', 'tag-artist'), ('archive-1', 'tag-system'),
             ('archive-1', 'tag-theme')",
        )
        .execute(&pool)
        .await
        .expect("insert archive tags");

        let response = TagHandler::get_tag_directory(
            State(pool),
            Query(TagDirectoryQuery {
                kind: Some("tag".to_string()),
                page_size: Some(1),
                ..Default::default()
            }),
        )
        .await
        .expect("directory query should succeed")
        .0;

        assert_eq!(response.total, 2);
        assert_eq!(response.page_numb, 1);
        assert_eq!(response.page_size, 1);
        assert!(response.has_next);
        assert_eq!(response.namespaces, vec!["artist", "general"]);
        assert_eq!(response.data[0].id, "tag-general");
        assert_eq!(response.data[0].archive_count, 2);
        assert!(response.data[0].aliases.is_empty());
    }

    #[tokio::test]
    async fn theme_directory_exposes_confirmed_aliases_and_searches_them() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite memory pool");
        setup_directory_schema(&pool).await;
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES
             ('theme-1', 'Space Opera', 'theme'),
             ('theme-legacy', 'Legacy theme', 'theme')",
        )
        .execute(&pool)
        .await
        .expect("insert theme tags");
        sqlx::query(
            "INSERT INTO canonical_theme_names (normalized_name, theme_tag_id) VALUES
             ('space opera', 'theme-1'), ('legacy theme', 'theme-legacy')",
        )
        .execute(&pool)
        .await
        .expect("insert canonical theme names");
        sqlx::query(
            "INSERT INTO tag_localizations (tag_id, locale, name, status)
             VALUES ('theme-1', 'zh-Hans', '太空歌剧', 'completed')",
        )
        .execute(&pool)
        .await
        .expect("insert theme localization");
        sqlx::query(
            "INSERT INTO archives (id, file_hash) VALUES
             ('archive-1', 'hash-1'), ('archive-stale', 'hash-current')",
        )
        .execute(&pool)
        .await
        .expect("insert archive");
        sqlx::query(
            "INSERT INTO content_analyses
             (id, archive_id, content_fingerprint, status, canonicalization_status, created_at)
             VALUES
             ('analysis-1', 'archive-1', 'hash-1', 'completed', 'completed', '2026-01-01'),
             ('analysis-2', 'archive-1', 'hash-1', 'completed', 'completed', '2026-01-02'),
             ('analysis-stale', 'archive-stale', 'hash-old', 'completed', 'completed', '2026-01-01')",
        )
        .execute(&pool)
        .await
        .expect("insert analysis");
        sqlx::query(
            "INSERT INTO content_analysis_themes
             (analysis_id, theme_tag_id, ordinal, generated_name, canonicalization_status)
             VALUES ('analysis-1', 'theme-1', 0, 'Space Opera', 'completed'),
                    ('analysis-2', 'theme-1', 0, 'Dream Worlds', 'completed'),
                    ('analysis-stale', 'theme-legacy', 0, 'Legacy theme', 'completed')",
        )
        .execute(&pool)
        .await
        .expect("insert canonical theme relations");

        let response = TagHandler::get_tag_directory(
            State(pool.clone()),
            Query(TagDirectoryQuery {
                kind: Some("theme".to_string()),
                ..Default::default()
            }),
        )
        .await
        .expect("theme directory query should succeed")
        .0;

        assert_eq!(response.total, 1);
        assert_eq!(response.namespaces, vec!["theme"]);
        assert_eq!(response.data[0].id, "theme-1");
        assert_eq!(response.data[0].localized_name.as_deref(), Some("太空歌剧"));
        assert_eq!(response.data[0].archive_count, 1);
        assert_eq!(response.data[0].aliases, vec!["dream worlds"]);

        let search_response = TagHandler::get_tag_directory(
            State(pool),
            Query(TagDirectoryQuery {
                kind: Some("theme".to_string()),
                query: Some("dream".to_string()),
                ..Default::default()
            }),
        )
        .await
        .expect("theme alias search should succeed")
        .0;
        assert_eq!(search_response.total, 1);
        assert_eq!(search_response.data[0].id, "theme-1");
    }

    fn test_cache_service() -> Arc<ArchiveCacheService> {
        Arc::new(ArchiveCacheService::new(ArchiveCacheConfig::default()))
    }

    fn test_auth_info() -> AuthInfo {
        AuthInfo {
            user_id: "user-1".to_string(),
            role: "admin".to_string(),
        }
    }

    #[tokio::test]
    async fn batch_delete_tag_archives_is_noop_for_missing_tag() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite memory pool");
        setup_tags_schema(&pool).await;

        let status = TagHandler::batch_delete_tag_archives(
            State(pool),
            Path("missing-tag".to_string()),
            axum::extract::Extension(test_auth_info()),
            axum::extract::Extension(test_cache_service()),
        )
        .await
        .expect("missing tag should be a no-op");

        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn batch_delete_rejects_system_managed_theme_tags() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite memory pool");
        setup_tags_schema(&pool).await;
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES ('theme-tag', 'Sample theme', 'theme')",
        )
        .execute(&pool)
        .await
        .expect("insert theme tag");
        sqlx::query("INSERT INTO archives (id, path) VALUES ('archive-1', '/tmp/archive.cbz')")
            .execute(&pool)
            .await
            .expect("insert archive");
        sqlx::query(
            "INSERT INTO archive_tags (archive_id, tag_id) VALUES ('archive-1', 'theme-tag')",
        )
        .execute(&pool)
        .await
        .expect("insert legacy theme relation");

        let result = TagHandler::batch_delete_tag_archives(
            State(pool.clone()),
            Path("theme-tag".to_string()),
            axum::extract::Extension(test_auth_info()),
            axum::extract::Extension(test_cache_service()),
        )
        .await;

        assert_eq!(result, Err(StatusCode::BAD_REQUEST));
        let archive_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM archives")
            .fetch_one(&pool)
            .await
            .expect("count archives");
        assert_eq!(archive_count, 1);
    }
}

// 独立函数用于路由注册
pub async fn create_tag(
    State(pool): State<Pool<Sqlite>>,
    Json(req): Json<CreateTagRequest>,
) -> Result<Json<TagModel>, StatusCode> {
    TagHandler::create_tag(State(pool), Json(req)).await
}

pub async fn list_tags(
    State(pool): State<Pool<Sqlite>>,
) -> Result<Json<Vec<TagModel>>, StatusCode> {
    TagHandler::list_tags(State(pool)).await
}

pub async fn get_tag_directory(
    State(pool): State<Pool<Sqlite>>,
    Query(params): Query<TagDirectoryQuery>,
) -> Result<Json<TagDirectoryResponse>, StatusCode> {
    TagHandler::get_tag_directory(State(pool), Query(params)).await
}

pub async fn batch_delete_tag_archives(
    State(pool): State<Pool<Sqlite>>,
    Path(tag_id): Path<String>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    axum::extract::Extension(archive_cache): axum::extract::Extension<Arc<ArchiveCacheService>>,
) -> Result<StatusCode, StatusCode> {
    TagHandler::batch_delete_tag_archives(
        State(pool),
        Path(tag_id),
        axum::extract::Extension(auth),
        axum::extract::Extension(archive_cache),
    )
    .await
}

pub async fn prune_unused_tags(State(pool): State<Pool<Sqlite>>) -> Result<StatusCode, StatusCode> {
    TagHandler::prune_unused_tags(State(pool)).await
}
