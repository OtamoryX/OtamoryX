ALTER TABLE content_analyses ADD COLUMN IF NOT EXISTS next_attempt_at TIMESTAMPTZ;
CREATE INDEX IF NOT EXISTS idx_content_analyses_claimable ON content_analyses(status, next_attempt_at, updated_at);

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
