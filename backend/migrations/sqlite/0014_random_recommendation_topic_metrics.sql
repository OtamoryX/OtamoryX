-- P8-5: immutable topic snapshots and experiment attribution for random recommendations.
ALTER TABLE random_recommendation_sessions ADD COLUMN algorithm_variant TEXT NOT NULL DEFAULT 'weighted-v1';
ALTER TABLE random_recommendation_sessions ADD COLUMN candidate_topics_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE random_recommendation_sessions ADD COLUMN exploration_topics_json TEXT NOT NULL DEFAULT '[]';

ALTER TABLE random_recommendation_items ADD COLUMN topics_json TEXT NOT NULL DEFAULT '[]';

CREATE INDEX IF NOT EXISTS idx_random_recommendation_sessions_variant_created
    ON random_recommendation_sessions(user_id, algorithm_variant, created_at);
