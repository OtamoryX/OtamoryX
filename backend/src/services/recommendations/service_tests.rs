use super::*;
use rand::{rngs::StdRng, SeedableRng};
use std::collections::HashSet;

fn archive(id: &str) -> Archive {
    let now = chrono::Utc::now();
    Archive {
        id: id.to_string(),
        title: id.to_string(),
        subtitle: None,
        subtitle_language: None,
        path: format!("/{id}.cbz"),
        file_size: 1,
        page_count: 1,
        hash: id.to_string(),
        created_at: now,
        updated_at: now,
        tags: Vec::new(),
    }
}

fn weighted(id: &str, tier: PreferenceTier, weight: f64) -> WeightedArchive {
    WeightedArchive {
        archive: archive(id),
        tier,
        weight,
    }
}

#[test]
fn exploration_ratio_defaults_and_rejects_out_of_range_values() {
    let params = RandomArchiveParams {
        count: None,
        tags: None,
        min_pages: None,
        max_pages: None,
        min_file_size: None,
        max_file_size: None,
        created_after: None,
        created_before: None,
        exclude_new: None,
        category_id: None,
        query: None,
        exploration_ratio: None,
    };
    assert_eq!(params.exploration_ratio().unwrap(), 0.25);
    assert!(RandomArchiveParams {
        exploration_ratio: Some(0.04),
        ..params.clone()
    }
    .exploration_ratio()
    .is_err());
    assert!(RandomArchiveParams {
        exploration_ratio: Some(0.51),
        ..params
    }
    .exploration_ratio()
    .is_err());
}

#[test]
fn weighted_selection_is_unique_and_honors_exploration_quota() {
    let candidates = vec![
        weighted("keep-1", PreferenceTier::Keep, 2.0),
        weighted("keep-2", PreferenceTier::Keep, 2.0),
        weighted("unknown-1", PreferenceTier::Unknown, 1.0),
        weighted("unknown-2", PreferenceTier::Unknown, 1.0),
        weighted("unknown-3", PreferenceTier::Unknown, 1.0),
        weighted("downrank-1", PreferenceTier::Downrank, 0.01),
    ];
    let mut rng = StdRng::seed_from_u64(7);
    let (selected, explored) = select_weighted_archives(candidates, 4, 0.25, &mut rng);
    let ids: HashSet<&str> = selected.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(selected.len(), ids.len());
    assert_eq!(selected.len(), 4);
    assert_eq!(explored, 2);
}

#[test]
fn empty_preference_pool_falls_back_to_unknown_and_downrank() {
    let candidates = vec![
        weighted("unknown-1", PreferenceTier::Unknown, 1.0),
        weighted("downrank-1", PreferenceTier::Downrank, 0.01),
    ];
    let mut rng = StdRng::seed_from_u64(11);
    let (selected, explored) = select_weighted_archives(candidates, 3, 0.25, &mut rng);
    assert_eq!(selected.len(), 2);
    assert_eq!(explored, 1);
}

#[test]
fn keep_weight_dominates_downrank_weight() {
    let mut keep_hits = 0;
    let mut downrank_hits = 0;
    for seed in 0..500 {
        let mut keep = vec![weighted("keep", PreferenceTier::Keep, 3.0)];
        let mut downrank = vec![weighted("downrank", PreferenceTier::Downrank, 0.01)];
        let mut rng = StdRng::seed_from_u64(seed);
        let picked = take_from_preference_pools(&mut keep, &mut downrank, &mut rng)
            .expect("one preference candidate");
        if picked.tier == PreferenceTier::Keep {
            keep_hits += 1;
        } else {
            downrank_hits += 1;
        }
    }
    assert!(
        keep_hits > 450,
        "keep={keep_hits}, downrank={downrank_hits}"
    );
}

#[test]
fn confidence_is_derived_from_nested_evidence() {
    let value =
        serde_json::json!({"all": [{"confidence": 0.9}, {"concept": {"confidence": 0.72}}]});
    assert_eq!(minimum_json_confidence(&value), Some(0.72));
}

#[test]
fn topic_snapshots_normalize_themes_and_high_confidence_concepts() {
    let result = crate::models::ContentAnalysisResult {
        themes: vec!["  Space   Opera ".to_string(), "SPACE opera".to_string()],
        concepts: vec![
            crate::models::ContentConcept {
                name: "Found   Family".to_string(),
                confidence: 0.8,
                evidence_pages: vec![1],
            },
            crate::models::ContentConcept {
                name: "Ignored".to_string(),
                confidence: 0.69,
                evidence_pages: vec![1],
            },
        ],
    };
    assert_eq!(
        normalized_topics(&result),
        vec!["found family", "space opera"]
    );
}

#[test]
fn baseline_assignment_is_stable_and_uses_a_twenty_percent_bucket() {
    assert_eq!(
        stable_experiment_bucket("user-a", 739_000),
        stable_experiment_bucket("user-a", 739_000)
    );
    let baseline_count = (0..10_000)
        .filter(|index| stable_experiment_bucket(&format!("user-{index}"), 739_000) < 20)
        .count();
    assert!(
        (1_700..=2_300).contains(&baseline_count),
        "{baseline_count}"
    );
}

#[test]
fn uniform_selection_excludes_auto_delete_without_weight_bias() {
    let candidates = vec![
        weighted("keep", PreferenceTier::Keep, 1000.0),
        weighted("unknown", PreferenceTier::Unknown, 0.0001),
        weighted("deleted", PreferenceTier::AutoDelete, 1.0),
    ];
    let mut rng = StdRng::seed_from_u64(3);
    let (selected, explored) = select_uniform_archives(candidates, 10, &mut rng);
    let ids: HashSet<&str> = selected.iter().map(|archive| archive.id.as_str()).collect();
    assert_eq!(ids, HashSet::from(["keep", "unknown"]));
    assert_eq!(explored, 1);
}

