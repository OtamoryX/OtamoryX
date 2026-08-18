-- A version cleanup is one recoverable operation, rather than a set of
-- unrelated permanent archive deletions.
CREATE TABLE IF NOT EXISTS trash_operations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    operation_type TEXT NOT NULL CHECK (operation_type IN ('version_cleanup')),
    group_key TEXT NOT NULL,
    keep_archive_id TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    migration_snapshot_json TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'active', 'restoring', 'restored', 'expired', 'failed')),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME NOT NULL DEFAULT (datetime('now', '+14 days')),
    restored_at DATETIME,
    UNIQUE(user_id, idempotency_key)
);
CREATE INDEX IF NOT EXISTS idx_trash_operations_group_status
    ON trash_operations (group_key, status, created_at);

CREATE TABLE IF NOT EXISTS trash_operation_members (
    id TEXT PRIMARY KEY,
    operation_id TEXT NOT NULL REFERENCES trash_operations(id) ON DELETE CASCADE,
    archive_id TEXT NOT NULL,
    trash_entry_id TEXT NOT NULL REFERENCES trash_entries(id) ON DELETE RESTRICT,
    migration_snapshot_json TEXT NOT NULL DEFAULT '{}',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(operation_id, archive_id),
    UNIQUE(trash_entry_id)
);

ALTER TABLE trash_entries ADD COLUMN operation_id TEXT;
ALTER TABLE trash_entries ADD COLUMN operation_type TEXT;
CREATE INDEX IF NOT EXISTS idx_trash_entries_operation
    ON trash_entries (operation_id, status, deleted_at);
