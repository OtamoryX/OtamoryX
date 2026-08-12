-- Generic AI job metadata. Existing rows remain compatible with the legacy queue.
ALTER TABLE archives ADD COLUMN IF NOT EXISTS subtitle TEXT;
ALTER TABLE archives ADD COLUMN IF NOT EXISTS subtitle_language TEXT;
ALTER TABLE archives ADD COLUMN IF NOT EXISTS subtitle_source_hash TEXT;

ALTER TABLE ai_processing_queue ADD COLUMN IF NOT EXISTS job_type TEXT NOT NULL DEFAULT 'auto_tagging';
ALTER TABLE ai_processing_queue ADD COLUMN IF NOT EXISTS payload TEXT;
ALTER TABLE ai_processing_queue ADD COLUMN IF NOT EXISTS source_hash TEXT;
ALTER TABLE ai_processing_queue ADD COLUMN IF NOT EXISTS dedupe_key TEXT;
ALTER TABLE ai_processing_queue ADD COLUMN IF NOT EXISTS next_run_at TIMESTAMPTZ;
ALTER TABLE ai_processing_queue ADD COLUMN IF NOT EXISTS lease_expires_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_ai_processing_queue_ready
    ON ai_processing_queue (status, next_run_at, priority DESC, created_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_processing_queue_active_dedupe
    ON ai_processing_queue (dedupe_key)
    WHERE dedupe_key IS NOT NULL AND status IN ('pending', 'processing');

CREATE TABLE IF NOT EXISTS archive_title_translations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    archive_id UUID NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
    source_title TEXT NOT NULL,
    source_hash TEXT NOT NULL,
    target_language TEXT NOT NULL,
    translated_title TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    provider TEXT,
    model TEXT,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    UNIQUE(archive_id, target_language, source_hash)
);

CREATE INDEX IF NOT EXISTS idx_archive_title_translations_archive
    ON archive_title_translations (archive_id, target_language, status);
