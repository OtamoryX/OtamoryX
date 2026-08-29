-- Incremental recommendation learning. This migration deliberately does not
-- backfill existing archives or behavior events.

CREATE TABLE IF NOT EXISTS preference_learning_state (
    id TEXT PRIMARY KEY CHECK (id = 'default'),
    cold_start_started_at DATETIME NOT NULL,
    algorithm_version TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO preference_learning_state
    (id, cold_start_started_at, algorithm_version)
VALUES
    ('default', CURRENT_TIMESTAMP, 'cold-start-v1');

CREATE TABLE IF NOT EXISTS archive_content_profiles (
    id TEXT PRIMARY KEY,
    archive_id TEXT NOT NULL,
    content_fingerprint TEXT NOT NULL,
    profile_version TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'running', 'completed', 'partial', 'retryable', 'failed')),
    profile_json TEXT NOT NULL DEFAULT '{}',
    expected_page_count INTEGER NOT NULL DEFAULT 0,
    actual_page_count INTEGER NOT NULL DEFAULT 0,
    sampled_page_count INTEGER NOT NULL DEFAULT 0,
    decoded_page_count INTEGER NOT NULL DEFAULT 0,
    coverage REAL NOT NULL DEFAULT 0,
    method_json TEXT NOT NULL DEFAULT '{}',
    last_error TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at DATETIME,
    completed_at DATETIME,
    next_attempt_at DATETIME,
    UNIQUE(archive_id, content_fingerprint, profile_version)
);

CREATE INDEX IF NOT EXISTS idx_archive_content_profiles_lookup
    ON archive_content_profiles (archive_id, profile_version, status, updated_at);

CREATE TABLE IF NOT EXISTS content_profile_jobs (
    id TEXT PRIMARY KEY,
    -- Keep the job after a manual delete so it can profile the recoverable
    -- trash snapshot and preserve the strongest negative feedback signal.
    archive_id TEXT NOT NULL,
    content_fingerprint TEXT NOT NULL,
    profile_version TEXT NOT NULL,
    trigger_source TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'running', 'completed', 'retryable', 'failed')),
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    next_attempt_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(archive_id, content_fingerprint, profile_version)
);

CREATE INDEX IF NOT EXISTS idx_content_profile_jobs_claimable
    ON content_profile_jobs (status, next_attempt_at, updated_at);

CREATE TABLE IF NOT EXISTS preference_feedback_aggregates (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    archive_id TEXT NOT NULL,
    open_count INTEGER NOT NULL DEFAULT 0,
    page_turn_count INTEGER NOT NULL DEFAULT 0,
    exit_count INTEGER NOT NULL DEFAULT 0,
    continue_count INTEGER NOT NULL DEFAULT 0,
    repeat_open_count INTEGER NOT NULL DEFAULT 0,
    restore_count INTEGER NOT NULL DEFAULT 0,
    correction_count INTEGER NOT NULL DEFAULT 0,
    max_page INTEGER NOT NULL DEFAULT 0,
    max_progress_ratio REAL NOT NULL DEFAULT 0,
    effective_read INTEGER NOT NULL DEFAULT 0,
    deep_read INTEGER NOT NULL DEFAULT 0,
    completed_read INTEGER NOT NULL DEFAULT 0,
    quick_exit INTEGER NOT NULL DEFAULT 0,
    manual_delete INTEGER NOT NULL DEFAULT 0,
    delete_stage TEXT,
    total_duration_ms INTEGER NOT NULL DEFAULT 0,
    max_duration_ms INTEGER NOT NULL DEFAULT 0,
    recommendation_exposure_count INTEGER NOT NULL DEFAULT 0,
    first_recommendation_position INTEGER,
    visibility_confidence REAL NOT NULL DEFAULT 0,
    algorithm_variants_json TEXT NOT NULL DEFAULT '[]',
    first_event_at DATETIME NOT NULL,
    last_event_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, archive_id)
);

CREATE INDEX IF NOT EXISTS idx_preference_feedback_user_time
    ON preference_feedback_aggregates (user_id, last_event_at);
CREATE INDEX IF NOT EXISTS idx_preference_feedback_archive
    ON preference_feedback_aggregates (archive_id, last_event_at);

CREATE TABLE IF NOT EXISTS preference_feedback_event_applied (
    behavior_event_id TEXT PRIMARY KEY REFERENCES user_behavior_events(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    archive_id TEXT NOT NULL,
    applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_preference_feedback_event_user
    ON preference_feedback_event_applied (user_id, applied_at);

-- Keep old candidate rows readable, but make new evidence auditable and
-- distinguish insufficient evidence from a candidate being observed.
ALTER TABLE preference_rule_candidates ADD COLUMN evidence_state TEXT NOT NULL DEFAULT 'insufficient_evidence';
ALTER TABLE preference_rule_candidates ADD COLUMN source TEXT NOT NULL DEFAULT 'legacy';
ALTER TABLE preference_rule_candidates ADD COLUMN feature_kind TEXT;
ALTER TABLE preference_rule_candidates ADD COLUMN profile_version TEXT;
ALTER TABLE preference_rule_candidates ADD COLUMN unique_archive_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN informative_result_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN effective_read_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN deep_read_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN manual_delete_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN quick_exit_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN conflict_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN baseline_rate REAL NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN lift REAL NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN direction_probability REAL NOT NULL DEFAULT 0.5;
ALTER TABLE preference_rule_candidates ADD COLUMN profile_coverage REAL NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN evidence_json TEXT NOT NULL DEFAULT '{}';

CREATE INDEX IF NOT EXISTS idx_preference_candidates_source_state
    ON preference_rule_candidates (user_id, source, evidence_state, updated_at);
