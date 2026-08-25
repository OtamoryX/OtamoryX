ALTER TABLE content_analysis_evidence
ADD COLUMN sources_json TEXT NOT NULL DEFAULT '[]';
