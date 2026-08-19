-- A manual model retry is intentionally bounded. Each reservation permits one request while
-- the model remains in provider cooldown, so concurrent workers cannot replay a full backlog.
ALTER TABLE ai_provider_states
ADD COLUMN IF NOT EXISTS force_attempts_remaining INTEGER NOT NULL DEFAULT 0;
