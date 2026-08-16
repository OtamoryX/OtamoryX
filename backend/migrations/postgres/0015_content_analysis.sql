CREATE TABLE IF NOT EXISTS content_analyses (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    archive_id UUID NOT NULL,
    content_fingerprint TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','running','completed','retryable','failed')),
    provider TEXT,
    model TEXT,
    prompt_version TEXT NOT NULL,
    result_json JSONB,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    lease_expires_at TIMESTAMPTZ,
    UNIQUE(archive_id, content_fingerprint, prompt_version)
);
CREATE INDEX IF NOT EXISTS idx_content_analyses_status ON content_analyses(status, updated_at);
CREATE TABLE IF NOT EXISTS content_analysis_evidence (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    analysis_id UUID NOT NULL REFERENCES content_analyses(id) ON DELETE CASCADE,
    page_number INTEGER NOT NULL,
    page_role TEXT NOT NULL,
    concepts_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    confidence DOUBLE PRECISION,
    summary TEXT NOT NULL,
    UNIQUE(analysis_id, page_number)
);
