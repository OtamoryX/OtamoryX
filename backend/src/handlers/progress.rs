use crate::middleware::{auth::AuthInfo, path_permission};
use crate::models::{
    BatchProgressRequest, BatchProgressResponse, ReadingProgress, ReadingProgressHistoryItem,
    ReadingProgressListQuery, RecordBehaviorEventRequest, UpdateProgressRequest,
};
use crate::services::CurationService;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProgressListFilter {
    All,
    Reading,
    Read,
}

fn parse_progress_list_filter(status: Option<&str>) -> Result<ProgressListFilter, ()> {
    match status.unwrap_or("all") {
        "all" => Ok(ProgressListFilter::All),
        "reading" => Ok(ProgressListFilter::Reading),
        "read" => Ok(ProgressListFilter::Read),
        _ => Err(()),
    }
}

fn progress_from_row(row: &SqliteRow) -> ReadingProgress {
    ReadingProgress {
        id: row.get::<Option<String>, _>("id").unwrap_or_default(),
        archive_id: row.get("archive_id"),
        user_id: row.get("user_id"),
        current_page: row.get::<i64, _>("current_page") as i32,
        total_pages: row.get::<i64, _>("total_pages") as i32,
        progress_percentage: row.get("progress_percentage"),
        last_read_at: chrono::DateTime::from_naive_utc_and_offset(
            row.get("last_read_at"),
            chrono::Utc,
        ),
        version: row.get("version"),
    }
}

async fn persist_progress(
    pool: &Pool<Sqlite>,
    user_id: &str,
    archive_id: &str,
    current_page: i32,
    total_pages: i32,
    expected_version: i64,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Option<ReadingProgress>, sqlx::Error> {
    let progress_percentage = if total_pages > 0 {
        (current_page as f64) / (total_pages as f64)
    } else {
        0.0
    };
    let progress_id = uuid::Uuid::new_v4().to_string();

    let result = sqlx::query(
        r#"
        INSERT INTO reading_progress (
            id, user_id, archive_id, current_page, total_pages, progress_percentage,
            version, last_read_at, created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?, ?)
        ON CONFLICT(user_id, archive_id) DO UPDATE SET
            current_page = excluded.current_page,
            total_pages = excluded.total_pages,
            progress_percentage = excluded.progress_percentage,
            version = reading_progress.version + 1,
            last_read_at = excluded.last_read_at,
            updated_at = excluded.updated_at
        WHERE reading_progress.version = ?
        "#,
    )
    .bind(progress_id)
    .bind(user_id)
    .bind(archive_id)
    .bind(current_page)
    .bind(total_pages)
    .bind(progress_percentage)
    .bind(now)
    .bind(now)
    .bind(now)
    .bind(expected_version)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Ok(None);
    }

    let row = sqlx::query(
        "SELECT id, user_id, archive_id, current_page, total_pages, progress_percentage, last_read_at, version
         FROM reading_progress WHERE archive_id = ? AND user_id = ?",
    )
    .bind(archive_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(Some(progress_from_row(&row)))
}

pub async fn get_progress(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    Path(archive_id): Path<String>,
) -> Result<Json<ReadingProgress>, StatusCode> {
    path_permission::authorize_archive_access(&pool, &auth, &archive_id).await?;
    let user_id = &auth.user_id;
    let row = sqlx::query(
        "SELECT id, user_id, archive_id, current_page, total_pages, progress_percentage, last_read_at 
         , version
         FROM reading_progress 
         WHERE archive_id = ? AND user_id = ?",
    )
    .bind(&archive_id)
    .bind(user_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        tracing::error!("Database error getting reading progress: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if let Some(progress_row) = row {
        let progress = progress_from_row(&progress_row);
        Ok(Json(progress))
    } else {
        // 如果没有进度记录，返回默认进度
        let total_pages = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(page_count, 0) FROM archives WHERE id = ?",
        )
        .bind(&archive_id)
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error getting archive page count: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })? as i32;
        let progress = ReadingProgress {
            id: String::new(),
            archive_id: archive_id.clone(),
            user_id: user_id.clone(),
            current_page: 1,
            total_pages,
            progress_percentage: 0.0,
            last_read_at: chrono::Utc::now(),
            version: 0,
        };
        Ok(Json(progress))
    }
}

