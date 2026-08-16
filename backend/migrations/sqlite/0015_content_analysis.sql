CREATE TABLE IF NOT EXISTS content_analyses (
    id TEXT PRIMARY KEY,
    archive_id TEXT NOT NULL,
    content_fingerprint TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','running','completed','retryable','failed')),
    provider TEXT,
    model TEXT,
    prompt_version TEXT NOT NULL,
    result_json TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at DATETIME,
    completed_at DATETIME,
    lease_expires_at DATETIME,
    UNIQUE(archive_id, content_fingerprint, prompt_version)
);
CREATE INDEX IF NOT EXISTS idx_content_analyses_status ON content_analyses(status, updated_at);
CREATE TABLE IF NOT EXISTS content_analysis_evidence (
    id TEXT PRIMARY KEY,
    analysis_id TEXT NOT NULL REFERENCES content_analyses(id) ON DELETE CASCADE,
    page_number INTEGER NOT NULL,
    page_role TEXT NOT NULL,
    concepts_json TEXT NOT NULL DEFAULT '[]',
    confidence REAL,
    summary TEXT NOT NULL,
    UNIQUE(analysis_id, page_number)
);
