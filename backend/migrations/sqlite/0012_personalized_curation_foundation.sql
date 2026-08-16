-- Phase 8 P8-1: durable user behavior and curation decision history.
-- archive_id intentionally has no foreign key so feedback remains queryable after deletion.
CREATE TABLE IF NOT EXISTS user_behavior_events (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    archive_id TEXT,
    event_type TEXT NOT NULL,
    event_key TEXT,
    page INTEGER,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    occurred_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, event_key),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_behavior_events_user_time
    ON user_behavior_events (user_id, occurred_at);
CREATE INDEX IF NOT EXISTS idx_behavior_events_archive_time
    ON user_behavior_events (archive_id, occurred_at);
CREATE INDEX IF NOT EXISTS idx_behavior_events_type
    ON user_behavior_events (event_type, occurred_at);

CREATE TABLE IF NOT EXISTS archive_dispositions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    archive_id TEXT NOT NULL,
    disposition TEXT NOT NULL CHECK (disposition IN ('keep', 'downrank', 'auto_delete', 'manual_delete', 'restored')),
    reason TEXT,
    source TEXT NOT NULL DEFAULT 'user',
    confidence REAL,
    rule_version TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_archive_dispositions_user_time
    ON archive_dispositions (user_id, created_at);
CREATE INDEX IF NOT EXISTS idx_archive_dispositions_archive
    ON archive_dispositions (archive_id, created_at);

CREATE TABLE IF NOT EXISTS trash_entries (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    archive_id TEXT NOT NULL,
    original_path TEXT NOT NULL,
    trash_path TEXT,
    reason TEXT,
    rule_version TEXT,
    model_confidence REAL,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'restored', 'expired')),
    deleted_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME,
    restored_at DATETIME,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_trash_entries_user_status
    ON trash_entries (user_id, status, deleted_at);
CREATE INDEX IF NOT EXISTS idx_trash_entries_expiry
    ON trash_entries (status, expires_at);
