use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing database...");

    // 创建数据库连接池
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite:otamoryx.db")
        .await?;

    // 创建migrations跟踪表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS _migrations (
            version TEXT PRIMARY KEY,
            executed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#
    )
    .execute(&pool)
    .await?;

    // 运行所有迁移
    let migrations = vec![
        ("20240101000001_init", include_str!("../../migrations/20240101000001_init.sql")),
        ("20240101000002_add_page_count", include_str!("../../migrations/20240101000002_add_page_count.sql")),
        ("20240101000003_add_categories_and_progress", include_str!("../../migrations/20240101000003_add_categories_and_progress.sql")),
    ];

    for (version, sql) in migrations {
        println!("Running migration: {}", version);
        
        // 执行迁移SQL
        sqlx::query(sql).execute(&pool).await?;
        
        // 记录迁移已执行
        sqlx::query("INSERT OR IGNORE INTO _migrations (version) VALUES (?)")
            .bind(version)
            .execute(&pool)
            .await?;
        
        println!("Migration {} completed", version);
    }

    println!("Database initialization completed successfully!");
    Ok(())
}