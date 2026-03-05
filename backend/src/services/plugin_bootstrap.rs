use chrono::Utc;
use serde_json::Value;
use sqlx::{Pool, Sqlite};
use tracing::info;

use super::{
    builtin_plugin_manifests, default_enabled_on_bootstrap, official_seed_plugin_manifests,
    PluginManifest, PluginManifestError,
};

#[derive(Debug, thiserror::Error)]
pub enum PluginBootstrapError {
    #[error("加载插件 manifest 失败: {0}")]
    Manifest(#[from] PluginManifestError),
    #[error("序列化插件 manifest 失败: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("写入插件记录失败: {0}")]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone)]
struct PluginSeedPayload {
    id: String,
    name: String,
    version: String,
    manifest_version: i32,
    plugin_api_version: i32,
    plugin_type: String,
    description: Option<String>,
    author: Option<String>,
    default_enabled: bool,
    permissions: Value,
    manifest: Value,
}

impl PluginSeedPayload {
    fn from_manifest(manifest: PluginManifest) -> Result<Self, serde_json::Error> {
        let id = manifest.id.clone();
        let name = manifest.name.clone();
        let version = manifest.version.clone();
        let description = if manifest.description.trim().is_empty() {
            None
        } else {
            Some(manifest.description.clone())
        };
        let author = if manifest.author.trim().is_empty() {
            None
        } else {
            Some(manifest.author.clone())
        };
        let manifest_version = manifest.manifest_version as i32;
        let plugin_api_version = manifest.plugin_api_version as i32;
        let permissions = serde_json::to_value(&manifest.permissions)?;
        let plugin_type = manifest.plugin_type.as_db_str().to_string();
        let default_enabled = default_enabled_on_bootstrap(&id);
        let manifest_json = serde_json::to_value(manifest)?;

        Ok(Self {
            id,
            name,
            version,
            manifest_version,
            plugin_api_version,
            plugin_type,
            description,
            author,
            default_enabled,
            permissions,
            manifest: manifest_json,
        })
    }
}

fn build_seed_payloads() -> Result<Vec<PluginSeedPayload>, PluginBootstrapError> {
    let mut manifests = builtin_plugin_manifests();
    manifests.extend(official_seed_plugin_manifests()?);

    manifests
        .into_iter()
        .map(PluginSeedPayload::from_manifest)
        .collect::<Result<Vec<_>, _>>()
        .map_err(PluginBootstrapError::from)
}

pub async fn bootstrap_seed_plugins(pool: &Pool<Sqlite>) -> Result<usize, PluginBootstrapError> {
    let payloads = build_seed_payloads()?;
    let now = Utc::now();
    let mut tx = pool.begin().await?;

    for payload in &payloads {
        sqlx::query(
            r#"
            INSERT INTO plugins (
                id,
                name,
                version,
                manifest_version,
                plugin_api_version,
                plugin_type,
                description,
                author,
                enabled,
                permissions,
                manifest,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                version = excluded.version,
                manifest_version = excluded.manifest_version,
                plugin_api_version = excluded.plugin_api_version,
                plugin_type = excluded.plugin_type,
                description = excluded.description,
                author = excluded.author,
                permissions = excluded.permissions,
                manifest = excluded.manifest,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&payload.id)
        .bind(&payload.name)
        .bind(&payload.version)
        .bind(payload.manifest_version)
        .bind(payload.plugin_api_version)
        .bind(&payload.plugin_type)
        .bind(&payload.description)
        .bind(&payload.author)
        .bind(payload.default_enabled)
        .bind(&payload.permissions)
        .bind(&payload.manifest)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    info!(
        "Plugin bootstrap ensured {} seed manifests (builtin + official)",
        payloads.len()
    );
    Ok(payloads.len())
}

#[cfg(test)]
mod tests {
    use super::{bootstrap_seed_plugins, build_seed_payloads};
    use crate::plugins::{BUILTIN_FILENAME_PARSER_ID, BUILTIN_TAG_COPIER_ID};
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn builds_seed_payload_with_manifest_and_permissions() {
        let payloads = build_seed_payloads().expect("seed payloads should build");
        assert_eq!(payloads.len(), 6);

        let filename = payloads
            .iter()
            .find(|payload| payload.id == BUILTIN_FILENAME_PARSER_ID)
            .expect("filename-parser payload should exist");
        assert!(filename.default_enabled);
        assert!(filename.permissions.is_object());
        assert!(filename.manifest.get("config_schema").is_some());

        let tag_copier = payloads
            .iter()
            .find(|payload| payload.id == BUILTIN_TAG_COPIER_ID)
            .expect("tag-copier payload should exist");
        assert!(!tag_copier.default_enabled);
    }

    #[tokio::test]
    async fn bootstrap_is_idempotent_and_preserves_user_config_and_enabled() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool");

        sqlx::query(
            r#"
            CREATE TABLE plugins (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                manifest_version INTEGER NOT NULL DEFAULT 1,
                plugin_api_version INTEGER NOT NULL DEFAULT 1,
                plugin_type TEXT NOT NULL,
                description TEXT,
                author TEXT,
                icon TEXT,
                enabled BOOLEAN NOT NULL DEFAULT FALSE,
                config TEXT,
                permissions TEXT,
                manifest TEXT,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
                last_executed_at DATETIME,
                execution_count INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("create plugins table");

        sqlx::query(
            r#"
            INSERT INTO plugins (
                id, name, version, manifest_version, plugin_api_version, plugin_type,
                enabled, config, permissions, manifest, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(BUILTIN_FILENAME_PARSER_ID)
        .bind("User Custom Name")
        .bind("0.0.1")
        .bind(1_i32)
        .bind(1_i32)
        .bind("metadata")
        .bind(false)
        .bind(r#"{"enabled":false,"custom":"keep"}"#)
        .bind("{}")
        .bind("{}")
        .execute(&pool)
        .await
        .expect("seed custom row");

        bootstrap_seed_plugins(&pool)
            .await
            .expect("first bootstrap succeeds");
        bootstrap_seed_plugins(&pool)
            .await
            .expect("second bootstrap succeeds");

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plugins")
            .fetch_one(&pool)
            .await
            .expect("query total plugins");
        assert_eq!(total, 6);

        let row: (bool, String) =
            sqlx::query_as("SELECT enabled, config FROM plugins WHERE id = ?")
                .bind(BUILTIN_FILENAME_PARSER_ID)
                .fetch_one(&pool)
                .await
                .expect("query filename parser row");

        assert!(!row.0, "bootstrap should not override user enabled flag");
        assert_eq!(
            row.1, r#"{"enabled":false,"custom":"keep"}"#,
            "bootstrap should preserve user config payload"
        );
    }
}
