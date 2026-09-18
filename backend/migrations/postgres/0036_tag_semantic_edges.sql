-- JEV Alpha Decisions observations for ordinary tag pairs.
-- This cache is deliberately independent from deterministic co-occurrence edges and never
-- rewrites archive_tags, aliases, profiles, or preference evidence.
CREATE TABLE IF NOT EXISTS tag_semantic_edges (
    tag_a_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    tag_b_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    relation_kind TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'observing',
    forward_choice TEXT,
    reverse_choice TEXT,
    selected_confidence DOUBLE PRECISION,
    forward_confidence DOUBLE PRECISION,
    reverse_confidence DOUBLE PRECISION,
    forward_probabilities_json TEXT NOT NULL DEFAULT '{}',
    reverse_probabilities_json TEXT NOT NULL DEFAULT '{}',
    pair_input_hash TEXT NOT NULL,
    candidate_algorithm_version TEXT NOT NULL,
    protocol_version TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    schema_version TEXT NOT NULL,
    profile_id TEXT,
    provider TEXT,
    model TEXT,
    input_tokens INTEGER,
    output_tokens INTEGER,
    total_tokens INTEGER,
    cost_usd DOUBLE PRECISION,
    latency_ms INTEGER,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    next_attempt_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (tag_a_id, tag_b_id),
    CHECK (tag_a_id < tag_b_id),
    CHECK (relation_kind IN ('same_meaning', 'related_nonreplaceable', 'broader_or_narrower', 'uncertain')),
    CHECK (status IN ('observing', 'published', 'uncertain', 'failed')),
    CHECK (forward_choice IS NULL OR forward_choice IN ('same_meaning', 'related_nonreplaceable', 'broader_or_narrower', 'unrelated', 'uncertain')),
    CHECK (reverse_choice IS NULL OR reverse_choice IN ('same_meaning', 'related_nonreplaceable', 'broader_or_narrower', 'unrelated', 'uncertain')),
    CHECK (selected_confidence IS NULL OR (selected_confidence >= 0.0 AND selected_confidence <= 1.0)),
    CHECK (forward_confidence IS NULL OR (forward_confidence >= 0.0 AND forward_confidence <= 1.0)),
    CHECK (reverse_confidence IS NULL OR (reverse_confidence >= 0.0 AND reverse_confidence <= 1.0)),
    CHECK (input_tokens IS NULL OR input_tokens >= 0),
    CHECK (output_tokens IS NULL OR output_tokens >= 0),
    CHECK (total_tokens IS NULL OR total_tokens >= 0),
    CHECK (cost_usd IS NULL OR cost_usd >= 0.0),
    CHECK (latency_ms IS NULL OR latency_ms >= 0),
    CHECK (attempts >= 0),
    CHECK (trim(pair_input_hash) <> ''),
    CHECK (trim(candidate_algorithm_version) <> ''),
    CHECK (trim(protocol_version) <> ''),
    CHECK (trim(prompt_version) <> ''),
    CHECK (trim(schema_version) <> '')
);

CREATE INDEX IF NOT EXISTS idx_tag_semantic_edges_a
    ON tag_semantic_edges (tag_a_id, status, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_tag_semantic_edges_b
    ON tag_semantic_edges (tag_b_id, status, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_tag_semantic_edges_status
    ON tag_semantic_edges (status, updated_at DESC);
