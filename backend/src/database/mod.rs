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

static SQLITE_MIGRATOR: Migrator = sqlx::migrate!("./migrations/sqlite");
static POSTGRES_MIGRATOR: Migrator = sqlx::migrate!("./migrations/postgres");

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
    POSTGRES_MIGRATOR.run(pool).await?;
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

    let migrate_result = SQLITE_MIGRATOR.run(&mut *conn).await;
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
