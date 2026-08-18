use super::security::{PluginSecurityError, PluginSecurityGateway};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr, CString};
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::time::Duration;

pub const OTAMORYX_HOST_API_V1_ABI: u32 = 1;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtamoryxSdkErrorCode {
    Ok = 0,
    Internal = -1,
    InvalidArgument = -2,
    PermissionDenied = -3,
    Unsupported = -4,
}

impl OtamoryxSdkErrorCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OtamoryxHostStringResult {
    pub code: i32,
    pub payload_json: *mut c_char,
}

impl OtamoryxHostStringResult {
    fn success(value: serde_json::Value) -> Self {
        Self::from_value(OtamoryxSdkErrorCode::Ok, value)
    }

    fn error(code: OtamoryxSdkErrorCode, message: impl Into<String>) -> Self {
        Self::from_value(code, json!({ "error": message.into() }))
    }

    fn from_value(code: OtamoryxSdkErrorCode, value: serde_json::Value) -> Self {
        let payload = serde_json::to_string(&value)
            .unwrap_or_else(|_| "{\"error\":\"serialize failed\"}".to_string());
        let payload_json = CString::new(payload)
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut());

        Self {
            code: code.as_i32(),
            payload_json,
        }
    }
}

pub type OtamoryxHostCallbackFn =
    unsafe extern "C" fn(*mut c_void, *const c_char) -> OtamoryxHostStringResult;
