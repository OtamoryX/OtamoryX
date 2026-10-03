-- Existing factors were trained while negative graph contributions affected recommendations.
-- Archive old dedup decisions so they cannot block learning under the new scoring semantics.
ALTER TABLE tag_relation_user_archive_feedback
RENAME TO tag_relation_user_archive_feedback_history;

CREATE TABLE tag_relation_user_archive_feedback (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    archive_id UUID NOT NULL,
    tag_a_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    tag_b_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    positive_feedback_at TIMESTAMPTZ,
    negative_feedback_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, archive_id, tag_a_id, tag_b_id),
    CHECK (tag_a_id < tag_b_id)
);

CREATE INDEX idx_tag_relation_user_archive_feedback_current_a
    ON tag_relation_user_archive_feedback (tag_a_id);
CREATE INDEX idx_tag_relation_user_archive_feedback_current_b
    ON tag_relation_user_archive_feedback (tag_b_id);

-- Reset only derived graph influence; preserve trials, outcome timestamps, and old ledger rows.
UPDATE tag_relation_user_factors
SET influence = 1.0, updated_at = CURRENT_TIMESTAMP
WHERE influence <> 1.0;
