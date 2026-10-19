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
        graph_attributions: Vec::new(),
    }
}

#[test]
fn exploration_ratio_defaults_and_rejects_out_of_range_values() {
    let params = RandomArchiveParams {
        count: None,
        tags: None,
        theme_ids: None,
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
fn random_theme_ids_are_carried_into_archive_filters() {
    let params: RandomArchiveParams =
        serde_json::from_str(r#"{"themeIds":"theme-space,theme-adventure","count":5}"#)
            .expect("random query should deserialize theme ids");
    let filters = ArchiveFilters::from_random_params(&params);
    assert_eq!(
        filters.theme_ids,
        Some(vec![
            "theme-space".to_string(),
            "theme-adventure".to_string()
        ])
    );
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
        weighted("auto-delete-1", PreferenceTier::AutoDelete, 100.0),
    ];
    let mut rng = StdRng::seed_from_u64(7);
    let (selected, explored) =
        select_weighted_archives(candidates, 4, 0.25, false, &HashSet::new(), &mut rng);
    let ids: HashSet<&str> = selected.iter().map(|item| item.id.as_str()).collect();
    let unknown_selected = selected
        .iter()
        .filter(|item| item.id.starts_with("unknown-"))
        .count();
    assert_eq!(selected.len(), ids.len());
    assert_eq!(selected.len(), 4);
    assert!(!ids.contains("auto-delete-1"));
    assert!(explored >= 1, "explored={explored}");
    assert_eq!(explored, unknown_selected);
}

#[test]
fn empty_preference_pool_falls_back_to_unknown_and_downrank() {
    let candidates = vec![
        weighted("unknown-1", PreferenceTier::Unknown, 1.0),
        weighted("downrank-1", PreferenceTier::Downrank, 0.01),
    ];
    let mut rng = StdRng::seed_from_u64(11);
    let (selected, explored) =
        select_weighted_archives(candidates, 3, 0.25, false, &HashSet::new(), &mut rng);
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
        let picked =
            take_from_preference_pools(&mut keep, &mut downrank, &HashSet::new(), &mut rng)
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
fn observing_multiplier_only_changes_unknown_weight() {
    for (signed_score, expected_tier) in [
        (0.0, PreferenceTier::Unknown),
        (1.0, PreferenceTier::Keep),
        (-1.0, PreferenceTier::Downrank),
    ] {
        let base = PreferenceScore {
            signed_score,
            soft_multiplier: 1.0,
            ..Default::default()
        };
        let positive_observing = PreferenceScore {
            soft_multiplier: 1.15,
            ..base
        };
        let negative_observing = PreferenceScore {
            soft_multiplier: 0.85,
            ..base
        };
        let (base_tier, base_weight) = tier_and_weight(&base);
        let (positive_tier, positive_weight) = tier_and_weight(&positive_observing);
        let (negative_tier, negative_weight) = tier_and_weight(&negative_observing);

        assert_eq!(base_tier, expected_tier);
        assert_eq!(positive_tier, expected_tier);
        assert_eq!(negative_tier, expected_tier);
        if expected_tier == PreferenceTier::Unknown {
            assert!(positive_weight > base_weight);
            assert!(negative_weight < base_weight);
        } else {
            assert_eq!(positive_weight, base_weight);
            assert_eq!(negative_weight, base_weight);
        }
    }
}

#[test]
fn observing_multiplier_keeps_formal_tier_boundary_unchanged() {
    let positive_boundary = PreferenceScore {
        signed_score: f64::EPSILON,
        soft_multiplier: 1.2,
        ..Default::default()
    };
    let negative_boundary = PreferenceScore {
        signed_score: -f64::EPSILON,
        soft_multiplier: 0.8,
        ..Default::default()
    };

    let (positive_tier, positive_weight) = tier_and_weight(&positive_boundary);
    let (negative_tier, negative_weight) = tier_and_weight(&negative_boundary);
    assert_eq!(positive_tier, PreferenceTier::Unknown);
    assert_eq!(negative_tier, PreferenceTier::Unknown);
    assert!((positive_weight - 1.2).abs() < f64::EPSILON);
    assert!((negative_weight - 0.8).abs() < f64::EPSILON);
}

#[test]
fn graph_score_changes_weight_continuously_without_changing_the_base_tier() {
    let neutral = PreferenceScore::default();
    let positive = PreferenceScore {
        graph_contribution: 0.001,
        ..Default::default()
    };
    let negative = PreferenceScore {
        graph_contribution: -0.001,
        ..Default::default()
    };
    let (neutral_tier, neutral_weight) = tier_and_weight(&neutral);
    let (positive_tier, positive_weight) = tier_and_weight(&positive);
    let (negative_tier, negative_weight) = tier_and_weight(&negative);

    assert_eq!(neutral_tier, PreferenceTier::Unknown);
    assert_eq!(positive_tier, neutral_tier);
    assert_eq!(negative_tier, neutral_tier);
    assert!((positive_weight / neutral_weight - 0.001_f64.exp()).abs() < 1e-12);
    assert!((negative_weight / neutral_weight - (-0.001_f64).exp()).abs() < 1e-12);
}

#[test]
fn weak_graph_candidates_compete_for_preferred_slots() {
    let mut graph_hits = 0;
    for seed in 0..500 {
        let (_, graph_weight) = tier_and_weight(&PreferenceScore {
            graph_contribution: 0.001,
            ..Default::default()
        });
        let (_, keep_weight) = tier_and_weight(&PreferenceScore {
            signed_score: 1.0,
            ..Default::default()
        });
        let candidates = vec![
            weighted("graph", PreferenceTier::Unknown, graph_weight),
            weighted("keep", PreferenceTier::Keep, keep_weight),
        ];
        let mut rng = StdRng::seed_from_u64(seed);
        let (selected, _) =
            select_weighted_archives(candidates, 1, 0.25, true, &HashSet::new(), &mut rng);
        if selected[0].id == "graph" {
            graph_hits += 1;
        }
    }
    assert!(graph_hits > 100, "graph={graph_hits}");
}

#[test]
fn disabled_graph_keeps_unknowns_in_the_exploration_pool() {
    for seed in 0..100 {
        let candidates = vec![
            weighted("unknown", PreferenceTier::Unknown, 100.0),
            weighted("keep", PreferenceTier::Keep, 1.0),
        ];
        let mut rng = StdRng::seed_from_u64(seed);
        let (selected, _) =
            select_weighted_archives(candidates, 1, 0.25, false, &HashSet::new(), &mut rng);
        assert_eq!(selected[0].id, "keep");
    }
}

#[test]
fn confidence_is_derived_from_nested_evidence() {
    let value =
        serde_json::json!({"all": [{"confidence": 0.9}, {"concept": {"confidence": 0.72}}]});
    assert_eq!(minimum_json_confidence(&value), Some(0.72));
}

#[tokio::test]
async fn topic_snapshots_use_only_completed_canonical_theme_ids() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("create sqlite pool");
    crate::database::run_sqlite_migrations(&pool)
        .await
        .expect("recommendation migrations should succeed");
    sqlx::query(
        "INSERT INTO archives (id, title, path, file_hash, file_size, page_count)
         VALUES ('archive-1', 'test archive', '/tmp/test.cbz', 'hash-1', 1, 1)",
    )
    .execute(&pool)
    .await
    .expect("insert archive");
    sqlx::query(
        "INSERT INTO tags (id, name, namespace) VALUES ('theme-1', 'Space Opera', 'theme')",
    )
    .execute(&pool)
    .await
    .expect("insert canonical theme tag");
    sqlx::query(
        "INSERT INTO content_analyses
         (id, archive_id, content_fingerprint, status, prompt_version, result_json,
          canonicalization_status, canonicalization_version)
         VALUES ('analysis-1', 'archive-1', 'hash-1', 'completed', 'content-v5',
                 '{\"themes\":[\"raw model label\"],\"selectedTags\":[]}',
                 'completed', 'theme-canonical-v1')",
    )
    .execute(&pool)
    .await
    .expect("insert completed analysis");
    sqlx::query(
        "INSERT INTO content_analysis_themes
         (analysis_id, theme_tag_id, ordinal, generated_name, canonicalization_status,
          canonicalization_version)
         VALUES ('analysis-1', 'theme-1', 0, 'Raw model label', 'completed', 'theme-canonical-v1')",
    )
    .execute(&pool)
    .await
    .expect("insert canonical theme snapshot");

    let service = RandomService::new(pool);
    let snapshots = service
        .load_topic_snapshots(&[archive("archive-1")])
        .await
        .expect("load canonical theme snapshot");
    assert_eq!(
        snapshots.get("archive-1"),
        Some(&vec!["theme:theme-1".to_string()])
    );
}

#[tokio::test]
async fn failed_or_pending_canonicalization_is_invisible_to_topic_snapshots() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("create sqlite pool");
    crate::database::run_sqlite_migrations(&pool)
        .await
        .expect("recommendation migrations should succeed");
    sqlx::query(
        "INSERT INTO archives (id, title, path, file_hash, file_size, page_count) VALUES
         ('archive-pending', 'pending', '/tmp/pending.cbz', 'hash-pending', 1, 1),
         ('archive-failed', 'failed', '/tmp/failed.cbz', 'hash-failed', 1, 1),
         ('archive-conflict', 'conflict', '/tmp/conflict.cbz', 'hash-conflict', 1, 1)",
    )
    .execute(&pool)
    .await
    .expect("insert archives");
    sqlx::query(
        "INSERT INTO tags (id, name, namespace) VALUES
         ('theme-pending', 'Pending theme', 'theme'),
         ('theme-failed', 'Failed theme', 'theme'),
         ('theme-conflict', 'Conflict theme', 'theme')",
    )
    .execute(&pool)
    .await
    .expect("insert canonical theme tags");
    sqlx::query(
        "INSERT INTO content_analyses
         (id, archive_id, content_fingerprint, status, prompt_version, result_json,
          canonicalization_status, canonicalization_version)
         VALUES
         ('analysis-pending', 'archive-pending', 'hash-pending', 'pending', 'content-v5',
          '{\"themes\":[\"raw pending\"],\"selectedTags\":[]}', 'pending', 'theme-canonical-v1'),
         ('analysis-failed', 'archive-failed', 'hash-failed', 'failed', 'content-v5',
          '{\"themes\":[\"raw failed\"],\"selectedTags\":[]}', 'failed', 'theme-canonical-v1'),
         ('analysis-conflict', 'archive-conflict', 'hash-conflict', 'completed', 'content-v5',
          '{\"themes\":[\"raw conflict\"],\"selectedTags\":[]}', 'duplicate_conflict', 'theme-canonical-v1')",
    )
    .execute(&pool)
    .await
    .expect("insert incomplete analyses");
    sqlx::query(
        "INSERT INTO content_analysis_themes
         (analysis_id, theme_tag_id, ordinal, generated_name, canonicalization_status,
          canonicalization_version)
         VALUES
         ('analysis-pending', 'theme-pending', 0, 'Raw pending', 'pending', 'theme-canonical-v1'),
         ('analysis-failed', 'theme-failed', 0, 'Raw failed', 'failed', 'theme-canonical-v1'),
         ('analysis-conflict', NULL, 0, 'Raw conflict', 'duplicate_conflict', 'theme-canonical-v1')",
    )
    .execute(&pool)
    .await
    .expect("insert incomplete theme snapshots");

    let service = RandomService::new(pool);
    let snapshots = service
        .load_topic_snapshots(&[
            archive("archive-pending"),
            archive("archive-failed"),
            archive("archive-conflict"),
        ])
        .await
        .expect("load topic snapshots");

    assert!(snapshots.is_empty());
}

#[tokio::test]
async fn observing_canonical_theme_rules_do_not_affect_recommendation_scores() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("create sqlite pool");
    crate::database::run_sqlite_migrations(&pool)
        .await
        .expect("recommendation migrations should succeed");
    sqlx::query(
        "INSERT INTO archives (id, title, path, file_hash, file_size, page_count)
         VALUES ('archive-theme', 'theme archive', '/tmp/theme.cbz', 'hash-theme', 1, 1)",
    )
    .execute(&pool)
    .await
    .expect("insert archive");
    sqlx::query(
        "INSERT INTO archive_content_profiles
         (id, archive_id, content_fingerprint, profile_version, status, profile_json,
          expected_page_count, actual_page_count, sampled_page_count, decoded_page_count,
          coverage, method_json, completed_at)
         VALUES ('profile-theme', 'archive-theme', 'hash-theme', 'profile-v1', 'completed',
                 '{\"profileVersion\":\"profile-v1\",\"contentFingerprint\":\"hash-theme\",\"expectedPageCount\":1,\"actualPageCount\":1,\"sampledPageCount\":1,\"decodedPageCount\":1,\"coverage\":1.0,\"features\":[{\"key\":\"theme:theme-1\",\"value\":1.0,\"kind\":\"canonical_theme_observing\"},{\"key\":\"theme:theme-legacy\",\"value\":1.0,\"kind\":\"binary\"}],\"measurements\":{}}',
                 1, 1, 1, 1, 1.0, '{}', CURRENT_TIMESTAMP)",
    )
    .execute(&pool)
    .await
    .expect("insert profile");

    for (suffix, kind) in [("1", "canonical_theme_observing"), ("legacy", "binary")] {
        let condition_key = format!("profile:profile-v1:theme:theme-{suffix}:eq:1");
        let condition_json = format!(
            "{{\"all\":[{{\"feature\":\"theme:theme-{suffix}\",\"operator\":\"eq\",\"value\":1.0}}]}}"
        );
        sqlx::query(
            "INSERT INTO preference_rule_candidates
             (id, user_id, condition_key, conditions_json, status, evidence_state,
              source, feature_kind, profile_version, unique_archive_count,
              informative_result_count)
             VALUES (?, 'user-theme', ?, ?, 'promoted', 'eligible', 'cold_start_v1', ?,
                     'profile-v1', 12, 12)",
        )
        .bind(format!("candidate-{suffix}"))
        .bind(&condition_key)
        .bind(&condition_json)
        .bind(kind)
        .execute(&pool)
        .await
        .expect("insert learned candidate");
        sqlx::query(
            "INSERT INTO preference_rules
             (id, user_id, name, rule_version, conditions_json, action,
              confidence_threshold, enabled, owner_role, source, preference_weight)
             VALUES (?, 'user-theme', ?, 'rule-v1', ?, 'keep', 0.95, 1, 'user',
                     'learned_cold_start', 1.0)",
        )
        .bind(format!("rule-{suffix}"))
        .bind(format!("theme rule {suffix}"))
        .bind(&condition_json)
        .execute(&pool)
        .await
        .expect("insert learned rule");
    }

    let scored = RandomService::new(pool)
        .score_candidates("user-theme", vec![archive("archive-theme")], None)
        .await
        .expect("score candidate archive");
    assert_eq!(scored.len(), 1);
    assert_eq!(scored[0].tier, PreferenceTier::Unknown);
}

