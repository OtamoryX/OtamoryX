use super::*;
use crate::models::TrashEntry;
use sqlx::sqlite::SqlitePoolOptions;
use std::path::{Path, PathBuf};
use uuid::Uuid;

async fn setup() -> (Pool<Sqlite>, std::path::PathBuf) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE archives (id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT, path TEXT NOT NULL, file_hash TEXT UNIQUE NOT NULL, file_size INTEGER NOT NULL, page_count INTEGER NOT NULL, created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL)").execute(&pool).await.unwrap();
    sqlx::query(
        "CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL, PRIMARY KEY (archive_id, tag_id), FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE, FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE)").execute(&pool).await.unwrap();
    sqlx::query("CREATE TABLE categories (id TEXT PRIMARY KEY, name TEXT NOT NULL)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE category_archives (category_id TEXT NOT NULL, archive_id TEXT NOT NULL, PRIMARY KEY (category_id, archive_id), FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE, FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE)")
            .execute(&pool)
            .await
            .unwrap();
    sqlx::query("CREATE TABLE reading_progress (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, archive_id TEXT NOT NULL, current_page INTEGER NOT NULL DEFAULT 1, total_pages INTEGER NOT NULL DEFAULT 0, progress_percentage REAL NOT NULL DEFAULT 0.0, last_read_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(user_id, archive_id), FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE)")
            .execute(&pool)
            .await
            .unwrap();
    sqlx::query("CREATE TABLE collections (id TEXT PRIMARY KEY, cover_archive_id TEXT, FOREIGN KEY (cover_archive_id) REFERENCES archives(id) ON DELETE SET NULL)")
            .execute(&pool)
            .await
            .unwrap();
    sqlx::query("CREATE TABLE trash_entries (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, archive_id TEXT NOT NULL, original_path TEXT NOT NULL, trash_path TEXT, reason TEXT, rule_version TEXT, rule_id TEXT, evaluation_id TEXT, model_confidence REAL, metadata_json TEXT NOT NULL, operation_id TEXT, operation_type TEXT, decision_key TEXT, status TEXT NOT NULL, deleted_at DATETIME NOT NULL, expires_at DATETIME, restored_at DATETIME, cleanup_attempts INTEGER NOT NULL DEFAULT 0, last_cleanup_attempt_at DATETIME, last_cleanup_error TEXT, expired_at DATETIME, restore_claimed_at DATETIME)").execute(&pool).await.unwrap();
    sqlx::query("CREATE TABLE trash_operations (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, operation_type TEXT NOT NULL, group_key TEXT NOT NULL, keep_archive_id TEXT NOT NULL, idempotency_key TEXT NOT NULL, migration_snapshot_json TEXT NOT NULL DEFAULT '{}', status TEXT NOT NULL, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, expires_at DATETIME, restored_at DATETIME)").execute(&pool).await.unwrap();
    sqlx::query("CREATE TABLE trash_operation_members (id TEXT PRIMARY KEY, operation_id TEXT NOT NULL, archive_id TEXT NOT NULL, trash_entry_id TEXT NOT NULL, migration_snapshot_json TEXT NOT NULL DEFAULT '{}', sequence INTEGER NOT NULL DEFAULT 0, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, UNIQUE(operation_id, archive_id), UNIQUE(operation_id, sequence), UNIQUE(trash_entry_id), FOREIGN KEY (operation_id) REFERENCES trash_operations(id) ON DELETE CASCADE, FOREIGN KEY (trash_entry_id) REFERENCES trash_entries(id) ON DELETE RESTRICT)").execute(&pool).await.unwrap();
    let temp_dir = std::env::temp_dir().join(format!("otamoryx-trash-test-{}", Uuid::new_v4()));
    tokio::fs::create_dir_all(&temp_dir).await.unwrap();
    (pool, temp_dir)
}

