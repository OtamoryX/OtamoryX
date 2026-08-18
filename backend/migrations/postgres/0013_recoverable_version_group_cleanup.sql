CREATE TABLE IF NOT EXISTS trash_operations (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    operation_type TEXT NOT NULL CHECK (operation_type IN ('version_cleanup')),
    group_key TEXT NOT NULL,
    keep_archive_id UUID NOT NULL,
    idempotency_key TEXT NOT NULL,
    migration_snapshot_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'active', 'restoring', 'restored', 'expired', 'failed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '14 days'),
    restored_at TIMESTAMPTZ,
    UNIQUE(user_id, idempotency_key)
);
CREATE INDEX IF NOT EXISTS idx_trash_operations_group_status
    ON trash_operations (group_key, status, created_at);

ALTER TABLE trash_entries ADD COLUMN IF NOT EXISTS operation_id UUID;
ALTER TABLE trash_entries ADD COLUMN IF NOT EXISTS operation_type TEXT;
CREATE INDEX IF NOT EXISTS idx_trash_entries_operation
    ON trash_entries (operation_id, status, deleted_at);

CREATE TABLE IF NOT EXISTS trash_operation_members (
    id UUID PRIMARY KEY,
    operation_id UUID NOT NULL REFERENCES trash_operations(id) ON DELETE CASCADE,
    archive_id UUID NOT NULL,
    trash_entry_id UUID NOT NULL REFERENCES trash_entries(id) ON DELETE RESTRICT,
    migration_snapshot_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(operation_id, archive_id),
    UNIQUE(trash_entry_id)
);
