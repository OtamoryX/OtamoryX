#[path = "plugin_host_api.rs"]
pub mod plugin_host_api;
#[path = "plugin_security.rs"]
pub mod plugin_security;

use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub use self::plugin_host_api::{
    DefaultHostCallbackBackend, HostCallbackBackend, OtamoryxHostApiHandle, OtamoryxHostApiV1,
};
pub use self::plugin_security::{PluginAuditRecord, PluginSecurityGateway, PluginSecurityPolicy};

#[derive(Debug, thiserror::Error)]
pub enum PluginExecutionError {
    #[error("插件执行超时（{timeout_ms}ms）")]
    Timeout { timeout_ms: u64 },
    #[error("插件内部崩溃")]
    Panicked,
    #[error("插件任务失败: {0}")]
    JoinFailed(String),
    #[error("插件返回错误: {0}")]
    PluginReturned(String),
    #[error("插件冷却中，{remaining_secs}秒后可再次执行")]
    Cooldown { remaining_secs: u32 },
    #[error("Host API 错误: {0}")]
    HostApi(String),
    #[error("执行器功能尚未实现: {0}")]
    NotImplemented(&'static str),
}

pub struct PluginExecutor {
    default_timeout: Duration,
    last_execution: Mutex<HashMap<String, Instant>>,
    host_api_handles: Mutex<HashMap<String, OtamoryxHostApiHandle>>,
    security_gateway: Arc<PluginSecurityGateway>,
    host_backend: Arc<dyn HostCallbackBackend>,
}

impl std::fmt::Debug for PluginExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let host_api_count = self
            .host_api_handles
            .lock()
            .map(|m| m.len())
            .unwrap_or_default();
        f.debug_struct("PluginExecutor")
            .field("default_timeout", &self.default_timeout)
            .field("host_api_count", &host_api_count)
            .finish()
    }
}

impl Default for PluginExecutor {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}

impl PluginExecutor {
    pub fn new(default_timeout: Duration) -> Self {
        Self::with_host_backend(default_timeout, Arc::new(DefaultHostCallbackBackend))
    }

    pub fn with_host_backend(
        default_timeout: Duration,
        host_backend: Arc<dyn HostCallbackBackend>,
    ) -> Self {
        Self {
            default_timeout,
            last_execution: Mutex::new(HashMap::new()),
            host_api_handles: Mutex::new(HashMap::new()),
            security_gateway: Arc::new(PluginSecurityGateway::default()),
            host_backend,
        }
    }

    pub fn default_timeout(&self) -> Duration {
        self.default_timeout
    }

    pub fn security_gateway(&self) -> Arc<PluginSecurityGateway> {
        Arc::clone(&self.security_gateway)
    }

    pub fn ensure_host_api(
        &self,
        plugin_id: &str,
        policy: PluginSecurityPolicy,
    ) -> Result<*const OtamoryxHostApiV1, PluginExecutionError> {
        if plugin_id.trim().is_empty() {
            return Err(PluginExecutionError::HostApi(
                "plugin_id 不能为空".to_string(),
            ));
        }

        self.security_gateway
            .upsert_policy(plugin_id.to_string(), policy)
            .map_err(|err| PluginExecutionError::HostApi(err.to_string()))?;

        let mut handles = self
            .host_api_handles
            .lock()
            .map_err(|_| PluginExecutionError::HostApi("host api 缓存锁中毒".to_string()))?;

        let handle = handles.entry(plugin_id.to_string()).or_insert_with(|| {
            OtamoryxHostApiHandle::new(
                plugin_id.to_string(),
                Arc::clone(&self.security_gateway),
                Arc::clone(&self.host_backend),
            )
        });

        Ok(handle.api_ptr())
    }

    pub fn remove_host_api(&self, plugin_id: &str) -> Result<(), PluginExecutionError> {
        let mut handles = self
            .host_api_handles
            .lock()
            .map_err(|_| PluginExecutionError::HostApi("host api 缓存锁中毒".to_string()))?;
        handles.remove(plugin_id);
        self.security_gateway
            .remove_policy(plugin_id)
            .map_err(|err| PluginExecutionError::HostApi(err.to_string()))?;
        Ok(())
    }

