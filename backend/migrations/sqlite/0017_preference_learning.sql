CREATE TABLE IF NOT EXISTS preference_learning_events (
    id TEXT PRIMARY KEY,
    behavior_event_id TEXT NOT NULL UNIQUE REFERENCES user_behavior_events(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','running','waiting_analysis','completed','retryable','failed')),
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    next_attempt_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_preference_learning_claimable ON preference_learning_events(status, next_attempt_at, updated_at);

CREATE TABLE IF NOT EXISTS preference_rule_candidates (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    condition_key TEXT NOT NULL,
    conditions_json TEXT NOT NULL,
    positive_score REAL NOT NULL DEFAULT 0,
    negative_score REAL NOT NULL DEFAULT 0,
    positive_support INTEGER NOT NULL DEFAULT 0,
    negative_support INTEGER NOT NULL DEFAULT 0,
    sample_archives_json TEXT NOT NULL DEFAULT '[]',
    confidence REAL NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'observing' CHECK (status IN ('observing','promoted','rejected')),
    last_learned_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, condition_key)
);
CREATE INDEX IF NOT EXISTS idx_preference_candidates_user_status ON preference_rule_candidates(user_id, status, confidence);

ALTER TABLE preference_rules ADD COLUMN source TEXT NOT NULL DEFAULT 'manual';
ALTER TABLE preference_rules ADD COLUMN preference_weight REAL NOT NULL DEFAULT 1.0;
ALTER TABLE preference_rules ADD COLUMN positive_support INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rules ADD COLUMN negative_support INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rules ADD COLUMN last_learned_at DATETIME;
