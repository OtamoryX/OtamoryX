ALTER TABLE tag_weighted_graph_policy DROP COLUMN enabled;

-- Existing active rows came from the experimental manual-approval lifecycle. Keep their scores
-- for diagnostics, but require new results to pass the current automatic activation checks.
UPDATE tag_relation_weight_edges
SET status = 'observing', revision = revision + 1, updated_at = NOW()
WHERE status = 'active';

DO $$
DECLARE
    setting_row RECORD;
    setting_value JSONB;
    recommendations JSONB;
    relation_settings JSONB;
BEGIN
    FOR setting_row IN SELECT key, value FROM settings WHERE key = 'ai_settings' LOOP
        BEGIN
            setting_value := setting_row.value::jsonb;
            recommendations := setting_value #> '{features,recommendations}';
            IF jsonb_typeof(recommendations) = 'object' THEN
                IF jsonb_typeof(recommendations -> 'tagGraphEnabled') <> 'boolean'
                   OR recommendations -> 'tagGraphEnabled' IS NULL THEN
                    recommendations := jsonb_set(
                        recommendations,
                        '{tagGraphEnabled}',
                        'true'::jsonb,
                        true
                    );
                END IF;
                relation_settings := recommendations -> 'tagRelation';
                IF jsonb_typeof(relation_settings) = 'object' THEN
                    recommendations := jsonb_set(
                        recommendations,
                        '{tagRelation}',
                        relation_settings - 'enabled',
                        true
                    );
                END IF;
                setting_value := jsonb_set(
                    setting_value,
                    '{features,recommendations}',
                    recommendations,
                    true
                );
                UPDATE settings
                SET value = setting_value::text, updated_at = NOW()
                WHERE key = setting_row.key;
            END IF;
        EXCEPTION WHEN OTHERS THEN
            -- A malformed unrelated settings row must not prevent application startup.
            NULL;
        END;
    END LOOP;
END $$;
