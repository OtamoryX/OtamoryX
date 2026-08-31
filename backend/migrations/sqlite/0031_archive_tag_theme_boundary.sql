-- Canonical theme identities are stored through content_analysis_themes, not archive_tags.
-- Existing legacy rows are left readable; all new writes are rejected at the database boundary.

CREATE TRIGGER IF NOT EXISTS prevent_theme_archive_tag_insert
BEFORE INSERT ON archive_tags
FOR EACH ROW
WHEN EXISTS (
    SELECT 1
    FROM tags
    WHERE id = NEW.tag_id
      AND lower(trim(namespace)) = 'theme'
)
BEGIN
    SELECT RAISE(ABORT, 'system-managed theme tags cannot be stored in archive_tags');
END;

CREATE TRIGGER IF NOT EXISTS prevent_theme_archive_tag_update
BEFORE UPDATE OF tag_id ON archive_tags
FOR EACH ROW
WHEN EXISTS (
    SELECT 1
    FROM tags
    WHERE id = NEW.tag_id
      AND lower(trim(namespace)) = 'theme'
)
BEGIN
    SELECT RAISE(ABORT, 'system-managed theme tags cannot be stored in archive_tags');
END;

CREATE TRIGGER IF NOT EXISTS prevent_theme_namespace_on_archive_tag
BEFORE UPDATE OF namespace ON tags
FOR EACH ROW
WHEN lower(trim(NEW.namespace)) = 'theme'
  AND EXISTS (
      SELECT 1
      FROM archive_tags
      WHERE tag_id = NEW.id
  )
BEGIN
    SELECT RAISE(ABORT, 'a tag used by archive_tags cannot become a system-managed theme');
END;
