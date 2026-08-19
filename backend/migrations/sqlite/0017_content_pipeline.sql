-- The historical ai_processing_queue is the durable project-wide job queue. Its name is kept
-- for upgrade compatibility; executor_lane makes non-LLM work explicit for operators.
ALTER TABLE ai_processing_queue ADD COLUMN executor_lane TEXT NOT NULL DEFAULT 'llm';
CREATE INDEX IF NOT EXISTS idx_ai_processing_queue_lane_ready
    ON ai_processing_queue (executor_lane, status, next_run_at, priority DESC, created_at);

-- A capability artifact is a compact, versioned statement about whether an archive has usable
-- translation, OCR, metadata or tag input for a particular content fingerprint.
CREATE TABLE IF NOT EXISTS archive_artifacts (
    id TEXT PRIMARY KEY,
    archive_id TEXT NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
    artifact_type TEXT NOT NULL,
    source TEXT NOT NULL,
    input_fingerprint TEXT NOT NULL,
    artifact_version TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'ready', 'empty', 'not_applicable', 'retryable', 'failed', 'stale')),
    data_json TEXT NOT NULL DEFAULT '{}',
    source_record_id TEXT,
    job_id TEXT REFERENCES ai_processing_queue(id) ON DELETE SET NULL,
    last_error TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    UNIQUE (archive_id, artifact_type, source, input_fingerprint, artifact_version)
);
CREATE INDEX IF NOT EXISTS idx_archive_artifacts_lookup
    ON archive_artifacts (archive_id, artifact_type, status, updated_at DESC);

-- Reconciliation creates required capability jobs. Synthesis consumes an immutable snapshot,
-- allowing partial data now and a richer replacement revision when a source later becomes ready.
CREATE TABLE IF NOT EXISTS content_analysis_runs (
    id TEXT PRIMARY KEY,
    archive_id TEXT NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
    content_fingerprint TEXT NOT NULL,
    policy_version TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'waiting_inputs', 'ready_to_synthesize', 'completed', 'partial', 'retryable', 'failed')),
    desired_inputs_json TEXT NOT NULL DEFAULT '[]',
    input_manifest_json TEXT NOT NULL DEFAULT '[]',
    last_error TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    UNIQUE (archive_id, content_fingerprint, policy_version)
);
CREATE INDEX IF NOT EXISTS idx_content_analysis_runs_ready
    ON content_analysis_runs (status, updated_at, archive_id);

CREATE TABLE IF NOT EXISTS content_analysis_run_inputs (
    run_id TEXT NOT NULL REFERENCES content_analysis_runs(id) ON DELETE CASCADE,
    artifact_id TEXT NOT NULL REFERENCES archive_artifacts(id) ON DELETE RESTRICT,
    artifact_type TEXT NOT NULL,
    required INTEGER NOT NULL DEFAULT 0,
    snapshot_json TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (run_id, artifact_id)
);

ALTER TABLE content_analyses ADD COLUMN run_id TEXT REFERENCES content_analysis_runs(id) ON DELETE SET NULL;
ALTER TABLE content_analyses ADD COLUMN source_manifest_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE content_analyses ADD COLUMN completeness_json TEXT NOT NULL DEFAULT '{}';
CREATE INDEX IF NOT EXISTS idx_content_analyses_run ON content_analyses (run_id, completed_at DESC);

CREATE TABLE IF NOT EXISTS ai_tagging_runs (
    id TEXT PRIMARY KEY,
    archive_id TEXT NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
    analysis_id TEXT REFERENCES content_analyses(id) ON DELETE SET NULL,
    job_id TEXT REFERENCES ai_processing_queue(id) ON DELETE SET NULL,
    content_fingerprint TEXT NOT NULL,
    provider TEXT,
    model TEXT,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'partial', 'failed', 'undone')),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME
);
CREATE INDEX IF NOT EXISTS idx_ai_tagging_runs_archive ON ai_tagging_runs (archive_id, created_at DESC);

CREATE TABLE IF NOT EXISTS ai_tag_suggestions (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES ai_tagging_runs(id) ON DELETE CASCADE,
    archive_id TEXT NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
    normalized_name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    namespace TEXT NOT NULL,
    confidence REAL NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
    evidence_json TEXT NOT NULL DEFAULT '[]',
    provenance_json TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'rejected', 'auto_applied', 'undone')),
    reviewed_at DATETIME,
    reviewed_by TEXT,
    edited_tag_id TEXT REFERENCES tags(id) ON DELETE SET NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (run_id, normalized_name, namespace)
);
CREATE INDEX IF NOT EXISTS idx_ai_tag_suggestions_pending
    ON ai_tag_suggestions (status, created_at DESC, archive_id);

CREATE TABLE IF NOT EXISTS ai_tag_applications (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES ai_tagging_runs(id) ON DELETE CASCADE,
    suggestion_id TEXT NOT NULL REFERENCES ai_tag_suggestions(id) ON DELETE CASCADE,
    archive_id TEXT NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    application_source TEXT NOT NULL CHECK (application_source IN ('automatic', 'review')),
    applied_by TEXT,
    created_archive_tag INTEGER NOT NULL DEFAULT 0,
    applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    undone_at DATETIME,
    undone_by TEXT,
    UNIQUE (suggestion_id)
);
CREATE INDEX IF NOT EXISTS idx_ai_tag_applications_active
    ON ai_tag_applications (archive_id, tag_id, undone_at);
