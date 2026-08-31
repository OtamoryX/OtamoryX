-- Store the application-normalized identity used by canonical theme resolution. The value is
-- populated by the canonicalization worker because SQLite cannot reproduce the worker's NFKC and
-- Unicode whitespace normalization. NULL is reserved for non-canonical legacy theme rows.
ALTER TABLE tags ADD COLUMN canonical_theme_normalized_name TEXT;

UPDATE tags
SET canonical_theme_normalized_name = (
    SELECT names.normalized_name
    FROM canonical_theme_names names
    WHERE names.theme_tag_id = tags.id
)
WHERE lower(trim(namespace)) = 'theme'
  AND EXISTS (
      SELECT 1
      FROM canonical_theme_names names
      WHERE names.theme_tag_id = tags.id
  );

CREATE UNIQUE INDEX IF NOT EXISTS idx_tags_canonical_theme_normalized_name
    ON tags (canonical_theme_normalized_name)
    WHERE lower(trim(namespace)) = 'theme'
      AND canonical_theme_normalized_name IS NOT NULL;
