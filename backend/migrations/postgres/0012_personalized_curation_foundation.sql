-- Phase 8 P8-1: durable user behavior and curation decision history.
-- archive_id intentionally has no foreign key so feedback remains queryable after deletion.
CREATE TABLE IF NOT EXISTS user_behavior_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    archive_id UUID,
    event_type TEXT NOT NULL,
    event_key TEXT,
    page INTEGER,
    metadata_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, event_key)
);

CREATE INDEX IF NOT EXISTS idx_behavior_events_user_time
    ON user_behavior_events (user_id, occurred_at);
CREATE INDEX IF NOT EXISTS idx_behavior_events_archive_time
    ON user_behavior_events (archive_id, occurred_at);
CREATE INDEX IF NOT EXISTS idx_behavior_events_type
    ON user_behavior_events (event_type, occurred_at);

CREATE TABLE IF NOT EXISTS archive_dispositions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    archive_id UUID NOT NULL,
    disposition TEXT NOT NULL CHECK (disposition IN ('keep', 'downrank', 'auto_delete', 'manual_delete', 'restored')),
    reason TEXT,
    source TEXT NOT NULL DEFAULT 'user',
    confidence DOUBLE PRECISION,
    rule_version TEXT,
    metadata_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    decision_key TEXT
);

CREATE INDEX IF NOT EXISTS idx_archive_dispositions_user_time
    ON archive_dispositions (user_id, created_at);
CREATE INDEX IF NOT EXISTS idx_archive_dispositions_archive
    ON archive_dispositions (archive_id, created_at);

CREATE TABLE IF NOT EXISTS trash_entries (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    archive_id UUID NOT NULL,
    original_path TEXT NOT NULL,
    trash_path TEXT,
    reason TEXT,
    rule_version TEXT,
    model_confidence DOUBLE PRECISION,
    metadata_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'restored', 'expired')),
    deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    restored_at TIMESTAMPTZ,
    cleanup_attempts INTEGER NOT NULL DEFAULT 0,
    last_cleanup_attempt_at TIMESTAMPTZ,
    last_cleanup_error TEXT,
    expired_at TIMESTAMPTZ,
    restore_claimed_at TIMESTAMPTZ,
    decision_key TEXT
);

CREATE INDEX IF NOT EXISTS idx_trash_entries_user_status
    ON trash_entries (user_id, status, deleted_at);
CREATE INDEX IF NOT EXISTS idx_trash_entries_expiry
    ON trash_entries (status, expires_at);

CREATE INDEX IF NOT EXISTS idx_trash_entries_cleanup_pending
    ON trash_entries (status, expired_at, last_cleanup_attempt_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_trash_entries_user_decision_key
    ON trash_entries (user_id, decision_key)
    WHERE decision_key IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_archive_dispositions_user_decision_key
    ON archive_dispositions (user_id, decision_key)
    WHERE decision_key IS NOT NULL;

CREATE TABLE IF NOT EXISTS content_analyses (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    archive_id UUID NOT NULL,
    content_fingerprint TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','running','completed','retryable','failed')),
    provider TEXT,
    model TEXT,
    prompt_version TEXT NOT NULL,
    result_json JSONB,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    lease_expires_at TIMESTAMPTZ,
    next_attempt_at TIMESTAMPTZ,
    UNIQUE(archive_id, content_fingerprint, prompt_version)
);
CREATE INDEX IF NOT EXISTS idx_content_analyses_status ON content_analyses(status, updated_at);
CREATE INDEX IF NOT EXISTS idx_content_analyses_claimable ON content_analyses(status, next_attempt_at, updated_at);

CREATE TABLE IF NOT EXISTS content_analysis_evidence (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    analysis_id UUID NOT NULL REFERENCES content_analyses(id) ON DELETE CASCADE,
    page_number INTEGER NOT NULL,
    page_role TEXT NOT NULL,
    concepts_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    confidence DOUBLE PRECISION,
    summary TEXT NOT NULL,
    UNIQUE(analysis_id, page_number)
);

CREATE TABLE IF NOT EXISTS preference_rules (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    name TEXT NOT NULL,
    rule_version TEXT NOT NULL,
    conditions_json JSONB NOT NULL,
    exceptions_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    action TEXT NOT NULL CHECK (action IN ('keep','downrank','auto_delete')),
    confidence_threshold DOUBLE PRECISION NOT NULL DEFAULT 0.85 CHECK (confidence_threshold >= 0 AND confidence_threshold <= 1),
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    owner_role TEXT NOT NULL DEFAULT 'user',
    false_positive_count INTEGER NOT NULL DEFAULT 0,
    auto_paused BOOLEAN NOT NULL DEFAULT FALSE,
    source TEXT NOT NULL DEFAULT 'manual',
    preference_weight DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    positive_support INTEGER NOT NULL DEFAULT 0,
    negative_support INTEGER NOT NULL DEFAULT 0,
    last_learned_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, id, rule_version)
);
CREATE INDEX IF NOT EXISTS idx_preference_rules_active ON preference_rules(user_id, enabled, auto_paused);

