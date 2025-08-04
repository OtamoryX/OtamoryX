use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::path::Path;
use std::time::Duration;
use tracing::{error, info};

pub async fn create_pool(database_url: &str) -> Result<Pool<Sqlite>, sqlx::Error> {
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

    let pool = SqlitePoolOptions::new()
        .max_connections(20)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Some(Duration::from_secs(600)))
        .connect_with(
            database_url
                .parse()
                .map_err(|e| sqlx::Error::Configuration(Box::new(e)))?,
        )
        .await?;

    // 运行迁移
    run_migrations(&pool).await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    info!("Initializing database schema...");

    // 执行完整的初始化SQL
    let init_sql = include_str!("../../migrations/init.sql");

    // 分割SQL语句并逐个执行
    for statement in init_sql.split(';') {
        let statement = statement.trim();
        if !statement.is_empty() && !statement.starts_with("--") {
            if let Err(e) = sqlx::query(statement).execute(pool).await {
                error!("Failed to execute SQL statement: {}", statement);
                error!("Error: {}", e);
                return Err(e);
            }
        }
    }

    info!("Database initialization completed successfully");
    Ok(())
}
