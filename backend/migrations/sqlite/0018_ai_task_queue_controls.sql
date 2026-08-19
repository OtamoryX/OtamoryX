-- Queue control is intentionally separate from model/provider availability. A paused task type
-- retains its durable work items; only new claims are prevented.
CREATE TABLE IF NOT EXISTS ai_queue_controls (
    job_type TEXT PRIMARY KEY,
    manually_paused INTEGER NOT NULL DEFAULT 0,
    force_next_model_attempt INTEGER NOT NULL DEFAULT 0,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