async fn install_theme_archive_tag_insert_guard(pool: &Pool<Sqlite>) {
    sqlx::query(
        "CREATE TRIGGER prevent_theme_archive_tag_insert
         BEFORE INSERT ON archive_tags
         FOR EACH ROW
         WHEN EXISTS (
             SELECT 1 FROM tags
             WHERE id = NEW.tag_id AND lower(trim(namespace)) = 'theme'
         )
         BEGIN
             SELECT RAISE(ABORT, 'system-managed theme tags cannot be stored in archive_tags');
         END",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_trash_entry(
    pool: &Pool<Sqlite>,
    id: &str,
    trash_path: Option<&Path>,
    status: &str,
    expires_at: &str,
) {
    sqlx::query(
        "INSERT INTO trash_entries
             (id, user_id, archive_id, original_path, trash_path, metadata_json, status,
              deleted_at, expires_at)
             VALUES (?, 'u1', ?, '/library/book.cbz', ?, '{}', ?, CURRENT_TIMESTAMP, ?)",
    )
    .bind(id)
    .bind(format!("archive-{id}"))
    .bind(trash_path.map(|path| path.to_string_lossy().to_string()))
    .bind(status)
    .bind(expires_at)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn permanently_purges_a_manual_entry() {
    let (pool, temp_dir) = setup().await;
    let trash_path = temp_dir.join("manual.cbz");
    tokio::fs::write(&trash_path, b"manual").await.unwrap();
    insert_trash_entry(
        &pool,
        "manual",
        Some(&trash_path),
        "active",
        "2999-01-01T00:00:00Z",
    )
    .await;

    TrashService::new(pool.clone())
        .purge_entry("u1", "manual")
        .await
        .unwrap();

    assert!(!trash_path.exists());
    let state: (String, Option<String>) =
        sqlx::query_as("SELECT status, expired_at FROM trash_entries WHERE id = 'manual'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(state.0, "expired");
    assert!(state.1.is_some());
}

#[tokio::test]
async fn permanently_purges_all_members_of_a_version_operation() {
    let (pool, temp_dir) = setup().await;
    sqlx::query(
        "INSERT INTO trash_operations
             (id, user_id, operation_type, group_key, keep_archive_id, idempotency_key, status)
             VALUES ('op-1', 'u1', 'version_cleanup', 'group', 'keeper', 'key', 'active')",
    )
    .execute(&pool)
    .await
    .unwrap();
    for (index, id) in ["member-a", "member-b"].iter().enumerate() {
        let path = temp_dir.join(format!("{id}.cbz"));
        tokio::fs::write(&path, id.as_bytes()).await.unwrap();
        insert_trash_entry(&pool, id, Some(&path), "active", "2999-01-01T00:00:00Z").await;
        sqlx::query(
                "UPDATE trash_entries SET operation_id = 'op-1', operation_type = 'version_cleanup' WHERE id = ?",
            )
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO trash_operation_members
                 (id, operation_id, archive_id, trash_entry_id, sequence)
                 VALUES (?, 'op-1', ?, ?, ?)",
        )
        .bind(format!("member-row-{index}"))
        .bind(format!("archive-{id}"))
        .bind(id)
        .bind(index as i64)
        .execute(&pool)
        .await
        .unwrap();
    }

    TrashService::new(pool.clone())
        .purge_operation("u1", "op-1")
        .await
        .unwrap();

    let status: String =
        sqlx::query_scalar("SELECT status FROM trash_operations WHERE id = 'op-1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "expired");
    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM trash_entries WHERE operation_id = 'op-1' AND status = 'active'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(active, 0);
}

async fn seed_version_cleanup(pool: &Pool<Sqlite>, temp_dir: &Path) -> (PathBuf, PathBuf) {
    let keeper_path = temp_dir.join("keeper.cbz");
    let source_path = temp_dir.join("source.cbz");
    tokio::fs::write(&keeper_path, b"keeper").await.unwrap();
    tokio::fs::write(&source_path, b"source").await.unwrap();
    for (id, title, path, hash) in [
        ("keeper", "Keeper", &keeper_path, "hash-keeper"),
        ("source", "Source", &source_path, "hash-source"),
    ] {
        sqlx::query(
            "INSERT INTO archives
                 (id, title, path, file_hash, file_size, page_count, created_at, updated_at)
                 VALUES (?, ?, ?, ?, 4, 20, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(id)
        .bind(title)
        .bind(path.to_string_lossy().as_ref())
        .bind(hash)
        .execute(pool)
        .await
        .unwrap();
    }
    for (id, name, namespace) in [
        ("tag-keeper", "Keeper", "test"),
        ("tag-source", "Source", "test"),
        ("theme-keeper", "Keeper theme", "theme"),
        ("theme-source", "Source theme", "theme"),
    ] {
        sqlx::query("INSERT INTO tags (id, name, namespace) VALUES (?, ?, ?)")
            .bind(id)
            .bind(name)
            .bind(namespace)
            .execute(pool)
            .await
            .unwrap();
    }
    for (id, name) in [("cat-keeper", "Keeper"), ("cat-source", "Source")] {
        sqlx::query("INSERT INTO categories (id, name) VALUES (?, ?)")
            .bind(id)
            .bind(name)
            .execute(pool)
            .await
            .unwrap();
    }
    sqlx::query(
        "INSERT INTO archive_tags (archive_id, tag_id) VALUES
        ('keeper', 'tag-keeper'), ('source', 'tag-source'),
        ('keeper', 'theme-keeper'), ('source', 'theme-source')",
    )
    .execute(pool)
    .await
    .unwrap();
    install_theme_archive_tag_insert_guard(pool).await;
    sqlx::query("INSERT INTO category_archives (category_id, archive_id) VALUES ('cat-keeper', 'keeper'), ('cat-source', 'source')")
            .execute(pool)
            .await
            .unwrap();
    sqlx::query(
            "INSERT INTO reading_progress
             (id, user_id, archive_id, current_page, total_pages, progress_percentage, last_read_at, created_at, updated_at)
             VALUES
             ('keeper-progress', 'u1', 'keeper', 2, 20, 0.1, '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z'),
             ('source-progress', 'u1', 'source', 5, 10, 0.5, '2024-02-01T00:00:00Z', '2024-02-01T00:00:00Z', '2024-02-01T00:00:00Z')",
        )
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
            "INSERT INTO trash_operations
             (id, user_id, operation_type, group_key, keep_archive_id, idempotency_key,
              migration_snapshot_json, status, expires_at)
             VALUES ('operation-1', 'u1', 'version_cleanup', 'group-1', 'keeper', 'key-1', '{}', 'pending', '2999-01-01T00:00:00Z')",
        )
        .execute(pool)
        .await
        .unwrap();
    (keeper_path, source_path)
}

async fn move_seeded_member(service: &TrashService) -> TrashEntry {
    service
        .move_version_group_member_to_trash("u1", "operation-1", "source", "keeper", 20)
        .await
        .unwrap()
}

#[tokio::test]
async fn moves_archive_and_restores_snapshot() {
    let (pool, temp_dir) = setup().await;
    let path = temp_dir.join("book.cbz");
    tokio::fs::write(&path, b"book").await.unwrap();
    sqlx::query("INSERT INTO archives (id, title, path, file_hash, file_size, page_count, created_at, updated_at) VALUES ('a1', 'Book', ?, 'hash-a1', 4, 2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
            .bind(path.to_string_lossy().as_ref()).execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tags (id, name, namespace) VALUES
         ('tag-ordinary', 'Ordinary', 'general'),
         ('tag-theme', 'Legacy theme', 'theme')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO archive_tags (archive_id, tag_id) VALUES
         ('a1', 'tag-ordinary'), ('a1', 'tag-theme')",
    )
    .execute(&pool)
    .await
    .unwrap();
    install_theme_archive_tag_insert_guard(&pool).await;
    sqlx::query("INSERT INTO categories (id, name) VALUES ('cat-1', 'Favorites')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO category_archives (category_id, archive_id) VALUES ('cat-1', 'a1')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO reading_progress (id, user_id, archive_id, current_page) VALUES ('progress-1', 'u1', 'a1', 7)")
            .execute(&pool)
            .await
            .unwrap();
    sqlx::query("INSERT INTO collections (id, cover_archive_id) VALUES ('collection-1', 'a1')")
        .execute(&pool)
        .await
        .unwrap();

    let service = TrashService::new(pool.clone());
    let entry = service
        .move_archive_to_trash("u1", "a1", Some("manual"), "user")
        .await
        .unwrap();
    assert_eq!(entry.status, "active");
    assert!(!path.exists());
    assert!(entry
        .trash_path
        .as_deref()
        .is_some_and(|path| std::path::Path::new(path).exists()));
    assert!(sqlx::query("SELECT id FROM archives WHERE id = 'a1'")
        .fetch_optional(&pool)
        .await
        .unwrap()
        .is_none());

    service.restore_entry("u1", &entry.id).await.unwrap();
    assert!(path.exists());
    assert!(sqlx::query("SELECT id FROM archives WHERE id = 'a1'")
        .fetch_optional(&pool)
        .await
        .unwrap()
        .is_some());
    assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM category_archives WHERE category_id = 'cat-1' AND archive_id = 'a1'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            1
        );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM archive_tags WHERE archive_id = 'a1' AND tag_id = 'tag-ordinary'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM archive_tags WHERE archive_id = 'a1' AND tag_id = 'tag-theme'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT current_page FROM reading_progress WHERE id = 'progress-1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        7
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT cover_archive_id FROM collections WHERE id = 'collection-1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "a1"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM trash_entries WHERE id = ?")
            .bind(&entry.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "restored"
    );
    let restore_claim_released: i64 =
        sqlx::query_scalar("SELECT restore_claimed_at IS NULL FROM trash_entries WHERE id = ?")
            .bind(&entry.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(restore_claim_released, 1);
    let _ = tokio::fs::remove_dir_all(temp_dir).await;
}

#[tokio::test]
async fn restores_version_cleanup_relations_as_one_operation() {
    let (pool, temp_dir) = setup().await;
    let (_keeper_path, source_path) = seed_version_cleanup(&pool, &temp_dir).await;
    let service = TrashService::new(pool.clone());
    let entry = move_seeded_member(&service).await;
    sqlx::query("UPDATE trash_operations SET status = 'active' WHERE id = 'operation-1'")
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT tag_id FROM archive_tags WHERE archive_id = 'keeper' ORDER BY tag_id",
        )
        .fetch_all(&pool)
        .await
        .unwrap(),
        vec![
            "tag-keeper".to_string(),
            "tag-source".to_string(),
            "theme-keeper".to_string(),
            "theme-source".to_string(),
        ]
    );
    assert_eq!(
            sqlx::query_as::<_, (i32, i32, f64)>(
                "SELECT current_page, total_pages, progress_percentage FROM reading_progress WHERE archive_id = 'keeper' AND user_id = 'u1'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            (10, 20, 0.5)
        );

    let restored = service
        .restore_operation("u1", "operation-1")
        .await
        .unwrap();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].id, entry.id);
    assert_eq!(restored[0].status, "restored");
    assert!(source_path.exists());
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT tag_id FROM archive_tags WHERE archive_id = 'keeper' ORDER BY tag_id",
        )
        .fetch_all(&pool)
        .await
        .unwrap(),
        vec!["tag-keeper".to_string(), "theme-keeper".to_string()]
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT tag_id FROM archive_tags WHERE archive_id = 'source' ORDER BY tag_id",
        )
        .fetch_all(&pool)
        .await
        .unwrap(),
        vec!["tag-source".to_string(), "theme-source".to_string()]
    );
    assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT category_id FROM category_archives WHERE archive_id = 'keeper' ORDER BY category_id",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "cat-keeper"
        );
    assert_eq!(
            sqlx::query_as::<_, (String, i32, i32, f64)>(
                "SELECT id, current_page, total_pages, progress_percentage FROM reading_progress WHERE archive_id = 'keeper' AND user_id = 'u1'",
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            ("keeper-progress".to_string(), 2, 20, 0.1)
        );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM archives WHERE id = 'source'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM reading_progress WHERE archive_id = 'source'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM trash_operations WHERE id = 'operation-1'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "restored"
    );
    let _ = tokio::fs::remove_dir_all(temp_dir).await;
}

