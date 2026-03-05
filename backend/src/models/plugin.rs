use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Plugin {
    #[serde(rename = "plugin_id")]
    #[sqlx(rename = "id")]
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    pub plugin_type: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub enabled: bool,
    pub config: Option<JsonValue>,
    pub execution_count: i64,
    pub last_executed_at: Option<DateTime<Utc>>,
    #[serde(rename = "installedAt")]
    pub installed_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstallPluginRequest {
    pub name: String,
    pub source: PluginSource,
}

#[derive(Debug, Clone, Deserialize)]
pub enum PluginSource {
    File {
        path: String,
    },
    Repository {
        url: String,
        version: Option<String>,
    },
    Local {
        path: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginConfigRequest {
    pub config: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PluginDetail {
    #[serde(rename = "plugin_id")]
    #[sqlx(rename = "id")]
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    pub manifest_version: i32,
    pub plugin_api_version: i32,
    pub plugin_type: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub icon: Option<String>,
    pub enabled: bool,
    pub config: Option<JsonValue>,
    pub permissions: Option<JsonValue>,
    pub manifest: Option<JsonValue>,
    pub execution_count: i64,
    pub last_executed_at: Option<DateTime<Utc>>,
    #[serde(rename = "installedAt")]
    pub installed_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfigSchemaResponse {
    pub plugin_id: String,
    pub config_schema: JsonValue,
    pub cooldown: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginExecuteRequest {
    #[serde(default)]
    pub archive_id: Option<String>,
    #[serde(default)]
    pub archive_ids: Vec<String>,
    #[serde(default)]
    pub oneshot_param: Option<String>,
    #[serde(default)]
    pub input: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginExecutionDispatchResult {
    pub plugin_id: String,
    pub archive_id: Option<String>,
    pub execution_id: Option<String>,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginExecuteResponse {
    pub plugin_id: String,
    pub total: usize,
    pub accepted: usize,
    pub failed: usize,
    pub results: Vec<PluginExecutionDispatchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PluginExecutionRecord {
    #[serde(rename = "execution_id")]
    #[sqlx(rename = "id")]
    pub execution_id: String,
    pub plugin_id: String,
    pub archive_id: Option<String>,
    pub execution_type: String,
    pub status: String,
    pub input_summary: Option<String>,
    pub output_summary: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: Option<i64>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginExecutionListResponse {
    pub total: i64,
    pub limit: u32,
    pub offset: u32,
    pub items: Vec<PluginExecutionRecord>,
}

pub type PluginDependencies = BTreeMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestInfo {
    #[serde(rename(serialize = "plugin_id", deserialize = "id"))]
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    pub manifest_version: u32,
    pub plugin_api_version: u32,
    #[serde(default)]
    pub plugin_dependencies: PluginDependencies,
    pub config_schema: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAbiInfo {
    #[serde(rename(serialize = "plugin_id", deserialize = "id"))]
    pub plugin_id: String,
    pub version: String,
    pub manifest_version: u32,
    pub plugin_api_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    pub manifest: ManifestInfo,
    pub abi: PluginAbiInfo,
    pub description: String,
    pub author: String,
    pub capabilities: Vec<PluginCapability>,
    pub permissions: PluginPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginCapability {
    MetadataExtraction,
    CustomEndpoint,
    ScheduledTask,
    ArchiveProcessing,
    SearchExtension,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPermissions {
    pub network: bool,
    pub filesystem_read: Vec<String>,
    pub database_read: bool,
    pub database_write: Vec<String>,
    pub custom_endpoints: bool,
    pub scheduled_tasks: bool,
}
