-- Preserve the order in which members were migrated so relation snapshots
-- can be reverted in the exact opposite order even when timestamps tie.
ALTER TABLE trash_operation_members ADD COLUMN sequence INTEGER NOT NULL DEFAULT 0;

UPDATE trash_operation_members AS current_member
SET sequence = (
    SELECT COUNT(*) - 1
    FROM trash_operation_members AS prior_member
    WHERE prior_member.operation_id = current_member.operation_id
      AND (
          prior_member.created_at < current_member.created_at
          OR (
              prior_member.created_at = current_member.created_at
              AND prior_member.id <= current_member.id
          )
      )
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_trash_operation_members_sequence
    ON trash_operation_members (operation_id, sequence);
