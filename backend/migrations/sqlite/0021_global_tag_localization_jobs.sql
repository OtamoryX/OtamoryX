-- Tag localization is global metadata. Archive-bound queue jobs keep their foreign-key
-- relationship; localization jobs use a NULL archive_id and are keyed by canonical tag value.
CREATE TABLE ai_processing_queue_new (
    id TEXT PRIMARY KEY,
    archive_id TEXT REFERENCES archives(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at DATETIME,
    completed_at DATETIME,
    job_type TEXT NOT NULL DEFAULT 'auto_tagging',
    payload TEXT,
    source_hash TEXT,
    dedupe_key TEXT,
    next_run_at DATETIME,
    lease_expires_at DATETIME,
    profile_id TEXT,
    executor_lane TEXT NOT NULL DEFAULT 'llm'
);

INSERT INTO ai_processing_queue_new
    (id, archive_id, status, priority, attempts, last_error, created_at, started_at, completed_at,
     job_type, payload, source_hash, dedupe_key, next_run_at, lease_expires_at, profile_id, executor_lane)
SELECT
    id, archive_id, status, priority, attempts, last_error, created_at, started_at, completed_at,
    job_type, payload, source_hash, dedupe_key, next_run_at, lease_expires_at, profile_id, executor_lane
FROM ai_processing_queue;

DROP TABLE ai_processing_queue;
ALTER TABLE ai_processing_queue_new RENAME TO ai_processing_queue;

CREATE INDEX idx_ai_processing_queue_ready
    ON ai_processing_queue (status, next_run_at, priority DESC, created_at);
CREATE UNIQUE INDEX idx_ai_processing_queue_active_dedupe
    ON ai_processing_queue (dedupe_key)
    WHERE dedupe_key IS NOT NULL AND status IN ('pending', 'processing');
CREATE INDEX idx_ai_processing_queue_profile_id
    ON ai_processing_queue (profile_id);
CREATE INDEX idx_ai_processing_queue_lane_ready
    ON ai_processing_queue (executor_lane, status, next_run_at, priority DESC, created_at);
