use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::time::Duration;
use tracing::{error, info};

pub async fn create_pool(database_url: &str) -> Result<Pool<Sqlite>, sqlx::Error> {
    // 对于 SQLite，确保数据库文件被创建
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
    info!("Running database migrations...");

    // 创建migrations表来跟踪已执行的迁移
    sqlx::query!(
        r#"
        CREATE TABLE IF NOT EXISTS _migrations (
            version TEXT PRIMARY KEY,
            executed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(pool)
    .await?;

    // 定义所有迁移
    let migrations = vec![
        (
            "20240101000001_init",
            include_str!("../../migrations/20240101000001_init.sql"),
        ),
        (
            "20240101000002_add_page_count",
            include_str!("../../migrations/20240101000002_add_page_count.sql"),
        ),
        (
            "20240101000003_add_categories_and_progress",
            include_str!("../../migrations/20240101000003_add_categories_and_progress.sql"),
        ),
    ];

    for (version, sql) in migrations {
        // 检查迁移是否已经执行
        let existing = sqlx::query!("SELECT version FROM _migrations WHERE version = ?", version)
            .fetch_optional(pool)
            .await?;

        if existing.is_none() {
            info!("Executing migration: {}", version);

            // 执行迁移SQL
            if let Err(e) = sqlx::query(sql).execute(pool).await {
                error!("Failed to execute migration {}: {}", version, e);
                return Err(e);
            }

            // 记录迁移已执行
            sqlx::query!("INSERT INTO _migrations (version) VALUES (?)", version)
                .execute(pool)
                .await?;

            info!("Migration {} completed", version);
        }
    }

    info!("All migrations completed successfully");
    Ok(())
}
