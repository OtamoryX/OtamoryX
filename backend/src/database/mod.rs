use sqlx::{
    migrate::Migrator,
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Executor, Pool, Postgres, Sqlite,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseType {
    Sqlite,
    Postgres,
}

impl DatabaseType {
    fn from_url(database_url: &str) -> Self {
        if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
            Self::Postgres
        } else {
            Self::Sqlite
        }
    }
}

#[derive(Debug, Clone)]
pub enum DatabasePool {
    Sqlite(Pool<Sqlite>),
    Postgres(Pool<Postgres>),
}

pub async fn create_database_pool(database_url: &str) -> Result<DatabasePool, sqlx::Error> {
    match DatabaseType::from_url(database_url) {
        DatabaseType::Postgres => {
            let pool = create_postgres_pool(database_url).await?;
            Ok(DatabasePool::Postgres(pool))
        }
        DatabaseType::Sqlite => {
            let pool = create_sqlite_pool(database_url).await?;
            Ok(DatabasePool::Sqlite(pool))
        }
    }
}

async fn create_postgres_pool(database_url: &str) -> Result<Pool<Postgres>, sqlx::Error> {
    info!(
        "Connecting to PostgreSQL database: {}",
        database_url.split('@').last().unwrap_or("hidden")
    );

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Some(Duration::from_secs(600)))
        .connect(database_url)
        .await?;

    run_postgres_migrations(&pool).await?;
    Ok(pool)
}

async fn create_sqlite_pool(database_url: &str) -> Result<Pool<Sqlite>, sqlx::Error> {
    prepare_sqlite_file(database_url)?;

    let connect_options = SqliteConnectOptions::from_str(database_url)
        .map_err(|e| sqlx::Error::Configuration(Box::new(e)))?
        .pragma("busy_timeout", "5000")
        .pragma("synchronous", "NORMAL")
        .pragma("cache_size", "-64000")
        .pragma("foreign_keys", "ON");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Some(Duration::from_secs(600)))
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                conn.execute("PRAGMA journal_mode = WAL;").await?;
                Ok(())
            })
        })
        .connect_with(connect_options)
        .await?;

    info!("SQLite connection pool created (max_connections=5, WAL mode enabled)");

    let row: (String,) = sqlx::query_as("PRAGMA journal_mode")
        .fetch_one(&pool)
        .await?;
    if row.0.to_lowercase() != "wal" {
        warn!(
            "SQLite WAL mode not active, current journal_mode: {}",
            row.0
        );
    } else {
        info!("SQLite WAL mode confirmed active");
    }

    run_sqlite_migrations(&pool).await?;
    Ok(pool)
}

fn prepare_sqlite_file(database_url: &str) -> Result<(), sqlx::Error> {
    if !database_url.starts_with("sqlite:") {
        return Ok(());
    }

    let db_path = database_url.strip_prefix("sqlite:").unwrap_or(database_url);
    if db_path == ":memory:" || db_path.contains("mode=memory") {
        return Ok(());
    }

    let path = Path::new(db_path);

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(sqlx::Error::Io)?;
            info!("Created database directory: {:?}", parent);
        }
    }

    if !path.exists() {
        info!("Creating database file at: {:?}", path);
        std::fs::File::create(path).map_err(sqlx::Error::Io)?;
    }

    Ok(())
}

async fn run_postgres_migrations(pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    info!("Running PostgreSQL migrations...");
    postgres_migrator().run(pool).await?;
    info!("PostgreSQL migrations completed successfully");
    Ok(())
}

pub(crate) async fn run_sqlite_migrations(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    info!("Running SQLite migrations...");

    // Compatibility migrations may recreate referenced tables.
    let mut conn = pool.acquire().await?;
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *conn)
        .await?;

    let migrate_result = sqlite_migrator().run(&mut *conn).await;
    let fk_restore_result = sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&mut *conn)
        .await;

    if let Err(err) = migrate_result {
        if let Err(restore_err) = fk_restore_result {
            warn!("Failed to restore SQLite foreign_keys after migration error: {restore_err}");
        }
        return Err(err.into());
    }

    fk_restore_result?;
    info!("SQLite migrations completed successfully");
    Ok(())
}

fn sqlite_migrator() -> Migrator {
    let mut migrator = sqlx::migrate!("./migrations/sqlite");
    migrator.set_ignore_missing(true);
    migrator
}