#[tokio::test]
async fn rolls_back_a_pending_version_cleanup_without_leaving_keeper_changes() {
    let (pool, temp_dir) = setup().await;
    let (_keeper_path, source_path) = seed_version_cleanup(&pool, &temp_dir).await;
    let service = TrashService::new(pool.clone());
    move_seeded_member(&service).await;

    service
        .rollback_version_cleanup("u1", "operation-1")
        .await
        .unwrap();

    assert!(source_path.exists());
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT tag_id FROM archive_tags WHERE archive_id = 'keeper' ORDER BY tag_id",
        )
        .fetch_all(&pool)
        .await
        .unwrap(),
        vec!["tag-keeper".to_string(), "theme-keeper".to_string()]
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT tag_id FROM archive_tags WHERE archive_id = 'source' ORDER BY tag_id",
        )
        .fetch_all(&pool)
        .await
        .unwrap(),
        vec!["tag-source".to_string(), "theme-source".to_string()]
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM trash_operations WHERE id = 'operation-1'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "failed"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM trash_entries WHERE operation_id = 'operation-1'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "restored"
    );
    let _ = tokio::fs::remove_dir_all(temp_dir).await;
}

#[tokio::test]
async fn rejects_single_entry_restore_for_version_cleanup() {
    let (pool, temp_dir) = setup().await;
    let (_keeper_path, _source_path) = seed_version_cleanup(&pool, &temp_dir).await;
    let service = TrashService::new(pool.clone());
    let entry = move_seeded_member(&service).await;

    let error = service.restore_entry("u1", &entry.id).await.unwrap_err();
    assert!(error
        .to_string()
        .contains("must be restored through their operation"));
    let _ = tokio::fs::remove_dir_all(temp_dir).await;
}

