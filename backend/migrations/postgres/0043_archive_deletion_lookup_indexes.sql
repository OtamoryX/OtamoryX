-- Trash snapshots and archive deletion both look up dependents by archive_id.
-- Index the foreign-key lookups, including those reached when queue rows cascade.
-- Status-first worker indexes cannot serve these lookups across job history.
CREATE INDEX IF NOT EXISTS idx_ai_processing_queue_archive
    ON ai_processing_queue (archive_id);
CREATE INDEX IF NOT EXISTS idx_ai_tag_suggestions_archive
    ON ai_tag_suggestions (archive_id);
CREATE INDEX IF NOT EXISTS idx_collections_cover_archive
    ON collections (cover_archive_id);
CREATE INDEX IF NOT EXISTS idx_archive_artifacts_job
    ON archive_artifacts (job_id);
CREATE INDEX IF NOT EXISTS idx_ai_tagging_runs_job
    ON ai_tagging_runs (job_id);
