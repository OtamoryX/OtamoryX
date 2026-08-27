-- Keep provider availability separate from task quality retries. The application encodes the
-- physical endpoint into the provider-state model key so profiles sharing one endpoint share a
-- breaker while different endpoints remain independent.
ALTER TABLE ai_provider_states
ADD COLUMN IF NOT EXISTS failure_count INTEGER NOT NULL DEFAULT 0;

ALTER TABLE ai_provider_states
ADD COLUMN IF NOT EXISTS probe_reserved_until TIMESTAMPTZ;

-- Before this migration the model key included the active profile ID. Those cooldowns are
-- transient and cannot be safely mapped to an endpoint/model key without the live settings JSON;
-- discard them once so the new breaker starts with a coherent physical-model identity.
DELETE FROM ai_provider_states;
