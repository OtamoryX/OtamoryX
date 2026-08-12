ALTER TABLE collections ADD COLUMN subtitle TEXT;

CREATE TABLE IF NOT EXISTS version_group_decisions (
    group_key TEXT PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'active',
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
