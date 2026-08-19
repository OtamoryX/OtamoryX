-- The historical ai_processing_queue is the project-wide durable queue. Retain its name for
-- upgrades while recording the executor lane for LLM, OCR, plugin and orchestration work.
ALTER TABLE ai_processing_queue ADD COLUMN IF NOT EXISTS executor_lane VARCHAR(32) NOT NULL DEFAULT 'llm';
CREATE INDEX IF NOT EXISTS idx_ai_processing_queue_lane_ready
    ON ai_processing_queue (executor_lane, status, next_run_at, priority DESC, created_at);

CREATE TABLE IF NOT EXISTS archive_artifacts (
    id UUID PRIMARY KEY,
    archive_id UUID NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
    artifact_type VARCHAR(64) NOT NULL,
    source VARCHAR(128) NOT NULL,
    input_fingerprint VARCHAR(128) NOT NULL,
    artifact_version VARCHAR(64) NOT NULL,
    status VARCHAR(32) NOT NULL CHECK (status IN ('pending', 'ready', 'empty', 'not_applicable', 'retryable', 'failed', 'stale')),
    data_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    source_record_id UUID,
    job_id UUID REFERENCES ai_processing_queue(id) ON DELETE SET NULL,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    UNIQUE (archive_id, artifact_type, source, input_fingerprint, artifact_version)
);
CREATE INDEX IF NOT EXISTS idx_archive_artifacts_lookup
    ON archive_artifacts (archive_id, artifact_type, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS content_analysis_runs (
    id UUID PRIMARY KEY,
    archive_id UUID NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
    content_fingerprint VARCHAR(128) NOT NULL,
    policy_version VARCHAR(64) NOT NULL,
    status VARCHAR(32) NOT NULL CHECK (status IN ('pending', 'waiting_inputs', 'ready_to_synthesize', 'completed', 'partial', 'retryable', 'failed')),
    desired_inputs_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    input_manifest_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    UNIQUE (archive_id, content_fingerprint, policy_version)
);
CREATE INDEX IF NOT EXISTS idx_content_analysis_runs_ready
    ON content_analysis_runs (status, updated_at, archive_id);

CREATE TABLE IF NOT EXISTS content_analysis_run_inputs (
    run_id UUID NOT NULL REFERENCES content_analysis_runs(id) ON DELETE CASCADE,
    artifact_id UUID NOT NULL REFERENCES archive_artifacts(id) ON DELETE RESTRICT,
    artifact_type VARCHAR(64) NOT NULL,
    required BOOLEAN NOT NULL DEFAULT FALSE,
    snapshot_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (run_id, artifact_id)
);

ALTER TABLE content_analyses ADD COLUMN IF NOT EXISTS run_id UUID REFERENCES content_analysis_runs(id) ON DELETE SET NULL;
ALTER TABLE content_analyses ADD COLUMN IF NOT EXISTS source_manifest_json JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE content_analyses ADD COLUMN IF NOT EXISTS completeness_json JSONB NOT NULL DEFAULT '{}'::jsonb;
CREATE INDEX IF NOT EXISTS idx_content_analyses_run ON content_analyses (run_id, completed_at DESC);

CREATE TABLE IF NOT EXISTS ai_tagging_runs (
    id UUID PRIMARY KEY,
    archive_id UUID NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
    analysis_id UUID REFERENCES content_analyses(id) ON DELETE SET NULL,
    job_id UUID REFERENCES ai_processing_queue(id) ON DELETE SET NULL,
    content_fingerprint VARCHAR(128) NOT NULL,
    provider TEXT,
    model TEXT,
    status VARCHAR(32) NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'partial', 'failed', 'undone')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_ai_tagging_runs_archive ON ai_tagging_runs (archive_id, created_at DESC);

CREATE TABLE IF NOT EXISTS ai_tag_suggestions (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES ai_tagging_runs(id) ON DELETE CASCADE,
    archive_id UUID NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
    normalized_name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    namespace VARCHAR(100) NOT NULL,
    confidence REAL NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
    evidence_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    provenance_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    status VARCHAR(32) NOT NULL CHECK (status IN ('pending', 'approved', 'rejected', 'auto_applied', 'undone')),
    reviewed_at TIMESTAMPTZ,
    reviewed_by UUID,
    edited_tag_id UUID REFERENCES tags(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (run_id, normalized_name, namespace)
);
CREATE INDEX IF NOT EXISTS idx_ai_tag_suggestions_pending
    ON ai_tag_suggestions (status, created_at DESC, archive_id);

CREATE TABLE IF NOT EXISTS ai_tag_applications (
    id UUID PRIMARY KEY,
    run_id UUID NOT NULL REFERENCES ai_tagging_runs(id) ON DELETE CASCADE,
    suggestion_id UUID NOT NULL REFERENCES ai_tag_suggestions(id) ON DELETE CASCADE,
    archive_id UUID NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    application_source VARCHAR(32) NOT NULL CHECK (application_source IN ('automatic', 'review')),
    applied_by UUID,
    created_archive_tag BOOLEAN NOT NULL DEFAULT FALSE,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    undone_at TIMESTAMPTZ,
    undone_by UUID,
    UNIQUE (suggestion_id)
);
CREATE INDEX IF NOT EXISTS idx_ai_tag_applications_active
    ON ai_tag_applications (archive_id, tag_id, undone_at);
