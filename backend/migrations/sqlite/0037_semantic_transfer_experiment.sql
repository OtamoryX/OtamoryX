ALTER TABLE random_recommendation_sessions
    ADD COLUMN semantic_arm TEXT;
ALTER TABLE random_recommendation_sessions
    ADD COLUMN semantic_weight REAL;
ALTER TABLE random_recommendation_sessions
    ADD COLUMN semantic_policy_version INTEGER;
ALTER TABLE random_recommendation_sessions
    ADD COLUMN semantic_eligible_count INTEGER NOT NULL DEFAULT 0;

ALTER TABLE random_recommendation_items
    ADD COLUMN semantic_edge_a_id TEXT;
ALTER TABLE random_recommendation_items
    ADD COLUMN semantic_edge_b_id TEXT;
ALTER TABLE random_recommendation_items
    ADD COLUMN semantic_base_weight REAL;
ALTER TABLE random_recommendation_items
    ADD COLUMN semantic_bonus REAL NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS semantic_transfer_policy (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    weight REAL NOT NULL DEFAULT 0.05 CHECK (weight >= 0.0 AND weight <= 0.3),
    reviewed_at DATETIME,
    version INTEGER NOT NULL DEFAULT 0
);

INSERT INTO semantic_transfer_policy (id, weight)
VALUES (1, 0.05)
ON CONFLICT (id) DO NOTHING;

CREATE INDEX IF NOT EXISTS idx_random_recommendation_items_semantic_edge_a_created
    ON random_recommendation_items (semantic_edge_a_id, created_at);
CREATE INDEX IF NOT EXISTS idx_random_recommendation_items_semantic_edge_b_created
    ON random_recommendation_items (semantic_edge_b_id, created_at);
