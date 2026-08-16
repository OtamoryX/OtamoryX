-- Track durable cleanup attempts so expired trash entries can be retried safely.
ALTER TABLE trash_entries ADD COLUMN cleanup_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE trash_entries ADD COLUMN last_cleanup_attempt_at DATETIME;
ALTER TABLE trash_entries ADD COLUMN last_cleanup_error TEXT;
ALTER TABLE trash_entries ADD COLUMN expired_at DATETIME;
ALTER TABLE trash_entries ADD COLUMN restore_claimed_at DATETIME;

CREATE INDEX IF NOT EXISTS idx_trash_entries_cleanup_pending
    ON trash_entries (status, expired_at, last_cleanup_attempt_at);
