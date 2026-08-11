-- Local, explainable identity facts and logical comic collections.
CREATE TABLE IF NOT EXISTS archive_identity_facts (
    archive_id TEXT PRIMARY KEY,
    raw_filename TEXT NOT NULL,
    parent_path TEXT NOT NULL,
    normalized_key TEXT NOT NULL,
    display_title TEXT NOT NULL,
    creator TEXT,
    unit_type TEXT NOT NULL DEFAULT 'unknown',
    volume_number TEXT,
    chapter_number TEXT,
    issue_number TEXT,
    edition_marker TEXT,
    confidence REAL NOT NULL DEFAULT 0,
    evidence_json TEXT NOT NULL DEFAULT '{}',
    parser_version TEXT NOT NULL DEFAULT 'collections-v1',
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_identity_normalized_key ON archive_identity_facts(normalized_key);

CREATE TABLE IF NOT EXISTS collections (
    id TEXT PRIMARY KEY,
    display_title TEXT NOT NULL,
    normalized_key TEXT NOT NULL,
    cover_archive_id TEXT,
    status TEXT NOT NULL DEFAULT 'auto',
    is_manual_locked BOOLEAN NOT NULL DEFAULT FALSE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(normalized_key),
    FOREIGN KEY (cover_archive_id) REFERENCES archives(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_collections_updated_at ON collections(updated_at);

CREATE TABLE IF NOT EXISTS collection_members (
    collection_id TEXT NOT NULL,
    archive_id TEXT PRIMARY KEY,
    unit_type TEXT NOT NULL DEFAULT 'unknown',
    volume_number TEXT,
    chapter_number TEXT,
    issue_number TEXT,
    raw_number TEXT,
    sort_key REAL NOT NULL DEFAULT 999999,
    variant_group_key TEXT,
    confidence REAL NOT NULL DEFAULT 0,
    membership_source TEXT NOT NULL DEFAULT 'auto',
    is_manual_locked BOOLEAN NOT NULL DEFAULT FALSE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE,
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_collection_members_collection ON collection_members(collection_id, sort_key);

CREATE TABLE IF NOT EXISTS collection_review_items (
    id TEXT PRIMARY KEY,
    archive_id TEXT NOT NULL,
    collection_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    evidence_json TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'pending',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(archive_id, collection_id),
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE,
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_collection_reviews_status ON collection_review_items(status, updated_at);

CREATE TABLE IF NOT EXISTS collection_exclusions (
    archive_id TEXT NOT NULL,
    collection_id TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (archive_id, collection_id),
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE,
    FOREIGN KEY (collection_id) REFERENCES collections(id) ON DELETE CASCADE
);
