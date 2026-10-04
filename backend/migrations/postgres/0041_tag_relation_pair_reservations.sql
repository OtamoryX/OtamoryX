-- Map one canonical scoring input/version to the existing durable batch job.
-- Queue attempts, leases and retries remain owned by ai_processing_queue.
CREATE TABLE IF NOT EXISTS tag_relation_pair_reservations (
    tag_a_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    tag_b_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    input_hash TEXT NOT NULL,
    scorer_version TEXT NOT NULL,
    queue_job_id UUID REFERENCES ai_processing_queue(id) ON DELETE CASCADE,
    PRIMARY KEY (tag_a_id, tag_b_id, input_hash, scorer_version),
    CHECK (tag_a_id < tag_b_id),
    CHECK (trim(input_hash) <> ''),
    CHECK (trim(scorer_version) <> '')
);

CREATE INDEX IF NOT EXISTS idx_tag_relation_pair_reservations_job
    ON tag_relation_pair_reservations (queue_job_id);

CREATE TABLE IF NOT EXISTS tag_relation_pair_reservation_bootstrap (
    queue_job_id UUID NOT NULL REFERENCES ai_processing_queue(id) ON DELETE CASCADE,
    scorer_version TEXT NOT NULL,
    PRIMARY KEY (queue_job_id, scorer_version),
    CHECK (trim(scorer_version) <> '')
);
