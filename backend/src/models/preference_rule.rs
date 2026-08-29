use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferenceRule {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub rule_version: String,
    pub conditions: Value,
    pub exceptions: Value,
    pub action: String,
    pub confidence_threshold: f64,
    pub enabled: bool,
    pub owner_role: String,
    pub false_positive_count: i32,
    pub auto_paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferenceRuleInput {
    pub name: String,
    pub rule_version: String,
    pub conditions: Value,
    #[serde(default)]
    pub exceptions: Value,
    pub action: String,
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: f64,
}

fn default_confidence_threshold() -> f64 {
    0.85
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferenceRuleEvaluation {
    pub id: String,
    pub analysis_id: String,
    pub rule_id: String,
    pub rule_version: String,
    pub matched: bool,
    pub matched_conditions: Value,
    pub evidence_pages: Vec<i32>,
    pub decision: String,
    pub execution_status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferenceRuleVersionStats {
    pub rule_id: String,
    pub rule_version: String,
    pub windows: Vec<PreferenceRuleVersionWindowStats>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferenceRuleVersionWindowStats {
    pub days: u16,
    pub matched_count: i64,
    pub unique_archive_count: i64,
    pub keep_count: i64,
    pub downrank_count: i64,
    pub auto_delete_count: i64,
    pub auto_delete_success_count: i64,
    pub restore_correction_count: i64,
    pub false_positive_rate: f64,
    pub last_matched_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferenceInsightCandidate {
    pub id: String,
    pub user_id: String,
    pub condition_key: String,
    pub conditions: Value,
    pub feature_kind: Option<String>,
    pub positive_score: f64,
    pub negative_score: f64,
    pub positive_support: i64,
    pub negative_support: i64,
    pub unique_archive_count: i64,
    pub informative_result_count: i64,
    pub effective_read_count: i64,
    pub deep_read_count: i64,
    pub manual_delete_count: i64,
    pub quick_exit_count: i64,
    pub conflict_count: i64,
    pub baseline_rate: f64,
    pub lift: f64,
    pub direction_probability: f64,
    pub profile_coverage: f64,
    pub evidence_state: String,
    pub status: String,
    pub source: String,
    pub sample_archives: Vec<String>,
    pub evidence: Value,
    pub last_learned_at: Option<String>,
}