pub async fn update_progress(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    Path(archive_id): Path<String>,
    Json(request): Json<UpdateProgressRequest>,
) -> Result<Json<ReadingProgress>, StatusCode> {
    path_permission::authorize_archive_access(&pool, &auth, &archive_id).await?;
    let user_id = &auth.user_id;
    // 获取档案的总页数
    let archive_info = sqlx::query!("SELECT page_count FROM archives WHERE id = ?", archive_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error getting archive info: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let total_pages = archive_info.ok_or(StatusCode::NOT_FOUND)?.page_count.max(0) as i32;

    if request.current_page < 1 || (total_pages > 0 && request.current_page > total_pages) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let now = chrono::Utc::now();

    let progress = persist_progress(
        &pool,
        user_id,
        &archive_id,
        request.current_page,
        total_pages,
        request.expected_version,
        now,
    )
    .await
    .map_err(|e| {
        tracing::error!("Database error updating reading progress: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let progress = progress.ok_or(StatusCode::CONFLICT)?;

    // 如果阅读超过第1页，自动移除"new"标签
    if request.current_page > 1 {
        let _ = remove_new_tag(&pool, &archive_id).await;
    }

    // Reading progress is the authoritative page-turn signal for Phase 8.
    // Feedback persistence must not make an otherwise successful progress update fail.
    let behavior_request = RecordBehaviorEventRequest {
        archive_id: Some(archive_id.clone()),
        event_type: "page_turn".to_string(),
        event_key: request.reader_session_id.as_ref().map(|session| {
            format!(
                "{}:{}:{}",
                session,
                request.current_page,
                now.timestamp_nanos_opt().unwrap_or_default()
            )
        }),
        page: Some(request.current_page),
        metadata: serde_json::json!({ "source": "progress", "readerSessionId": request.reader_session_id, "recommendationSessionId": request.recommendation_session_id }),
        occurred_at: Some(now),
    };
    if let Err(error) = CurationService::new(pool.clone())
        .record_event(user_id, &behavior_request)
        .await
    {
        tracing::warn!(
            "Failed to record page-turn behavior for archive {}: {}",
            archive_id,
            error
        );
    }

    tracing::info!(
        "Updated progress for archive {}: page {}/{} ({}%)",
        archive_id,
        request.current_page,
        total_pages,
        (progress.progress_percentage * 100.0) as i32
    );

    Ok(Json(progress))
}

pub async fn list_progress(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    Query(query): Query<ReadingProgressListQuery>,
) -> Result<Json<Vec<ReadingProgressHistoryItem>>, StatusCode> {
    let filter =
        parse_progress_list_filter(query.status.as_deref()).map_err(|_| StatusCode::BAD_REQUEST)?;
    let status_clause = match filter {
        ProgressListFilter::All => "",
        ProgressListFilter::Reading => " AND rp.progress_percentage < 1.0",
        ProgressListFilter::Read => " AND rp.progress_percentage >= 1.0",
    };
    let sql = format!(
        "SELECT rp.id, rp.user_id, rp.archive_id, rp.current_page, rp.total_pages,
                rp.progress_percentage, rp.last_read_at, rp.version,
                a.title, a.subtitle, a.subtitle_language, a.page_count, a.path
         FROM reading_progress rp
         INNER JOIN archives a ON a.id = rp.archive_id
         WHERE rp.user_id = ?{status_clause}
         ORDER BY rp.last_read_at DESC, rp.archive_id DESC"
    );
    let rows = sqlx::query(&sql)
        .bind(&auth.user_id)
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Database error listing reading progress: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let user_paths = if auth.role == "admin" {
        Vec::new()
    } else {
        path_permission::get_user_paths(&pool, &auth.user_id).await?
    };
    let items = rows
        .into_iter()
        .filter(|row| {
            path_permission::has_path_permission_with_paths(
                &auth.role,
                &user_paths,
                row.get("path"),
            )
        })
        .map(|row| {
            let progress = progress_from_row(&row);
            ReadingProgressHistoryItem {
                status: if progress.progress_percentage >= 1.0 {
                    "read".to_string()
                } else {
                    "reading".to_string()
                },
                title: row.get("title"),
                subtitle: row.get("subtitle"),
                subtitle_language: row.get("subtitle_language"),
                page_count: row.get::<i64, _>("page_count") as i32,
                progress,
            }
        })
        .collect();

    Ok(Json(items))
}

async fn remove_new_tag(pool: &Pool<Sqlite>, archive_id: &str) -> Result<(), sqlx::Error> {
    let new_tag_id =
        sqlx::query!("SELECT id FROM tags WHERE name = 'new' AND namespace = 'system'")
            .fetch_optional(pool)
            .await?;

    if let Some(tag) = new_tag_id {
        sqlx::query!(
            "DELETE FROM archive_tags WHERE archive_id = ? AND tag_id = ?",
            archive_id,
            tag.id
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn get_batch_progress(
    State(pool): State<Pool<Sqlite>>,
    axum::extract::Extension(auth): axum::extract::Extension<AuthInfo>,
    Json(request): Json<BatchProgressRequest>,
) -> Result<Json<BatchProgressResponse>, StatusCode> {
    let user_id = &auth.user_id;
    if request.archive_ids.is_empty() {
        return Ok(Json(BatchProgressResponse {
            progress: HashMap::new(),
        }));
    }

    // 限制批量请求的数量以防止性能问题
    let archive_ids = if request.archive_ids.len() > 100 {
        &request.archive_ids[..100]
    } else {
        &request.archive_ids
    };

    let user_paths = if auth.role == "admin" {
        Vec::new()
    } else {
        path_permission::get_user_paths(&pool, &auth.user_id).await?
    };

    // Resolve requested IDs to existing, authorized archives before reading progress or page counts.
    let archive_placeholders = archive_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let archive_sql = format!(
        "SELECT id, path, COALESCE(page_count, 0) AS page_count FROM archives WHERE id IN ({})",
        archive_placeholders
    );
    let mut archive_query = sqlx::query(&archive_sql);
    for archive_id in archive_ids {
        archive_query = archive_query.bind(archive_id);
    }
    let archive_rows = archive_query.fetch_all(&pool).await.map_err(|e| {
        tracing::error!("Database error getting batch archive access: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let mut page_count_map = HashMap::new();
    let mut authorized_archive_ids = Vec::new();
    for row in archive_rows {
        let id: String = row.get("id");
        let path: String = row.get("path");
        if auth.role == "admin"
            || path_permission::has_path_permission_with_paths(&auth.role, &user_paths, &path)
        {
            page_count_map.insert(id.clone(), row.get::<i64, _>("page_count") as i32);
            authorized_archive_ids.push(id);
        }
    }
    if authorized_archive_ids.is_empty() {
        return Ok(Json(BatchProgressResponse {
            progress: HashMap::new(),
        }));
    }

    // 构建IN查询的占位符
    let placeholders = authorized_archive_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let query = format!(
        "SELECT rp.id, rp.user_id, rp.archive_id, rp.current_page, rp.total_pages, 
                rp.progress_percentage, rp.last_read_at, rp.version
         FROM reading_progress rp
         WHERE rp.archive_id IN ({}) AND rp.user_id = ?",
        placeholders
    );

    let mut query_builder = sqlx::query(&query);

    // 添加archive_ids参数
    for archive_id in &authorized_archive_ids {
        query_builder = query_builder.bind(archive_id);
    }

    // 添加user_id参数
    query_builder = query_builder.bind(&user_id);

    let rows = query_builder.fetch_all(&pool).await.map_err(|e| {
        tracing::error!("Database error getting batch reading progress: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut progress_map = HashMap::new();

    // 处理已存在的进度记录
    for row in rows {
        let archive_id: String = row.get("archive_id");
        let progress = progress_from_row(&row);
        progress_map.insert(archive_id, progress);
    }

    // 为没有进度记录的档案创建默认进度，并获取档案的实际页数
    let missing_ids: Vec<&String> = authorized_archive_ids
        .iter()
        .filter(|id| !progress_map.contains_key(*id))
        .collect();

    if !missing_ids.is_empty() {
        for archive_id in &missing_ids {
            let total_pages = page_count_map.get(*archive_id).copied().unwrap_or(0);
            let progress = ReadingProgress {
                id: String::new(),
                archive_id: (*archive_id).clone(),
                user_id: user_id.clone(),
                current_page: 1,
                total_pages,
                progress_percentage: 0.0,
                last_read_at: chrono::Utc::now(),
                version: 0,
            };
            progress_map.insert((*archive_id).clone(), progress);
        }
    }

    tracing::info!(
        "Retrieved batch progress for {} archives for user {}",
        progress_map.len(),
        user_id
    );

    Ok(Json(BatchProgressResponse {
        progress: progress_map,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Extension, Query, State};
    use chrono::NaiveDateTime;
    use sqlx::sqlite::SqlitePoolOptions;

    fn test_auth(user_id: &str) -> AuthInfo {
        AuthInfo {
            user_id: user_id.to_string(),
            role: "user".to_string(),
        }
    }

    async fn progress_pool() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("create sqlite test pool");
        sqlx::query(
            "CREATE TABLE reading_progress (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                archive_id TEXT NOT NULL,
                current_page INTEGER NOT NULL,
                total_pages INTEGER NOT NULL,
                progress_percentage REAL NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                last_read_at DATETIME NOT NULL,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
                UNIQUE(user_id, archive_id)
            )",
        )
        .execute(&pool)
        .await
        .expect("create reading progress table");
        pool
    }

    #[test]
    fn accepts_only_supported_history_filters() {
        assert_eq!(
            parse_progress_list_filter(None),
            Ok(ProgressListFilter::All)
        );
        assert_eq!(
            parse_progress_list_filter(Some("reading")),
            Ok(ProgressListFilter::Reading)
        );
        assert_eq!(
            parse_progress_list_filter(Some("read")),
            Ok(ProgressListFilter::Read)
        );
        assert_eq!(parse_progress_list_filter(Some("finished")), Err(()));
    }

    #[tokio::test]
    async fn stale_progress_version_cannot_overwrite_newer_progress() {
        let pool = progress_pool().await;
        let first_time = chrono::DateTime::from_naive_utc_and_offset(
            NaiveDateTime::parse_from_str("2026-09-05 10:00:00", "%Y-%m-%d %H:%M:%S")
                .expect("valid timestamp"),
            chrono::Utc,
        );
        let first = persist_progress(&pool, "user-1", "archive-1", 3, 10, 0, first_time)
            .await
            .expect("initial progress write")
            .expect("initial write should succeed");
        assert_eq!(first.version, 1);

        let stale = persist_progress(&pool, "user-1", "archive-1", 2, 10, 0, first_time)
            .await
            .expect("stale progress write should be handled");
        assert!(stale.is_none());

        let current_time = first_time + chrono::Duration::minutes(1);
        let current = persist_progress(&pool, "user-1", "archive-1", 5, 10, 1, current_time)
            .await
            .expect("current progress write")
            .expect("current write should succeed");
        assert_eq!(current.current_page, 5);
        assert_eq!(current.version, 2);
    }

    #[tokio::test]
    async fn lists_current_user_progress_in_recent_order_and_filters_status() {
        let pool = progress_pool().await;
        sqlx::query(
            "CREATE TABLE archives (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                subtitle TEXT,
                subtitle_language TEXT,
                path TEXT NOT NULL,
                page_count INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("create archives table");
        sqlx::query("CREATE TABLE user_paths (user_id TEXT NOT NULL, path TEXT NOT NULL)")
            .execute(&pool)
            .await
            .expect("create user paths table");
        for (id, title, page_count, path) in [
            ("archive-1", "Older book", 10_i64, "/books/older.cbz"),
            ("archive-2", "Finished book", 8_i64, "/books/finished.cbz"),
            ("archive-3", "Other user book", 12_i64, "/books/other.cbz"),
        ] {
            sqlx::query("INSERT INTO archives (id, title, page_count, path) VALUES (?, ?, ?, ?)")
                .bind(id)
                .bind(title)
                .bind(page_count)
                .bind(path)
                .execute(&pool)
                .await
                .expect("insert archive");
        }
        let first_time = NaiveDateTime::parse_from_str("2026-09-05 10:00:00", "%Y-%m-%d %H:%M:%S")
            .expect("valid timestamp");
        let second_time = NaiveDateTime::parse_from_str("2026-09-05 11:00:00", "%Y-%m-%d %H:%M:%S")
            .expect("valid timestamp");
        for (user_id, archive_id, page, total, percentage, timestamp, version) in [
            (
                "user-1",
                "archive-1",
                3_i64,
                10_i64,
                0.3_f64,
                first_time,
                1_i64,
            ),
            (
                "user-1",
                "archive-2",
                8_i64,
                8_i64,
                1.0_f64,
                second_time,
                2_i64,
            ),
            (
                "user-2",
                "archive-3",
                2_i64,
                12_i64,
                0.16_f64,
                second_time,
                1_i64,
            ),
        ] {
            sqlx::query(
                "INSERT INTO reading_progress
                 (id, user_id, archive_id, current_page, total_pages, progress_percentage,
                  version, last_read_at, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            )
            .bind(format!("progress-{archive_id}"))
            .bind(user_id)
            .bind(archive_id)
            .bind(page)
            .bind(total)
            .bind(percentage)
            .bind(version)
            .bind(timestamp)
            .execute(&pool)
            .await
            .expect("insert progress");
        }

        let Json(all) = list_progress(
            State(pool.clone()),
            Extension(test_auth("user-1")),
            Query(ReadingProgressListQuery { status: None }),
        )
        .await
        .expect("list all progress");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].progress.archive_id, "archive-2");
        assert_eq!(all[0].status, "read");
        assert_eq!(all[1].progress.archive_id, "archive-1");
        assert_eq!(all[1].status, "reading");

        let Json(reading) = list_progress(
            State(pool.clone()),
            Extension(test_auth("user-1")),
            Query(ReadingProgressListQuery {
                status: Some("reading".to_string()),
            }),
        )
        .await
        .expect("list reading progress");
        assert_eq!(reading.len(), 1);
        assert_eq!(reading[0].progress.archive_id, "archive-1");

        let Json(read) = list_progress(
            State(pool),
            Extension(test_auth("user-1")),
            Query(ReadingProgressListQuery {
                status: Some("read".to_string()),
            }),
        )
        .await
        .expect("list read progress");
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].progress.archive_id, "archive-2");
    }
}