pub type OtamoryxHostFreeStringFn = unsafe extern "C" fn(*mut c_char);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OtamoryxHostApiV1 {
    pub abi_version: u32,
    pub host_ctx: *mut c_void,
    pub http_request: Option<OtamoryxHostCallbackFn>,
    pub db_query: Option<OtamoryxHostCallbackFn>,
    pub fs_read: Option<OtamoryxHostCallbackFn>,
    pub fs_write: Option<OtamoryxHostCallbackFn>,
    pub free_string: Option<OtamoryxHostFreeStringFn>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HostHttpRequest {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HostDbQueryRequest {
    pub sql: String,
    #[serde(default)]
    pub table: Option<String>,
    #[serde(default)]
    pub write: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HostFsReadRequest {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HostFsWriteRequest {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub create_parent: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum HostBackendError {
    #[error("能力未实现: {0}")]
    Unsupported(&'static str),
    #[error("参数非法: {0}")]
    InvalidArgument(String),
    #[error("请求失败: {0}")]
    Request(#[from] reqwest::Error),
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("内部错误: {0}")]
    Internal(String),
}

pub trait HostCallbackBackend: Send + Sync {
    fn http_request(&self, request: HostHttpRequest)
        -> Result<serde_json::Value, HostBackendError>;
    fn db_query(&self, request: HostDbQueryRequest) -> Result<serde_json::Value, HostBackendError>;
    fn fs_read(&self, request: HostFsReadRequest) -> Result<serde_json::Value, HostBackendError>;
    fn fs_write(&self, request: HostFsWriteRequest) -> Result<serde_json::Value, HostBackendError>;
}

#[derive(Debug, Default)]
pub struct DefaultHostCallbackBackend;

impl HostCallbackBackend for DefaultHostCallbackBackend {
    fn http_request(
        &self,
        request: HostHttpRequest,
    ) -> Result<serde_json::Value, HostBackendError> {
        let method = reqwest::Method::from_bytes(request.method.as_bytes())
            .map_err(|err| HostBackendError::InvalidArgument(format!("非法 HTTP method: {err}")));
        let method = method?;

        let timeout = request
            .timeout_ms
            .map(Duration::from_millis)
            .unwrap_or_else(|| Duration::from_secs(15));

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|err| HostBackendError::Internal(format!("创建 tokio runtime 失败: {err}")))?;

        let client = reqwest::Client::builder().timeout(timeout).build()?;
        let mut builder = client.request(method, &request.url);

        for (key, value) in &request.headers {
            builder = builder.header(key, value);
        }

        if let Some(body) = request.body {
            builder = builder.body(body);
        }

        let response = runtime.block_on(builder.send())?;
        let status = response.status().as_u16();

        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_string(),
                    value.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect::<HashMap<_, _>>();

        let body = runtime.block_on(response.text())?;
        Ok(json!({
            "status": status,
            "headers": headers,
            "body": body,
        }))
    }

    fn db_query(
        &self,
        _request: HostDbQueryRequest,
    ) -> Result<serde_json::Value, HostBackendError> {
        Err(HostBackendError::Unsupported(
            "TODO: wire db_query to shared database executor",
        ))
    }

    fn fs_read(&self, request: HostFsReadRequest) -> Result<serde_json::Value, HostBackendError> {
        let content = std::fs::read_to_string(&request.path)?;
        Ok(json!({
            "path": request.path,
            "content": content,
        }))
    }

    fn fs_write(&self, request: HostFsWriteRequest) -> Result<serde_json::Value, HostBackendError> {
        if request.create_parent {
            if let Some(parent) = std::path::Path::new(&request.path).parent() {
                std::fs::create_dir_all(parent)?;
            }
        }

        std::fs::write(&request.path, request.content.as_bytes())?;
        Ok(json!({
            "path": request.path,
            "written_bytes": request.content.len(),
        }))
    }
}

pub struct HostCallbackContext {
    pub plugin_id: String,
    pub security: Arc<PluginSecurityGateway>,
    pub backend: Arc<dyn HostCallbackBackend>,
}

pub struct OtamoryxHostApiHandle {
    ctx: Box<HostCallbackContext>,
    api: OtamoryxHostApiV1,
}

impl OtamoryxHostApiHandle {
    pub fn new(
        plugin_id: impl Into<String>,
        security: Arc<PluginSecurityGateway>,
        backend: Arc<dyn HostCallbackBackend>,
    ) -> Self {
        let mut ctx = Box::new(HostCallbackContext {
            plugin_id: plugin_id.into(),
            security,
            backend,
        });
        let host_ctx = (&mut *ctx) as *mut HostCallbackContext as *mut c_void;

        let api = OtamoryxHostApiV1 {
            abi_version: OTAMORYX_HOST_API_V1_ABI,
            host_ctx,
            http_request: Some(otamoryx_host_http_request),
            db_query: Some(otamoryx_host_db_query),
            fs_read: Some(otamoryx_host_fs_read),
            fs_write: Some(otamoryx_host_fs_write),
            free_string: Some(otamoryx_host_free_string),
        };

        Self { ctx, api }
    }

    pub fn api(&self) -> &OtamoryxHostApiV1 {
        &self.api
    }

    pub fn api_ptr(&self) -> *const OtamoryxHostApiV1 {
        &self.api as *const OtamoryxHostApiV1
    }

    pub fn plugin_id(&self) -> &str {
        &self.ctx.plugin_id
    }
}

#[derive(Debug, thiserror::Error)]
enum HostDispatchError {
    #[error("参数为空")]
    NullPointer,
    #[error("UTF-8 解码失败")]
    InvalidUtf8,
    #[error("JSON 解析失败: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error(transparent)]
    Security(#[from] PluginSecurityError),
    #[error(transparent)]
    Backend(#[from] HostBackendError),
}

fn map_dispatch_error(error: &HostDispatchError) -> OtamoryxSdkErrorCode {
    match error {
        HostDispatchError::NullPointer
        | HostDispatchError::InvalidUtf8
        | HostDispatchError::InvalidJson(_) => OtamoryxSdkErrorCode::InvalidArgument,
        HostDispatchError::Security(security_error) => match security_error {
            PluginSecurityError::InvalidArgument { .. } => OtamoryxSdkErrorCode::InvalidArgument,
            PluginSecurityError::PermissionDenied { .. } => OtamoryxSdkErrorCode::PermissionDenied,
            PluginSecurityError::MissingPolicy { .. } | PluginSecurityError::Internal { .. } => {
                OtamoryxSdkErrorCode::Internal
            }
        },
        HostDispatchError::Backend(backend_error) => match backend_error {
            HostBackendError::Unsupported(_) => OtamoryxSdkErrorCode::Unsupported,
            HostBackendError::InvalidArgument(_) => OtamoryxSdkErrorCode::InvalidArgument,
            HostBackendError::Request(_)
            | HostBackendError::Io(_)
            | HostBackendError::Internal(_) => OtamoryxSdkErrorCode::Internal,
        },
    }
}

fn parse_payload<'a>(payload_ptr: *const c_char) -> Result<&'a str, HostDispatchError> {
    if payload_ptr.is_null() {
        return Err(HostDispatchError::NullPointer);
    }

    // SAFETY: payload_ptr 非空，且约定由调用方传入 NUL 结尾 C 字符串。
    let raw = unsafe { CStr::from_ptr(payload_ptr) }
        .to_str()
        .map_err(|_| HostDispatchError::InvalidUtf8)?;
    Ok(raw)
}

fn get_context<'a>(ctx_ptr: *mut c_void) -> Result<&'a HostCallbackContext, HostDispatchError> {
    if ctx_ptr.is_null() {
        return Err(HostDispatchError::NullPointer);
    }

    // SAFETY: ctx_ptr 在 OtamoryxHostApiHandle 生命周期内总是指向 HostCallbackContext。
    unsafe {
        (ctx_ptr as *const HostCallbackContext)
            .as_ref()
            .ok_or(HostDispatchError::NullPointer)
    }
}

fn dispatch_callback<F>(
    ctx_ptr: *mut c_void,
    payload_ptr: *const c_char,
    f: F,
) -> OtamoryxHostStringResult
where
    F: FnOnce(&HostCallbackContext, &str) -> Result<serde_json::Value, HostDispatchError>,
{
    let execution = panic::catch_unwind(AssertUnwindSafe(|| {
        let ctx = get_context(ctx_ptr)?;
        let payload = parse_payload(payload_ptr)?;
        f(ctx, payload)
    }));

    match execution {
        Ok(Ok(value)) => OtamoryxHostStringResult::success(value),
        Ok(Err(err)) => OtamoryxHostStringResult::error(map_dispatch_error(&err), err.to_string()),
        Err(_) => {
            OtamoryxHostStringResult::error(OtamoryxSdkErrorCode::Internal, "host callback panic")
        }
    }
}

pub unsafe extern "C" fn otamoryx_host_http_request(
    ctx_ptr: *mut c_void,
    payload_ptr: *const c_char,
) -> OtamoryxHostStringResult {
    dispatch_callback(ctx_ptr, payload_ptr, |ctx, payload| {
        let request: HostHttpRequest = serde_json::from_str(payload)?;
        let _domain =
            ctx.security
                .check_http_request(&ctx.plugin_id, &request.method, &request.url)?;
        ctx.backend
            .http_request(request)
            .map_err(HostDispatchError::from)
    })
}

pub unsafe extern "C" fn otamoryx_host_db_query(
    ctx_ptr: *mut c_void,
    payload_ptr: *const c_char,
) -> OtamoryxHostStringResult {
    dispatch_callback(ctx_ptr, payload_ptr, |ctx, payload| {
        let request: HostDbQueryRequest = serde_json::from_str(payload)?;
        let _table = ctx.security.check_db_query(
            &ctx.plugin_id,
            &request.sql,
            request.table.as_deref(),
            request.write,
        )?;
        ctx.backend
            .db_query(request)
            .map_err(HostDispatchError::from)
    })
}

pub unsafe extern "C" fn otamoryx_host_fs_read(
    ctx_ptr: *mut c_void,
    payload_ptr: *const c_char,
) -> OtamoryxHostStringResult {
    dispatch_callback(ctx_ptr, payload_ptr, |ctx, payload| {
        let mut request: HostFsReadRequest = serde_json::from_str(payload)?;
        let normalized = ctx.security.check_fs_read(&ctx.plugin_id, &request.path)?;
        request.path = normalized.to_string_lossy().to_string();
        ctx.backend
            .fs_read(request)
            .map_err(HostDispatchError::from)
    })
}

pub unsafe extern "C" fn otamoryx_host_fs_write(
    ctx_ptr: *mut c_void,
    payload_ptr: *const c_char,
) -> OtamoryxHostStringResult {
    dispatch_callback(ctx_ptr, payload_ptr, |ctx, payload| {
        let mut request: HostFsWriteRequest = serde_json::from_str(payload)?;
        let normalized = ctx.security.check_fs_write(&ctx.plugin_id, &request.path)?;
        request.path = normalized.to_string_lossy().to_string();
        ctx.backend
            .fs_write(request)
            .map_err(HostDispatchError::from)
    })
}

pub unsafe extern "C" fn otamoryx_host_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }

    // SAFETY: ptr 仅由 CString::into_raw 分配，并由该函数回收一次。
    let _ = unsafe { CString::from_raw(ptr) };
}
