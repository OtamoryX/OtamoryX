use crate::services::plugin_executor::{
    OtamoryxHostApiV1, PluginExecutionError, PluginExecutor, PluginSecurityPolicy,
};
use crate::services::plugin_manifest::{PluginManifest, PluginManifestError, PluginType};
use serde_json::Value;
use std::collections::HashMap;
use std::ffi::{c_char, c_void};
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum PluginManagerError {
    #[error("插件管理器尚未初始化")]
    NotInitialized,
    #[error("插件不存在: {0}")]
    NotFound(String),
    #[error("插件未启用: {0}")]
    Disabled(String),
    #[error("插件管理功能尚未实现: {0}")]
    NotImplemented(&'static str),
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("manifest 错误: {0}")]
    Manifest(#[from] PluginManifestError),
    #[error("执行错误: {0}")]
    Execution(#[from] PluginExecutionError),
}

pub type PluginCreateFn = unsafe extern "C" fn() -> *mut c_void;
pub type PluginDestroyFn = unsafe extern "C" fn(*mut c_void);
pub type PluginInfoFn = unsafe extern "C" fn() -> *mut c_char;
pub type PluginFreeStringFn = unsafe extern "C" fn(*mut c_char);
pub type PluginExecMetadataFn = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_char;
pub type PluginExecScriptFn = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_char;

#[derive(Debug, Clone, Copy, Default)]
pub struct PluginFfiCache {
    pub create: Option<PluginCreateFn>,
    pub destroy: Option<PluginDestroyFn>,
    pub info: Option<PluginInfoFn>,
    pub free_string: Option<PluginFreeStringFn>,
    pub exec_metadata: Option<PluginExecMetadataFn>,
    pub exec_script: Option<PluginExecScriptFn>,
}

pub struct PluginInstance {
    pub manifest: PluginManifest,
    pub enabled: bool,
    pub library_path: PathBuf,
    pub ffi: PluginFfiCache,
    pub library: Option<libloading::Library>,
    pub handle: Option<NonNull<c_void>>,
}

#[derive(Debug, Clone)]
pub struct PluginListItem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub plugin_type: PluginType,
    pub enabled: bool,
    pub loaded: bool,
}

pub struct PluginManager {
    loaded: HashMap<String, PluginInstance>,
    manifests: HashMap<String, PluginManifest>,
    executor: PluginExecutor,
    plugins_dir: PathBuf,
    initialized: bool,
}

impl PluginManager {
    pub fn new<P: Into<PathBuf>>(plugins_dir: P) -> Self {
        Self {
            loaded: HashMap::new(),
            manifests: HashMap::new(),
            executor: PluginExecutor::new(Duration::from_secs(30)),
            plugins_dir: plugins_dir.into(),
            initialized: false,
        }
    }

    pub fn with_executor<P: Into<PathBuf>>(plugins_dir: P, executor: PluginExecutor) -> Self {
        Self {
            loaded: HashMap::new(),
            manifests: HashMap::new(),
            executor,
            plugins_dir: plugins_dir.into(),
            initialized: false,
        }
    }

    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }

    pub fn executor(&self) -> &PluginExecutor {
        &self.executor
    }

    pub async fn initialize(&mut self) -> Result<(), PluginManagerError> {
        std::fs::create_dir_all(&self.plugins_dir)?;
        self.initialized = true;
        Ok(())
    }

    pub async fn install(&mut self, _package: &Path) -> Result<PluginManifest, PluginManagerError> {
        self.ensure_initialized()?;
        Err(PluginManagerError::NotImplemented(
            "TODO: unpack package, parse manifest, and load dynamic library",
        ))
    }

    pub async fn uninstall(&mut self, plugin_id: &str) -> Result<(), PluginManagerError> {
        self.ensure_initialized()?;
        if !self.manifests.contains_key(plugin_id) && !self.loaded.contains_key(plugin_id) {
            return Err(PluginManagerError::NotFound(plugin_id.to_string()));
        }

        self.executor.remove_host_api(plugin_id)?;

        Err(PluginManagerError::NotImplemented(
            "TODO: unload dynamic library and remove plugin artifacts",
        ))
    }

    pub async fn set_enabled(
        &mut self,
        plugin_id: &str,
        enabled: bool,
    ) -> Result<(), PluginManagerError> {
        self.ensure_initialized()?;

        if self.loaded.contains_key(plugin_id) {
            if let Some(instance) = self.loaded.get_mut(plugin_id) {
                instance.enabled = enabled;
            }

            if enabled {
                self.ensure_host_api_for_plugin(plugin_id)?;
            } else {
                self.executor.remove_host_api(plugin_id)?;
            }
            return Ok(());
        }

        if self.manifests.contains_key(plugin_id) {
            return Err(PluginManagerError::NotImplemented(
                "TODO: enable/disable should load or unload plugin runtime instance",
            ));
        }

        Err(PluginManagerError::NotFound(plugin_id.to_string()))
    }

    pub async fn configure(
        &mut self,
        plugin_id: &str,
        _config: Value,
    ) -> Result<(), PluginManagerError> {
        self.ensure_initialized()?;

        if !self.manifests.contains_key(plugin_id) {
            return Err(PluginManagerError::NotFound(plugin_id.to_string()));
        }

        Err(PluginManagerError::NotImplemented(
            "TODO: persist config and call plugin configure hook",
        ))
    }

    pub fn list(&self) -> Vec<PluginListItem> {
        let mut items = self
            .manifests
            .iter()
            .map(|(id, manifest)| {
                let loaded = self.loaded.contains_key(id);
                let enabled = self.loaded.get(id).map(|p| p.enabled).unwrap_or(false);
                PluginListItem {
                    id: manifest.id.clone(),
                    name: manifest.name.clone(),
                    version: manifest.version.clone(),
                    plugin_type: manifest.plugin_type.clone(),
                    enabled,
                    loaded,
                }
            })
            .collect::<Vec<_>>();

        items.sort_by(|a, b| a.id.cmp(&b.id));
        items
    }

    pub async fn exec_metadata(
        &self,
        plugin_id: &str,
        _archive_info: &Value,
    ) -> Result<Value, PluginManagerError> {
        self.ensure_initialized()?;
        self.ensure_enabled(plugin_id)?;
        let host_api = self.ensure_host_api_for_plugin(plugin_id)?;

        self.executor
            .call_ffi_with_host_api::<Value>(plugin_id, "exec_metadata", &[], Some(host_api), None)
            .await
            .map_err(PluginManagerError::from)
    }

    pub async fn exec_script(
        &self,
        plugin_id: &str,
        oneshot: Option<&str>,
    ) -> Result<Value, PluginManagerError> {
        self.ensure_initialized()?;
        self.ensure_enabled(plugin_id)?;
        let host_api = self.ensure_host_api_for_plugin(plugin_id)?;

        let args = oneshot.map_or_else(Vec::new, |arg| vec![arg]);

        self.executor
            .call_ffi_with_host_api::<Value>(plugin_id, "exec_script", &args, Some(host_api), None)
            .await
            .map_err(PluginManagerError::from)
    }

    fn ensure_host_api_for_plugin(
        &self,
        plugin_id: &str,
    ) -> Result<*const OtamoryxHostApiV1, PluginManagerError> {
        let manifest = self.resolve_manifest(plugin_id)?;
        let policy = PluginSecurityPolicy::from_manifest(manifest);
        self.executor
            .ensure_host_api(plugin_id, policy)
            .map_err(PluginManagerError::from)
    }

    fn resolve_manifest(&self, plugin_id: &str) -> Result<&PluginManifest, PluginManagerError> {
        if let Some(instance) = self.loaded.get(plugin_id) {
            return Ok(&instance.manifest);
        }

        self.manifests
            .get(plugin_id)
            .ok_or_else(|| PluginManagerError::NotFound(plugin_id.to_string()))
    }

    fn ensure_initialized(&self) -> Result<(), PluginManagerError> {
        if self.initialized {
            Ok(())
        } else {
            Err(PluginManagerError::NotInitialized)
        }
    }

    fn ensure_enabled(&self, plugin_id: &str) -> Result<(), PluginManagerError> {
        let instance = self
            .loaded
            .get(plugin_id)
            .ok_or_else(|| PluginManagerError::NotFound(plugin_id.to_string()))?;

        if instance.enabled {
            Ok(())
        } else {
            Err(PluginManagerError::Disabled(plugin_id.to_string()))
        }
    }
}
