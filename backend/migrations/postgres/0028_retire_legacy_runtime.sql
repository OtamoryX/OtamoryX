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

DO $$
DECLARE
    setting_row RECORD;
    setting_value JSONB;
    default_value CONSTANT JSONB := '{"enabled":false,"activeModelId":"ppocrv5-mobile-zh","image":{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304},"failurePolicy":{"skipUnreadablePages":true,"maxPageRetries":1}}'::jsonb;
    base_value CONSTANT JSONB := '{"image":{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304},"failurePolicy":{"skipUnreadablePages":true,"maxPageRetries":1}}'::jsonb;
    default_image CONSTANT JSONB := '{"targetLongEdge":2048,"preferredDecodeBytes":100663296,"jpegQuality":86,"maxOutputBytes":2097152,"largeImageLongEdge":2560,"largeImageDecodeBytes":268435456,"largeImageJpegQuality":88,"largeImageMaxOutputBytes":4194304}'::jsonb;
    default_failure_policy CONSTANT JSONB := '{"skipUnreadablePages":true,"maxPageRetries":1}'::jsonb;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM _sqlx_migrations WHERE version = 22) THEN
        FOR setting_row IN SELECT key, value FROM settings WHERE key = 'ocr_settings' LOOP
            BEGIN
                setting_value := setting_row.value::jsonb;
                IF jsonb_typeof(setting_value) = 'object' THEN
                    setting_value := jsonb_set(
                        jsonb_set(
                            base_value || setting_value,
                            '{image}',
                            default_image || CASE
                                WHEN jsonb_typeof(setting_value -> 'image') = 'object' THEN setting_value -> 'image'
                                ELSE '{}'::jsonb
                            END
                        ),
                        '{failurePolicy}',
                        default_failure_policy || CASE
                            WHEN jsonb_typeof(setting_value -> 'failurePolicy') = 'object' THEN setting_value -> 'failurePolicy'
                            ELSE '{}'::jsonb
                        END
                    );
                ELSE
                    setting_value := default_value;
                END IF;
                UPDATE settings
                SET value = setting_value::text, updated_at = NOW()
                WHERE key = setting_row.key;
            EXCEPTION WHEN OTHERS THEN
                UPDATE settings
                SET value = default_value::text, updated_at = NOW()
                WHERE key = setting_row.key;
            END;
        END LOOP;
    END IF;
END $$;

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
DO $$
DECLARE
    setting_row RECORD;
    setting_value JSONB;
BEGIN
    FOR setting_row IN SELECT key, value FROM settings WHERE key = 'ai_settings' LOOP
        BEGIN
            setting_value := setting_row.value::jsonb;
            IF jsonb_typeof(setting_value) = 'object'
               AND jsonb_typeof(setting_value -> 'connection') = 'object' THEN
                setting_value := setting_value - 'settings_version' - 'settingsVersion';
                setting_value := setting_value #- ARRAY['execution', 'maxConcurrentTasks'];
                setting_value := setting_value #- ARRAY['execution', 'max_concurrent_tasks'];
                setting_value := setting_value #- ARRAY['connection', 'ollamaAutoNumCtx'];
                setting_value := setting_value #- ARRAY['connection', 'ollama_auto_num_ctx'];
                setting_value := setting_value #- ARRAY['features', 'titleTranslation', 'temperature'];
                setting_value := setting_value #- ARRAY['features', 'titleTranslation', 'ollamaRepeatPenalty'];
                setting_value := setting_value #- ARRAY['features', 'titleTranslation', 'ollama_repeat_penalty'];
                setting_value := setting_value #- ARRAY['features', 'titleTranslation', 'ollamaRepeatLastN'];
                setting_value := setting_value #- ARRAY['features', 'titleTranslation', 'ollama_repeat_last_n'];
                setting_value := setting_value #- ARRAY['features', 'titleTranslation', 'structuredOutputMode'];
                setting_value := setting_value #- ARRAY['features', 'titleTranslation', 'structured_output_mode'];
                setting_value := setting_value #- ARRAY['features', 'tagLocalization', 'execution', 'additionalInstructions'];
                setting_value := setting_value #- ARRAY['features', 'tagLocalization', 'execution', 'additional_instructions'];
                setting_value := setting_value #- ARRAY['features', 'tag_localization', 'execution', 'additionalInstructions'];
                setting_value := setting_value #- ARRAY['features', 'tag_localization', 'execution', 'additional_instructions'];
                UPDATE settings
                SET value = setting_value::text, updated_at = NOW()
                WHERE key = setting_row.key;
            END IF;
        EXCEPTION WHEN OTHERS THEN
            NULL;
        END;
    END LOOP;
END $$;
