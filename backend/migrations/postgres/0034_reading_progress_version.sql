ALTER TABLE reading_progress
    ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;

CREATE INDEX IF NOT EXISTS idx_reading_progress_user_last_read
    ON reading_progress(user_id, last_read_at DESC);
