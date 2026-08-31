-- Canonical themes are system-managed recommendation identities. Existing content-analysis
-- rows are deliberately left as legacy raw results; production canonicalization starts at the
-- next completed analysis and does not backfill historical themes.
ALTER TABLE content_analyses ADD COLUMN canonicalization_status TEXT NOT NULL DEFAULT 'legacy';
ALTER TABLE content_analyses ADD COLUMN canonicalization_version TEXT;
ALTER TABLE content_analyses ADD COLUMN canonicalization_error TEXT;
ALTER TABLE content_analysis_runs ADD COLUMN canonicalization_status TEXT NOT NULL DEFAULT 'legacy';

DROP VIEW IF EXISTS recommendation_non_metadata_tag_candidates;
CREATE VIEW recommendation_non_metadata_tag_candidates AS
SELECT
    t.id,
    t.name,
    t.namespace,
    COUNT(DISTINCT at.archive_id) AS archive_count
FROM tags t
JOIN archive_tags at ON at.tag_id = t.id
WHERE lower(trim(t.namespace)) <> 'theme'
  AND NOT EXISTS (
      SELECT 1
      FROM recommendation_metadata_namespaces m
      WHERE lower(m.namespace) = lower(t.namespace)
  )
GROUP BY t.id, t.name, t.namespace;

CREATE TABLE IF NOT EXISTS canonical_theme_names (
    normalized_name TEXT PRIMARY KEY,
    theme_tag_id TEXT NOT NULL UNIQUE REFERENCES tags(id) ON DELETE CASCADE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS content_analysis_themes (
    analysis_id TEXT NOT NULL REFERENCES content_analyses(id) ON DELETE CASCADE,
    theme_tag_id TEXT REFERENCES tags(id) ON DELETE SET NULL,
    ordinal INTEGER NOT NULL,
    generated_name TEXT NOT NULL,
    canonicalization_status TEXT NOT NULL
        CHECK (canonicalization_status IN ('pending', 'completed', 'deduplicated', 'duplicate_conflict', 'failed')),
    canonicalization_version TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (analysis_id, ordinal),
    UNIQUE (analysis_id, theme_tag_id)
);

CREATE INDEX IF NOT EXISTS idx_content_analysis_themes_tag
    ON content_analysis_themes (theme_tag_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_content_analysis_themes_analysis
    ON content_analysis_themes (analysis_id, canonicalization_status, ordinal);

CREATE TABLE IF NOT EXISTS canonical_theme_embeddings (
    id TEXT PRIMARY KEY,
    theme_tag_id TEXT REFERENCES tags(id) ON DELETE SET NULL,
    normalized_name TEXT NOT NULL,
    input_hash TEXT NOT NULL,
    provider TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    model TEXT NOT NULL,
    dimensions INTEGER NOT NULL,
    config_version TEXT NOT NULL,
    vector_json TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (input_hash, provider, endpoint, model, dimensions, config_version)
);

CREATE INDEX IF NOT EXISTS idx_canonical_theme_embeddings_lookup
    ON canonical_theme_embeddings (provider, endpoint, model, config_version, input_hash);

CREATE TABLE IF NOT EXISTS theme_synonym_judgments (
    id TEXT PRIMARY KEY,
    pair_key TEXT NOT NULL,
    left_input_hash TEXT NOT NULL,
    right_input_hash TEXT NOT NULL,
    left_name TEXT NOT NULL,
    right_name TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    profile_id TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    schema_version TEXT NOT NULL,
    first_is_synonym INTEGER,
    reverse_is_synonym INTEGER,
    final_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (final_status IN ('pending', 'first_false', 'confirmed', 'failed')),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (pair_key, provider, model, profile_id, prompt_version, schema_version)
);

CREATE INDEX IF NOT EXISTS idx_theme_synonym_judgments_pair
    ON theme_synonym_judgments (pair_key, updated_at DESC);

CREATE TABLE IF NOT EXISTS theme_synonym_judgment_attempts (
    id TEXT PRIMARY KEY,
    judgment_id TEXT NOT NULL REFERENCES theme_synonym_judgments(id) ON DELETE CASCADE,
    direction TEXT NOT NULL CHECK (direction IN ('forward', 'reverse')),
    response_json TEXT,
    parse_status TEXT NOT NULL,
    is_synonym INTEGER,
    error TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_theme_synonym_judgment_attempts_judgment
    ON theme_synonym_judgment_attempts (judgment_id, created_at);
