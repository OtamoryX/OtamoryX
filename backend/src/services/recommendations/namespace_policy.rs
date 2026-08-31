use anyhow::Result;
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;

pub const METADATA_NAMESPACE_POLICY_VERSION: &str = "metadata-v1";
pub const CANONICAL_THEME_NAMESPACE: &str = "theme";

pub fn is_system_managed_theme_namespace(namespace: &str) -> bool {
    namespace
        .trim()
        .eq_ignore_ascii_case(CANONICAL_THEME_NAMESPACE)
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NonMetadataTagCandidate {
    pub id: String,
    pub name: String,
    pub namespace: String,
    pub archive_count: i64,
}

/// Loads the namespace policy without changing the raw tag identity or associations.
pub async fn load_metadata_namespace_set(pool: &Pool<Sqlite>) -> Result<HashSet<String>> {
    let namespaces: Vec<String> = sqlx::query_scalar(
        "SELECT namespace
         FROM recommendation_metadata_namespaces
         WHERE policy_version = ?
         ORDER BY namespace",
    )
    .bind(METADATA_NAMESPACE_POLICY_VERSION)
    .fetch_all(pool)
    .await?;
    Ok(namespaces
        .into_iter()
        .map(|value| value.to_ascii_lowercase())
        .collect())
}

/// Returns the current cluster input candidate set. The view excludes only the
/// high-confidence metadata namespace policy; it does not claim that every
/// remaining tag is ready for publication.
pub async fn load_non_metadata_tag_candidates(
    pool: &Pool<Sqlite>,
) -> Result<Vec<NonMetadataTagCandidate>> {
    Ok(sqlx::query_as(
        "SELECT id, name, namespace, archive_count
         FROM recommendation_non_metadata_tag_candidates
         ORDER BY namespace, name",
    )
    .fetch_all(pool)
    .await?)
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
            .expect("SQLite test database should connect");
        crate::database::run_sqlite_migrations(&pool)
            .await
            .expect("metadata namespace migration should succeed");
        pool
    }

    #[tokio::test]
    async fn metadata_namespace_policy_contains_only_the_high_confidence_routes() {
        let pool = test_pool().await;
        let namespaces = load_metadata_namespace_set(&pool).await.unwrap();

        assert!(namespaces.contains("filename_token"));
        assert!(namespaces.contains("artist"));
        assert!(namespaces.contains("date_added"));
        assert!(!namespaces.contains("general"));
        assert!(!namespaces.contains("female"));
    }

    #[tokio::test]
    async fn non_metadata_candidate_view_excludes_metadata_without_rewriting_tags() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO archives (id, title, path, file_hash, file_size, page_count)
             VALUES ('archive-1', 'test archive', '/tmp/test.cbz', 'hash-1', 1, 10)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES
             ('tag-metadata', '2026-01-01', 'date_added_iso8601'),
             ('tag-content', 'sample signal', 'general')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO archive_tags (archive_id, tag_id) VALUES
             ('archive-1', 'tag-metadata'), ('archive-1', 'tag-content')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let candidates = load_non_metadata_tag_candidates(&pool).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, "tag-content");

        let stored_tag_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM archive_tags")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored_tag_count, 2);
    }
}
