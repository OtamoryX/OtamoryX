-- The first release exposed official external manifests before there was an external runtime.
-- Make stale records honest rather than leaving permanent "pending" executions in the UI.
UPDATE plugin_executions
SET status = 'failed',
    error_message = '旧版本只创建了执行记录，未实际调度插件。请在升级后重新执行。',
    completed_at = CURRENT_TIMESTAMP
WHERE plugin_id IN ('ehentai-metadata', 'nhentai-metadata')
  AND status IN ('pending', 'running');

UPDATE plugins
SET execution_count = 0,
    last_executed_at = NULL,
    updated_at = CURRENT_TIMESTAMP
WHERE id IN ('ehentai-metadata', 'nhentai-metadata');
