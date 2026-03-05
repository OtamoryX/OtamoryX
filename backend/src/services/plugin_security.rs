use crate::services::plugin_manifest::PluginManifest;
use chrono::{DateTime, Utc};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, RwLock};

const DEFAULT_AUDIT_CAPACITY: usize = 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginSecurityAction {
    HttpRequest,
    DbQuery,
    FsRead,
    FsWrite,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginAuditTargetKind {
    Domain,
    Path,
    Table,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuditRecord {
    pub timestamp: DateTime<Utc>,
    pub plugin_id: String,
    pub action: PluginSecurityAction,
    pub target_kind: PluginAuditTargetKind,
    pub target: String,
    pub allowed: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PluginSecurityPolicy {
    pub allow_network: bool,
    pub allowed_domains: Vec<String>,
    pub filesystem_read: Vec<PathBuf>,
    pub filesystem_write: Vec<PathBuf>,
    pub database_read: bool,
    pub database_write_tables: Vec<String>,
}

impl Default for PluginSecurityPolicy {
    fn default() -> Self {
        Self {
            allow_network: false,
            allowed_domains: Vec::new(),
            filesystem_read: Vec::new(),
            filesystem_write: Vec::new(),
            database_read: false,
            database_write_tables: Vec::new(),
        }
    }
}

impl PluginSecurityPolicy {
    pub fn from_manifest(manifest: &PluginManifest) -> Self {
        let allowed_domains: Vec<String> = manifest
            .permissions
            .network
            .iter()
            .map(|domain| domain.trim().to_ascii_lowercase())
            .filter(|domain| !domain.is_empty())
            .collect();

        let allow_network = !allowed_domains.is_empty();

        let policy = Self {
            allow_network,
            allowed_domains,
            filesystem_read: manifest
                .permissions
                .filesystem_read
                .iter()
                .map(PathBuf::from)
                .collect(),
            filesystem_write: manifest
                .permissions
                .filesystem_write
                .iter()
                .map(PathBuf::from)
                .collect(),
            database_read: manifest.permissions.database_read,
            database_write_tables: manifest
                .permissions
                .database_write
                .iter()
                .map(|table| table.to_ascii_lowercase())
                .collect(),
        };

        policy
    }

    pub fn with_allowed_domains(mut self, domains: Vec<String>) -> Self {
        self.allowed_domains = domains;
        self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PluginSecurityError {
    #[error("未找到插件安全策略: {plugin_id}")]
    MissingPolicy { plugin_id: String },
    #[error("参数非法: {message}")]
    InvalidArgument { message: String },
    #[error("权限拒绝: {reason}")]
    PermissionDenied { reason: String },
    #[error("权限网关内部错误: {message}")]
    Internal { message: String },
}

#[derive(Debug)]
pub struct PluginSecurityGateway {
    policies: RwLock<HashMap<String, PluginSecurityPolicy>>,
    audit_records: Mutex<VecDeque<PluginAuditRecord>>,
    max_audit_records: usize,
}

impl Default for PluginSecurityGateway {
    fn default() -> Self {
        Self::new(DEFAULT_AUDIT_CAPACITY)
    }
}

impl PluginSecurityGateway {
    pub fn new(max_audit_records: usize) -> Self {
        Self {
            policies: RwLock::new(HashMap::new()),
            audit_records: Mutex::new(VecDeque::new()),
            max_audit_records: max_audit_records.max(1),
        }
    }

    pub fn upsert_policy(
        &self,
        plugin_id: impl Into<String>,
        policy: PluginSecurityPolicy,
    ) -> Result<(), PluginSecurityError> {
        let plugin_id = plugin_id.into();
        let mut policies = self
            .policies
            .write()
            .map_err(|_| PluginSecurityError::Internal {
                message: "策略表锁中毒".to_string(),
            })?;
        policies.insert(plugin_id, policy);
        Ok(())
    }

    pub fn remove_policy(&self, plugin_id: &str) -> Result<(), PluginSecurityError> {
        let mut policies = self
            .policies
            .write()
            .map_err(|_| PluginSecurityError::Internal {
                message: "策略表锁中毒".to_string(),
            })?;
        policies.remove(plugin_id);
        Ok(())
    }

    pub fn recent_audit_records(&self) -> Result<Vec<PluginAuditRecord>, PluginSecurityError> {
        let records = self
            .audit_records
            .lock()
            .map_err(|_| PluginSecurityError::Internal {
                message: "审计表锁中毒".to_string(),
            })?;
        Ok(records.iter().cloned().collect())
    }

    pub fn check_http_request(
        &self,
        plugin_id: &str,
        method: &str,
        url: &str,
    ) -> Result<String, PluginSecurityError> {
        if method.trim().is_empty() {
            let err = PluginSecurityError::InvalidArgument {
                message: "http method 不能为空".to_string(),
            };
            self.audit_denied(
                plugin_id,
                PluginSecurityAction::HttpRequest,
                PluginAuditTargetKind::Domain,
                "<empty-method>".to_string(),
                err.to_string(),
            );
            return Err(err);
        }

        let parsed = match Url::parse(url) {
            Ok(url) => url,
            Err(err) => {
                let sec_err = PluginSecurityError::InvalidArgument {
                    message: format!("url 解析失败: {err}"),
                };
                self.audit_denied(
                    plugin_id,
                    PluginSecurityAction::HttpRequest,
                    PluginAuditTargetKind::Domain,
                    url.to_string(),
                    sec_err.to_string(),
                );
                return Err(sec_err);
            }
        };

        let host = match parsed.host_str() {
            Some(host) => host.to_ascii_lowercase(),
            None => {
                let err = PluginSecurityError::InvalidArgument {
                    message: "url 缺少 host".to_string(),
                };
                self.audit_denied(
                    plugin_id,
                    PluginSecurityAction::HttpRequest,
                    PluginAuditTargetKind::Domain,
                    url.to_string(),
                    err.to_string(),
                );
                return Err(err);
            }
        };

        let policy = self.require_policy(plugin_id, PluginSecurityAction::HttpRequest, &host)?;

        if !policy.allow_network {
            let err = PluginSecurityError::PermissionDenied {
                reason: "manifest 未声明 network 权限".to_string(),
            };
            self.audit_denied(
                plugin_id,
                PluginSecurityAction::HttpRequest,
                PluginAuditTargetKind::Domain,
                host,
                err.to_string(),
            );
            return Err(err);
        }

        if !domain_allowed(&host, &policy.allowed_domains) {
            let err = PluginSecurityError::PermissionDenied {
                reason: format!("域名不在白名单: {host}"),
            };
            self.audit_denied(
                plugin_id,
                PluginSecurityAction::HttpRequest,
                PluginAuditTargetKind::Domain,
                host,
                err.to_string(),
            );
            return Err(err);
        }

        self.audit_allowed(
            plugin_id,
            PluginSecurityAction::HttpRequest,
            PluginAuditTargetKind::Domain,
            host.clone(),
        );
        Ok(host)
    }

    pub fn check_fs_read(
        &self,
        plugin_id: &str,
        path: &str,
    ) -> Result<PathBuf, PluginSecurityError> {
        self.check_fs_path(plugin_id, PluginSecurityAction::FsRead, path, |policy| {
            &policy.filesystem_read
        })
    }

    pub fn check_fs_write(
        &self,
        plugin_id: &str,
        path: &str,
    ) -> Result<PathBuf, PluginSecurityError> {
        self.check_fs_path(plugin_id, PluginSecurityAction::FsWrite, path, |policy| {
            &policy.filesystem_write
        })
    }

    pub fn check_db_query(
        &self,
        plugin_id: &str,
        sql: &str,
        explicit_table: Option<&str>,
        write: bool,
    ) -> Result<String, PluginSecurityError> {
        let sql = sql.trim();
        if sql.is_empty() {
            let err = PluginSecurityError::InvalidArgument {
                message: "sql 不能为空".to_string(),
            };
            self.audit_denied(
                plugin_id,
                PluginSecurityAction::DbQuery,
                PluginAuditTargetKind::Table,
                "<empty-sql>".to_string(),
                err.to_string(),
            );
            return Err(err);
        }

        let operation =
            detect_sql_operation(sql).ok_or_else(|| PluginSecurityError::InvalidArgument {
                message: "无法识别 sql 操作类型".to_string(),
            });

        let operation = match operation {
            Ok(op) => op,
            Err(err) => {
                self.audit_denied(
                    plugin_id,
                    PluginSecurityAction::DbQuery,
                    PluginAuditTargetKind::Table,
                    "<unknown-table>".to_string(),
                    err.to_string(),
                );
                return Err(err);
            }
        };

        let is_write = write || operation == SqlOperation::Write;
        let table = explicit_table
            .map(normalize_table_name)
            .or_else(|| extract_table_from_sql(sql, operation))
            .filter(|table| !table.is_empty())
            .ok_or_else(|| PluginSecurityError::InvalidArgument {
                message: "无法从 sql 中提取表名，请显式提供 table".to_string(),
            });

        let table = match table {
            Ok(table) => table,
            Err(err) => {
                self.audit_denied(
                    plugin_id,
                    PluginSecurityAction::DbQuery,
                    PluginAuditTargetKind::Table,
                    "<unknown-table>".to_string(),
                    err.to_string(),
                );
                return Err(err);
            }
        };

        let table_lower = table.to_ascii_lowercase();
        let policy = self.require_policy(plugin_id, PluginSecurityAction::DbQuery, &table_lower)?;

        if is_write {
            let allowed = policy
                .database_write_tables
                .iter()
                .any(|allowed| allowed == "*" || allowed == &table_lower);

            if !allowed {
                let err = PluginSecurityError::PermissionDenied {
                    reason: format!("写入表未授权: {table_lower}"),
                };
                self.audit_denied(
                    plugin_id,
                    PluginSecurityAction::DbQuery,
                    PluginAuditTargetKind::Table,
                    table_lower,
                    err.to_string(),
                );
                return Err(err);
            }
        } else if !policy.database_read {
            let err = PluginSecurityError::PermissionDenied {
                reason: "manifest 未声明 database_read 权限".to_string(),
            };
            self.audit_denied(
                plugin_id,
                PluginSecurityAction::DbQuery,
                PluginAuditTargetKind::Table,
                table_lower,
                err.to_string(),
            );
            return Err(err);
        }

        self.audit_allowed(
            plugin_id,
            PluginSecurityAction::DbQuery,
            PluginAuditTargetKind::Table,
            table_lower.clone(),
        );
        Ok(table_lower)
    }

    fn check_fs_path<F>(
        &self,
        plugin_id: &str,
        action: PluginSecurityAction,
        path: &str,
        allowlist_fn: F,
    ) -> Result<PathBuf, PluginSecurityError>
    where
        F: Fn(&PluginSecurityPolicy) -> &[PathBuf],
    {
        if path.trim().is_empty() {
            let err = PluginSecurityError::InvalidArgument {
                message: "path 不能为空".to_string(),
            };
            self.audit_denied(
                plugin_id,
                action,
                PluginAuditTargetKind::Path,
                "<empty-path>".to_string(),
                err.to_string(),
            );
            return Err(err);
        }

        let normalized_target = normalize_path(path)
            .map_err(|message| PluginSecurityError::InvalidArgument { message })?;

        let policy = self.require_policy(
            plugin_id,
            action,
            normalized_target.to_string_lossy().as_ref(),
        )?;

        let allowlist = allowlist_fn(&policy);
        let allowed = allowlist
            .iter()
            .filter_map(|item| normalize_path(item.to_string_lossy().as_ref()).ok())
            .any(|allowed_root| normalized_target.starts_with(allowed_root));

        if !allowed {
            let err = PluginSecurityError::PermissionDenied {
                reason: format!("路径未授权: {}", normalized_target.display()),
            };
            self.audit_denied(
                plugin_id,
                action,
                PluginAuditTargetKind::Path,
                normalized_target.to_string_lossy().to_string(),
                err.to_string(),
            );
            return Err(err);
        }

        self.audit_allowed(
            plugin_id,
            action,
            PluginAuditTargetKind::Path,
            normalized_target.to_string_lossy().to_string(),
        );
        Ok(normalized_target)
    }

    fn require_policy(
        &self,
        plugin_id: &str,
        action: PluginSecurityAction,
        target: &str,
    ) -> Result<PluginSecurityPolicy, PluginSecurityError> {
        let policies = self
            .policies
            .read()
            .map_err(|_| PluginSecurityError::Internal {
                message: "策略表锁中毒".to_string(),
            })?;

        let policy =
            policies
                .get(plugin_id)
                .cloned()
                .ok_or_else(|| PluginSecurityError::MissingPolicy {
                    plugin_id: plugin_id.to_string(),
                });

        if let Err(err) = &policy {
            self.audit_denied(
                plugin_id,
                action,
                match action {
                    PluginSecurityAction::HttpRequest => PluginAuditTargetKind::Domain,
                    PluginSecurityAction::DbQuery => PluginAuditTargetKind::Table,
                    PluginSecurityAction::FsRead | PluginSecurityAction::FsWrite => {
                        PluginAuditTargetKind::Path
                    }
                },
                target.to_string(),
                err.to_string(),
            );
        }

        policy
    }

    fn audit_allowed(
        &self,
        plugin_id: &str,
        action: PluginSecurityAction,
        target_kind: PluginAuditTargetKind,
        target: String,
    ) {
        self.push_audit(PluginAuditRecord {
            timestamp: Utc::now(),
            plugin_id: plugin_id.to_string(),
            action,
            target_kind,
            target,
            allowed: true,
            reason: None,
        });
    }

    fn audit_denied(
        &self,
        plugin_id: &str,
        action: PluginSecurityAction,
        target_kind: PluginAuditTargetKind,
        target: String,
        reason: String,
    ) {
        self.push_audit(PluginAuditRecord {
            timestamp: Utc::now(),
            plugin_id: plugin_id.to_string(),
            action,
            target_kind,
            target,
            allowed: false,
            reason: Some(reason),
        });
    }

    fn push_audit(&self, record: PluginAuditRecord) {
        if record.allowed {
            tracing::info!(
                plugin_id = %record.plugin_id,
                action = ?record.action,
                target_kind = ?record.target_kind,
                target = %record.target,
                "plugin host callback permission granted"
            );
        } else {
            tracing::warn!(
                plugin_id = %record.plugin_id,
                action = ?record.action,
                target_kind = ?record.target_kind,
                target = %record.target,
                reason = %record.reason.as_deref().unwrap_or(""),
                "plugin host callback permission denied"
            );
        }

        if let Ok(mut records) = self.audit_records.lock() {
            if records.len() >= self.max_audit_records {
                records.pop_front();
            }
            records.push_back(record);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SqlOperation {
    Read,
    Write,
}

fn detect_sql_operation(sql: &str) -> Option<SqlOperation> {
    let token = sql
        .split_whitespace()
        .next()
        .map(|token| token.to_ascii_lowercase())?;

    match token.as_str() {
        "select" | "with" | "pragma" => Some(SqlOperation::Read),
        "insert" | "update" | "delete" | "replace" | "create" | "alter" | "drop" => {
            Some(SqlOperation::Write)
        }
        _ => None,
    }
}

fn extract_table_from_sql(sql: &str, operation: SqlOperation) -> Option<String> {
    let tokens = tokenize_sql(sql);
    if tokens.is_empty() {
        return None;
    }

    let lower = tokens
        .iter()
        .map(|token| token.to_ascii_lowercase())
        .collect::<Vec<_>>();

    let table_raw = match operation {
        SqlOperation::Read => {
            if let Some(index) = lower.iter().position(|token| token == "from") {
                tokens.get(index + 1)
            } else {
                None
            }
        }
        SqlOperation::Write => match lower.first().map(String::as_str) {
            Some("insert") | Some("replace") => {
                if let Some(index) = lower.iter().position(|token| token == "into") {
                    tokens.get(index + 1)
                } else {
                    None
                }
            }
            Some("update") => tokens.get(1),
            Some("delete") => {
                if let Some(index) = lower.iter().position(|token| token == "from") {
                    tokens.get(index + 1)
                } else {
                    None
                }
            }
            _ => tokens.get(1),
        },
    };

    table_raw.map(|table| normalize_table_name(table))
}

fn tokenize_sql(sql: &str) -> Vec<String> {
    let mut normalized = String::with_capacity(sql.len());

    for ch in sql.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '"' || ch == '`' {
            normalized.push(ch);
        } else {
            normalized.push(' ');
        }
    }

    normalized
        .split_whitespace()
        .map(ToString::to_string)
        .collect()
}

fn normalize_table_name(input: &str) -> String {
    let trimmed = input.trim().trim_matches('`').trim_matches('"');
    trimmed
        .rsplit('.')
        .next()
        .unwrap_or(trimmed)
        .trim_matches('`')
        .trim_matches('"')
        .to_ascii_lowercase()
}

fn domain_allowed(host: &str, allowed_domains: &[String]) -> bool {
    if allowed_domains.is_empty() {
        return false;
    }

    let host = host.to_ascii_lowercase();

    allowed_domains.iter().any(|entry| {
        let entry = entry.trim().to_ascii_lowercase();
        if entry == "*" {
            return true;
        }

        if let Some(suffix) = entry.strip_prefix("*.") {
            return host == suffix || host.ends_with(&format!(".{suffix}"));
        }

        host == entry
    })
}

fn normalize_path(raw: &str) -> Result<PathBuf, String> {
    let input = Path::new(raw);
    let absolute = if input.is_absolute() {
        input.to_path_buf()
    } else {
        let cwd = std::env::current_dir().map_err(|err| format!("读取当前目录失败: {err}"))?;
        cwd.join(input)
    };

    Ok(lexical_normalize(&absolute))
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
        }
    }

    normalized
}