fn postgres_migrator() -> Migrator {
    let mut migrator = sqlx::migrate!("./migrations/postgres");
    migrator.set_ignore_missing(true);
    migrator
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn applies_forward_cleanup_when_versions_are_pending() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("SQLite test database should connect");

        let mut initial_migrator = sqlx::migrate!("./migrations/sqlite");
        initial_migrator.migrations = std::borrow::Cow::Owned(
            initial_migrator
                .migrations
                .iter()
                .filter(|migration| migration.version == 1)
                .cloned()
                .collect(),
        );
        initial_migrator
            .run(&pool)
            .await
            .expect("initial migration should succeed");

        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(&pool)
            .await
            .expect("foreign keys should be disabled for the legacy schema fixture");
        sqlx::query("DROP TABLE users")
            .execute(&pool)
            .await
            .expect("legacy users table should be replaceable");
        sqlx::query(
            "CREATE TABLE users (\
                id TEXT PRIMARY KEY,\
                username TEXT UNIQUE NOT NULL,\
                email TEXT UNIQUE NOT NULL,\
                role TEXT NOT NULL DEFAULT 'user',\
                password_hash TEXT NOT NULL,\
                api_key TEXT UNIQUE NOT NULL,\
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,\
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP\
            )",
        )
        .execute(&pool)
        .await
        .expect("legacy users table should be created");
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("foreign keys should be restored after the legacy schema fixture");

        sqlx::query(
            "INSERT INTO users (id, username, email, role, password_hash, api_key) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind("user-1")
        .bind("migration-user")
        .bind("")
        .bind("user")
        .bind("test-hash")
        .bind("test-api-key")
        .execute(&pool)
        .await
        .expect("legacy user should be inserted");

        sqlx::query(
            "INSERT INTO archives (id, title, path, file_hash, file_size, page_count) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind("archive-1")
        .bind("legacy archive")
        .bind("/tmp/legacy.cbz")
        .bind("legacy-hash")
        .bind(1_i64)
        .bind(1_i64)
        .execute(&pool)
        .await
        .expect("legacy archive should be inserted");

        sqlx::query("INSERT INTO tags (id, name, namespace) VALUES (?, ?, ?)")
            .bind("tag-1")
            .bind("legacy tag")
            .bind("general")
            .execute(&pool)
            .await
            .expect("legacy tag should be inserted");

        sqlx::query(
            "INSERT INTO ai_generated_tags \
             (id, archive_id, tag_id, tag_name, confidence, approved) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind("generated-tag-1")
        .bind("archive-1")
        .bind("tag-1")
        .bind("legacy tag")
        .bind(0.95_f64)
        .bind(true)
        .execute(&pool)
        .await
        .expect("approved legacy tag should be inserted");

        sqlx::query(
            "INSERT INTO plugins (id, name, version, execution_count, last_executed_at) \
             VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)",
        )
        .bind("ehentai-metadata")
        .bind("e-hentai metadata")
        .bind("1.0.0")
        .bind(3_i64)
        .execute(&pool)
        .await
        .expect("legacy plugin should be inserted");

        sqlx::query(
            "INSERT INTO plugin_executions (id, plugin_id, execution_type, status) \
             VALUES (?, ?, ?, ?)",
        )
        .bind("execution-1")
        .bind("ehentai-metadata")
        .bind("manual")
        .bind("pending")
        .execute(&pool)
        .await
        .expect("legacy plugin execution should be inserted");

        sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?)")
            .bind("ocr_settings")
            .bind(r#"{"enabled":true,"activeModelId":"legacy-model"}"#)
            .execute(&pool)
            .await
            .expect("legacy OCR settings should be inserted");

        sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?)")
            .bind("ai_connection_api_key")
            .bind("legacy-api-key")
            .execute(&pool)
            .await
            .expect("legacy AI API key should be inserted");

        sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?)")
            .bind("ai_settings")
            .bind(
                r#"{"settingsVersion":4,"settings_version":4,"connection":{"provider":"ollama"},"execution":{"maxConcurrentTasks":4},"features":{"titleTranslation":{"temperature":0.4,"structuredOutputMode":"jsonSchema"},"tagLocalization":{"execution":{"additionalInstructions":"obsolete"}},"contentUnderstanding":{"execution":{"thinkingOutputTokenLimit":null}}}}"#,
            )
            .execute(&pool)
            .await
            .expect("legacy AI settings should be inserted");

        run_sqlite_migrations(&pool)
            .await
            .expect("all current migrations should succeed");

        let email: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE id = ?")
            .bind("user-1")
            .fetch_one(&pool)
            .await
            .expect("migrated user should be readable");
        assert_eq!(email, None);

        let execution_status: String =
            sqlx::query_scalar("SELECT status FROM plugin_executions WHERE id = ?")
                .bind("execution-1")
                .fetch_one(&pool)
                .await
                .expect("migrated plugin execution should be readable");
        assert_eq!(execution_status, "failed");

        let execution_count: i64 =
            sqlx::query_scalar("SELECT execution_count FROM plugins WHERE id = ?")
                .bind("ehentai-metadata")
                .fetch_one(&pool)
                .await
                .expect("migrated plugin should be readable");
        assert_eq!(execution_count, 0);

        let ocr_settings: String =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'ocr_settings'")
                .fetch_one(&pool)
                .await
                .expect("migrated OCR settings should be readable");
        let ocr_settings: serde_json::Value =
            serde_json::from_str(&ocr_settings).expect("migrated OCR settings should be JSON");
        assert!(ocr_settings.get("image").is_some());
        assert!(ocr_settings.get("failurePolicy").is_some());

        let profile_api_key: String = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'ai_connection_api_key:default'",
        )
        .fetch_one(&pool)
        .await
        .expect("legacy API key should move to the default profile");
        assert_eq!(profile_api_key, "legacy-api-key");
        let legacy_key_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM settings WHERE key = 'ai_connection_api_key'")
                .fetch_one(&pool)
                .await
                .expect("legacy API key count should be readable");
        assert_eq!(legacy_key_count, 0);

        let approved_tag_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM archive_tags WHERE archive_id = ? AND tag_id = ?",
        )
        .bind("archive-1")
        .bind("tag-1")
        .fetch_one(&pool)
        .await
        .expect("approved legacy tag association should be readable");
        assert_eq!(approved_tag_count, 1);

        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES ('theme-tag', 'Legacy theme', 'theme')",
        )
        .execute(&pool)
        .await
        .expect("theme tag should remain insertable without an archive relation");
        let direct_theme_relation = sqlx::query(
            "INSERT INTO archive_tags (archive_id, tag_id) VALUES ('archive-1', 'theme-tag')",
        )
        .execute(&pool)
        .await;
        assert!(direct_theme_relation.is_err());

        sqlx::query(
            "INSERT INTO tags (id, name, namespace) VALUES ('guard-tag', 'Guard tag', 'general')",
        )
        .execute(&pool)
        .await
        .expect("guard tag should be inserted");
        sqlx::query(
            "INSERT INTO archive_tags (archive_id, tag_id) VALUES ('archive-1', 'guard-tag')",
        )
        .execute(&pool)
        .await
        .expect("guard relation should be inserted");
        let namespace_update =
            sqlx::query("UPDATE tags SET namespace = 'theme' WHERE id = 'guard-tag'")
                .execute(&pool)
                .await;
        assert!(namespace_update.is_err());

        let ai_settings: String =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'ai_settings'")
                .fetch_one(&pool)
                .await
                .expect("AI settings should remain readable");
        let ai_settings: serde_json::Value =
            serde_json::from_str(&ai_settings).expect("AI settings should remain JSON");
        assert!(ai_settings
            .pointer("/execution/maxConcurrentTasks")
            .is_none());
        assert!(ai_settings.pointer("/settingsVersion").is_none());
        assert!(ai_settings.pointer("/settings_version").is_none());
        assert!(ai_settings
            .pointer("/features/titleTranslation/temperature")
            .is_none());
        assert!(ai_settings
            .pointer("/features/tagLocalization/execution/additionalInstructions")
            .is_none());
        assert_eq!(
            ai_settings
                .pointer("/features/contentUnderstanding/execution/thinkingOutputTokenLimit")
                .and_then(serde_json::Value::as_u64),
            Some(8_192)
        );

        let legacy_table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'ai_generated_tags'",
        )
        .fetch_one(&pool)
        .await
        .expect("legacy tag table state should be readable");
        assert_eq!(legacy_table_count, 0);

        let applied_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("migration records should be readable");
        let expected_migration_count =
            sqlx::migrate!("./migrations/sqlite").migrations.len() as i64;
        assert_eq!(applied_count, expected_migration_count);

        run_sqlite_migrations(&pool)
            .await
            .expect("already applied current versions should validate by checksum");
    }

    #[tokio::test]
    async fn preserves_current_data_when_retired_versions_are_already_applied() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("SQLite test database should connect");

        let mut initial_migrator = sqlx::migrate!("./migrations/sqlite");
        initial_migrator.migrations = std::borrow::Cow::Owned(
            initial_migrator
                .migrations
                .iter()
                .filter(|migration| migration.version < 28)
                .cloned()
                .collect(),
        );
        initial_migrator
            .run(&pool)
            .await
            .expect("current migrations before cleanup should succeed");

        for version in [2_i64, 10_i64, 22_i64] {
            sqlx::query(
                "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) \
                 VALUES (?, ?, TRUE, ?, ?)",
            )
            .bind(version)
            .bind(format!("retired-{version}"))
            .bind(Vec::<u8>::new())
            .bind(0_i64)
            .execute(&pool)
            .await
            .expect("retired migration marker should be inserted");
        }

        sqlx::query(
            "INSERT INTO users (id, username, email, role, password_hash, api_key) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind("current-user")
        .bind("current-user")
        .bind("")
        .bind("user")
        .bind("current-hash")
        .bind("current-api-key")
        .execute(&pool)
        .await
        .expect("current user should be inserted");

        sqlx::query(
            "INSERT INTO plugins (id, name, version, execution_count, last_executed_at) \
             VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP)",
        )
        .bind("ehentai-metadata")
        .bind("e-hentai metadata")
        .bind("2.0.0")
        .bind(7_i64)
        .execute(&pool)
        .await
        .expect("current plugin should be inserted");

        sqlx::query(
            "INSERT INTO plugin_executions (id, plugin_id, execution_type, status) \
             VALUES (?, ?, ?, ?)",
        )
        .bind("current-execution")
        .bind("ehentai-metadata")
        .bind("manual")
        .bind("pending")
        .execute(&pool)
        .await
        .expect("current plugin execution should be inserted");

        sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?)")
            .bind("ocr_settings")
            .bind(
                r#"{"enabled":true,"activeModelId":"current-model","image":{"targetLongEdge":3072,"preferredDecodeBytes":100663296,"jpegQuality":90,"maxOutputBytes":2097152,"largeImageLongEdge":3584,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":92,"largeImageMaxOutputBytes":4194304},"failurePolicy":{"skipUnreadablePages":false,"maxPageRetries":4}}"#,
            )
            .execute(&pool)
            .await
            .expect("current OCR settings should be inserted");

        run_sqlite_migrations(&pool)
            .await
            .expect("cleanup should preserve databases with retired markers");

        let email: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE id = ?")
            .bind("current-user")
            .fetch_one(&pool)
            .await
            .expect("current user should remain readable");
        assert_eq!(email, Some(String::new()));

        let execution_status: String =
            sqlx::query_scalar("SELECT status FROM plugin_executions WHERE id = ?")
                .bind("current-execution")
                .fetch_one(&pool)
                .await
                .expect("current plugin execution should remain readable");
        assert_eq!(execution_status, "pending");

        let execution_count: i64 =
            sqlx::query_scalar("SELECT execution_count FROM plugins WHERE id = ?")
                .bind("ehentai-metadata")
                .fetch_one(&pool)
                .await
                .expect("current plugin should remain readable");
        assert_eq!(execution_count, 7);

        let ocr_settings: String =
            sqlx::query_scalar("SELECT value FROM settings WHERE key = 'ocr_settings'")
                .fetch_one(&pool)
                .await
                .expect("current OCR settings should remain readable");
        let ocr_settings: serde_json::Value =
            serde_json::from_str(&ocr_settings).expect("current OCR settings should remain JSON");
        assert_eq!(ocr_settings["activeModelId"], "current-model");
        assert_eq!(ocr_settings["image"]["targetLongEdge"], 3072);
        assert_eq!(ocr_settings["failurePolicy"]["maxPageRetries"], 4);
    }
}
