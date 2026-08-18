ALTER TABLE trash_operation_members
    ADD COLUMN IF NOT EXISTS sequence INTEGER NOT NULL DEFAULT 0;

WITH ranked_members AS (
    SELECT id,
           ROW_NUMBER() OVER (
               PARTITION BY operation_id
               ORDER BY created_at, id
           ) - 1 AS member_sequence
    FROM trash_operation_members
)
UPDATE trash_operation_members AS members
SET sequence = ranked_members.member_sequence
FROM ranked_members
WHERE members.id = ranked_members.id;

CREATE UNIQUE INDEX IF NOT EXISTS idx_trash_operation_members_sequence
    ON trash_operation_members (operation_id, sequence);
