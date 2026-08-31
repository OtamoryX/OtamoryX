-- Store the application-normalized identity used by canonical theme resolution. The value is
-- populated by the canonicalization worker because PostgreSQL cannot reproduce the worker's NFKC
-- and Unicode whitespace normalization without an application-specific extension.
ALTER TABLE tags ADD COLUMN IF NOT EXISTS canonical_theme_normalized_name TEXT;

UPDATE tags
SET canonical_theme_normalized_name = (
    SELECT names.normalized_name
    FROM canonical_theme_names names
    WHERE names.theme_tag_id = tags.id
)
WHERE lower(btrim(namespace)) = 'theme'
  AND EXISTS (
      SELECT 1
      FROM canonical_theme_names names
      WHERE names.theme_tag_id = tags.id
  );

CREATE UNIQUE INDEX IF NOT EXISTS idx_tags_canonical_theme_normalized_name
    ON tags (canonical_theme_normalized_name)
    WHERE lower(btrim(namespace)) = 'theme'
      AND canonical_theme_normalized_name IS NOT NULL;
