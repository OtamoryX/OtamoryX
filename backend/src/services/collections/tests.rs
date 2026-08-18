use super::*;
use sqlx::sqlite::SqlitePoolOptions;

fn archive(path: &str) -> ArchiveRow {
    ArchiveRow {
        id: "a".into(),
        title: "title".into(),
        path: path.into(),
    }
}

async fn version_cleanup_idempotency_pool() -> Pool<Sqlite> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql(
            "CREATE TABLE trash_operations (
                id TEXT PRIMARY KEY, user_id TEXT NOT NULL, operation_type TEXT NOT NULL,
                idempotency_key TEXT NOT NULL, migration_snapshot_json TEXT NOT NULL, status TEXT NOT NULL
            );
            CREATE TABLE trash_entries (id TEXT PRIMARY KEY, status TEXT NOT NULL);
            CREATE TABLE trash_operation_members (
                operation_id TEXT NOT NULL, archive_id TEXT NOT NULL, trash_entry_id TEXT NOT NULL
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
    pool
}

async fn insert_version_cleanup_operation(
    pool: &Pool<Sqlite>,
    status: &str,
    failed_archive_ids: Vec<String>,
) {
    let snapshot = VersionCleanupRequestSnapshot {
        version: 1,
        group_id: "versions-group".to_string(),
        group_key: "group".to_string(),
        keep_archive_id: "keeper".to_string(),
        delete_archive_ids: vec!["source-a".to_string(), "source-b".to_string()],
        failed_archive_ids,
    };
    sqlx::query(
        "INSERT INTO trash_operations
             (id, user_id, operation_type, idempotency_key, migration_snapshot_json, status)
             VALUES ('operation', 'user', 'version_cleanup', 'key', ?, ?)",
    )
    .bind(serde_json::to_string(&snapshot).unwrap())
    .bind(status)
    .execute(pool)
    .await
    .unwrap();
}

#[test]
fn distinguishes_revision_and_magazine_numbers() {
    let revision = parse_identity(&archive("/x/[artist] Work [v2].zip"));
    assert_eq!(revision.edition_marker.as_deref(), Some("v2"));
    assert_eq!(revision.volume_number, None);
    let issue = parse_identity(&archive("/x/[artist] Work (COMIC X-EROS #107).zip"));
    assert_eq!(issue.issue_number.as_deref(), Some("107"));
    assert_eq!(issue.unit_type, "issue");
}

#[test]
fn parses_chapter_and_volume_markers() {
    let chapter = parse_identity(&archive("/x/[artist] Work Chapter 2 [Chinese].zip"));
    assert_eq!(chapter.chapter_number.as_deref(), Some("2"));
    let volume = parse_identity(&archive("/x/[artist] Work Vol.03 [DL版].zip"));
    assert_eq!(volume.volume_number.as_deref(), Some("3"));
}