#[test]
fn random_candidates_are_filtered_by_path_before_sampling() {
    let targets = vec![
        ArchiveDeleteTarget {
            id: "allowed".to_string(),
            path: "/library/allowed.cbz".to_string(),
        },
        ArchiveDeleteTarget {
            id: "private".to_string(),
            path: "/private/private.cbz".to_string(),
        },
    ];

    assert_eq!(
        permitted_archive_ids("user", &["/library/*".to_string()], targets),
        vec!["allowed"]
    );
}

#[tokio::test]
async fn random_candidates_are_user_scoped_and_exclude_trash_and_paths() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("create sqlite pool");
    for statement in [
            "CREATE TABLE archives (id TEXT PRIMARY KEY, title TEXT NOT NULL, subtitle TEXT, subtitle_language TEXT, path TEXT NOT NULL, file_hash TEXT UNIQUE NOT NULL, file_size INTEGER NOT NULL, page_count INTEGER NOT NULL, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)",
            "CREATE TABLE archive_tags (archive_id TEXT NOT NULL, tag_id TEXT NOT NULL)",
            "CREATE TABLE tags (id TEXT PRIMARY KEY, name TEXT NOT NULL, namespace TEXT NOT NULL)",
            "CREATE TABLE trash_entries (archive_id TEXT NOT NULL, status TEXT NOT NULL)",
            "CREATE TABLE user_paths (user_id TEXT NOT NULL, path TEXT NOT NULL)",
            "CREATE TABLE content_analyses (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, status TEXT NOT NULL, result_json TEXT, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)",
            "CREATE TABLE preference_rules (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, rule_version TEXT NOT NULL, confidence_threshold REAL NOT NULL, preference_weight REAL NOT NULL DEFAULT 1.0, enabled INTEGER NOT NULL, auto_paused INTEGER NOT NULL, owner_role TEXT NOT NULL)",
            "CREATE TABLE preference_rule_evaluations (id TEXT PRIMARY KEY, analysis_id TEXT NOT NULL, rule_id TEXT NOT NULL, rule_version TEXT NOT NULL, matched INTEGER NOT NULL, decision TEXT NOT NULL, matched_conditions_json TEXT NOT NULL)",
            "CREATE TABLE archive_dispositions (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, user_id TEXT NOT NULL, disposition TEXT NOT NULL, confidence REAL, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)",
            "CREATE TABLE user_behavior_events (archive_id TEXT, user_id TEXT NOT NULL, event_type TEXT NOT NULL, occurred_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)",
        ] {
            sqlx::query(statement)
                .execute(&pool)
                .await
                .expect("create random test table");
        }
    for (id, path) in [
        ("a", "/library/a.cbz"),
        ("b", "/library/b.cbz"),
        ("c", "/library/c.cbz"),
        ("private", "/private/private.cbz"),
    ] {
        sqlx::query(
                "INSERT INTO archives (id,title,path,file_hash,file_size,page_count) VALUES (?,?,?,?,1,1)",
            )
            .bind(id)
            .bind(id)
            .bind(path)
            .bind(format!("hash-{id}"))
            .execute(&pool)
            .await
            .expect("insert random archive");
    }
    sqlx::query("INSERT INTO trash_entries (archive_id,status) VALUES ('c','active')")
        .execute(&pool)
        .await
        .expect("insert active trash entry");
    sqlx::query("INSERT INTO user_paths (user_id,path) VALUES ('user-a','/library/*')")
        .execute(&pool)
        .await
        .expect("insert user path");
    sqlx::query("INSERT INTO content_analyses (id,archive_id,status) VALUES ('analysis-a','a','completed'),('analysis-b','b','completed')")
            .execute(&pool)
            .await
            .expect("insert analyses");
    sqlx::query("INSERT INTO preference_rules (id,user_id,rule_version,confidence_threshold,enabled,auto_paused,owner_role) VALUES ('rule-a','user-a','1',0.8,1,0,'user'),('rule-b','user-b','1',0.8,1,0,'user')")
            .execute(&pool)
            .await
            .expect("insert rules");
    sqlx::query("INSERT INTO preference_rule_evaluations (id,analysis_id,rule_id,rule_version,matched,decision,matched_conditions_json) VALUES ('eval-a','analysis-a','rule-a','1',1,'keep','{\"confidence\":0.95}'),('eval-b','analysis-b','rule-b','1',1,'keep','{\"confidence\":0.95}')")
            .execute(&pool)
            .await
            .expect("insert evaluations");

    let service = RandomService::new(pool.clone());
    let scored = service
        .score_candidates("user-a", vec![archive("a"), archive("b")])
        .await
        .expect("score candidates");
    let tiers: HashMap<String, PreferenceTier> = scored
        .into_iter()
        .map(|item| (item.archive.id, item.tier))
        .collect();
    assert_eq!(tiers.get("a"), Some(&PreferenceTier::Keep));
    assert_eq!(tiers.get("b"), Some(&PreferenceTier::Unknown));

    let params = RandomArchiveParams {
        count: Some(20),
        tags: None,
        min_pages: None,
        max_pages: None,
        min_file_size: None,
        max_file_size: None,
        created_after: None,
        created_before: None,
        exclude_new: None,
        category_id: None,
        query: None,
        exploration_ratio: Some(0.25),
    };
    let result = service
        .get_random_archives_for_user(params, "user-a", "user")
        .await
        .expect("select random candidates");
    let ids: HashSet<&str> = result.iter().map(|item| item.id.as_str()).collect();
    assert!(ids.contains("a") && ids.contains("b"));
    assert!(!ids.contains("c"));
    assert!(!ids.contains("private"));
}
