ALTER TABLE archive_identity_facts ADD COLUMN IF NOT EXISTS raw_number TEXT;
ALTER TABLE archive_identity_facts ADD COLUMN IF NOT EXISTS content_unit_key TEXT;
CREATE INDEX IF NOT EXISTS idx_identity_content_unit_key ON archive_identity_facts(content_unit_key);
