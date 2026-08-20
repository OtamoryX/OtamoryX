-- Tag localization is global metadata and therefore has no archive owner.
ALTER TABLE ai_processing_queue
    ALTER COLUMN archive_id DROP NOT NULL;
