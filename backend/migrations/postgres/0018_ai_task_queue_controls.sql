-- Queue control is intentionally separate from model/provider availability. A paused task type
-- retains its durable work items; only new claims are prevented.
CREATE TABLE IF NOT EXISTS ai_queue_controls (
    job_type TEXT PRIMARY KEY,
    manually_paused BOOLEAN NOT NULL DEFAULT FALSE,
    force_next_model_attempt BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