#[test]
fn baseline_assignment_is_stable_and_uses_a_twenty_percent_bucket() {
    assert_eq!(
        stable_experiment_bucket("user-a"),
        stable_experiment_bucket("user-a")
    );
    let baseline_count = (0..10_000)
        .filter(|index| stable_experiment_bucket(&format!("user-{index}")) < 20)
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

#[test]
fn graph_recall_stays_inside_existing_archive_scope() {
    let filters = ArchiveFilters {
        archive_ids: Some(vec![
            "category-match".to_string(),
            "already-seen".to_string(),
        ]),
        ..ArchiveFilters::default()
    };
    let recalled = graph_recall_archive_ids(
        vec![
            "category-match".to_string(),
            "outside-category".to_string(),
            "already-seen".to_string(),
        ],
        &filters,
        &[archive("already-seen")],
    );

    assert_eq!(recalled, vec!["category-match"]);
}

#[test]
fn graph_recall_requires_path_scope_for_non_admin_users() {
    assert!(!graph_recall_has_path_scope("user", &[]));
    assert!(graph_recall_has_path_scope(
        "user",
        &["/library/*".to_string()]
    ));
    assert!(graph_recall_has_path_scope("admin", &[]));

    let mut allowed = archive("allowed");
    allowed.path = "/library/allowed.cbz".to_string();
    let mut private = archive("private");
    private.path = "/private/private.cbz".to_string();
    let filtered =
        filter_graph_recall_archives("user", &["/library/*".to_string()], vec![allowed, private]);
    assert_eq!(
        filtered
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        ["allowed"]
    );
}

#[test]
fn graph_recall_intersects_archive_scope_and_excludes_trash() {
    let filters = ArchiveFilters {
        archive_ids: Some(vec![
            "category-match".to_string(),
            "already-seen".to_string(),
        ]),
        exclude_archive_ids: Some(vec!["trash-match".to_string()]),
        ..ArchiveFilters::default()
    };
    let recalled = graph_recall_archive_ids(
        vec![
            "category-match".to_string(),
            "outside-category".to_string(),
            "already-seen".to_string(),
            "trash-match".to_string(),
        ],
        &filters,
        &[archive("already-seen")],
    );

    assert_eq!(recalled, vec!["category-match"]);
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
            "CREATE TABLE tag_localizations (tag_id TEXT NOT NULL, locale TEXT NOT NULL, name TEXT, status TEXT NOT NULL, PRIMARY KEY(tag_id, locale))",
            "CREATE TABLE trash_entries (archive_id TEXT NOT NULL, status TEXT NOT NULL)",
            "CREATE TABLE user_paths (user_id TEXT NOT NULL, path TEXT NOT NULL)",
            "CREATE TABLE content_analyses (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, status TEXT NOT NULL, result_json TEXT, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)",
            "CREATE TABLE preference_rules (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, rule_version TEXT NOT NULL, confidence_threshold REAL NOT NULL, preference_weight REAL NOT NULL DEFAULT 1.0, enabled INTEGER NOT NULL, auto_paused INTEGER NOT NULL, owner_role TEXT NOT NULL)",
            "CREATE TABLE preference_rule_evaluations (id TEXT PRIMARY KEY, analysis_id TEXT NOT NULL, rule_id TEXT NOT NULL, rule_version TEXT NOT NULL, matched INTEGER NOT NULL, decision TEXT NOT NULL, matched_conditions_json TEXT NOT NULL)",
            "CREATE TABLE archive_dispositions (id TEXT PRIMARY KEY, archive_id TEXT NOT NULL, user_id TEXT NOT NULL, disposition TEXT NOT NULL, source TEXT NOT NULL DEFAULT 'user', confidence REAL, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)",
            "CREATE TABLE user_behavior_events (archive_id TEXT, user_id TEXT NOT NULL, event_type TEXT NOT NULL, occurred_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)",
            "CREATE TABLE preference_feedback_aggregates (user_id TEXT NOT NULL, archive_id TEXT NOT NULL, effective_read INTEGER NOT NULL DEFAULT 0, deep_read INTEGER NOT NULL DEFAULT 0, completed_read INTEGER NOT NULL DEFAULT 0, quick_exit INTEGER NOT NULL DEFAULT 0, manual_delete INTEGER NOT NULL DEFAULT 0)",
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
        .score_candidates("user-a", vec![archive("a"), archive("b")], None)
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
        theme_ids: None,
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

#[test]
fn exposure_strength_counts_only_cards_that_were_seen() {
    assert_eq!(exposure_strength(0, 0, false), 1.0);
    assert_eq!(exposure_strength(0, VISIBLE_EXPOSURE_SLOTS - 1, false), 1.0);
    assert_eq!(exposure_strength(1, 0, false), 0.5);
    // Cards past the first screen were never scrolled into view, so they are not exposure.
    assert_eq!(exposure_strength(0, VISIBLE_EXPOSURE_SLOTS, false), 0.0);
    assert_eq!(exposure_strength(0, 9, false), 0.0);
    // Once the carousel reports a card as visible, its position no longer matters.
    assert_eq!(exposure_strength(0, VISIBLE_EXPOSURE_SLOTS, true), 1.0);
    assert_eq!(exposure_strength(1, 9, true), 0.5);
    assert_eq!(exposure_decay_multiplier(0.0), 1.0);
}

#[test]
fn exposure_decay_penalises_repeats_without_excluding_them() {
    let single = exposure_decay_multiplier(exposure_strength(0, 0, false));
    let repeated = exposure_decay_multiplier(2.0 * exposure_strength(0, 0, false));
    let offscreen = exposure_decay_multiplier(exposure_strength(0, 9, false));

    assert!(single < 0.2, "visible card should drop sharply: {single}");
    assert!(repeated < single);
    assert_eq!(offscreen, 1.0, "off-screen cards must not be downweighted");
    assert!(repeated >= MIN_EXPOSURE_DECAY_MULTIPLIER.min(1.0));
    assert!(exposure_decay_multiplier(10_000.0) >= 0.01);
}

fn select_round_with_weights(
    candidates: &[WeightedArchive],
    first_round: &[Archive],
    apply_decay: bool,
    rng: &mut StdRng,
) -> Vec<String> {
    let strengths: HashMap<String, f64> = first_round
        .iter()
        .enumerate()
        .map(|(position, archive)| {
            (
                archive.id.clone(),
                exposure_strength(0, position as i64, false),
            )
        })
        .collect();
    let recently_exposed = if apply_decay {
        first_round
            .iter()
            .take(VISIBLE_EXPOSURE_SLOTS as usize)
            .map(|archive| archive.id.clone())
            .collect::<HashSet<_>>()
    } else {
        HashSet::new()
    };
    let pooled: Vec<WeightedArchive> = candidates
        .iter()
        .map(|candidate| {
            let mut candidate = WeightedArchive {
                archive: candidate.archive.clone(),
                tier: candidate.tier,
                weight: candidate.weight,
                graph_attributions: Vec::new(),
            };
            if apply_decay {
                if let Some(strength) = strengths.get(&candidate.archive.id) {
                    candidate.weight *= exposure_decay_multiplier(*strength);
                }
            }
            candidate
        })
        .collect();
    select_weighted_archives(pooled, 12, 0.25, false, &recently_exposed, rng)
        .0
        .into_iter()
        .map(|archive| archive.id)
        .collect()
}

#[test]
fn exposure_decay_cuts_refresh_repeats_for_heavy_favourites() {
    // Production shape: a few archives carry a keep-tier weight roughly 3.5x the unknown tier,
    // so an undecayed draw keeps returning them while the rest of the candidate pool rotates.
    let candidates: Vec<WeightedArchive> = (0..100)
        .map(|index| {
            let weight = if index < 10 { 3.5 } else { 1.0 };
            weighted(
                &format!("archive-{index:03}"),
                PreferenceTier::Unknown,
                weight,
            )
        })
        .collect();
    let rounds = 300;
    let mut rng = StdRng::seed_from_u64(11);
    let mut baseline_repeats = 0;
    let mut decayed_repeats = 0;
    for _ in 0..rounds {
        let first = select_round_with_weights(&candidates, &[], false, &mut rng);
        let first_candidates: Vec<Archive> = first.iter().map(|id| archive(id)).collect();
        let visible_ids: HashSet<String> = first
            .iter()
            .take(VISIBLE_EXPOSURE_SLOTS as usize)
            .cloned()
            .collect();
        baseline_repeats +=
            select_round_with_weights(&candidates, &first_candidates, false, &mut rng)
                .iter()
                .filter(|archive_id| visible_ids.contains(*archive_id))
                .count();
        decayed_repeats +=
            select_round_with_weights(&candidates, &first_candidates, true, &mut rng)
                .iter()
                .filter(|archive_id| visible_ids.contains(*archive_id))
                .count();
    }
    let baseline_mean = baseline_repeats as f64 / rounds as f64;
    let decayed_mean = decayed_repeats as f64 / rounds as f64;
    assert!(
        baseline_mean > 0.3,
        "visible favourites should repeat across refreshes without decay: {baseline_mean:.2} cards"
    );
    assert_eq!(
        decayed_mean, 0.0,
        "a refresh should not repeat the cards that were on the previous first screen"
    );
}

#[tokio::test]
async fn recent_exposure_uses_audited_card_positions() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("create sqlite pool");
    for statement in [
        "CREATE TABLE random_recommendation_sessions (id TEXT PRIMARY KEY, user_id TEXT NOT NULL, created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP)",
        "CREATE TABLE random_recommendation_items (session_id TEXT NOT NULL, user_id TEXT NOT NULL, archive_id TEXT NOT NULL, position INTEGER NOT NULL, opened_at DATETIME, effective_read_at DATETIME, visible_at DATETIME)",
    ] {
        sqlx::query(statement)
            .execute(&pool)
            .await
            .expect("create exposure test table");
    }
    sqlx::query("INSERT INTO random_recommendation_sessions (id,user_id,created_at) VALUES ('newest','user-a',datetime('now')),('older','user-a',datetime('now','-5 minutes')),('foreign','user-b',datetime('now'))")
        .execute(&pool)
        .await
        .expect("insert exposure sessions");
    for (archive_id, position) in [("leading", 0_i64), ("trailing", 9_i64)] {
        sqlx::query(
            "INSERT INTO random_recommendation_items (session_id,user_id,archive_id,position) VALUES ('newest','user-a',?,?),('older','user-a',?,?)",
        )
        .bind(archive_id)
        .bind(position)
        .bind(archive_id)
        .bind(position)
        .execute(&pool)
        .await
        .expect("insert exposure items");
    }
    sqlx::query(
        "INSERT INTO random_recommendation_items (session_id,user_id,archive_id,position) VALUES ('foreign','user-b','other-user-item',0),('newest','user-b','other-user-item',0)",
    )
    .execute(&pool)
    .await
    .expect("insert foreign exposure items");
    sqlx::query(
        "INSERT INTO random_recommendation_items (session_id,user_id,archive_id,position,opened_at) VALUES ('newest','user-a','opened-item',1,datetime('now'))",
    )
    .execute(&pool)
    .await
    .expect("insert opened exposure item");
    sqlx::query(
        "INSERT INTO random_recommendation_items (session_id,user_id,archive_id,position,effective_read_at) VALUES ('newest','user-a','finished-item',2,datetime('now'))",
    )
    .execute(&pool)
    .await
    .expect("insert finished exposure item");
    sqlx::query(
        "INSERT INTO random_recommendation_items (session_id,user_id,archive_id,position,visible_at) VALUES ('newest','user-a','scrolled-item',5,datetime('now')),('newest','user-a','unseen-item',6,NULL)",
    )
    .execute(&pool)
    .await
    .expect("insert scrolled exposure items");

    let strengths = RandomService::new(pool)
        .load_recent_exposure_strengths("user-a")
        .await
        .expect("load recent exposure");
    assert_eq!(strengths.len(), 4);
    // Leading card in both sessions: 1.0 (newest) + 0.5 (older) of accumulated exposure.
    assert_eq!(strengths.get("leading"), Some(&1.5));
    // Cards past the first screen only count after the carousel reported them as visible.
    assert!(!strengths.contains_key("trailing"));
    assert_eq!(strengths.get("scrolled-item"), Some(&1.0));
    assert!(!strengths.contains_key("unseen-item"));
    assert!(!strengths.contains_key("other-user-item"));
    // A refresh is meant to surface a different archive, so opening (and not finishing) or
    // finishing a served card must not exempt it from the recent-exposure draw.
    assert_eq!(strengths.get("opened-item"), Some(&1.0));
    assert_eq!(strengths.get("finished-item"), Some(&1.0));
}

#[tokio::test]
async fn impressions_mark_only_viewed_cards_of_the_owned_session() {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("create sqlite pool");
    for statement in [
        "CREATE TABLE random_recommendation_sessions (id TEXT PRIMARY KEY, user_id TEXT NOT NULL)",
        "CREATE TABLE random_recommendation_items (session_id TEXT NOT NULL, user_id TEXT NOT NULL, archive_id TEXT NOT NULL, position INTEGER NOT NULL, visible_at DATETIME)",
    ] {
        sqlx::query(statement)
            .execute(&pool)
            .await
            .expect("create impression test table");
    }
    sqlx::query("INSERT INTO random_recommendation_sessions (id,user_id) VALUES ('owned','user-a'),('foreign','user-b')")
        .execute(&pool)
        .await
        .expect("insert impression sessions");
    for position in 0..6_i64 {
        sqlx::query(
            "INSERT INTO random_recommendation_items (session_id,user_id,archive_id,position) VALUES ('owned','user-a',?,?),('foreign','user-b',?,?)",
        )
        .bind(format!("archive-{position}"))
        .bind(position)
        .bind(format!("foreign-{position}"))
        .bind(position)
        .execute(&pool)
        .await
        .expect("insert impression items");
    }

    let service = RandomService::new(pool.clone());
    let recorded = service
        .record_visible_positions("user-a", "owned", &[1, 4, 4, 200])
        .await
        .expect("record impressions");
    assert_eq!(recorded, Some(2));

    let visible: Vec<i64> = sqlx::query_scalar(
        "SELECT position FROM random_recommendation_items WHERE session_id='owned' AND visible_at IS NOT NULL ORDER BY position",
    )
    .fetch_all(&pool)
    .await
    .expect("load visible positions");
    assert_eq!(visible, vec![1, 4]);

    let foreign_visible: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM random_recommendation_items WHERE session_id='foreign' AND visible_at IS NOT NULL",
    )
    .fetch_one(&pool)
    .await
    .expect("count foreign impressions");
    assert_eq!(foreign_visible, 0);

    // A session that belongs to someone else must not be writable through this endpoint.
    let foreign = service
        .record_visible_positions("user-a", "foreign", &[0])
        .await
        .expect("reject foreign session");
    assert_eq!(foreign, None);
}
