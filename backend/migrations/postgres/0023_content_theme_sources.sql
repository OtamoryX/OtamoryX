ALTER TABLE content_analysis_evidence
ADD COLUMN IF NOT EXISTS sources_json JSONB NOT NULL DEFAULT '[]'::jsonb;
