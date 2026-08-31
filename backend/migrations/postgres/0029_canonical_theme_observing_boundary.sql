-- Canonical themes are observation-only identities. Disable any learned rule produced by an
-- earlier runtime and normalize its candidate state before the new learner starts.
UPDATE preference_rule_candidates
SET feature_kind = 'canonical_theme_observing',
    evidence_state = 'observing',
    status = 'observing',
    updated_at = NOW()
WHERE source = 'cold_start_v1'
  AND (
      conditions_json::text LIKE '%"feature":"theme:%'
      OR conditions_json::text LIKE '%"feature": "theme:%'
  );

UPDATE preference_rules
SET enabled = FALSE,
    updated_at = NOW()
WHERE source = 'learned_cold_start'
  AND (
      conditions_json::text LIKE '%"feature":"theme:%'
      OR conditions_json::text LIKE '%"feature": "theme:%'
  );
