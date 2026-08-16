CREATE TABLE IF NOT EXISTS random_recommendation_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    filters_json TEXT NOT NULL DEFAULT '{}',
    exploration_ratio REAL NOT NULL,
    candidate_count INTEGER NOT NULL DEFAULT 0,
    keep_count INTEGER NOT NULL DEFAULT 0,
    unknown_count INTEGER NOT NULL DEFAULT 0,
    downrank_count INTEGER NOT NULL DEFAULT 0,
    returned_count INTEGER NOT NULL DEFAULT 0,
    explored_count INTEGER NOT NULL DEFAULT 0,
    algorithm_version TEXT NOT NULL DEFAULT 'weighted-v1',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME NOT NULL DEFAULT (datetime('now', '+90 days'))
);
CREATE INDEX IF NOT EXISTS idx_random_recommendation_sessions_user_created
    ON random_recommendation_sessions(user_id, created_at);

CREATE TABLE IF NOT EXISTS random_recommendation_items (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES random_recommendation_sessions(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    archive_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    preference_tier TEXT NOT NULL,
    sampling_weight REAL NOT NULL,
    is_exploration INTEGER NOT NULL DEFAULT 0,
    opened_at DATETIME,
    effective_read_at DATETIME,
    quick_exit_at DATETIME,
    manual_delete_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(session_id, archive_id)
);
CREATE INDEX IF NOT EXISTS idx_random_recommendation_items_lookup
    ON random_recommendation_items(user_id, session_id, archive_id);
CREATE INDEX IF NOT EXISTS idx_random_recommendation_items_expiry
    ON random_recommendation_items(created_at);
