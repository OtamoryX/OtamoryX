-- A title can be classified independently from a translation. Keeping the decision by source
-- hash makes batch model checks reusable and prevents a title from being checked repeatedly.
CREATE TABLE IF NOT EXISTS archive_title_language_detections (
    archive_id TEXT NOT NULL,
    target_language TEXT NOT NULL,
    source_hash TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    is_target_language BOOLEAN,
    decision_source TEXT,
    last_error TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    PRIMARY KEY (archive_id, target_language, source_hash),
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_title_language_detections_ready
    ON archive_title_language_detections (target_language, status, created_at);