#[tokio::test]
async fn idempotency_reuses_an_active_cleanup_without_rebuilding_its_group() {
    let pool = version_cleanup_idempotency_pool().await;
    insert_version_cleanup_operation(&pool, "active", Vec::new()).await;
    for (archive_id, entry_id) in [("source-a", "entry-a"), ("source-b", "entry-b")] {
        sqlx::query("INSERT INTO trash_entries (id, status) VALUES (?, 'active')")
            .bind(entry_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO trash_operation_members (operation_id, archive_id, trash_entry_id)
                 VALUES ('operation', ?, ?)",
        )
        .bind(archive_id)
        .bind(entry_id)
        .execute(&pool)
        .await
        .unwrap();
    }

    let response = load_idempotent_version_cleanup(
        &pool,
        "user",
        "key",
        "versions-group",
        "keeper",
        &["source-a".to_string(), "source-b".to_string()],
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(response.operation_id, "operation");
    assert_eq!(response.deleted, 2);

    let mismatch = load_idempotent_version_cleanup(
        &pool,
        "user",
        "key",
        "versions-group",
        "other-keeper",
        &["source-a".to_string(), "source-b".to_string()],
    )
    .await
    .unwrap_err();
    assert!(mismatch.to_string().contains("idempotency key"));
}

#[tokio::test]
async fn idempotency_preserves_a_failed_cleanup_response() {
    let pool = version_cleanup_idempotency_pool().await;
    insert_version_cleanup_operation(&pool, "failed", vec!["source-b".to_string()]).await;

    let response = load_idempotent_version_cleanup(
        &pool,
        "user",
        "key",
        "versions-group",
        "keeper",
        &["source-a".to_string(), "source-b".to_string()],
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(response.deleted, 0);
    assert_eq!(response.failed_archive_ids, vec!["source-b".to_string()]);
}

#[tokio::test]
async fn lists_confirmed_collections_before_manual_and_pending_collections() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql(
            "CREATE TABLE collections (
                id TEXT PRIMARY KEY, display_title TEXT NOT NULL, subtitle TEXT,
                cover_archive_id TEXT, status TEXT NOT NULL, is_manual_locked BOOLEAN NOT NULL,
                normalized_key TEXT NOT NULL, updated_at DATETIME NOT NULL
            );
            CREATE TABLE collection_members (
                collection_id TEXT NOT NULL, archive_id TEXT NOT NULL, variant_group_key TEXT
            );
            CREATE TABLE collection_review_items (collection_id TEXT NOT NULL, status TEXT NOT NULL);",
        )
        .execute(&pool)
        .await
        .unwrap();

    for (id, title, status, locked) in [
        ("pending", "A pending", "needs_review", false),
        ("manual", "B manual", "manual", true),
        ("auto", "C confirmed", "auto", false),
    ] {
        sqlx::query("INSERT INTO collections (id, display_title, status, is_manual_locked, normalized_key, updated_at) VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)")
                .bind(id)
                .bind(title)
                .bind(status)
                .bind(locked)
                .bind(id)
                .execute(&pool)
                .await
                .unwrap();
        for suffix in ["one", "two"] {
            sqlx::query("INSERT INTO collection_members (collection_id, archive_id) VALUES (?, ?)")
                .bind(id)
                .bind(format!("{id}-{suffix}"))
                .execute(&pool)
                .await
                .unwrap();
        }
    }

    let ids: Vec<String> = list_collections(&pool, None, None, None)
        .await
        .unwrap()
        .into_iter()
        .map(|collection| collection.id)
        .collect();
    assert_eq!(ids, ["auto", "manual", "pending"]);

    let explicit_ids: Vec<String> =
        list_collections(&pool, None, Some("recognitionPriority"), Some("desc"))
            .await
            .unwrap()
            .into_iter()
            .map(|collection| collection.id)
            .collect();
    assert_eq!(explicit_ids, ["auto", "manual", "pending"]);
}

