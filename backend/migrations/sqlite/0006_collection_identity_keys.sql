ALTER TABLE archive_identity_facts ADD COLUMN raw_number TEXT;
ALTER TABLE archive_identity_facts ADD COLUMN content_unit_key TEXT;
CREATE INDEX IF NOT EXISTS idx_identity_content_unit_key ON archive_identity_facts(content_unit_key);