#[tokio::test]
async fn rejects_version_cleanup_restore_after_keeper_relation_drift() {
    let (pool, temp_dir) = setup().await;
    let (_keeper_path, source_path) = seed_version_cleanup(&pool, &temp_dir).await;
    let service = TrashService::new(pool.clone());
    let entry = move_seeded_member(&service).await;
    sqlx::query("UPDATE trash_operations SET status = 'active' WHERE id = 'operation-1'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO tags (id, name, namespace) VALUES ('tag-external', 'External', 'test')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO archive_tags (archive_id, tag_id) VALUES ('keeper', 'tag-external')")
        .execute(&pool)
        .await
        .unwrap();

    let error = service
        .restore_operation("u1", "operation-1")
        .await
        .unwrap_err();
    assert!(error.to_string().contains("changed since cleanup"));
    assert!(!source_path.exists());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM archives WHERE id = 'source'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM trash_entries WHERE id = ?")
            .bind(&entry.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "active"
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM trash_operations WHERE id = 'operation-1'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "active"
    );
    let _ = tokio::fs::remove_dir_all(temp_dir).await;
}

#[tokio::test]
async fn defers_expiration_while_a_restore_claim_is_fresh() {
    let (pool, temp_dir) = setup().await;
    let path = temp_dir.join("claimed.cbz");
    tokio::fs::write(&path, b"claimed").await.unwrap();
    insert_trash_entry(
        &pool,
        "claimed",
        Some(&path),
        "active",
        "2000-01-01T00:00:00Z",
    )
    .await;
    sqlx::query(
        "UPDATE trash_entries SET restore_claimed_at = CURRENT_TIMESTAMP WHERE id = 'claimed'",
    )
    .execute(&pool)
    .await
    .unwrap();

    let report = TrashService::new(pool.clone())
        .cleanup_expired_entries(100)
        .await
        .unwrap();
    assert_eq!(report, TrashCleanupReport::default());
    assert!(path.exists());
    let _ = tokio::fs::remove_dir_all(temp_dir).await;
}

