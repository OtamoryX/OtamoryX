-- Retire compatibility-only runtime paths after the current schema is in place.
-- Versions 2, 10, and 22 were removed from the source tree. A database that had already
-- applied one of them must not run these statements again, while a database that skipped one
-- still needs the data/schema effect before the new runtime starts.

-- Version 2: the current base schema is already nullable, so the remaining durable effect is
-- normalizing empty values. DROP NOT NULL is harmless for databases that already ran version 2.
ALTER TABLE users ALTER COLUMN email DROP NOT NULL;
UPDATE users
SET email = NULL
WHERE email = ''
  AND NOT EXISTS (SELECT 1 FROM _sqlx_migrations WHERE version = 2);

-- Version 10: only repair records from installations that never ran that migration. New
-- executions are created after migrations complete, so current plugin activity is untouched.
UPDATE plugin_executions
SET status = 'failed',
    error_message = '旧版本只创建了执行记录，未实际调度插件。请在升级后重新执行。',
    completed_at = NOW()
WHERE plugin_id IN ('ehentai-metadata', 'nhentai-metadata')
  AND status IN ('pending', 'running')
  AND NOT EXISTS (
      SELECT 1 FROM _sqlx_migrations WHERE version = 10
  );

UPDATE plugins
SET execution_count = 0,
    last_executed_at = NULL,
    updated_at = NOW()
WHERE id IN ('ehentai-metadata', 'nhentai-metadata')
  AND NOT EXISTS (
      SELECT 1 FROM _sqlx_migrations WHERE version = 10
  );

-- Version 22: seed the fields required by the current OCR API only when the old migration was
-- skipped. Existing user values win over defaults.
INSERT INTO settings (key, value, updated_at)
SELECT
    'ocr_settings',
    '{"enabled":false,"activeModelId":"ppocrv5-mobile-zh","image":{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304},"failurePolicy":{"skipUnreadablePages":true,"maxPageRetries":1}}',
    NOW()
WHERE NOT EXISTS (SELECT 1 FROM settings WHERE key = 'ocr_settings')
  AND NOT EXISTS (SELECT 1 FROM _sqlx_migrations WHERE version = 22)
ON CONFLICT (key) DO NOTHING;

UPDATE settings
SET value = CASE
        WHEN jsonb_typeof(value::jsonb) = 'object' THEN (
            jsonb_set(
                jsonb_set(
                    '{"image":{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304},"failurePolicy":{"skipUnreadablePages":true,"maxPageRetries":1}}'::jsonb
                    || value::jsonb,
                    '{image}',
                    '{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304}'::jsonb
                    || CASE
                        WHEN jsonb_typeof(value::jsonb -> 'image') = 'object' THEN value::jsonb -> 'image'
                        ELSE '{}'::jsonb
                    END
                ),
                '{failurePolicy}',
                '{"skipUnreadablePages":true,"maxPageRetries":1}'::jsonb
                || CASE
                    WHEN jsonb_typeof(value::jsonb -> 'failurePolicy') = 'object' THEN value::jsonb -> 'failurePolicy'
                    ELSE '{}'::jsonb
                END
            )
        )::text
        ELSE '{"enabled":false,"activeModelId":"ppocrv5-mobile-zh","image":{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304},"failurePolicy":{"skipUnreadablePages":true,"maxPageRetries":1}}'
    END,
    updated_at = NOW()
WHERE key = 'ocr_settings'
  AND NOT EXISTS (SELECT 1 FROM _sqlx_migrations WHERE version = 22);

-- Move the former single-profile API key into the current per-profile key before removing the
-- old storage key. A profile-specific value always wins.
INSERT INTO settings (key, value, updated_at)
SELECT 'ai_connection_api_key:default', value, NOW()
FROM settings
WHERE key = 'ai_connection_api_key'
  AND btrim(value) <> ''
ON CONFLICT (key) DO UPDATE
SET value = EXCLUDED.value,
    updated_at = EXCLUDED.updated_at
WHERE btrim(settings.value) = '';
DELETE FROM settings WHERE key = 'ai_connection_api_key';

-- The suggestion/application pipeline replaced ai_generated_tags. Preserve approved associations;
-- rejected and unresolved rows are obsolete review state and are intentionally discarded.
INSERT INTO archive_tags (archive_id, tag_id)
SELECT archive_id, tag_id
FROM ai_generated_tags
JOIN archives ON archives.id = ai_generated_tags.archive_id
JOIN tags ON tags.id = ai_generated_tags.tag_id
WHERE approved IS TRUE AND tag_id IS NOT NULL
ON CONFLICT DO NOTHING;
DROP TABLE IF EXISTS ai_generated_tags;

-- Remove fields that were only emitted by the retired AI settings contract. Current task and
-- profile values already live in their canonical nested objects.
UPDATE settings
SET value = (
        value::jsonb
        - 'settings_version'
        - 'settingsVersion'
        #- ARRAY[
            'execution', 'maxConcurrentTasks'
        ]
        #- ARRAY[
            'execution', 'max_concurrent_tasks'
        ]
        #- ARRAY[
            'connection', 'ollamaAutoNumCtx'
        ]
        #- ARRAY[
            'connection', 'ollama_auto_num_ctx'
        ]
        #- ARRAY[
            'features', 'titleTranslation', 'temperature'
        ]
        #- ARRAY[
            'features', 'titleTranslation', 'ollamaRepeatPenalty'
        ]
        #- ARRAY[
            'features', 'titleTranslation', 'ollama_repeat_penalty'
        ]
        #- ARRAY[
            'features', 'titleTranslation', 'ollamaRepeatLastN'
        ]
        #- ARRAY[
            'features', 'titleTranslation', 'ollama_repeat_last_n'
        ]
        #- ARRAY[
            'features', 'titleTranslation', 'structuredOutputMode'
        ]
        #- ARRAY[
            'features', 'titleTranslation', 'structured_output_mode'
        ]
        #- ARRAY[
            'features', 'tagLocalization', 'execution', 'additionalInstructions'
        ]
        #- ARRAY[
            'features', 'tagLocalization', 'execution', 'additional_instructions'
        ]
        #- ARRAY[
            'features', 'tag_localization', 'execution', 'additionalInstructions'
        ]
        #- ARRAY[
            'features', 'tag_localization', 'execution', 'additional_instructions'
        ]
    )::text,
    updated_at = NOW()
WHERE key = 'ai_settings'
  AND jsonb_typeof(value::jsonb) = 'object'
  AND jsonb_typeof(value::jsonb -> 'connection') = 'object';
