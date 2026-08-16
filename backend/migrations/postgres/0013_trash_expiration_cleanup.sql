-- Track durable cleanup attempts so expired trash entries can be retried safely.
ALTER TABLE trash_entries ADD COLUMN IF NOT EXISTS cleanup_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE trash_entries ADD COLUMN IF NOT EXISTS last_cleanup_attempt_at TIMESTAMPTZ;
ALTER TABLE trash_entries ADD COLUMN IF NOT EXISTS last_cleanup_error TEXT;
ALTER TABLE trash_entries ADD COLUMN IF NOT EXISTS expired_at TIMESTAMPTZ;
ALTER TABLE trash_entries ADD COLUMN IF NOT EXISTS restore_claimed_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_trash_entries_cleanup_pending
    ON trash_entries (status, expired_at, last_cleanup_attempt_at);