#[tokio::test]
async fn expires_due_entries_without_touching_future_or_restored_entries() {
    let (pool, temp_dir) = setup().await;
    let due_path = temp_dir.join("due.cbz");
    let future_path = temp_dir.join("future.cbz");
    let restored_path = temp_dir.join("restored.cbz");
    tokio::fs::write(&due_path, b"due").await.unwrap();
    tokio::fs::write(&future_path, b"future").await.unwrap();
    tokio::fs::write(&restored_path, b"restored").await.unwrap();
    insert_trash_entry(
        &pool,
        "due",
        Some(&due_path),
        "active",
        "2000-01-01T00:00:00Z",
    )
    .await;
    insert_trash_entry(
        &pool,
        "future",
        Some(&future_path),
        "active",
        "2999-01-01T00:00:00Z",
    )
    .await;
    insert_trash_entry(
        &pool,
        "restored",
        Some(&restored_path),
        "restored",
        "2000-01-01T00:00:00Z",
    )
    .await;

    let report = TrashService::new(pool.clone())
        .cleanup_expired_entries(100)
        .await
        .unwrap();
    assert_eq!(
        report,
        TrashCleanupReport {
            claimed: 1,
            deleted_files: 1,
            missing_files: 0,
            failed: 0,
        }
    );
    assert!(!due_path.exists());
    assert!(future_path.exists());
    assert!(restored_path.exists());

    let due = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT status, cleanup_attempts, expired_at IS NOT NULL
             FROM trash_entries WHERE id = 'due'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(due, ("expired".to_string(), 1, 1));
    let _ = tokio::fs::remove_dir_all(temp_dir).await;
}

