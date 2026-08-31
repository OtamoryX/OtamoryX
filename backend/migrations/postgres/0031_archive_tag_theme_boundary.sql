-- Canonical theme identities are stored through content_analysis_themes, not archive_tags.
-- Existing legacy rows are left readable; all new writes are rejected at the database boundary.

CREATE OR REPLACE FUNCTION prevent_theme_archive_tag_relation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_TABLE_NAME = 'archive_tags' THEN
        IF EXISTS (
            SELECT 1
            FROM tags
            WHERE id = NEW.tag_id
              AND lower(btrim(namespace)) = 'theme'
        ) THEN
            RAISE EXCEPTION 'system-managed theme tags cannot be stored in archive_tags';
        END IF;
    ELSIF lower(btrim(NEW.namespace)) = 'theme'
      AND EXISTS (
          SELECT 1
          FROM archive_tags
          WHERE tag_id = NEW.id
      ) THEN
        RAISE EXCEPTION 'a tag used by archive_tags cannot become a system-managed theme';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS prevent_theme_archive_tag_insert_or_update ON archive_tags;
CREATE TRIGGER prevent_theme_archive_tag_insert_or_update
BEFORE INSERT OR UPDATE OF tag_id ON archive_tags
FOR EACH ROW
EXECUTE FUNCTION prevent_theme_archive_tag_relation();

DROP TRIGGER IF EXISTS prevent_theme_namespace_on_archive_tag ON tags;
CREATE TRIGGER prevent_theme_namespace_on_archive_tag
BEFORE UPDATE OF namespace ON tags
FOR EACH ROW
EXECUTE FUNCTION prevent_theme_archive_tag_relation();