#[tokio::test]
async fn approving_a_member_keeps_the_collection_automatic_until_manually_locked() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql(
            "CREATE TABLE collections (
                id TEXT PRIMARY KEY, status TEXT NOT NULL, is_manual_locked BOOLEAN NOT NULL,
                updated_at DATETIME NOT NULL
            );
            CREATE TABLE collection_members (
                collection_id TEXT NOT NULL, archive_id TEXT NOT NULL, membership_source TEXT,
                is_manual_locked BOOLEAN NOT NULL, confidence REAL, updated_at DATETIME NOT NULL
            );
            CREATE TABLE collection_review_items (
                id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, collection_id TEXT NOT NULL,
                status TEXT NOT NULL, updated_at DATETIME NOT NULL
            );
            INSERT INTO collections VALUES ('collection', 'needs_review', FALSE, CURRENT_TIMESTAMP);
            INSERT INTO collection_members VALUES ('collection', 'member-a', 'auto', FALSE, 0.55, CURRENT_TIMESTAMP);
            INSERT INTO collection_members VALUES ('collection', 'member-b', 'auto', FALSE, 0.55, CURRENT_TIMESTAMP);
            INSERT INTO collection_review_items VALUES ('review-a', 'member-a', 'collection', 'pending', CURRENT_TIMESTAMP);
            INSERT INTO collection_review_items VALUES ('review-b', 'member-b', 'collection', 'pending', CURRENT_TIMESTAMP);",
        )
        .execute(&pool)
        .await
        .unwrap();

    apply_review(&pool, "review-a", "approve").await.unwrap();
    let after_first: (String, bool) =
        sqlx::query_as("SELECT status, is_manual_locked FROM collections WHERE id = 'collection'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(after_first, ("needs_review".to_string(), false));

    apply_review(&pool, "review-b", "approve").await.unwrap();
    let after_second: (String, bool) =
        sqlx::query_as("SELECT status, is_manual_locked FROM collections WHERE id = 'collection'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(after_second, ("auto".to_string(), false));
}

#[tokio::test]
async fn rebuild_groups_numbered_siblings_idempotently() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql(
            "CREATE TABLE archives (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT,
                path TEXT NOT NULL, file_hash TEXT NOT NULL, file_size INTEGER NOT NULL,
                page_count INTEGER NOT NULL, created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL
            );
            CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL);
            CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL);",
        )
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/sqlite/0004_collections.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/sqlite/0005_collection_versions.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/sqlite/0006_collection_identity_keys.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    for (id, path) in [
        ("one", "/library/[Artist] Demo Story (2) [Chinese].cbz"),
        ("two", "/library/[Artist] Demo Story (3) [Chinese].cbz"),
        (
            "issue",
            "/library/[Artist] Other Story (COMIC X-EROS #107) [v2].cbz",
        ),
    ] {
        sqlx::query("INSERT INTO archives (id, title, path, file_hash, file_size, page_count, created_at, updated_at) VALUES (?, ?, ?, ?, 1, 20, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
                .bind(id).bind(id).bind(path).bind(format!("hash-{id}")).execute(&pool).await.unwrap();
    }

    let first = rebuild_collections(&pool).await.unwrap();
    assert_eq!(first.parsed_archives, 3);
    assert_eq!(first.created_collections, 1);
    let collections = list_collections(&pool, None, None, None).await.unwrap();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].member_count, 2);
    let detail = get_collection(&pool, &collections[0].id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(detail.members.len(), 2);
    assert_eq!(detail.members[0].raw_number.as_deref(), Some("2"));
    assert_eq!(detail.members[1].raw_number.as_deref(), Some("3"));

    rebuild_collections(&pool).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM collection_members")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn same_content_versions_are_not_created_as_a_collection() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql(
            "CREATE TABLE archives (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT,
                path TEXT NOT NULL, file_hash TEXT NOT NULL, file_size INTEGER NOT NULL,
                page_count INTEGER NOT NULL, created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL
            );
            CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL);
            CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL);",
        )
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/sqlite/0004_collections.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/sqlite/0005_collection_versions.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/sqlite/0006_collection_identity_keys.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    for (id, path, size) in [
        (
            "one",
            "/library/[Artist] Demo Story (2) [Chinese].cbz",
            100_i64,
        ),
        (
            "two",
            "/library/[Artist] Demo Story (2) [English].cbz",
            200_i64,
        ),
    ] {
        sqlx::query("INSERT INTO archives (id, title, path, file_hash, file_size, page_count, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 20, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
                .bind(id).bind(id).bind(path).bind(format!("hash-{id}")).bind(size).execute(&pool).await.unwrap();
    }

    rebuild_collections(&pool).await.unwrap();
    assert!(list_collections(&pool, None, None, None)
        .await
        .unwrap()
        .is_empty());
    let versions = list_version_groups(&pool, None, None, None).await.unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].members.len(), 2);
}

#[tokio::test]
async fn stage_numbers_are_collection_members_not_versions() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql(
            "CREATE TABLE archives (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT,
                path TEXT NOT NULL, file_hash TEXT NOT NULL, file_size INTEGER NOT NULL,
                page_count INTEGER NOT NULL, created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL
            );
            CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL);
            CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL);",
        )
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/sqlite/0004_collections.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/sqlite/0005_collection_versions.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/sqlite/0006_collection_identity_keys.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    for (id, stage) in [("three", 3), ("four", 4)] {
        let path = format!("/library/[NR] BLANC Stage {stage} (Comic Exe) [DL].cbz");
        sqlx::query("INSERT INTO archives (id, title, path, file_hash, file_size, page_count, created_at, updated_at) VALUES (?, ?, ?, ?, 1, 20, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
                .bind(id).bind(format!("BLANC Stage {stage}")).bind(path).bind(format!("hash-{id}")).execute(&pool).await.unwrap();
    }

    rebuild_collections(&pool).await.unwrap();
    let collections = list_collections(&pool, None, None, None).await.unwrap();
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].display_title, "BLANC Stage");
    assert_eq!(collections[0].content_count, 2);
    assert!(list_version_groups(&pool, None, None, None)
        .await
        .unwrap()
        .is_empty());
}

