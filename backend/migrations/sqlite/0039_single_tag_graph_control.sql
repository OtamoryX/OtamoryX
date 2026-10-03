ALTER TABLE tag_weighted_graph_policy DROP COLUMN enabled;

-- Existing active rows came from the experimental manual-approval lifecycle. Keep their scores
-- for diagnostics, but require new results to pass the current automatic activation checks.
UPDATE tag_relation_weight_edges
SET status = 'observing', revision = revision + 1, updated_at = CURRENT_TIMESTAMP
WHERE status = 'active';

UPDATE settings
SET value = json_remove(
        json_set(
            value,
            '$.features.recommendations.tagGraphEnabled',
            CASE
                WHEN json_type(value, '$.features.recommendations.tagGraphEnabled') = 'false'
                    THEN json('false')
                ELSE json('true')
            END
        ),
        '$.features.recommendations.tagRelation.enabled'
    ),
    updated_at = CURRENT_TIMESTAMP
WHERE key = 'ai_settings'
  AND json_valid(value)
  AND json_type(value) = 'object'
  AND json_type(value, '$.features.recommendations') = 'object';
