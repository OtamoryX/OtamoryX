use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

use crate::plugins::{
    BUILTIN_COMICINFO_PARSER_ID, BUILTIN_DATE_ADDED_ID, BUILTIN_FILENAME_PARSER_ID,
    BUILTIN_TAG_COPIER_ID,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    Metadata,
    #[serde(rename = "download", alias = "downloader")]
    Download,
    Processor,
    Analyzer,
    Script,
    Endpoint,
}

impl PluginType {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::Metadata => "metadata",
            Self::Download => "download",
            Self::Processor => "processor",
            Self::Analyzer => "analyzer",
            Self::Script => "script",
            Self::Endpoint => "endpoint",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PluginDependencies {
    pub items: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginPermissions {
    #[serde(default)]
    pub network: Vec<String>,
    #[serde(default)]
    pub filesystem_read: Vec<String>,
    #[serde(default)]
    pub filesystem_write: Vec<String>,
    #[serde(default)]
    pub database_read: bool,
    #[serde(default)]
    pub database_write: Vec<String>,
    #[serde(default)]
    pub custom_endpoints: bool,
    #[serde(default)]
    pub scheduled_tasks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEventSubscription {
    pub event: String,
    #[serde(default)]
    pub filters: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginEventSubscriptionDecl {
    Event(String),
    Detailed(PluginEventSubscription),
}

impl PluginEventSubscriptionDecl {
    pub fn event_name(&self) -> &str {
        match self {
            Self::Event(event) => event.as_str(),
            Self::Detailed(rule) => rule.event.as_str(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginEventsDeclaration {
    #[serde(default)]
    pub subscribe: Vec<PluginEventSubscriptionDecl>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginScheduleDeclaration {
    #[serde(default)]
    pub cron: String,
    #[serde(default)]
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub manifest_version: u32,
    pub plugin_api_version: u32,
    pub plugin_type: PluginType,
    pub description: String,
    pub author: String,
    pub cooldown: Option<u32>,
    pub oneshot_arg: Option<String>,
    pub plugin_dependencies: PluginDependencies,
    pub permissions: PluginPermissions,
    pub events: PluginEventsDeclaration,
    pub schedule: Option<PluginScheduleDeclaration>,
    pub config_schema: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct RawPluginSection {
    id: String,
    name: String,
    version: String,
    manifest_version: u32,
    plugin_api_version: u32,
    #[serde(rename = "type")]
    plugin_type: PluginType,
    #[serde(default)]
    description: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    cooldown: Option<u32>,
    #[serde(default)]
    oneshot_arg: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawPluginManifest {
    plugin: RawPluginSection,
    #[serde(default)]
    plugin_dependencies: HashMap<String, String>,
    #[serde(default)]
    permissions: PluginPermissions,
    #[serde(default)]
    events: PluginEventsDeclaration,
    #[serde(default)]
    schedule: Option<PluginScheduleDeclaration>,
    #[serde(default = "default_config_schema")]
    config_schema: Value,
}

fn default_config_schema() -> Value {
    Value::Object(Default::default())
}

#[derive(Debug, thiserror::Error)]
pub enum PluginManifestError {
    #[error("读取 plugin manifest 失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("解析 plugin manifest 失败: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("manifest 缺少必填字段: {0}")]
    MissingField(&'static str),
    #[error("manifest 字段非法: {0}")]
    InvalidField(&'static str),
}

impl PluginManifest {
    pub fn from_path(path: &Path) -> Result<Self, PluginManifestError> {
        let raw = std::fs::read_to_string(path)?;
        Self::from_toml_str(&raw)
    }

    pub fn from_toml_str(raw: &str) -> Result<Self, PluginManifestError> {
        let raw_manifest: RawPluginManifest = toml::from_str(raw)?;
        let manifest = Self {
            id: raw_manifest.plugin.id,
            name: raw_manifest.plugin.name,
            version: raw_manifest.plugin.version,
            manifest_version: raw_manifest.plugin.manifest_version,
            plugin_api_version: raw_manifest.plugin.plugin_api_version,
            plugin_type: raw_manifest.plugin.plugin_type,
            description: raw_manifest.plugin.description,
            author: raw_manifest.plugin.author,
            cooldown: raw_manifest.plugin.cooldown,
            oneshot_arg: raw_manifest.plugin.oneshot_arg,
            plugin_dependencies: PluginDependencies {
                items: raw_manifest.plugin_dependencies,
            },
            permissions: raw_manifest.permissions,
            events: raw_manifest.events,
            schedule: raw_manifest.schedule,
            config_schema: raw_manifest.config_schema,
        };

        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), PluginManifestError> {
        if self.id.trim().is_empty() {
            return Err(PluginManifestError::MissingField("plugin.id"));
        }
        if !is_valid_plugin_id(&self.id) {
            return Err(PluginManifestError::InvalidField("plugin.id"));
        }

        if self.name.trim().is_empty() {
            return Err(PluginManifestError::MissingField("plugin.name"));
        }

        if self.version.trim().is_empty() {
            return Err(PluginManifestError::MissingField("plugin.version"));
        }

        if self.manifest_version != 1 {
            return Err(PluginManifestError::InvalidField("plugin.manifest_version"));
        }

        if self.plugin_api_version != 1 {
            return Err(PluginManifestError::InvalidField(
                "plugin.plugin_api_version",
            ));
        }

        if !self.config_schema.is_object() {
            return Err(PluginManifestError::InvalidField("config_schema"));
        }

        for domain in &self.permissions.network {
            if domain.trim().is_empty() {
                return Err(PluginManifestError::InvalidField("permissions.network"));
            }
        }

        for decl in &self.events.subscribe {
            if decl.event_name().trim().is_empty() {
                return Err(PluginManifestError::InvalidField("events.subscribe.event"));
            }
        }

        if let Some(schedule) = &self.schedule {
            if schedule.cron.trim().is_empty() {
                return Err(PluginManifestError::InvalidField("schedule.cron"));
            }

            if cron::Schedule::from_str(&schedule.cron).is_err() {
                return Err(PluginManifestError::InvalidField("schedule.cron"));
            }

            if let Some(timezone) = &schedule.timezone {
                if timezone.trim().is_empty() {
                    return Err(PluginManifestError::InvalidField("schedule.timezone"));
                }
            }
        }

        Ok(())
    }
}

pub fn builtin_plugin_manifests() -> Vec<PluginManifest> {
    vec![
        PluginManifest {
            id: BUILTIN_FILENAME_PARSER_ID.to_string(),
            name: "Filename Parser".to_string(),
            version: "1.0.0".to_string(),
            manifest_version: 1,
            plugin_api_version: 1,
            plugin_type: PluginType::Metadata,
            description: "Parse deterministic metadata hints from archive filenames.".to_string(),
            author: "OtamoryX Team".to_string(),
            cooldown: None,
            oneshot_arg: None,
            plugin_dependencies: PluginDependencies::default(),
            permissions: PluginPermissions::default(),
            events: PluginEventsDeclaration {
                subscribe: vec![PluginEventSubscriptionDecl::Event(
                    "archive_added".to_string(),
                )],
            },
            schedule: None,
            config_schema: json!({
                "type": "object",
                "additional_properties": false,
                "properties": {}
            }),
        },
        PluginManifest {
            id: BUILTIN_COMICINFO_PARSER_ID.to_string(),
            name: "ComicInfo Parser".to_string(),
            version: "1.0.0".to_string(),
            manifest_version: 1,
            plugin_api_version: 1,
            plugin_type: PluginType::Metadata,
            description: "Parse embedded ComicInfo.xml metadata when available.".to_string(),
            author: "OtamoryX Team".to_string(),
            cooldown: None,
            oneshot_arg: None,
            plugin_dependencies: PluginDependencies::default(),
            permissions: PluginPermissions::default(),
            events: PluginEventsDeclaration {
                subscribe: vec![PluginEventSubscriptionDecl::Event(
                    "archive_added".to_string(),
                )],
            },
            schedule: None,
            config_schema: json!({
                "type": "object",
                "additional_properties": false,
                "properties": {}
            }),
        },
        PluginManifest {
            id: BUILTIN_DATE_ADDED_ID.to_string(),
            name: "Date Added".to_string(),
            version: "1.0.0".to_string(),
            manifest_version: 1,
            plugin_api_version: 1,
            plugin_type: PluginType::Metadata,
            description: "Attach deterministic date_added metadata for new archives.".to_string(),
            author: "OtamoryX Team".to_string(),
            cooldown: None,
            oneshot_arg: None,
            plugin_dependencies: PluginDependencies::default(),
            permissions: PluginPermissions::default(),
            events: PluginEventsDeclaration {
                subscribe: vec![PluginEventSubscriptionDecl::Event(
                    "archive_added".to_string(),
                )],
            },
            schedule: None,
            config_schema: json!({
                "type": "object",
                "additional_properties": false,
                "properties": {}
            }),
        },
        PluginManifest {
            id: BUILTIN_TAG_COPIER_ID.to_string(),
            name: "Tag Copier".to_string(),
            version: "1.0.0".to_string(),
            manifest_version: 1,
            plugin_api_version: 1,
            plugin_type: PluginType::Metadata,
            description: "Utility plugin to apply user-selected tags in batch.".to_string(),
            author: "OtamoryX Team".to_string(),
            cooldown: None,
            oneshot_arg: Some("Tag copier payload (handled by host API).".to_string()),
            plugin_dependencies: PluginDependencies::default(),
            permissions: PluginPermissions::default(),
            events: PluginEventsDeclaration::default(),
            schedule: None,
            config_schema: json!({
                "type": "object",
                "additional_properties": false,
                "properties": {}
            }),
        },
    ]
}

pub fn official_seed_plugin_manifests() -> Result<Vec<PluginManifest>, PluginManifestError> {
    let ehentai = include_str!("../../../examples/plugins/official/ehentai-metadata/plugin.toml");
    let nhentai = include_str!("../../../examples/plugins/official/nhentai-metadata/plugin.toml");
    Ok(vec![
        PluginManifest::from_toml_str(ehentai)?,
        PluginManifest::from_toml_str(nhentai)?,
    ])
}

pub fn default_enabled_on_bootstrap(plugin_id: &str) -> bool {
    matches!(
        plugin_id,
        BUILTIN_FILENAME_PARSER_ID | BUILTIN_COMICINFO_PARSER_ID | BUILTIN_DATE_ADDED_ID
    )
}

fn is_valid_plugin_id(plugin_id: &str) -> bool {
    let bytes = plugin_id.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    if bytes.first() == Some(&b'-') || bytes.last() == Some(&b'-') {
        return false;
    }

    bytes
        .iter()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

#[cfg(test)]
mod tests {
    use super::{
        builtin_plugin_manifests, default_enabled_on_bootstrap, official_seed_plugin_manifests,
        PluginManifest,
    };
    use crate::plugins::{
        BUILTIN_COMICINFO_PARSER_ID, BUILTIN_DATE_ADDED_ID, BUILTIN_FILENAME_PARSER_ID,
        BUILTIN_TAG_COPIER_ID,
    };

    #[test]
    fn parses_official_ehentai_manifest() {
        let raw = include_str!("../../../examples/plugins/official/ehentai-metadata/plugin.toml");
        let manifest = PluginManifest::from_toml_str(raw).expect("ehentai manifest should parse");
        assert_eq!(manifest.id, "ehentai-metadata");
        assert_eq!(manifest.manifest_version, 1);
        assert_eq!(manifest.plugin_api_version, 1);
        assert_eq!(
            manifest.permissions.network,
            vec!["api.e-hentai.org".to_string(), "e-hentai.org".to_string()]
        );
    }

    #[test]
    fn parses_official_nhentai_manifest() {
        let raw = include_str!("../../../examples/plugins/official/nhentai-metadata/plugin.toml");
        let manifest = PluginManifest::from_toml_str(raw).expect("nhentai manifest should parse");
        assert_eq!(manifest.id, "nhentai-metadata");
        assert_eq!(manifest.manifest_version, 1);
        assert_eq!(manifest.plugin_api_version, 1);
        assert_eq!(
            manifest.permissions.network,
            vec!["nhentai.net".to_string()]
        );
    }

    #[test]
    fn builds_builtin_plugin_manifests() {
        let manifests = builtin_plugin_manifests();
        assert_eq!(manifests.len(), 4);

        for manifest in manifests {
            manifest
                .validate()
                .expect("builtin manifest should satisfy v1 schema");
            assert!(
                manifest.config_schema.is_object(),
                "builtin manifest must include object config_schema"
            );
        }
    }

    #[test]
    fn includes_builtin_and_official_seed_ids() {
        let mut ids: Vec<String> = builtin_plugin_manifests()
            .into_iter()
            .map(|manifest| manifest.id)
            .collect();
        ids.extend(
            official_seed_plugin_manifests()
                .expect("official manifests should parse")
                .into_iter()
                .map(|manifest| manifest.id),
        );

        assert_eq!(
            ids,
            vec![
                BUILTIN_FILENAME_PARSER_ID.to_string(),
                BUILTIN_COMICINFO_PARSER_ID.to_string(),
                BUILTIN_DATE_ADDED_ID.to_string(),
                BUILTIN_TAG_COPIER_ID.to_string(),
                "ehentai-metadata".to_string(),
                "nhentai-metadata".to_string()
            ]
        );
    }

    #[test]
    fn applies_default_enabled_policy() {
        assert!(default_enabled_on_bootstrap(BUILTIN_FILENAME_PARSER_ID));
        assert!(default_enabled_on_bootstrap(BUILTIN_COMICINFO_PARSER_ID));
        assert!(default_enabled_on_bootstrap(BUILTIN_DATE_ADDED_ID));
        assert!(!default_enabled_on_bootstrap(BUILTIN_TAG_COPIER_ID));
        assert!(!default_enabled_on_bootstrap("ehentai-metadata"));
    }
}
