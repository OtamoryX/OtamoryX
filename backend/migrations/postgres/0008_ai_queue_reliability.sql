CREATE TABLE IF NOT EXISTS ai_provider_states (
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    blocked_until TIMESTAMPTZ,
    last_error TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (provider, model)
);

CREATE TABLE IF NOT EXISTS ai_job_attempts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    job_id UUID NOT NULL REFERENCES ai_processing_queue(id) ON DELETE CASCADE,
    attempt_number INTEGER NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at TIMESTAMPTZ,
    outcome TEXT,
    error TEXT,
    UNIQUE(job_id, attempt_number)
);

CREATE INDEX IF NOT EXISTS idx_ai_job_attempts_job ON ai_job_attempts (job_id, attempt_number);

CREATE TABLE IF NOT EXISTS ai_queue_scheduler_state (
    id TEXT PRIMARY KEY CHECK (id = 'default'),
    last_job_type TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO ai_queue_scheduler_state (id, last_job_type)
VALUES ('default', NULL)
ON CONFLICT (id) DO NOTHING;

UPDATE ai_processing_queue q
SET status = 'pending', completed_at = NULL, next_run_at = NOW()
WHERE q.job_type = 'title_translation'
  AND q.status = 'failed'
  AND (
      q.last_error LIKE 'AI provider returned HTTP 429%'
      OR q.last_error LIKE 'AI title translation request failed:%'
      OR q.last_error LIKE 'Invalid AI provider response:%'
      OR q.last_error = 'AI provider response has no assistant content'
  )
  AND EXISTS (
      SELECT 1 FROM archive_title_translations t
      WHERE t.archive_id = q.archive_id
        AND t.source_hash = q.source_hash
        AND t.status = 'failed'
  )
  AND q.id = (
      SELECT newer.id FROM ai_processing_queue newer
      WHERE newer.job_type = q.job_type
        AND newer.dedupe_key = q.dedupe_key
        AND newer.status = 'failed'
      ORDER BY newer.created_at DESC, newer.id DESC
      LIMIT 1
  )
  AND NOT EXISTS (
      SELECT 1 FROM ai_processing_queue active
      WHERE active.job_type = q.job_type
        AND active.dedupe_key = q.dedupe_key
        AND active.status IN ('pending', 'processing')
  );
