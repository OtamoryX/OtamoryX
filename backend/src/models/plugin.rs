use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub config: Option<JsonValue>,
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
    File { path: String },
    Repository { url: String, version: Option<String> },
    Local { path: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginConfigRequest {
    pub config: JsonValue,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
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