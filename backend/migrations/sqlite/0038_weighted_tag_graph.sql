-- Provider-neutral signed relation cache. Existing categorical semantic observations remain
-- available for read-only inspection and are never converted into these numeric weights.
CREATE TABLE IF NOT EXISTS tag_relation_weight_edges (
    tag_a_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    tag_b_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    signed_weight REAL NOT NULL,
    confidence REAL,
    score_a_to_b REAL,
    score_b_to_a REAL,
    input_hash TEXT NOT NULL,
    scorer_version TEXT NOT NULL,
    profile_id TEXT,
    provider TEXT,
    model TEXT,
    status TEXT NOT NULL DEFAULT 'observing',
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (tag_a_id, tag_b_id),
    CHECK (tag_a_id < tag_b_id),
    CHECK (signed_weight >= -1.0 AND signed_weight <= 1.0),
    CHECK (confidence IS NULL OR (confidence >= 0.0 AND confidence <= 1.0)),
    CHECK (score_a_to_b IS NULL OR (score_a_to_b >= -1.0 AND score_a_to_b <= 1.0)),
    CHECK (score_b_to_a IS NULL OR (score_b_to_a >= -1.0 AND score_b_to_a <= 1.0)),
    CHECK (trim(input_hash) <> ''),
    CHECK (trim(scorer_version) <> ''),
    CHECK (status IN ('observing', 'active', 'rejected'))
);

CREATE INDEX IF NOT EXISTS idx_tag_relation_weight_edges_a
    ON tag_relation_weight_edges (tag_a_id, status, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_tag_relation_weight_edges_b
    ON tag_relation_weight_edges (tag_b_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS tag_relation_user_factors (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tag_a_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    tag_b_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    influence REAL NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, tag_a_id, tag_b_id),
    CHECK (tag_a_id < tag_b_id),
    CHECK (influence >= 0.0 AND influence <= 1.0)
);

-- Retains per-archive outcome deduplication after recommendation sessions expire or are removed.
-- archive_id intentionally has no foreign key so deleting an archive cannot erase this ledger.
CREATE TABLE IF NOT EXISTS tag_relation_user_archive_feedback (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    archive_id TEXT NOT NULL,
    tag_a_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    tag_b_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    positive_feedback_at DATETIME,
    negative_feedback_at DATETIME,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, archive_id, tag_a_id, tag_b_id),
    CHECK (tag_a_id < tag_b_id)
);

CREATE INDEX IF NOT EXISTS idx_tag_relation_user_archive_feedback_a
    ON tag_relation_user_archive_feedback (tag_a_id);
CREATE INDEX IF NOT EXISTS idx_tag_relation_user_archive_feedback_b
    ON tag_relation_user_archive_feedback (tag_b_id);

-- The 0.3 gain is the explicitly provisional starting value from the reviewed proposal.
-- Activation stays off until numeric edge quality is evaluated on a fixed holdout.
CREATE TABLE IF NOT EXISTS tag_weighted_graph_policy (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
    global_gain REAL NOT NULL DEFAULT 0.3 CHECK (global_gain >= 0.0 AND global_gain <= 1.0),
    version INTEGER NOT NULL DEFAULT 0,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO tag_weighted_graph_policy (id, enabled, global_gain)
VALUES (1, 0, 0.3)
ON CONFLICT (id) DO NOTHING;

CREATE TABLE IF NOT EXISTS random_recommendation_graph_trials (
    id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL REFERENCES random_recommendation_items(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tag_a_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    tag_b_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    signed_contribution REAL NOT NULL,
    source_archive_ids_json TEXT NOT NULL,
    positive_feedback_at DATETIME,
    negative_feedback_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (item_id, tag_a_id, tag_b_id),
    CHECK (tag_a_id < tag_b_id),
    CHECK (signed_contribution >= -1.0 AND signed_contribution <= 1.0),
    CHECK (json_valid(source_archive_ids_json))
);

CREATE INDEX IF NOT EXISTS idx_random_recommendation_graph_trials_item
    ON random_recommendation_graph_trials (item_id, user_id);