CREATE TABLE IF NOT EXISTS preference_rule_evaluations (
    id UUID PRIMARY KEY,
    analysis_id UUID NOT NULL REFERENCES content_analyses(id) ON DELETE CASCADE,
    rule_id UUID NOT NULL,
    rule_version TEXT NOT NULL,
    matched BOOLEAN NOT NULL,
    matched_conditions_json JSONB NOT NULL,
    evidence_pages_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    decision TEXT NOT NULL CHECK (decision IN ('keep','downrank','auto_delete','no_match')),
    execution_status TEXT NOT NULL DEFAULT 'pending',
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(analysis_id, rule_id, rule_version)
);
CREATE INDEX IF NOT EXISTS idx_preference_evaluations_archive ON preference_rule_evaluations(analysis_id, created_at);

CREATE TABLE IF NOT EXISTS preference_rule_corrections (
    id UUID PRIMARY KEY,
    evaluation_id UUID NOT NULL REFERENCES preference_rule_evaluations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    correction TEXT NOT NULL,
    metadata_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS preference_learning_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    behavior_event_id UUID NOT NULL UNIQUE REFERENCES user_behavior_events(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','running','waiting_analysis','completed','retryable','failed')),
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    next_attempt_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_preference_learning_claimable ON preference_learning_events(status, next_attempt_at, updated_at);

CREATE TABLE IF NOT EXISTS preference_rule_candidates (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL,
    condition_key TEXT NOT NULL,
    conditions_json JSONB NOT NULL,
    positive_score DOUBLE PRECISION NOT NULL DEFAULT 0,
    negative_score DOUBLE PRECISION NOT NULL DEFAULT 0,
    positive_support INTEGER NOT NULL DEFAULT 0,
    negative_support INTEGER NOT NULL DEFAULT 0,
    sample_archives_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    confidence DOUBLE PRECISION NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'observing' CHECK (status IN ('observing','promoted','rejected')),
    last_learned_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, condition_key)
);
CREATE INDEX IF NOT EXISTS idx_preference_candidates_user_status ON preference_rule_candidates(user_id, status, confidence);

CREATE TABLE IF NOT EXISTS random_recommendation_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL,
    filters_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    exploration_ratio DOUBLE PRECISION NOT NULL,
    candidate_count INTEGER NOT NULL DEFAULT 0,
    keep_count INTEGER NOT NULL DEFAULT 0,
    unknown_count INTEGER NOT NULL DEFAULT 0,
    downrank_count INTEGER NOT NULL DEFAULT 0,
    returned_count INTEGER NOT NULL DEFAULT 0,
    explored_count INTEGER NOT NULL DEFAULT 0,
    algorithm_version TEXT NOT NULL DEFAULT 'weighted-v1',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '90 days')
);
CREATE INDEX IF NOT EXISTS idx_random_recommendation_sessions_user_created
    ON random_recommendation_sessions(user_id, created_at);

CREATE TABLE IF NOT EXISTS random_recommendation_items (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id UUID NOT NULL REFERENCES random_recommendation_sessions(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    archive_id UUID NOT NULL,
    position INTEGER NOT NULL,
    preference_tier TEXT NOT NULL,
    sampling_weight DOUBLE PRECISION NOT NULL,
    is_exploration BOOLEAN NOT NULL DEFAULT FALSE,
    opened_at TIMESTAMPTZ,
    effective_read_at TIMESTAMPTZ,
    quick_exit_at TIMESTAMPTZ,
    manual_delete_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(session_id, archive_id)
);
CREATE INDEX IF NOT EXISTS idx_random_recommendation_items_lookup
    ON random_recommendation_items(user_id, session_id, archive_id);
CREATE INDEX IF NOT EXISTS idx_random_recommendation_items_expiry
    ON random_recommendation_items(created_at);
