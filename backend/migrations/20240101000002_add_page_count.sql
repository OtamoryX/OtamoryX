-- Add missing page_count column to archives table
ALTER TABLE archives ADD COLUMN page_count INTEGER NOT NULL DEFAULT 0;

-- Ensure the "new" special tag exists
INSERT OR IGNORE INTO tags (id, name, namespace) 
VALUES ('new-tag-id', 'new', 'system');

-- Create index for performance
CREATE INDEX IF NOT EXISTS idx_archives_created_at ON archives(created_at);
CREATE INDEX IF NOT EXISTS idx_archive_tags_tag_id ON archive_tags(tag_id);
CREATE INDEX IF NOT EXISTS idx_tags_namespace ON tags(namespace);