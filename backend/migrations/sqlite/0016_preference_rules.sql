ALTER TABLE content_analyses ADD COLUMN next_attempt_at DATETIME;
CREATE INDEX IF NOT EXISTS idx_content_analyses_claimable ON content_analyses(status, next_attempt_at, updated_at);

CREATE TABLE IF NOT EXISTS preference_rules (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    rule_version TEXT NOT NULL,
    conditions_json TEXT NOT NULL,
    exceptions_json TEXT NOT NULL DEFAULT '{}',
    action TEXT NOT NULL CHECK (action IN ('keep','downrank','auto_delete')),
    confidence_threshold REAL NOT NULL DEFAULT 0.85 CHECK (confidence_threshold >= 0 AND confidence_threshold <= 1),
    enabled INTEGER NOT NULL DEFAULT 0,
    owner_role TEXT NOT NULL DEFAULT 'user',
    false_positive_count INTEGER NOT NULL DEFAULT 0,
    auto_paused INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, id, rule_version)
);
CREATE INDEX IF NOT EXISTS idx_preference_rules_active ON preference_rules(user_id, enabled, auto_paused);

CREATE TABLE IF NOT EXISTS preference_rule_evaluations (
    id TEXT PRIMARY KEY,
    analysis_id TEXT NOT NULL REFERENCES content_analyses(id) ON DELETE CASCADE,
    rule_id TEXT NOT NULL,
    rule_version TEXT NOT NULL,
    matched INTEGER NOT NULL,
    matched_conditions_json TEXT NOT NULL,
    evidence_pages_json TEXT NOT NULL DEFAULT '[]',
    decision TEXT NOT NULL CHECK (decision IN ('keep','downrank','auto_delete','no_match')),
    execution_status TEXT NOT NULL DEFAULT 'pending',
    error TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(analysis_id, rule_id, rule_version)
);
CREATE INDEX IF NOT EXISTS idx_preference_evaluations_archive ON preference_rule_evaluations(analysis_id, created_at);

CREATE TABLE IF NOT EXISTS preference_rule_corrections (
    id TEXT PRIMARY KEY,
    evaluation_id TEXT NOT NULL REFERENCES preference_rule_evaluations(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL,
    correction TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
