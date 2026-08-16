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
