-- Retire compatibility-only runtime paths after the current schema is in place.
--
-- Versions 2, 10, and 22 were removed from the source tree. A database that had already
-- applied one of them must not run these statements again, while a database that skipped one
-- still needs the data/schema effect before the new runtime starts.

-- Version 2: rebuild the historical users table so databases that skipped the retired migration
-- get the nullable email constraint. Copy the value first, then normalize only databases that do
-- not have the retired migration marker; this preserves empty strings in current databases.
DROP TABLE IF EXISTS users_email_migration_tmp;

CREATE TABLE users_email_migration_tmp (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE,
    role TEXT NOT NULL DEFAULT 'user',
    password_hash TEXT NOT NULL,
    api_key TEXT UNIQUE NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO users_email_migration_tmp (
    id, username, email, role, password_hash, api_key, created_at, updated_at
)
SELECT
    id, username, email, role, password_hash, api_key, created_at, updated_at
FROM users;

DROP TABLE users;

ALTER TABLE users_email_migration_tmp RENAME TO users;

UPDATE users
SET email = NULL
WHERE email = ''
  AND NOT EXISTS (SELECT 1 FROM _sqlx_migrations WHERE version = 2);

-- Version 10: only repair records from installations that never ran that migration. New
-- executions are created after migrations complete, so current plugin activity is untouched.
UPDATE plugin_executions
SET status = 'failed',
    error_message = '旧版本只创建了执行记录，未实际调度插件。请在升级后重新执行。',
    completed_at = CURRENT_TIMESTAMP
WHERE plugin_id IN ('ehentai-metadata', 'nhentai-metadata')
  AND status IN ('pending', 'running')
  AND NOT EXISTS (
      SELECT 1 FROM _sqlx_migrations WHERE version = 10
  );

UPDATE plugins
SET execution_count = 0,
    last_executed_at = NULL,
    updated_at = CURRENT_TIMESTAMP
WHERE id IN ('ehentai-metadata', 'nhentai-metadata')
  AND NOT EXISTS (
      SELECT 1 FROM _sqlx_migrations WHERE version = 10
  );

-- Version 22: seed the fields required by the current OCR API only when the old migration was
-- skipped. Existing user values win over defaults.
INSERT OR IGNORE INTO settings (key, value, updated_at)
SELECT
    'ocr_settings',
    '{"enabled":false,"activeModelId":"ppocrv5-mobile-zh","image":{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304},"failurePolicy":{"skipUnreadablePages":true,"maxPageRetries":1}}',
    CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM settings WHERE key = 'ocr_settings')
  AND NOT EXISTS (SELECT 1 FROM _sqlx_migrations WHERE version = 22);

UPDATE settings
SET value = CASE
        WHEN json_valid(value) AND json_type(value) = 'object' THEN json_set(
            json_patch(
                '{"image":{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304},"failurePolicy":{"skipUnreadablePages":true,"maxPageRetries":1}}',
                value
            ),
            '$.image', json_patch(
                '{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304}',
                CASE
                    WHEN json_type(value, '$.image') = 'object' THEN json_extract(value, '$.image')
                    ELSE '{}'
                END
            ),
            '$.failurePolicy', json_patch(
                '{"skipUnreadablePages":true,"maxPageRetries":1}',
                CASE
                    WHEN json_type(value, '$.failurePolicy') = 'object' THEN json_extract(value, '$.failurePolicy')
                    ELSE '{}'
                END
            )
        )
        ELSE '{"enabled":false,"activeModelId":"ppocrv5-mobile-zh","image":{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304},"failurePolicy":{"skipUnreadablePages":true,"maxPageRetries":1}}'
    END,
    updated_at = CURRENT_TIMESTAMP
WHERE key = 'ocr_settings'
  AND NOT EXISTS (SELECT 1 FROM _sqlx_migrations WHERE version = 22);

-- Move the former single-profile API key into the current per-profile key before removing the
-- old storage key. A profile-specific value always wins.
INSERT OR IGNORE INTO settings (key, value, updated_at)
SELECT 'ai_connection_api_key:default', value, CURRENT_TIMESTAMP
FROM settings
WHERE key = 'ai_connection_api_key'
  AND trim(value) <> '';
UPDATE settings
SET value = (SELECT value FROM settings WHERE key = 'ai_connection_api_key'),
    updated_at = CURRENT_TIMESTAMP
WHERE key = 'ai_connection_api_key:default'
  AND trim(value) = ''
  AND EXISTS (
      SELECT 1 FROM settings
      WHERE key = 'ai_connection_api_key'
        AND trim(value) <> ''
  );
DELETE FROM settings WHERE key = 'ai_connection_api_key';

-- The suggestion/application pipeline replaced ai_generated_tags. Preserve approved associations;
-- rejected and unresolved rows are obsolete review state and are intentionally discarded.
INSERT OR IGNORE INTO archive_tags (archive_id, tag_id)
SELECT archive_id, tag_id
FROM ai_generated_tags
JOIN archives ON archives.id = ai_generated_tags.archive_id
JOIN tags ON tags.id = ai_generated_tags.tag_id
WHERE approved = 1 AND tag_id IS NOT NULL;
DROP TABLE IF EXISTS ai_generated_tags;

-- Remove fields that were only emitted by the retired AI settings contract. Current task and
-- profile values already live in their canonical nested objects.
UPDATE settings
SET value = json_remove(
        value,
        '$.settings_version',
        '$.settingsVersion',
        '$.execution.maxConcurrentTasks',
        '$.execution.max_concurrent_tasks',
        '$.connection.ollamaAutoNumCtx',
        '$.connection.ollama_auto_num_ctx',
        '$.features.titleTranslation.temperature',
        '$.features.titleTranslation.ollamaRepeatPenalty',
        '$.features.titleTranslation.ollama_repeat_penalty',
        '$.features.titleTranslation.ollamaRepeatLastN',
        '$.features.titleTranslation.ollama_repeat_last_n',
        '$.features.titleTranslation.structuredOutputMode',
        '$.features.titleTranslation.structured_output_mode',
        '$.features.tagLocalization.execution.additionalInstructions',
        '$.features.tagLocalization.execution.additional_instructions',
        '$.features.tag_localization.execution.additionalInstructions',
        '$.features.tag_localization.execution.additional_instructions'
    ),
    updated_at = CURRENT_TIMESTAMP
WHERE key = 'ai_settings'
  AND json_valid(value)
  AND json_type(value) = 'object'
  AND json_type(value, '$.connection') = 'object';
