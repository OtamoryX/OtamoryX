//! Positive ordinary-tag interest seeds from existing cold-start learner state.

use anyhow::Result;
use serde_json::Value;
use sqlx::{Pool, Row, Sqlite};
use std::collections::HashMap;

use crate::services::preferences::learning::{condition_feature_key, condition_is_ordinary_tag};
use crate::services::recommendations::namespace_policy::load_metadata_namespace_set;

// Provisional values from the fixed-tag replay proposal: prior=8, half-baseline correction,
// and a 0.5 score cap. The replay establishes bounded response, not an optimal calibration.
const TAG_SEED_PRIOR: f64 = 8.0;
const BASELINE_CALIBRATION: f64 = 0.5;
const TAG_SEED_CAP: f64 = 0.5;

pub async fn positive_tag_seeds(
    pool: &Pool<Sqlite>,
    user_id: &str,
) -> Result<HashMap<String, f64>> {
    let metadata = load_metadata_namespace_set(pool).await?;
    let rows = sqlx::query(
        "SELECT c.conditions_json, c.feature_kind, c.status, c.evidence_state,
                c.positive_support, c.negative_support, c.evidence_json, r.action
         FROM preference_rule_candidates c
         LEFT JOIN preference_rules r ON r.user_id = c.user_id
             AND r.conditions_json = c.conditions_json AND r.source = 'learned_cold_start'
             AND r.enabled = 1 AND r.auto_paused = 0
         WHERE c.user_id = ? AND c.source = 'cold_start_v1'
           AND ((c.status = 'observing'
                 AND c.evidence_state IN ('observing', 'insufficient_evidence'))
             OR (c.status = 'promoted' AND c.evidence_state = 'eligible' AND r.action = 'keep'))",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let mut identities = HashMap::new();
    for row in rows {
        if row.get::<Option<String>, _>("feature_kind").as_deref() != Some("binary") {
            continue;
        }
        let Ok(condition) = serde_json::from_str::<Value>(&row.get::<String, _>("conditions_json"))
        else {
            continue;
        };
        if !condition_is_ordinary_tag(&condition, &metadata) {
            continue;
        }
        let Some(key) = condition_feature_key(&condition) else {
            continue;
        };
        let Some(identity) = key.strip_prefix("tag:") else {
            continue;
        };
        let positive_support = row.get::<i64, _>("positive_support").max(0) as f64;
        let negative_support = row.get::<i64, _>("negative_support").max(0) as f64;
        let evidence_mass = positive_support + negative_support;
        if evidence_mass <= 0.0 {
            continue;
        }
        let evidence: Value = serde_json::from_str(row.get::<String, _>("evidence_json").as_str())
            .unwrap_or_else(|_| Value::Object(Default::default()));
        let Some(candidate_net) = evidence.get("candidateNet").and_then(Value::as_f64) else {
            continue;
        };
        let Some(baseline_net) = evidence.get("baselineNet").and_then(Value::as_f64) else {
            continue;
        };
        if !candidate_net.is_finite() || !baseline_net.is_finite() {
            continue;
        }
        let strength = continuous_tag_seed_strength(
            positive_support,
            negative_support,
            candidate_net,
            baseline_net,
        );
        if strength > 0.0 {
            identities
                .entry(identity.to_ascii_lowercase())
                .and_modify(|value: &mut f64| *value = value.max(strength))
                .or_insert(strength);
        }
    }
    if identities.is_empty() {
        return Ok(HashMap::new());
    }
    let mut seeds = HashMap::new();
    for row in sqlx::query(
        "SELECT id, lower(trim(namespace)) || ':' || lower(trim(name)) AS identity FROM tags",
    )
    .fetch_all(pool)
    .await?
    {
        if let Some(strength) = identities.get(&row.get::<String, _>("identity")) {
            seeds.insert(row.get("id"), *strength);
        }
    }
    Ok(seeds)
}

fn continuous_tag_seed_strength(
    positive_support: f64,
    negative_support: f64,
    candidate_net: f64,
    baseline_net: f64,
) -> f64 {
    let evidence_mass = positive_support.max(0.0) + negative_support.max(0.0);
    if !evidence_mass.is_finite() || evidence_mass == 0.0 {
        return 0.0;
    }
    let relative_signal = candidate_net - BASELINE_CALIBRATION * baseline_net;
    if !relative_signal.is_finite() {
        return 0.0;
    }
    (evidence_mass / (evidence_mass + TAG_SEED_PRIOR) * relative_signal)
        .clamp(-TAG_SEED_CAP, TAG_SEED_CAP)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_direct_positive_archive_produces_a_small_continuous_seed() {
        let strength = continuous_tag_seed_strength(1.0, 0.0, 1.0, 0.0);
        assert!((strength - 1.0 / 9.0).abs() < f64::EPSILON);
    }

    #[test]
    fn negative_delete_evidence_never_becomes_a_positive_seed() {
        let strength = continuous_tag_seed_strength(0.0, 1.0, -1.0, 0.0);
        assert!(strength < 0.0);
    }

    #[test]
    fn promoted_keep_uses_the_same_continuous_baseline_adjustment() {
        let strength = continuous_tag_seed_strength(4.0, 0.0, 0.2, 0.8);
        assert!(strength < 0.0);
        assert!(strength > -TAG_SEED_CAP);
    }
}
