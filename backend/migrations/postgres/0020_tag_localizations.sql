-- Canonical tag identity remains in `tags`. Localized names are display/search data and must
-- never create a second archive_tags association for the same semantic tag.
CREATE TABLE IF NOT EXISTS tag_localizations (
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    locale VARCHAR(32) NOT NULL,
    name VARCHAR(255),
    status VARCHAR(32) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'completed', 'failed')),
    source VARCHAR(32) NOT NULL DEFAULT 'llm' CHECK (source IN ('llm', 'manual')),
    provider VARCHAR(255),
    model VARCHAR(255),
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    PRIMARY KEY (tag_id, locale)
);

CREATE INDEX IF NOT EXISTS idx_tag_localizations_locale_name
    ON tag_localizations(locale, name);
