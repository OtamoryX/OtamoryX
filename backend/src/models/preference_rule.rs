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
