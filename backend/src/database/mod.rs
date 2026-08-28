use sqlx::{
    migrate::{Migration, MigrationType, Migrator},
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Executor, Pool, Postgres, Sqlite,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tracing::{info, warn};

const SQLITE_RETIRED_USERS_EMAIL_NULLABLE_SQL: &str = r#"DROP TABLE IF EXISTS users_email_migration_tmp;

CREATE TABLE users_email_migration_tmp (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE,
    role TEXT NOT NULL DEFAULT 'user',
    password_hash TEXT NOT NULL,
    api_key TEXT UNIQUE NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO users_email_migration_tmp (
    id, username, email, role, password_hash, api_key, created_at, updated_at
)
SELECT
    id, username, NULLIF(email, ''), role, password_hash, api_key, created_at, updated_at
FROM users;

DROP TABLE users;

ALTER TABLE users_email_migration_tmp RENAME TO users;
"#;

const SQLITE_RETIRED_EHENTAI_METADATA_SQL: &str = r#"-- The first release exposed official external manifests before there was an external runtime.
-- Make stale records honest rather than leaving permanent "pending" executions in the UI.
UPDATE plugin_executions
SET status = 'failed',
    error_message = '旧版本只创建了执行记录，未实际调度插件。请在升级后重新执行。',
    completed_at = CURRENT_TIMESTAMP
WHERE plugin_id IN ('ehentai-metadata', 'nhentai-metadata')
  AND status IN ('pending', 'running');

UPDATE plugins
SET execution_count = 0,
    last_executed_at = NULL,
    updated_at = CURRENT_TIMESTAMP
WHERE id IN ('ehentai-metadata', 'nhentai-metadata');
"#;

const SQLITE_RETIRED_OCR_SETTINGS_SQL: &str = r#"-- Normalize the persisted OCR settings before the new API contract is used.
INSERT INTO settings (key, value, updated_at)
SELECT
    'ocr_settings',
    '{"enabled":false,"activeModelId":"ppocrv5-mobile-zh","image":{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304},"failurePolicy":{"skipUnreadablePages":true,"maxPageRetries":1}}',
    CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM settings WHERE key = 'ocr_settings');

UPDATE settings
SET value = json_patch(
        '{"image":{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304},"failurePolicy":{"skipUnreadablePages":true,"maxPageRetries":1}}',
        value
    ),
    updated_at = CURRENT_TIMESTAMP
WHERE key = 'ocr_settings';
"#;

const POSTGRES_RETIRED_USERS_EMAIL_NULLABLE_SQL: &str = r#"ALTER TABLE users ALTER COLUMN email DROP NOT NULL;

UPDATE users
SET email = NULL
WHERE email = '';
"#;

const POSTGRES_RETIRED_EHENTAI_METADATA_SQL: &str = r#"-- The first release exposed official external manifests before there was an external runtime.
-- Make stale records honest rather than leaving permanent "pending" executions in the UI.
UPDATE plugin_executions
SET status = 'failed',
    error_message = '旧版本只创建了执行记录，未实际调度插件。请在升级后重新执行。',
    completed_at = NOW()
WHERE plugin_id IN ('ehentai-metadata', 'nhentai-metadata')
  AND status IN ('pending', 'running');

UPDATE plugins
SET execution_count = 0,
    last_executed_at = NULL,
    updated_at = NOW()
WHERE id IN ('ehentai-metadata', 'nhentai-metadata');
"#;

const POSTGRES_RETIRED_OCR_SETTINGS_SQL: &str = r#"-- Normalize the persisted OCR settings before the new API contract is used.
INSERT INTO settings (key, value, updated_at)
VALUES (
    'ocr_settings',
    '{"enabled":false,"activeModelId":"ppocrv5-mobile-zh","image":{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304},"failurePolicy":{"skipUnreadablePages":true,"maxPageRetries":1}}',
    NOW()
)
ON CONFLICT (key) DO NOTHING;

UPDATE settings
SET value = (
        '{"image":{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304},"failurePolicy":{"skipUnreadablePages":true,"maxPageRetries":1}}'::jsonb
        || value::jsonb
    )::text,
    updated_at = NOW()
WHERE key = 'ocr_settings';
"#;

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

async fn run_sqlite_migrations(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
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
    with_retired_migrations(
        sqlx::migrate!("./migrations/sqlite"),
        [
            retired_migration(
                2,
                "users_email_nullable_compat",
                SQLITE_RETIRED_USERS_EMAIL_NULLABLE_SQL,
                "dcad6988e835f0a21cdaccf2c44fc4c10a03f4a03b85e651970063e5e129a3a8d0764f3e98c29326cd772bfee278d4de",
            ),
            retired_migration(
                10,
                "ehentai_metadata_runtime",
                SQLITE_RETIRED_EHENTAI_METADATA_SQL,
                "ac435ac27d191c3639822f1f1b5e746d1c1516a0ca8234665f385dd58f3dc61bed1129832aa9f9da38917c4238e7c92d",
            ),
            retired_migration(
                22,
                "ocr_settings_image_policy",
                SQLITE_RETIRED_OCR_SETTINGS_SQL,
                "b827bd722294a7a42b5f29de67d806d3756cd204dcb787a9686e34b5877f3e7f7764d6588f71f43bda0fa892bbefb6aa",
            ),
        ],
    )
}

fn postgres_migrator() -> Migrator {
    with_retired_migrations(
        sqlx::migrate!("./migrations/postgres"),
        [
            retired_migration(
                2,
                "users_email_nullable_compat",
                POSTGRES_RETIRED_USERS_EMAIL_NULLABLE_SQL,
                "d62a11c24b1cf7df7c351056698128044268fd63e2ef1ee8326dee0ab4e27f26fccfb4742ecc54a15cc299ded4fb03c4",
            ),
            retired_migration(
                10,
                "ehentai_metadata_runtime",
                POSTGRES_RETIRED_EHENTAI_METADATA_SQL,
                "79320a3a71be18d1cf6da4737aa57e777fe3bb0156c4b5c4b06517c0d5f1851bbb3b6d450b968a4189574d78d635d9d2",
            ),
            retired_migration(
                22,
                "ocr_settings_image_policy",
                POSTGRES_RETIRED_OCR_SETTINGS_SQL,
                "5516da298c2d79f16f3d568f71ced5b5f45f6321202f64cafacdf49e1a5f49c550fbc29bb23b58f8582e5235277ea33a",
            ),
        ],
    )
}

fn with_retired_migrations(
    mut migrator: Migrator,
    retired: impl IntoIterator<Item = Migration>,
) -> Migrator {
    let mut migrations = migrator.migrations.to_vec();
    migrations.extend(retired);
    migrations.sort_unstable_by_key(|migration| migration.version);
    migrator.migrations = std::borrow::Cow::Owned(migrations);
    migrator
}

fn retired_migration(
    version: i64,
    description: &'static str,
    sql: &'static str,
    checksum_hex: &'static str,
) -> Migration {
    assert_eq!(
        checksum_hex.len(),
        96,
        "retired migration checksum must be SHA-384"
    );
    let checksum = checksum_hex
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]))
        .collect();

    Migration {
        version,
        description: std::borrow::Cow::Borrowed(description),
        migration_type: MigrationType::Simple,
        sql: std::borrow::Cow::Borrowed(sql),
        checksum: std::borrow::Cow::Owned(checksum),
        no_tx: false,
    }
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => panic!("invalid hexadecimal migration checksum"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn applies_retired_migration_effects_when_versions_are_pending() {
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

        run_sqlite_migrations(&pool)
            .await
            .expect("all migrations including retired versions should succeed");

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

        let applied_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("migration records should be readable");
        assert_eq!(applied_count, 23);

        run_sqlite_migrations(&pool)
            .await
            .expect("already applied retired versions should validate by checksum");
    }
}