#[tokio::test]
async fn marks_missing_expired_files_complete_idempotently() {
    let (pool, temp_dir) = setup().await;
    let missing_path = temp_dir.join("missing.cbz");
    insert_trash_entry(
        &pool,
        "missing",
        Some(&missing_path),
        "active",
        "2000-01-01T00:00:00Z",
    )
    .await;

    let service = TrashService::new(pool.clone());
    let first = service.cleanup_expired_entries(100).await.unwrap();
    assert_eq!(first.missing_files, 1);
    assert_eq!(first.failed, 0);
    let second = service.cleanup_expired_entries(100).await.unwrap();
    assert_eq!(second, TrashCleanupReport::default());

    let complete: i64 =
        sqlx::query_scalar("SELECT expired_at IS NOT NULL FROM trash_entries WHERE id = 'missing'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(complete, 1);
    let _ = tokio::fs::remove_dir_all(temp_dir).await;
}

#[tokio::test]
async fn retries_failed_file_deletion_after_the_file_becomes_removable() {
    let (pool, temp_dir) = setup().await;
    let blocked_path = temp_dir.join("blocked.cbz");
    tokio::fs::create_dir(&blocked_path).await.unwrap();
    insert_trash_entry(
        &pool,
        "blocked",
        Some(&blocked_path),
        "active",
        "2000-01-01T00:00:00Z",
    )
    .await;

    let service = TrashService::new(pool.clone());
    let failed = service.cleanup_expired_entries(100).await.unwrap();
    assert_eq!(failed.claimed, 1);
    assert_eq!(failed.failed, 1);
    let state = sqlx::query_as::<_, (String, i64, Option<String>, i64)>(
        "SELECT status, cleanup_attempts, last_cleanup_error, expired_at IS NOT NULL
             FROM trash_entries WHERE id = 'blocked'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(state.0, "expired");
    assert_eq!(state.1, 1);
    assert!(state.2.is_some());
    assert_eq!(state.3, 0);

    tokio::fs::remove_dir(&blocked_path).await.unwrap();
    tokio::fs::write(&blocked_path, b"retry").await.unwrap();
    sqlx::query(
        "UPDATE trash_entries
             SET last_cleanup_attempt_at = datetime('now', '-10 minutes') WHERE id = 'blocked'",
    )
    .execute(&pool)
    .await
    .unwrap();

    let retried = service.cleanup_expired_entries(100).await.unwrap();
    assert_eq!(retried.claimed, 1);
    assert_eq!(retried.deleted_files, 1);
    assert_eq!(retried.failed, 0);
    assert!(!blocked_path.exists());
    let state = sqlx::query_as::<_, (i64, Option<String>, i64)>(
        "SELECT cleanup_attempts, last_cleanup_error, expired_at IS NOT NULL
             FROM trash_entries WHERE id = 'blocked'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(state.0, 2);
    assert_eq!(state.1, None);
    assert_eq!(state.2, 1);
    let _ = tokio::fs::remove_dir_all(temp_dir).await;
}

#[tokio::test]
async fn retries_cleanup_after_final_database_update_fails() {
    let (pool, temp_dir) = setup().await;
    let path = temp_dir.join("finalization.cbz");
    tokio::fs::write(&path, b"finalization").await.unwrap();
    insert_trash_entry(
        &pool,
        "finalization",
        Some(&path),
        "active",
        "2000-01-01T00:00:00Z",
    )
    .await;
    sqlx::query(
        "CREATE TRIGGER fail_trash_finalization
             BEFORE UPDATE OF expired_at ON trash_entries
             WHEN NEW.expired_at IS NOT NULL
             BEGIN SELECT RAISE(ABORT, 'forced finalization failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();

    let service = TrashService::new(pool.clone());
    let failed = service.cleanup_expired_entries(100).await.unwrap();
    assert_eq!(failed.deleted_files, 1);
    assert_eq!(failed.failed, 1);
    assert!(!path.exists());
    let pending = sqlx::query_as::<_, (i64, Option<String>)>(
        "SELECT expired_at IS NOT NULL, last_cleanup_error
             FROM trash_entries WHERE id = 'finalization'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(pending.0, 0);
    assert!(pending.1.is_some());

    sqlx::query("DROP TRIGGER fail_trash_finalization")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE trash_entries
             SET last_cleanup_attempt_at = datetime('now', '-10 minutes')
             WHERE id = 'finalization'",
    )
    .execute(&pool)
    .await
    .unwrap();
    let retried = service.cleanup_expired_entries(100).await.unwrap();
    assert_eq!(retried.claimed, 1);
    assert_eq!(retried.missing_files, 1);
    assert_eq!(retried.failed, 0);

    let completed: i64 = sqlx::query_scalar(
        "SELECT expired_at IS NOT NULL FROM trash_entries WHERE id = 'finalization'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(completed, 1);
    let _ = tokio::fs::remove_dir_all(temp_dir).await;
}
