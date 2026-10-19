-- Record which recommendation cards actually entered the carousel viewport. The first screen is
-- assumed seen, but cards further right only count after the client reports them as visible.
ALTER TABLE random_recommendation_items
    ADD COLUMN IF NOT EXISTS visible_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_random_recommendation_items_visible
    ON random_recommendation_items (session_id, position);