    pub fn recent_security_audits(&self) -> Result<Vec<PluginAuditRecord>, PluginExecutionError> {
        self.security_gateway
            .recent_audit_records()
            .map_err(|err| PluginExecutionError::HostApi(err.to_string()))
    }

    pub async fn run_guarded<Fut, T>(
        &self,
        fut: Fut,
        timeout: Option<Duration>,
    ) -> Result<T, PluginExecutionError>
    where
        Fut: Future<Output = Result<T, PluginExecutionError>> + Send + 'static,
        T: Send + 'static,
    {
        let duration = timeout.unwrap_or(self.default_timeout);
        let mut handle = tokio::spawn(fut);

        match tokio::time::timeout(duration, &mut handle).await {
            Ok(join_result) => match join_result {
                Ok(output) => output,
                Err(err) if err.is_panic() => Err(PluginExecutionError::Panicked),
                Err(err) => Err(PluginExecutionError::JoinFailed(err.to_string())),
            },
            Err(_) => {
                handle.abort();
                Err(PluginExecutionError::Timeout {
                    timeout_ms: duration.as_millis() as u64,
                })
            }
        }
    }

    pub async fn run_blocking_guarded<F, T>(
        &self,
        f: F,
        timeout: Option<Duration>,
    ) -> Result<T, PluginExecutionError>
    where
        F: FnOnce() -> Result<T, PluginExecutionError> + Send + 'static,
        T: Send + 'static,
    {
        let duration = timeout.unwrap_or(self.default_timeout);
        let mut handle = tokio::task::spawn_blocking(f);

        match tokio::time::timeout(duration, &mut handle).await {
            Ok(join_result) => match join_result {
                Ok(output) => output,
                Err(err) if err.is_panic() => Err(PluginExecutionError::Panicked),
                Err(err) => Err(PluginExecutionError::JoinFailed(err.to_string())),
            },
            Err(_) => {
                handle.abort();
                Err(PluginExecutionError::Timeout {
                    timeout_ms: duration.as_millis() as u64,
                })
            }
        }
    }

    pub fn check_cooldown(
        &self,
        plugin_id: &str,
        cooldown_secs: u32,
    ) -> Result<(), PluginExecutionError> {
        if cooldown_secs == 0 {
            return Ok(());
        }

        let mut guard = self
            .last_execution
            .lock()
            .map_err(|_| PluginExecutionError::JoinFailed("cooldown state poisoned".to_string()))?;

        if let Some(last) = guard.get(plugin_id) {
            let elapsed_secs = last.elapsed().as_secs();
            let cooldown_secs_u64 = cooldown_secs as u64;
            if elapsed_secs < cooldown_secs_u64 {
                return Err(PluginExecutionError::Cooldown {
                    remaining_secs: (cooldown_secs_u64 - elapsed_secs) as u32,
                });
            }
        }

        guard.insert(plugin_id.to_string(), Instant::now());
        Ok(())
    }

    pub async fn call_ffi<R>(
        &self,
        plugin_id: &str,
        func_name: &str,
        args: &[&str],
        timeout: Option<Duration>,
    ) -> Result<R, PluginExecutionError>
    where
        R: DeserializeOwned + Send + 'static,
    {
        self.call_ffi_with_host_api(plugin_id, func_name, args, None, timeout)
            .await
    }

    pub async fn call_ffi_with_host_api<R>(
        &self,
        plugin_id: &str,
        func_name: &str,
        args: &[&str],
        host_api: Option<*const OtamoryxHostApiV1>,
        timeout: Option<Duration>,
    ) -> Result<R, PluginExecutionError>
    where
        R: DeserializeOwned + Send + 'static,
    {
        let plugin_id = plugin_id.to_string();
        let func_name = func_name.to_string();
        let args = args
            .iter()
            .map(|arg| (*arg).to_string())
            .collect::<Vec<_>>();
        let host_api_addr = host_api.map(|ptr| ptr as usize);

        self.run_blocking_guarded(
            move || {
                let host_api = host_api_addr.map(|addr| addr as *const OtamoryxHostApiV1);
                let _ = (plugin_id, func_name, args, host_api);
                // TODO: 绑定 libloading Symbol，并把 host_api 作为入参传给插件入口函数。
                Err(PluginExecutionError::NotImplemented(
                    "TODO: call_ffi_with_host_api needs symbol binding and host api bridge",
                ))
            },
            timeout,
        )
        .await
    }
}
