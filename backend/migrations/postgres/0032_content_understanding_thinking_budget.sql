-- Give content understanding its own reasoning budget. Existing task overrides are preserved;
-- only the old inherited null default is materialized as the new task default.
DO $$
DECLARE
    setting_row RECORD;
    setting_value JSONB;
BEGIN
    FOR setting_row IN SELECT key, value FROM settings WHERE key = 'ai_settings' LOOP
        BEGIN
            setting_value := setting_row.value::jsonb;
            IF jsonb_typeof(setting_value) = 'object'
               AND jsonb_typeof(setting_value -> 'features') = 'object'
               AND jsonb_typeof(setting_value #> '{features,contentUnderstanding,execution}') = 'object'
               AND (
                   setting_value #> '{features,contentUnderstanding,execution,thinkingOutputTokenLimit}' IS NULL
                   OR jsonb_typeof(setting_value #> '{features,contentUnderstanding,execution,thinkingOutputTokenLimit}') = 'null'
               ) THEN
                setting_value := jsonb_set(
                    setting_value,
                    '{features,contentUnderstanding,execution,thinkingOutputTokenLimit}',
                    '8192'::jsonb,
                    true
                );
                UPDATE settings
                SET value = setting_value::text,
                    updated_at = NOW()
                WHERE key = setting_row.key;
            END IF;
        EXCEPTION WHEN OTHERS THEN
            -- A malformed unrelated settings row must not prevent application startup.
            NULL;
        END;
    END LOOP;
END $$;
