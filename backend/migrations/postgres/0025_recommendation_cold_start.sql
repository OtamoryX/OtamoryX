-- Incremental recommendation learning. This migration deliberately does not
-- backfill existing archives or behavior events.

CREATE TABLE IF NOT EXISTS preference_learning_state (
    id TEXT PRIMARY KEY CHECK (id = 'default'),
    cold_start_started_at TIMESTAMPTZ NOT NULL,
    algorithm_version TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO preference_learning_state
    (id, cold_start_started_at, algorithm_version)
VALUES
    ('default', NOW(), 'cold-start-v1')
ON CONFLICT (id) DO NOTHING;

CREATE TABLE IF NOT EXISTS archive_content_profiles (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    archive_id UUID NOT NULL,
    content_fingerprint TEXT NOT NULL,
    profile_version TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'running', 'completed', 'partial', 'retryable', 'failed')),
    profile_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    expected_page_count INTEGER NOT NULL DEFAULT 0,
    actual_page_count INTEGER NOT NULL DEFAULT 0,
    sampled_page_count INTEGER NOT NULL DEFAULT 0,
    decoded_page_count INTEGER NOT NULL DEFAULT 0,
    coverage DOUBLE PRECISION NOT NULL DEFAULT 0,
    method_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_error TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    next_attempt_at TIMESTAMPTZ,
    UNIQUE(archive_id, content_fingerprint, profile_version)
);

CREATE INDEX IF NOT EXISTS idx_archive_content_profiles_lookup
    ON archive_content_profiles (archive_id, profile_version, status, updated_at);

CREATE TABLE IF NOT EXISTS content_profile_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    -- Keep the job after a manual delete so it can profile the recoverable
    -- trash snapshot and preserve the strongest negative feedback signal.
    archive_id UUID NOT NULL,
    content_fingerprint TEXT NOT NULL,
    profile_version TEXT NOT NULL,
    trigger_source TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'running', 'completed', 'retryable', 'failed')),
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    next_attempt_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(archive_id, content_fingerprint, profile_version)
);

CREATE INDEX IF NOT EXISTS idx_content_profile_jobs_claimable
    ON content_profile_jobs (status, next_attempt_at, updated_at);

CREATE TABLE IF NOT EXISTS preference_feedback_aggregates (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    archive_id UUID NOT NULL,
    open_count INTEGER NOT NULL DEFAULT 0,
    page_turn_count INTEGER NOT NULL DEFAULT 0,
    exit_count INTEGER NOT NULL DEFAULT 0,
    continue_count INTEGER NOT NULL DEFAULT 0,
    repeat_open_count INTEGER NOT NULL DEFAULT 0,
    restore_count INTEGER NOT NULL DEFAULT 0,
    correction_count INTEGER NOT NULL DEFAULT 0,
    max_page INTEGER NOT NULL DEFAULT 0,
    max_progress_ratio DOUBLE PRECISION NOT NULL DEFAULT 0,
    effective_read BOOLEAN NOT NULL DEFAULT FALSE,
    deep_read BOOLEAN NOT NULL DEFAULT FALSE,
    completed_read BOOLEAN NOT NULL DEFAULT FALSE,
    quick_exit BOOLEAN NOT NULL DEFAULT FALSE,
    manual_delete INTEGER NOT NULL DEFAULT 0,
    delete_stage TEXT,
    total_duration_ms BIGINT NOT NULL DEFAULT 0,
    max_duration_ms BIGINT NOT NULL DEFAULT 0,
    recommendation_exposure_count INTEGER NOT NULL DEFAULT 0,
    first_recommendation_position INTEGER,
    visibility_confidence DOUBLE PRECISION NOT NULL DEFAULT 0,
    algorithm_variants_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    first_event_at TIMESTAMPTZ NOT NULL,
    last_event_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, archive_id)
);

CREATE INDEX IF NOT EXISTS idx_preference_feedback_user_time
    ON preference_feedback_aggregates (user_id, last_event_at);
CREATE INDEX IF NOT EXISTS idx_preference_feedback_archive
    ON preference_feedback_aggregates (archive_id, last_event_at);

CREATE TABLE IF NOT EXISTS preference_feedback_event_applied (
    behavior_event_id UUID PRIMARY KEY REFERENCES user_behavior_events(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    archive_id UUID NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_preference_feedback_event_user
    ON preference_feedback_event_applied (user_id, applied_at);

ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS evidence_state TEXT NOT NULL DEFAULT 'insufficient_evidence';
ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'legacy';
ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS feature_kind TEXT;
ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS profile_version TEXT;
ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS unique_archive_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS informative_result_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS effective_read_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS deep_read_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS manual_delete_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS quick_exit_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS conflict_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS baseline_rate DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS lift DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS direction_probability DOUBLE PRECISION NOT NULL DEFAULT 0.5;
ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS profile_coverage DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE preference_rule_candidates ADD COLUMN IF NOT EXISTS evidence_json JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE INDEX IF NOT EXISTS idx_preference_candidates_source_state
    ON preference_rule_candidates (user_id, source, evidence_state, updated_at);
