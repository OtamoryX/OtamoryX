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

ALTER TABLE preference_rules ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'manual';
ALTER TABLE preference_rules ADD COLUMN IF NOT EXISTS preference_weight DOUBLE PRECISION NOT NULL DEFAULT 1.0;
ALTER TABLE preference_rules ADD COLUMN IF NOT EXISTS positive_support INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rules ADD COLUMN IF NOT EXISTS negative_support INTEGER NOT NULL DEFAULT 0;
ALTER TABLE preference_rules ADD COLUMN IF NOT EXISTS last_learned_at TIMESTAMPTZ;
