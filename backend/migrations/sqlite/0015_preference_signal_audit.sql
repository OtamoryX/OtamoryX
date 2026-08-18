-- P8-3: retain immutable learning inputs so candidate scores can be recomputed
-- with a different decay policy without losing the original user behavior.
CREATE TABLE IF NOT EXISTS preference_candidate_signals (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    condition_key TEXT NOT NULL,
    archive_id TEXT NOT NULL,
    behavior_event_id TEXT NOT NULL REFERENCES user_behavior_events(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    raw_score REAL NOT NULL,
    occurred_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, behavior_event_id, condition_key)
);
CREATE INDEX IF NOT EXISTS idx_preference_candidate_signals_candidate_time
    ON preference_candidate_signals (user_id, condition_key, occurred_at);
CREATE INDEX IF NOT EXISTS idx_preference_candidate_signals_archive_time
    ON preference_candidate_signals (user_id, archive_id, occurred_at);

-- Keep automatic deletion provenance in relational columns. Historical rows
-- remain valid and may leave these nullable fields empty.
ALTER TABLE trash_entries ADD COLUMN rule_id TEXT;
ALTER TABLE trash_entries ADD COLUMN evaluation_id TEXT;
CREATE INDEX IF NOT EXISTS idx_trash_entries_rule_version
    ON trash_entries (rule_id, rule_version, deleted_at);
CREATE INDEX IF NOT EXISTS idx_trash_entries_evaluation
    ON trash_entries (evaluation_id);
