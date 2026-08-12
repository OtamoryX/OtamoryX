-- Persist provider-wide cooldowns so concurrent workers do not keep exhausting a known
-- rate-limited provider. Job rows remain the current state; attempts are retained for audit.
CREATE TABLE IF NOT EXISTS ai_provider_states (
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    blocked_until DATETIME,
    last_error TEXT,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (provider, model)
);

CREATE TABLE IF NOT EXISTS ai_job_attempts (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL,
    attempt_number INTEGER NOT NULL,
    started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at DATETIME,
    outcome TEXT,
    error TEXT,
    FOREIGN KEY (job_id) REFERENCES ai_processing_queue(id) ON DELETE CASCADE,
    UNIQUE(job_id, attempt_number)
);

CREATE INDEX IF NOT EXISTS idx_ai_job_attempts_job ON ai_job_attempts (job_id, attempt_number);

CREATE TABLE IF NOT EXISTS ai_queue_scheduler_state (
    id TEXT PRIMARY KEY CHECK (id = 'default'),
    last_job_type TEXT,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO ai_queue_scheduler_state (id, last_job_type) VALUES ('default', NULL);

-- Older releases treated temporary provider failures as terminal after a few seconds. Recover
-- only entries whose latest translation is still failed; successful later attempts stay intact.
UPDATE ai_processing_queue
SET status = 'pending', completed_at = NULL, next_run_at = CURRENT_TIMESTAMP
WHERE job_type = 'title_translation'
  AND status = 'failed'
  AND (
      last_error LIKE 'AI provider returned HTTP 429%'
      OR last_error LIKE 'AI title translation request failed:%'
      OR last_error LIKE 'Invalid AI provider response:%'
      OR last_error = 'AI provider response has no assistant content'
  )
  AND EXISTS (
      SELECT 1 FROM archive_title_translations t
      WHERE t.archive_id = ai_processing_queue.archive_id
        AND t.source_hash = ai_processing_queue.source_hash
        AND t.status = 'failed'
  )
  AND id = (
      SELECT newer.id FROM ai_processing_queue newer
      WHERE newer.job_type = ai_processing_queue.job_type
        AND newer.dedupe_key = ai_processing_queue.dedupe_key
        AND newer.status = 'failed'
      ORDER BY newer.created_at DESC, newer.rowid DESC
      LIMIT 1
  )
  AND NOT EXISTS (
      SELECT 1 FROM ai_processing_queue active
      WHERE active.job_type = ai_processing_queue.job_type
        AND active.dedupe_key = ai_processing_queue.dedupe_key
        AND active.status IN ('pending', 'processing')
  );
