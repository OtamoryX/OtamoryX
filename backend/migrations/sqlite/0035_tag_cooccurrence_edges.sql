-- Deterministic ordinary-tag co-occurrence graph. It is a recall/explanation aid only:
-- it never rewrites archive_tags, profiles, or learned preference evidence.
CREATE TABLE IF NOT EXISTS tag_cooccurrence_edges (
    tag_a_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    tag_b_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    coarchive_count INTEGER NOT NULL,
    tag_a_archive_count INTEGER NOT NULL,
    tag_b_archive_count INTEGER NOT NULL,
    jaccard REAL NOT NULL,
    relation_kind TEXT NOT NULL DEFAULT 'cooccurrence',
    algorithm_version TEXT NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (tag_a_id, tag_b_id),
    CHECK (tag_a_id < tag_b_id),
    CHECK (coarchive_count >= 2),
    CHECK (tag_a_archive_count >= coarchive_count),
    CHECK (tag_b_archive_count >= coarchive_count),
    CHECK (jaccard >= 0.0 AND jaccard <= 1.0),
    CHECK (trim(relation_kind) <> ''),
    CHECK (trim(algorithm_version) <> '')
);

CREATE INDEX IF NOT EXISTS idx_tag_cooccurrence_edges_a
    ON tag_cooccurrence_edges (tag_a_id, jaccard DESC, coarchive_count DESC);
CREATE INDEX IF NOT EXISTS idx_tag_cooccurrence_edges_b
    ON tag_cooccurrence_edges (tag_b_id, jaccard DESC, coarchive_count DESC);
