-- Replace idle queue polling with durable dependency state and event-driven wakeups.
-- Retry deadlines remain in next_run_at/next_attempt_at and are handled by timers.

DROP INDEX IF EXISTS idx_ai_processing_queue_active_dedupe;
CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_processing_queue_active_dedupe
    ON ai_processing_queue (dedupe_key)
    WHERE dedupe_key IS NOT NULL
      AND status IN ('pending', 'processing', 'waiting_dependency');

ALTER TABLE preference_rule_evaluations ADD COLUMN next_attempt_at DATETIME;
CREATE INDEX IF NOT EXISTS idx_preference_rule_evaluations_retry
    ON preference_rule_evaluations (execution_status, next_attempt_at);
