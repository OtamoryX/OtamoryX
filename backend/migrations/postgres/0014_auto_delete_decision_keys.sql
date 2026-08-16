-- Make automatic archive disposition deliveries auditable and idempotent.
ALTER TABLE trash_entries ADD COLUMN IF NOT EXISTS decision_key TEXT;
ALTER TABLE archive_dispositions ADD COLUMN IF NOT EXISTS decision_key TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_trash_entries_user_decision_key
    ON trash_entries (user_id, decision_key)
    WHERE decision_key IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_archive_dispositions_user_decision_key
    ON archive_dispositions (user_id, decision_key)
    WHERE decision_key IS NOT NULL;
