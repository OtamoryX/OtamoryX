-- Give content understanding its own reasoning budget. Existing task overrides are preserved;
-- only the old inherited null default is materialized as the new task default.
UPDATE settings
SET value = json_set(
        value,
        '$.features.contentUnderstanding.execution.thinkingOutputTokenLimit',
        8192
    ),
    updated_at = CURRENT_TIMESTAMP
WHERE key = 'ai_settings'
  AND json_valid(value)
  AND json_type(value) = 'object'
  AND json_type(value, '$.features.contentUnderstanding.execution') = 'object'
  AND (
      json_type(value, '$.features.contentUnderstanding.execution.thinkingOutputTokenLimit') IS NULL
      OR json_type(value, '$.features.contentUnderstanding.execution.thinkingOutputTokenLimit') = 'null'
  );
