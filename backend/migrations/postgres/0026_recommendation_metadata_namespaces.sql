-- Keep objectively contextual namespaces out of future content clustering.
-- This migration only creates policy data; it does not rewrite existing tags.

CREATE TABLE IF NOT EXISTS recommendation_metadata_namespaces (
    namespace TEXT PRIMARY KEY,
    reason TEXT NOT NULL,
    policy_version TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (btrim(namespace) <> '')
);

CREATE INDEX IF NOT EXISTS idx_recommendation_metadata_namespaces_policy
    ON recommendation_metadata_namespaces (policy_version, namespace);

INSERT INTO recommendation_metadata_namespaces
    (namespace, reason, policy_version)
VALUES
    ('artist', 'work creator identity', 'metadata-v1'),
    ('author', 'work author identity', 'metadata-v1'),
    ('category', 'catalog classification', 'metadata-v1'),
    ('chapter', 'publication or file structure', 'metadata-v1'),
    ('character', 'work entity identity', 'metadata-v1'),
    ('date_added', 'library ingestion timestamp', 'metadata-v1'),
    ('date_added_iso8601', 'library ingestion timestamp', 'metadata-v1'),
    ('filename_token', 'filename-derived packaging data', 'metadata-v1'),
    ('group', 'creator group identity', 'metadata-v1'),
    ('language', 'work language attribute', 'metadata-v1'),
    ('location', 'source location attribute', 'metadata-v1'),
    ('metadata_source', 'metadata source identity', 'metadata-v1'),
    ('parody', 'work or series identity', 'metadata-v1'),
    ('source', 'external source identity', 'metadata-v1'),
    ('system', 'application state', 'metadata-v1'),
    ('volume', 'publication or file structure', 'metadata-v1'),
    ('year', 'publication year attribute', 'metadata-v1')
ON CONFLICT (namespace) DO NOTHING;

CREATE VIEW IF NOT EXISTS recommendation_non_metadata_tag_candidates AS
SELECT
    t.id,
    t.name,
    t.namespace,
    COUNT(DISTINCT at.archive_id) AS archive_count
FROM tags t
JOIN archive_tags at ON at.tag_id = t.id
WHERE NOT EXISTS (
    SELECT 1
    FROM recommendation_metadata_namespaces m
    WHERE lower(m.namespace) = lower(t.namespace)
)
GROUP BY t.id, t.name, t.namespace;
