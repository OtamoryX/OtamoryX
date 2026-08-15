ALTER TABLE ai_processing_queue ADD COLUMN profile_id TEXT;

CREATE INDEX IF NOT EXISTS idx_ai_processing_queue_profile_id
    ON ai_processing_queue (profile_id);
