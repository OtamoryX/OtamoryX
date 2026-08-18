-- P8-3: retain immutable learning inputs so candidate scores can be recomputed
-- with a different decay policy without losing the original user behavior.
CREATE TABLE IF NOT EXISTS preference_candidate_signals (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    condition_key TEXT NOT NULL,
    archive_id UUID NOT NULL,
    behavior_event_id UUID NOT NULL REFERENCES user_behavior_events(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    raw_score DOUBLE PRECISION NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, behavior_event_id, condition_key)
);
CREATE INDEX IF NOT EXISTS idx_preference_candidate_signals_candidate_time
    ON preference_candidate_signals (user_id, condition_key, occurred_at);
CREATE INDEX IF NOT EXISTS idx_preference_candidate_signals_archive_time
    ON preference_candidate_signals (user_id, archive_id, occurred_at);

-- Keep automatic deletion provenance in relational columns. Historical rows
-- remain valid and may leave these nullable fields empty.
ALTER TABLE trash_entries ADD COLUMN IF NOT EXISTS rule_id UUID;
ALTER TABLE trash_entries ADD COLUMN IF NOT EXISTS evaluation_id UUID;
ALTER TABLE trash_entries
    ADD CONSTRAINT trash_entries_rule_id_fkey
    FOREIGN KEY (rule_id) REFERENCES preference_rules(id) ON DELETE SET NULL;
ALTER TABLE trash_entries
    ADD CONSTRAINT trash_entries_evaluation_id_fkey
    FOREIGN KEY (evaluation_id) REFERENCES preference_rule_evaluations(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_trash_entries_rule_version
    ON trash_entries (rule_id, rule_version, deleted_at);
CREATE INDEX IF NOT EXISTS idx_trash_entries_evaluation
    ON trash_entries (evaluation_id);
