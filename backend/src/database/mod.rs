use sqlx::{
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Executor, Pool, Postgres, Sqlite,
};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub enum DatabaseType {
    Sqlite,
    Postgres,
}

#[derive(Debug, Clone)]
pub enum DatabasePool {
    Sqlite(Pool<Sqlite>),
    Postgres(Pool<Postgres>),
}

pub async fn create_database_pool(database_url: &str) -> Result<DatabasePool, sqlx::Error> {
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        let pool = create_postgres_pool(database_url).await?;
        Ok(DatabasePool::Postgres(pool))
    } else {
        let pool = create_sqlite_pool(database_url).await?;
        Ok(DatabasePool::Sqlite(pool))
    }
}

pub async fn create_pool(database_url: &str) -> Result<Pool<Sqlite>, sqlx::Error> {
    create_sqlite_pool(database_url).await
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

    // 运行PostgreSQL迁移
    run_postgres_migrations(&pool).await?;

    Ok(pool)
}

async fn create_sqlite_pool(database_url: &str) -> Result<Pool<Sqlite>, sqlx::Error> {
    // 对于 SQLite，确保数据库文件和目录被创建
    if database_url.starts_with("sqlite:") {
        let db_path = database_url.strip_prefix("sqlite:").unwrap_or(database_url);
        let path = Path::new(db_path);

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| sqlx::Error::Io(e))?;
                info!("Created database directory: {:?}", parent);
            }
        }

        // 如果数据库文件不存在，先创建一个空文件
        if !path.exists() {
            info!("Creating database file at: {:?}", path);
            std::fs::File::create(path).map_err(|e| sqlx::Error::Io(e))?;
        }
    }

    // 解析连接选项并添加关键 PRAGMA 设置
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
                // WAL 模式是数据库级设置，在每个连接上设置以确保生效
                conn.execute("PRAGMA journal_mode = WAL;").await?;
                Ok(())
            })
        })
        .connect_with(connect_options)
        .await?;

    info!("SQLite connection pool created (max_connections=5, WAL mode enabled)");

    // 验证 WAL 模式是否成功启用
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

    // 运行迁移
    run_sqlite_migrations(&pool).await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    run_sqlite_migrations(pool).await
}

async fn run_postgres_migrations(pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    info!("Initializing PostgreSQL database schema...");

    let init_sql = include_str!("../../migrations/postgres_init.sql");
    let statements = parse_sql_statements(init_sql);

    for statement in statements {
        if let Err(e) = sqlx::query(&statement).execute(pool).await {
            error!("Failed to execute SQL: {}", statement);
            return Err(e);
        }
    }

    info!("PostgreSQL database initialization completed successfully");
    Ok(())
}

async fn run_sqlite_migrations(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    info!("Initializing database schema...");

    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(pool)
        .await?;

    let init_sql = include_str!("../../migrations/init.sql");
    let statements = parse_sql_statements(init_sql);

    for statement in statements {
        if let Err(e) = sqlx::query(&statement).execute(pool).await {
            error!("Failed to execute SQL: {}", statement);
            sqlx::query("PRAGMA foreign_keys = ON")
                .execute(pool)
                .await
                .ok();
            return Err(e);
        }
    }

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(pool)
        .await?;
    info!("Database initialization completed successfully");
    Ok(())
}

/// Smart SQL script parser that correctly splits multiple SQL statements
///
/// Handles the following cases:
/// - Semicolons within string literals are not treated as statement separators
/// - Ignores single-line comments (starting with --)
/// - Maintains integrity of multi-line statements
/// - Removes empty statements
fn parse_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new(); // Store parsed SQL statements
    let mut current = String::new(); // Current statement being built
    let mut in_string = false; // Whether we're inside a string literal
    let mut in_comment = false; // Whether we're inside a single-line comment
    let mut chars = sql.chars().peekable(); // Character iterator with lookahead support

    while let Some(ch) = chars.next() {
        match ch {
            // Handle single quotes: toggle string state, preserve quotes
            '\'' if !in_comment => {
                in_string = !in_string;
                current.push(ch);
            }
            // Handle comment start: enter comment mode when detecting --
            '-' if !in_string && !in_comment && chars.peek() == Some(&'-') => {
                chars.next(); // consume second -
                in_comment = true;
            }
            // Handle comment end: newline ends single-line comment
            '\n' if in_comment => {
                in_comment = false;
                current.push(ch); // preserve newline
            }
            // Handle statement separator: only semicolons outside strings and comments
            ';' if !in_string && !in_comment => {
                let stmt = current.trim();
                if !stmt.is_empty() {
                    statements.push(stmt.to_string());
                }
                current.clear(); // start new statement
            }
            // Other characters: add to current statement when not in comment
            _ if !in_comment => current.push(ch),
            // Characters in comments: ignore
            _ => {}
        }
    }

    // Handle the last statement (may not end with semicolon)
    let stmt = current.trim();
    if !stmt.is_empty() {
        statements.push(stmt.to_string());
    }

    statements
}
