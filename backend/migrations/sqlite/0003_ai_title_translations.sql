-- Generic AI job metadata. Existing rows remain compatible with the legacy queue.
ALTER TABLE archives ADD COLUMN subtitle TEXT;
ALTER TABLE archives ADD COLUMN subtitle_language TEXT;
ALTER TABLE archives ADD COLUMN subtitle_source_hash TEXT;

ALTER TABLE ai_processing_queue ADD COLUMN job_type TEXT NOT NULL DEFAULT 'auto_tagging';
ALTER TABLE ai_processing_queue ADD COLUMN payload TEXT;
ALTER TABLE ai_processing_queue ADD COLUMN source_hash TEXT;
ALTER TABLE ai_processing_queue ADD COLUMN dedupe_key TEXT;
ALTER TABLE ai_processing_queue ADD COLUMN next_run_at DATETIME;
ALTER TABLE ai_processing_queue ADD COLUMN lease_expires_at DATETIME;

CREATE INDEX IF NOT EXISTS idx_ai_processing_queue_ready
    ON ai_processing_queue (status, next_run_at, priority DESC, created_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_processing_queue_active_dedupe
    ON ai_processing_queue (dedupe_key)
    WHERE dedupe_key IS NOT NULL AND status IN ('pending', 'processing');

CREATE TABLE IF NOT EXISTS archive_title_translations (
    id TEXT PRIMARY KEY,
    archive_id TEXT NOT NULL,
    source_title TEXT NOT NULL,
    source_hash TEXT NOT NULL,
    target_language TEXT NOT NULL,
    translated_title TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    provider TEXT,
    model TEXT,
    last_error TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    UNIQUE(archive_id, target_language, source_hash),
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_archive_title_translations_archive
    ON archive_title_translations (archive_id, target_language, status);
