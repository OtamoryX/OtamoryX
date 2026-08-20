-- Canonical tag identity remains in `tags`. Localized names are display/search data and must
-- never create a second archive_tags association for the same semantic tag.
CREATE TABLE IF NOT EXISTS tag_localizations (
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    locale TEXT NOT NULL,
    name TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'completed', 'failed')),
    source TEXT NOT NULL DEFAULT 'llm' CHECK (source IN ('llm', 'manual')),
    provider TEXT,
    model TEXT,
    last_error TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    PRIMARY KEY (tag_id, locale)
);

CREATE INDEX IF NOT EXISTS idx_tag_localizations_locale_name
    ON tag_localizations(locale, name);