#[test]
fn infers_a_missing_first_number_but_keeps_multiple_unnumbered_files_as_versions() {
    let mut sequence = vec![
        parse_identity(&ArchiveRow {
            id: "one".into(),
            title: "Demo".into(),
            path: "/library/[Artist] Demo [DL].cbz".into(),
        }),
        parse_identity(&ArchiveRow {
            id: "two".into(),
            title: "Demo 2".into(),
            path: "/library/[Artist] Demo 2 [DL].cbz".into(),
        }),
    ];
    infer_missing_first_numbers(&mut sequence);
    assert_eq!(sequence[0].raw_number.as_deref(), Some("1"));
    assert_eq!(sequence[0].unit_type, "unknown");

    let mut duplicates = vec![
        parse_identity(&ArchiveRow {
            id: "a".into(),
            title: "Same".into(),
            path: "/library/[Artist] Same [Chinese].cbz".into(),
        }),
        parse_identity(&ArchiveRow {
            id: "b".into(),
            title: "Same".into(),
            path: "/library/[Artist] Same [English].cbz".into(),
        }),
    ];
    infer_missing_first_numbers(&mut duplicates);
    assert!(duplicates.iter().all(|fact| fact.raw_number.is_none()));
    assert_eq!(
        content_unit_key(&duplicates[0]),
        content_unit_key(&duplicates[1])
    );
}

