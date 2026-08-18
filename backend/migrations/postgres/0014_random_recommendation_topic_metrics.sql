-- P8-5: immutable topic snapshots and experiment attribution for random recommendations.
ALTER TABLE random_recommendation_sessions
    ADD COLUMN IF NOT EXISTS algorithm_variant TEXT NOT NULL DEFAULT 'weighted-v1';
ALTER TABLE random_recommendation_sessions
    ADD COLUMN IF NOT EXISTS candidate_topics_json JSONB NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE random_recommendation_sessions
    ADD COLUMN IF NOT EXISTS exploration_topics_json JSONB NOT NULL DEFAULT '[]'::jsonb;

ALTER TABLE random_recommendation_items
    ADD COLUMN IF NOT EXISTS topics_json JSONB NOT NULL DEFAULT '[]'::jsonb;

CREATE INDEX IF NOT EXISTS idx_random_recommendation_sessions_variant_created
    ON random_recommendation_sessions(user_id, algorithm_variant, created_at);