#[tokio::test]
async fn groups_terminal_sequence_suffixes_across_languages_without_grouping_a_lone_part() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::raw_sql(
            "CREATE TABLE archives (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT,
                path TEXT NOT NULL, file_hash TEXT NOT NULL, file_size INTEGER NOT NULL,
                page_count INTEGER NOT NULL, created_at DATETIME NOT NULL, updated_at DATETIME NOT NULL
            );
            CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL);
            CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL);",
        )
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/sqlite/0004_collections.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/sqlite/0005_collection_versions.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/sqlite/0006_collection_identity_keys.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    for (id, title, path) in [
        (
            "base",
            "星降る図書館",
            "/library/[ロケットモンキー] 星降る図書館 (コミックメガストア Vol.2) [中国翻訳] [DL版].cbz",
        ),
        (
            "middle",
            "星降る図書館 中編",
            "/library/[ロケットモンキー] 星降る図書館 中編 (コミックメガストア Vol.3) [中国翻訳] [DL版].cbz",
        ),
        (
            "lone",
            "雨宿りの午後 中編",
            "/library/[青空堂] 雨宿りの午後 中編 [中国翻訳] [DL版].cbz",
        ),
        (
            "chapter24",
            "放課後の図書室 第24話",
            "/library/[あずせ] 放課後の図書室 第24話 (アナンガ・ランガ Vol.104) [中国翻訳].cbz",
        ),
        (
            "chapter25",
            "放課後の図書室 第25話",
            "/library/[あずせ] 放課後の図書室 第25話 (アナンガ・ランガ Vol.106) [中国翻訳].cbz",
        ),
        (
            "base_story",
            "コダマちゃんの冒険",
            "/library/[ワクセイブロ] コダマちゃんの冒険 [中国翻訳] [DL版].cbz",
        ),
        (
            "story_two",
            "コダマちゃんの冒険その2",
            "/library/[ワクセイブロ] コダマちゃんの冒険その2 [中国翻訳] [DL版].cbz",
        ),
        (
            "story_three",
            "コダマちゃんの冒険その3",
            "/library/[ワクセイブロ] コダマちゃんの冒険その3 [English] [Digital].cbz",
        ),
        (
            "cn24",
            "星空补习班 第24话",
            "/library/[林墨] 星空补习班 第24话 [中文] [全彩].cbz",
        ),
        (
            "cn25",
            "星空补习班 第25话",
            "/library/[林墨] 星空补习班 第25话 [Chinese] [Full Color].cbz",
        ),
        (
            "en4",
            "City Library Chronicles Chapter 4",
            "/library/[North Star Studio] City Library Chronicles Chapter 4 [English] [Digital].cbz",
        ),
        (
            "en5",
            "City Library Chronicles Chapter 5",
            "/library/[North Star Studio] City Library Chronicles Chapter 5 [Chinese] [Digital].cbz",
        ),
        (
            "kr2",
            "봄날의 도서관 제2화",
            "/library/[달빛작가] 봄날의 도서관 제2화 [한국어] [Digital].cbz",
        ),
        (
            "kr3",
            "봄날의 도서관 제3화",
            "/library/[달빛작가] 봄날의 도서관 제3화 [Chinese] [Digital].cbz",
        ),
    ] {
        sqlx::query("INSERT INTO archives (id, title, path, file_hash, file_size, page_count, created_at, updated_at) VALUES (?, ?, ?, ?, 1, 20, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
                .bind(id).bind(title).bind(path).bind(format!("hash-{id}")).execute(&pool).await.unwrap();
    }

    let japanese_chapter = parse_identity(&archive(
        "/library/[あずせ] 放課後の図書室 第25話 [中国翻訳].cbz",
    ));
    assert_eq!(japanese_chapter.chapter_number.as_deref(), Some("25"));
    assert_eq!(japanese_chapter.normalized_key, "放課後の図書室");
    let chinese_chapter = parse_identity(&archive("/library/[林墨] 星空补习班 第25话 [中文].cbz"));
    assert_eq!(chinese_chapter.chapter_number.as_deref(), Some("25"));
    assert_eq!(chinese_chapter.normalized_key, "星空补习班");

    rebuild_collections(&pool).await.unwrap();
    let collections = list_collections(&pool, None, None, None).await.unwrap();
    assert_eq!(
        collections.len(),
        6,
        "formed collections: {:?}",
        collections
            .iter()
            .map(|collection| (&collection.display_title, collection.content_count))
            .collect::<Vec<_>>()
    );
    assert!(collections
        .iter()
        .any(|collection| collection.display_title == "星降る図書館"
            && collection.content_count == 2));
    assert!(collections
        .iter()
        .any(|collection| collection.display_title == "放課後の図書室"
            && collection.content_count == 2));
    assert!(collections.iter().any(
        |collection| collection.display_title == "コダマちゃんの冒険"
            && collection.content_count == 3
    ));
    assert!(collections.iter().any(
        |collection| collection.display_title == "星空补习班" && collection.content_count == 2
    ));
    assert!(collections.iter().any(|collection| collection.display_title
        == "City Library Chronicles"
        && collection.content_count == 2));
    assert!(collections
        .iter()
        .any(|collection| collection.display_title == "봄날의 도서관"
            && collection.content_count == 2));

    let middle = parse_identity(&archive("/library/[Artist] 星降る図書館 中編 [DL].cbz"));
    assert_eq!(middle.unit_type, "chapter");
    assert_eq!(middle.chapter_number.as_deref(), Some("2"));
    assert_eq!(middle.normalized_key, "星降る図書館");

    let story_part = parse_identity(&archive(
        "/library/[Artist] コダマちゃんの冒険その3 [DL].cbz",
    ));
    assert_eq!(story_part.chapter_number.as_deref(), Some("3"));
    assert_eq!(story_part.normalized_key, "コダマちゃんの冒険");

    let korean_chapter = parse_identity(&archive(
        "/library/[달빛작가] 봄날의 도서관 제3화 [Chinese].cbz",
    ));
    assert_eq!(korean_chapter.chapter_number.as_deref(), Some("3"));
    assert_eq!(korean_chapter.normalized_key, "봄날의 도서관");

    assert!(terminal_sequence_suffix("連載 第01-06話").is_none());
    assert!(terminal_sequence_suffix("短編集 その1-2").is_none());
}
