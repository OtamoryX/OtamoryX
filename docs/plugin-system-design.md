# OtamoryX 插件系统设计文档

**版本**: 1.0 Draft
**日期**: 2026-03-01
**状态**: 设计阶段
**参考**: LANraragi Plugin System, 现有 OtamoryX 架构

---

## 1. 设计目标与原则

### 1.1 设计目标

- **简洁性**: 参考 LANraragi 的设计哲学，让插件开发者只需关注核心业务逻辑
- **类型安全**: 充分利用 Rust 的类型系统，在编译期捕获尽可能多的错误
- **安全可控**: v1 先提供声明式权限、审批和审计；强隔离（子进程/WASM）作为后续版本目标
- **易于开发**: 提供清晰的 SDK、模板和文档，降低插件开发门槛
- **与现有架构集成**: 基于已有的 `ProcessingPipeline`、`ProcessorPool`、trait 接口等进行扩展

### 1.2 设计原则

| 原则 | 说明 |
|------|------|
| 约定优于配置 | 遵循标准目录结构和命名约定，减少样板代码 |
| 最小权限 | 插件默认无任何特殊权限，需显式声明所需能力 |
| 渐进式复杂度 | 简单插件只需实现一个函数，高级功能按需引入 |
| 稳定 ABI | 通过 C FFI + JSON 消息传递避免 Rust ABI 不稳定问题 |
| 故障隔离 | 单个插件的错误不应导致主系统崩溃 |

### 1.3 v1 边界声明

v1 动态库插件运行在主进程内，属于 `trusted plugin` 模式。  
这意味着:

- 权限声明用于**安装审核、管理员确认、审计告警和 SDK 限制**
- 权限声明**不等价于 OS 级强沙箱**
- 仅建议安装可信来源插件

强隔离（子进程执行或 WASM 运行时）放在 v2+ 版本。

---

## 2. 与 LANraragi 的对比分析

### 2.1 LANraragi 插件系统概览

LANraragi 的插件系统以**极简**著称:

- **4 种插件类型**: Metadata（元数据）、Login（登录）、Download（下载）、Script（脚本）
- **Perl 模块**: 每个插件是一个 `.pm` 文件，实现 `plugin_info()` + 一个类型特定函数
- **自动发现**: 基于 `Module::Pluggable` 扫描特定目录下的模块
- **配置存储**: Redis 哈希，参数类型仅 `bool`/`string`/`int`
- **执行方式**: 同步调用或通过 Minion 任务队列异步执行
- **零沙箱**: 插件与主系统共享同一进程，拥有完全的系统访问权限

### 2.2 我们借鉴什么

| LANraragi 特性 | OtamoryX 适配方案 |
|---------------|------------------|
| 按类型分类插件 | 保留，定义 6 种插件类型对应我们的领域需求 |
| `plugin_info()` 元数据函数 | 保留，改为 `plugin.toml` 声明式配置 + `Plugin` trait |
| 单一类型特定函数 | 保留核心理念，每种类型对应一个核心 trait |
| 参数配置系统 | 扩展为 schema 驱动配置（v1 为 JSON Schema 子集） |
| 目录扫描发现 | 保留，扫描 `plugins/` 目录下的 `.so`/`.dll`/`.dylib` |
| One-shot 参数 | 保留，支持手动执行时传入临时参数 |
| 自动执行（新文件触发） | 保留，集成到 `FileMonitorService` 和 `ProcessingPipeline` |

### 2.3 我们改进什么

| 改进点 | 说明 |
|--------|------|
| 安全可控 | LANraragi 无任何限制，我们引入权限声明 + 审批 + 审计 + 运行时保护 |
| 异步优先 | LANraragi 大量同步操作，我们全面 async/await |
| 结构化输出 | LANraragi 返回松散的 hash，我们使用强类型 JSON |
| 插件间通信 | LANraragi 的 Login 插件通过 `login_from` 关联，我们设计通用的依赖声明 |
| 开发体验 | 提供 Rust crate SDK、项目模板、热重载支持 |

---

## 3. 插件类型定义

### 3.1 类型总览

基于 OtamoryX 的核心领域和 LANraragi 的经验，定义以下 6 种插件类型:

```
┌─────────────────────────────────────────────────────────┐
│                    OtamoryX 插件类型                       │
├──────────────┬──────────────────────────────────────────┤
│ Metadata     │ 从外部来源获取/生成档案的标签和元数据          │
│ Download     │ 从 URL 下载档案文件到本地                    │
│ Processor    │ 对档案内容进行处理（图像优化、格式转换等）       │
│ Analyzer     │ 内容分析（AI标签、相似度检测、内容分类）        │
│ Script       │ 通用脚本（批量操作、数据迁移、维护任务）        │
│ Endpoint     │ 注册自定义 API 端点，扩展系统功能              │
└──────────────┴──────────────────────────────────────────┘
```

### 3.2 各类型详细定义

#### 3.2.1 Metadata 插件

**用途**: 类似 LANraragi 的 Metadata 插件，从文件名、内嵌信息或外部网站获取元数据。

**核心接口**:
```rust
#[async_trait]
pub trait MetadataPlugin: Plugin {
    /// 从档案提取/获取元数据
    /// archive_info: 档案的基本信息（ID、标题、路径、已有标签等）
    /// params: 用户配置的参数
    /// oneshot: 手动执行时传入的临时参数（如URL、ID等）
    async fn get_tags(
        &self,
        ctx: &PluginContext,
        archive_info: &ArchiveInfo,
    ) -> Result<MetadataResult, PluginError>;
}

pub struct ArchiveInfo {
    pub id: String,
    pub title: String,
    pub file_path: String,
    pub file_hash: String,
    pub existing_tags: Vec<Tag>,
    pub page_count: i32,
    pub oneshot_param: Option<String>,
}

pub struct MetadataResult {
    pub tags: Vec<TagEntry>,        // namespace:name 格式
    pub title: Option<String>,      // 新标题（可选）
    pub summary: Option<String>,    // 摘要（可选）
    pub source_url: Option<String>, // 来源URL（可选）
}

pub struct TagEntry {
    pub namespace: String,  // artist, character, parody, language, general, ...
    pub name: String,
    pub confidence: Option<f32>,  // 置信度（0.0-1.0），None 表示确定
}
```

**执行时机**:
- 新档案导入时自动执行（如果启用了 auto-run）
- 用户在档案详情页手动触发
- 通过 API 批量触发

**典型场景**:
- 从文件名解析 `[Artist] Title (Series)` 格式
- 读取 ComicInfo.xml 内嵌元数据
- 调用 EHentai/nhentai 等网站 API 获取标签
- 基于正则表达式规则自动打标

#### 3.2.2 Download 插件

**用途**: 类似 LANraragi 的 Download 插件，给定 URL 后提供下载能力。

**核心接口**:
```rust
#[async_trait]
pub trait DownloadPlugin: Plugin {
    /// 此插件能处理的 URL 模式（正则表达式）
    fn url_pattern(&self) -> &str;

    /// 给定一个 URL，返回可下载的直链或本地文件路径
    async fn provide_url(
        &self,
        ctx: &PluginContext,
        url: &str,
    ) -> Result<DownloadResult, PluginError>;
}

pub enum DownloadResult {
    /// 返回直接下载链接
    DirectUrl {
        url: String,
        filename: Option<String>,
        headers: Option<HashMap<String, String>>,
    },
    /// 插件已下载文件到本地
    LocalFile {
        path: String,
        filename: Option<String>,
    },
}
```

#### 3.2.3 Processor 插件

**用途**: 对档案文件或其中的图像进行处理，如优化、转换、水印等。

**核心接口**:
```rust
#[async_trait]
pub trait ProcessorPlugin: Plugin {
    /// 处理单个档案
    async fn process_archive(
        &self,
        ctx: &PluginContext,
        archive_info: &ArchiveInfo,
        pages: &[PageData],
    ) -> Result<ProcessingResult, PluginError>;

    /// 支持的档案格式
    fn supported_formats(&self) -> Vec<String>;
}

pub struct PageData {
    pub index: usize,
    pub filename: String,
    pub data: Vec<u8>,
    pub mime_type: String,
}

pub struct ProcessingResult {
    pub pages_processed: usize,
    pub pages_failed: usize,
    pub output_files: Vec<OutputFile>,
    pub stats: HashMap<String, serde_json::Value>,
}
```

#### 3.2.4 Analyzer 插件

**用途**: 内容分析类插件，用于 AI 标签生成、相似度检测等。与 Metadata 的区别在于 Analyzer 侧重于分析图像内容本身，而 Metadata 侧重于从外部来源获取已有的元数据。

**核心接口**:
```rust
#[async_trait]
pub trait AnalyzerPlugin: Plugin {
    /// 分析档案内容
    async fn analyze(
        &self,
        ctx: &PluginContext,
        archive_info: &ArchiveInfo,
        sample_pages: &[PageData],  // 采样页面，而非全部
    ) -> Result<AnalysisResult, PluginError>;

    /// 此插件是否需要采样全部页面
    fn requires_all_pages(&self) -> bool { false }

    /// 采样数量（默认取前3页 + 随机2页）
    fn sample_count(&self) -> usize { 5 }
}

pub struct AnalysisResult {
    pub tags: Vec<TagEntry>,
    pub categories: Vec<String>,        // 建议分类
    pub content_rating: Option<String>,  // safe / questionable / explicit
    pub similar_archives: Vec<SimilarArchive>,
    pub extra: HashMap<String, serde_json::Value>,
}
```

#### 3.2.5 Script 插件

**用途**: 类似 LANraragi 的 Script 插件，执行通用的一次性或定时任务。

**核心接口**:
```rust
#[async_trait]
pub trait ScriptPlugin: Plugin {
    /// 执行脚本
    async fn run(
        &self,
        ctx: &PluginContext,
        oneshot_param: Option<&str>,
    ) -> Result<ScriptResult, PluginError>;

    /// 定时执行的 cron 表达式（可选）
    fn schedule(&self) -> Option<&str> { None }
}

pub struct ScriptResult {
    pub message: String,
    pub data: serde_json::Value,
}
```

**典型场景**:
- 批量从旧系统迁移数据
- 定期清理临时文件
- 批量重新扫描标签
- 数据库维护和统计

#### 3.2.6 Endpoint 插件

**用途**: 注册自定义 HTTP API 端点，提供独立的 Web 功能。

**核心接口**:
```rust
#[async_trait]
pub trait EndpointPlugin: Plugin {
    /// 返回此插件注册的路由定义
    fn routes(&self) -> Vec<RouteDefinition>;

    /// 处理请求
    async fn handle_request(
        &self,
        ctx: &PluginContext,
        request: PluginRequest,
    ) -> Result<PluginResponse, PluginError>;
}

pub struct RouteDefinition {
    pub method: HttpMethod,     // GET, POST, PUT, DELETE
    pub path: String,           // 相对路径，如 "/status"
    pub description: String,
}

// 所有 Endpoint 插件的路由统一挂载在:
// /api/v1/plugins/{plugin_id}/api/...
```

---

## 4. 插件结构与规范

### 4.1 插件目录结构

```
my-plugin/
├── plugin.toml          # 插件元数据和配置声明（必须）
├── Cargo.toml           # Rust 项目配置
├── src/
│   └── lib.rs           # 插件入口（必须）
├── README.md            # 插件文档（推荐）
└── tests/               # 测试（推荐）
```

### 4.2 plugin.toml 规范

`plugin.toml` 是插件的声明文件，类似 LANraragi 的 `plugin_info()` 但更结构化:

```toml
[plugin]
# === 必填字段 ===
id = "ehentai-metadata"              # 稳定标识符（小写，连字符分隔，不可变）
name = "E-Hentai Metadata"           # 显示名称（可变）
version = "1.0.0"                    # 语义化版本
type = "metadata"                    # metadata/download/processor/analyzer/script/endpoint
description = "从 E-Hentai 获取档案元数据和标签"
author = "OtamoryX Community"
manifest_version = 1                 # plugin.toml 结构版本
plugin_api_version = 1               # FFI/SDK ABI 版本（必须匹配主程序）

# === 可选字段 ===
icon = "icon.png"                    # 插件图标（相对路径或 data:image URI）
homepage = "https://github.com/..."
license = "MIT"
min_app_version = "0.6.5"            # 最低兼容的 OtamoryX 版本
cooldown = 3                         # 执行冷却时间（秒），用于避免频繁请求
oneshot_arg = "E-Hentai Gallery URL or ID"  # one-shot 参数描述

# === 插件依赖 ===
[plugin_dependencies]
# 依赖其他插件（如 Metadata 插件依赖 Login 插件）
login_from = "ehentai-login"         # 类似 LANraragi 的 login_from

# === 权限声明 ===
[permissions]
network = ["api.e-hentai.org", "exhentai.org"]  # 允许访问的域名列表，空数组=无网络
filesystem_read = []                  # 允许读取的路径模式
filesystem_write = []                 # 允许写入的路径模式
database_read = true                  # 是否可读数据库
database_write = ["tags", "archive_tags"]  # 可写的表列表

# === 用户可配置参数（JSON Schema 子集）===
[config_schema]
type = "object"
additional_properties = false
required = ["use_exhentai", "tag_languages", "max_retries"]

[config_schema.properties.use_exhentai]
type = "boolean"
title = "使用 ExHentai"
description = "使用 ExHentai 而非 E-Hentai（需要登录插件）"
default = false

[config_schema.properties.tag_languages]
type = "string"
title = "标签语言"
description = "优先使用的标签语言（逗号分隔）"
default = "japanese,chinese,english"

[config_schema.properties.fetch_covers]
type = "boolean"
title = "获取封面"
description = "是否从 E-Hentai 下载封面替换本地封面"
default = false

[config_schema.properties.max_retries]
type = "integer"
title = "最大重试次数"
description = "API 请求失败时的最大重试次数"
default = 3
minimum = 0
maximum = 10

# === URL 匹配（仅 Download 类型）===
[download]
url_regex = "https?://(e-hentai|exhentai)\\.org/g/\\d+/[a-f0-9]+/"

# === 定时任务（仅 Script 类型）===
[schedule]
cron = "0 3 * * *"      # 每天凌晨3点执行
timezone = "UTC"
```

> 注意: Rust crate 依赖只能写在 `Cargo.toml` 的 `[dependencies]`，不能写在 `plugin.toml`。

### 4.3 插件入口规范（C FFI 接口）

为了解决 Rust ABI 不稳定的问题，插件通过 C FFI 导出一组标准函数，主系统通过 `libloading` 加载:

```rust
// === 必须导出的函数 ===

/// 创建插件实例，返回不透明指针
/// config_json: 用户配置参数的 JSON 字符串
/// host_api: 宿主能力回调表（OtamoryxHostApiV1）
#[no_mangle]
pub extern "C" fn otamoryx_plugin_create(
    config_json: *const c_char,
    host_api: *const c_void,
) -> *mut c_void;

/// 销毁插件实例
#[no_mangle]
pub extern "C" fn otamoryx_plugin_destroy(
    instance: *mut c_void,
);

/// 获取插件信息（返回 JSON 字符串，调用方负责释放）
/// 必须包含: id, version, plugin_api_version, manifest_version
#[no_mangle]
pub extern "C" fn otamoryx_plugin_info() -> *mut c_char;

/// 释放插件返回的字符串
#[no_mangle]
pub extern "C" fn otamoryx_free_string(s: *mut c_char);

// 主程序加载插件时必须执行:
// 1) 读取 plugin.toml 的 plugin_api_version
// 2) 调用 otamoryx_plugin_info() 并核对 plugin_api_version
// 3) 两者都必须与主程序支持版本一致，否则拒绝加载

// === 类型特定函数（按 plugin.toml 中的 type 选择） ===

/// Metadata 插件: 获取标签
#[no_mangle]
pub extern "C" fn otamoryx_get_tags(
    instance: *mut c_void,
    archive_info_json: *const c_char,
    context_json: *const c_char,
) -> *mut c_char;  // 返回 MetadataResult JSON

/// Download 插件: 获取下载链接
#[no_mangle]
pub extern "C" fn otamoryx_provide_url(
    instance: *mut c_void,
    url: *const c_char,
    context_json: *const c_char,
) -> *mut c_char;  // 返回 DownloadResult JSON

/// Processor 插件: 处理档案
#[no_mangle]
pub extern "C" fn otamoryx_process_archive(
    instance: *mut c_void,
    archive_info_json: *const c_char,
    context_json: *const c_char,
) -> *mut c_char;  // 返回 ProcessingResult JSON

/// Analyzer 插件: 分析内容
#[no_mangle]
pub extern "C" fn otamoryx_analyze(
    instance: *mut c_void,
    archive_info_json: *const c_char,
    sample_pages_json: *const c_char,
    context_json: *const c_char,
) -> *mut c_char;  // 返回 AnalysisResult JSON

/// Script 插件: 执行脚本
#[no_mangle]
pub extern "C" fn otamoryx_run_script(
    instance: *mut c_void,
    oneshot_param: *const c_char,  // 可为 null
    context_json: *const c_char,
) -> *mut c_char;  // 返回 ScriptResult JSON

/// Endpoint 插件: 获取路由定义
#[no_mangle]
pub extern "C" fn otamoryx_get_routes(
    instance: *mut c_void,
) -> *mut c_char;  // 返回 Vec<RouteDefinition> JSON

/// Endpoint 插件: 处理请求
#[no_mangle]
pub extern "C" fn otamoryx_handle_request(
    instance: *mut c_void,
    request_json: *const c_char,
    context_json: *const c_char,
) -> *mut c_char;  // 返回 PluginResponse JSON
```

### 4.4 插件 SDK（otamoryx-plugin-sdk crate）

提供一个 SDK crate 让开发者不必直接处理 FFI:

```rust
// 在 Cargo.toml 中:
// [dependencies]
// otamoryx-plugin-sdk = "0.1"

use otamoryx_plugin_sdk::prelude::*;

pub struct EHentaiMetadata {
    config: MyConfig,
    client: HttpClient,  // SDK 提供的受限 HTTP 客户端
}

// 使用宏自动生成 FFI 入口函数
#[otamoryx_plugin]
impl Plugin for EHentaiMetadata {
    fn new(config: serde_json::Value) -> Result<Self, PluginError> {
        let config: MyConfig = serde_json::from_value(config)?;
        Ok(Self {
            config,
            client: HttpClient::new(),
        })
    }
}

#[otamoryx_metadata]  // 自动生成 otamoryx_get_tags FFI 函数
#[async_trait]
impl MetadataPlugin for EHentaiMetadata {
    async fn get_tags(
        &self,
        ctx: &PluginContext,
        archive_info: &ArchiveInfo,
    ) -> Result<MetadataResult, PluginError> {
        let url = archive_info.oneshot_param
            .as_deref()
            .ok_or(PluginError::MissingParam("需要 E-Hentai URL".into()))?;

        let response = self.client.get(url).await?;
        let tags = parse_ehentai_tags(&response)?;

        Ok(MetadataResult {
            tags,
            title: Some(parsed_title),
            summary: None,
            source_url: Some(url.to_string()),
        })
    }
}
```

**SDK 提供的核心能力**:
- `HttpClient` — 受权限控制的 HTTP 客户端（仅允许 `plugin.toml` 声明的域名）
- `PluginLogger` — 结构化日志（写入 `data/logs/plugins/{name}.log`）
- `PluginContext` — 包含配置读取、临时目录等基础信息；HTTP/DB/FS 通过 host callback 句柄访问
- 过程宏 `#[otamoryx_plugin]` / `#[otamoryx_metadata]` 等，自动生成 FFI 胶水代码
- 类型定义（`ArchiveInfo`, `MetadataResult`, `TagEntry` 等）
- 错误类型和工具函数

---

## 5. 插件生命周期

### 5.1 生命周期状态机

```
                    ┌──────────┐
                    │ Uploaded │  用户上传插件包
                    └────┬─────┘
                         │ validate()
                         ▼
                    ┌──────────┐
                    │ Validated│  通过安全检查
                    └────┬─────┘
                         │ install()
                         ▼
                    ┌──────────┐
              ┌────►│ Disabled │◄─────────┐
              │     └────┬─────┘          │
              │          │ enable()       │ disable()
              │          ▼                │
              │     ┌──────────┐          │
              │     │ Loading  │──────────┤ (加载失败)
              │     └────┬─────┘          │
              │          │ loaded         │
              │          ▼                │
              │     ┌──────────┐          │
              │     │ Enabled  │──────────┘
              │     └────┬─────┘
              │          │ uninstall()
              │          ▼
              │     ┌──────────┐
              └─────│Uninstalled│
                    └──────────┘
```

### 5.2 各阶段详细说明

| 阶段 | 操作 | 说明 |
|------|------|------|
| **上传** | 用户通过 Web UI 或 API 上传 `.tar.gz` 插件包 | 插件包解压到临时目录 |
| **验证** | 检查 `plugin.toml` 合法性、动态库签名、权限声明 | 拒绝无效或危险的插件 |
| **安装** | 将插件文件复制到 `data/plugins/{name}/`，写入数据库记录 | 插件进入 Disabled 状态 |
| **启用** | `libloading` 加载动态库，调用 `otamoryx_plugin_create` | 创建插件实例，注册到 PluginManager |
| **执行** | 根据类型调用对应的 FFI 函数 | 通过 JSON 传递参数和返回值 |
| **禁用** | 调用 `otamoryx_plugin_destroy`，卸载动态库 | 清理资源，注销钩子 |
| **卸载** | 删除插件文件和数据库记录 | 保留或清理插件产生的数据（用户选择） |
| **更新** | 禁用旧版 → 替换文件 → 启用新版 | 保留用户配置 |

---

## 6. 核心架构设计

### 6.1 系统架构图

```
┌─────────────────────────────────────────────────────────────┐
│                        Web UI / API                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │ Plugin Config│  │ Plugin Store │  │ One-shot Execute │  │
│  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘  │
└─────────┼─────────────────┼───────────────────┼─────────────┘
          │                 │                   │
          ▼                 ▼                   ▼
┌─────────────────────────────────────────────────────────────┐
│                     Plugin API Layer                         │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                  PluginManager                        │   │
│  │  ┌─────────┐ ┌──────────┐ ┌───────────┐ ┌────────┐  │   │
│  │  │Registry │ │ Executor │ │ Scheduler │ │ Config │  │   │
│  │  └─────────┘ └──────────┘ └───────────┘ └────────┘  │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              PluginSecurity                           │   │
│  │  ┌───────────┐ ┌────────────┐ ┌──────────────────┐  │   │
│  │  │Permission │ │ Validator  │ │ RuntimeGuard     │  │   │
│  │  │ Checker   │ │            │ │                  │  │   │
│  │  └───────────┘ └────────────┘ └──────────────────┘  │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────┬───────────────────────────────────┘
                          │  C FFI (JSON messages)
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    Plugin Instances                           │
│  ┌──────────┐ ┌───────────┐ ┌──────────┐ ┌──────────────┐  │
│  │ Metadata │ │ Download  │ │Processor │ │ Analyzer     │  │
│  │ Plugins  │ │ Plugins   │ │ Plugins  │ │ Plugins      │  │
│  └──────────┘ └───────────┘ └──────────┘ └──────────────┘  │
│  ┌──────────┐ ┌───────────┐                                 │
│  │ Script   │ │ Endpoint  │                                 │
│  │ Plugins  │ │ Plugins   │                                 │
│  └──────────┘ └───────────┘                                 │
└─────────────────────────────────────────────────────────────┘
```

### 6.2 PluginManager 服务

`PluginManager` 是插件系统的核心，负责管理所有插件的生命周期:

```rust
// backend/src/services/plugin_manager.rs

pub struct PluginManager {
    /// 已加载的插件实例（plugin_id → PluginInstance）
    loaded: HashMap<String, PluginInstance>,

    /// 插件注册表（所有已安装的插件元信息）
    registry: PluginRegistry,

    /// 插件执行器
    executor: PluginExecutor,

    /// 定时任务调度器
    scheduler: PluginScheduler,

    /// 安全检查器
    security: PluginSecurity,

    /// 数据库连接
    db: Pool<Sqlite>,

    /// 插件目录路径
    plugins_dir: PathBuf,
}

struct PluginInstance {
    /// 动态库句柄
    lib: libloading::Library,

    /// 插件不透明指针（由 otamoryx_plugin_create 返回）
    handle: *mut c_void,

    /// 从 plugin.toml 解析的元信息
    manifest: PluginManifest,

    /// 当前状态
    status: PluginStatus,

    /// FFI 函数指针缓存（原始函数指针，避免在结构体中持有 Symbol 的生命周期问题）
    ffi: PluginFfi,
}

/// 缓存的 FFI 函数指针
struct PluginFfi {
    destroy: unsafe extern "C" fn(*mut c_void),
    info: unsafe extern "C" fn() -> *mut c_char,
    free_string: unsafe extern "C" fn(*mut c_char),
    // 类型特定函数指针（按需加载）
    get_tags: Option<unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char) -> *mut c_char>,
    provide_url: Option<unsafe extern "C" fn(...) -> *mut c_char>,
    process_archive: Option<unsafe extern "C" fn(...) -> *mut c_char>,
    analyze: Option<unsafe extern "C" fn(...) -> *mut c_char>,
    run_script: Option<unsafe extern "C" fn(...) -> *mut c_char>,
    get_routes: Option<unsafe extern "C" fn(...) -> *mut c_char>,
    handle_request: Option<unsafe extern "C" fn(...) -> *mut c_char>,
}

impl PluginManager {
    /// 启动时扫描并加载所有已启用的插件
    pub async fn initialize(&mut self) -> Result<()>;

    /// 安装新插件
    pub async fn install(&mut self, package: &Path) -> Result<PluginManifest>;

    /// 卸载插件
    pub async fn uninstall(&mut self, plugin_id: &str) -> Result<()>;

    /// 启用/禁用插件
    pub async fn set_enabled(&mut self, plugin_id: &str, enabled: bool) -> Result<()>;

    /// 更新插件配置
    pub async fn configure(&mut self, plugin_id: &str, config: Value) -> Result<()>;

    /// 获取所有插件信息
    pub fn list_plugins(&self) -> Vec<PluginInfo>;

    /// 获取指定类型的已启用插件
    pub fn get_enabled_plugins(&self, plugin_type: PluginType) -> Vec<&PluginInstance>;

    /// 根据 URL 查找匹配的 Download 插件
    pub fn find_downloader_for_url(&self, url: &str) -> Option<&PluginInstance>;

    /// 执行 Metadata 插件
    pub async fn exec_metadata(
        &self,
        plugin_id: &str,
        archive_info: &ArchiveInfo,
    ) -> Result<MetadataResult>;

    /// 对档案执行所有已启用的 Metadata 插件
    pub async fn exec_all_metadata(
        &self,
        archive_info: &ArchiveInfo,
    ) -> Result<Vec<(String, MetadataResult)>>;

    /// 执行 Script 插件
    pub async fn exec_script(
        &self,
        plugin_id: &str,
        oneshot: Option<&str>,
    ) -> Result<ScriptResult>;
}
```

### 6.3 PluginRegistry

```rust
pub struct PluginRegistry {
    /// 已安装的插件清单（从数据库和文件系统加载）
    manifests: HashMap<String, PluginManifest>,
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
    pub icon: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub min_app_version: Option<String>,
    pub cooldown: Option<u32>,
    pub oneshot_arg: Option<String>,
    pub plugin_dependencies: PluginDependencies,
    pub permissions: PluginPermissions,
    pub config_schema: serde_json::Value,
    pub download_config: Option<DownloadConfig>,
    pub schedule_config: Option<ScheduleConfig>,
}
```

### 6.4 PluginExecutor

```rust
pub struct PluginExecutor {
    /// 执行超时时间（默认 30 秒）
    default_timeout: Duration,

    /// 执行记录（用于冷却控制）
    last_execution: HashMap<String, Instant>,
}

impl PluginExecutor {
    /// 通过 FFI 调用插件函数（在 tokio::task::spawn_blocking 中执行）
    pub async fn call_ffi<R: DeserializeOwned>(
        &self,
        instance: &PluginInstance,
        func_name: &str,
        args: &[&str],
        timeout: Duration,
    ) -> Result<R>;

    /// 检查冷却时间
    pub fn check_cooldown(&self, plugin_id: &str, cooldown: u32) -> bool;
}
```

### 6.5 与现有系统的集成点

#### 6.5.1 与 ProcessingPipeline 集成

修改 `processing_pipeline.rs`，在档案处理流程中加入插件调用:

```rust
impl ProcessingPipeline {
    pub async fn process_archive(&self, archive_path: &Path) -> Result<(), ProcessingError> {
        // ... 现有的扫描和去重逻辑 ...

        let archive_id = self.storage.create_archive_record(&scan_result).await?;

        // 现有任务
        let mut tasks = vec![
            ProcessingTask::new(archive_id.clone(), TaskType::MetadataExtraction, 1),
            ProcessingTask::new(archive_id.clone(), TaskType::ThumbnailGeneration, 1),
        ];

        // === 新增：插件任务 ===
        // 获取所有启用的 Metadata 插件
        for plugin in self.plugin_manager.get_enabled_plugins(PluginType::Metadata) {
            tasks.push(ProcessingTask::new(
                archive_id.clone(),
                TaskType::PluginMetadata(plugin.manifest.id.clone()),
                2, // 低于内置任务的优先级
            ));
        }

        // 获取所有启用的 Analyzer 插件
        for plugin in self.plugin_manager.get_enabled_plugins(PluginType::Analyzer) {
            tasks.push(ProcessingTask::new(
                archive_id.clone(),
                TaskType::PluginAnalysis(plugin.manifest.id.clone()),
                0, // 最低优先级
            ));
        }

        for task in tasks {
            self.task_queue.enqueue(task).await;
        }

        Ok(())
    }
}
```

#### 6.5.2 与 AppState 集成

```rust
pub struct AppState {
    pub db: DatabasePool,
    pub file_monitor: Arc<FileMonitorService>,
    pub archive_cache: Arc<ArchiveCacheService>,
    pub plugin_manager: Arc<RwLock<PluginManager>>,  // 新增
}
```

#### 6.5.3 与路由系统集成

```rust
// Endpoint 插件的路由动态注册
let plugin_routes = Router::new()
    .route(
        "/api/v1/plugins/:plugin_id/api/*path",
        any(plugin_endpoint_handler),
    );

// plugin_endpoint_handler 负责:
// 1. 从 URL 中提取 plugin_id
// 2. 在 PluginManager 中查找对应的 Endpoint 插件
// 3. 构造 PluginRequest
// 4. 调用 otamoryx_handle_request FFI
// 5. 将 PluginResponse 转换为 Axum Response
```

#### 6.5.4 与 FileMonitorService 集成

```rust
// 在 FileMonitorService 检测到新文件时
impl FileMonitorService {
    async fn on_new_file(&self, path: &Path) {
        // 现有处理逻辑...

        // 新增：通知插件系统
        // 已在 ProcessingPipeline 中通过任务队列处理
    }
}
```

---

## 7. 安全模型

### 7.1 权限系统

```
┌──────────────────────────────────────┐
│          Permission Layers           │
├──────────────────────────────────────┤
│  Layer 1: 声明式权限 (plugin.toml)    │  安装时审核
│  Layer 2: 管理员审批                   │  启用时确认
│  Layer 3: 运行时检查                   │  每次执行时验证
└──────────────────────────────────────┘
```

**v1 安全边界（必须明确）**:

| 运行模式 | 隔离级别 | v1 状态 |
|------|------|------|
| 动态库同进程 | 无内存/系统调用硬隔离 | ✅ 默认 |
| 子进程沙箱 | 进程级隔离 | ⏳ v2+ |
| WASM 运行时 | 沙箱隔离 | ⏳ v2+ |

**权限类型**:

| 权限 | 说明 | 检查时机 |
|------|------|---------|
| `network` | 网络访问，指定允许的域名列表 | HTTP 请求前 |
| `filesystem_read` | 文件读取，指定允许的路径模式 | 文件操作前 |
| `filesystem_write` | 文件写入，指定允许的路径模式 | 文件操作前 |
| `database_read` | 数据库读取 | SQL 执行前 |
| `database_write` | 数据库写入，指定允许的表 | SQL 执行前 |
| `custom_endpoints` | 注册自定义 API 端点 | 加载时 |
| `scheduled_tasks` | 注册定时任务 | 加载时 |

### 7.2 安全检查流程

```rust
pub struct PluginSecurity {
    /// 权限策略
    policy: SecurityPolicy,
}

impl PluginSecurity {
    /// 安装时验证
    pub fn validate_package(&self, package: &Path) -> Result<ValidationReport>;

    /// 检查网络访问权限
    pub fn check_network(&self, plugin: &str, domain: &str) -> bool;

    /// 检查文件系统访问权限
    pub fn check_filesystem(&self, plugin: &str, path: &str, write: bool) -> bool;

    /// 检查数据库写入权限
    pub fn check_database_write(&self, plugin: &str, table: &str) -> bool;
}

pub struct ValidationReport {
    pub valid: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub permissions_summary: String,
}
```

### 7.3 插件隔离策略

由于 Rust 动态库在同一进程空间运行，v1 不提供强隔离。我们采用以下防护策略:

1. **超时控制**: 每个 FFI 调用都有超时限制，通过 `tokio::time::timeout` 实现
2. **panic 捕获**: FFI 调用包裹在 `std::panic::catch_unwind` 中
3. **权限网关**: 对使用 SDK 的插件进行域名/路径/表级访问控制
4. **资源限制**: 通过配置限制插件可使用的内存和 CPU 时间
5. **审计日志**: 所有插件操作记录到日志，便于事后追溯
6. **部署建议**: 生产环境仅安装可信插件；不可信插件等待 v2 沙箱能力

```rust
/// FFI 调用的安全包装
async fn safe_ffi_call<R: DeserializeOwned>(
    instance: &PluginInstance,
    call: impl FnOnce() -> *mut c_char + Send + 'static,
    timeout: Duration,
) -> Result<R, PluginError> {
    let result = tokio::time::timeout(
        timeout,
        tokio::task::spawn_blocking(move || {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(call))
        }),
    )
    .await
    .map_err(|_| PluginError::Timeout)?
    .map_err(|e| PluginError::TaskFailed(e.to_string()))?
    .map_err(|_| PluginError::PluginPanicked)?;

    // 解析 JSON 返回值
    let json_str = unsafe { CStr::from_ptr(result) }.to_str()?;
    let parsed: R = serde_json::from_str(json_str)?;

    // 释放插件分配的字符串
    (instance.ffi.free_string)(result);

    Ok(parsed)
}
```

---

## 8. 数据库设计

### 8.1 新增/修改表

```sql
-- 插件注册表（修改现有 plugins 表）
CREATE TABLE IF NOT EXISTS plugins (
    id TEXT PRIMARY KEY,                -- 稳定插件标识符（plugin_id）
    name TEXT NOT NULL,                 -- 显示名称
    version TEXT NOT NULL,
    manifest_version INTEGER NOT NULL DEFAULT 1,
    plugin_api_version INTEGER NOT NULL DEFAULT 1,
    plugin_type TEXT NOT NULL,         -- metadata/download/processor/analyzer/script/endpoint
    description TEXT,
    author TEXT,
    icon TEXT,                          -- Base64 图标或文件路径
    enabled INTEGER NOT NULL DEFAULT 0,
    config TEXT,                        -- 用户配置参数 JSON
    permissions TEXT,                   -- 权限声明 JSON
    manifest TEXT,                      -- 完整的 plugin.toml 内容 JSON
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_executed_at TIMESTAMP,         -- 最后执行时间
    execution_count INTEGER DEFAULT 0   -- 累计执行次数
);

-- 插件执行记录
CREATE TABLE IF NOT EXISTS plugin_executions (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    archive_id TEXT REFERENCES archives(id) ON DELETE SET NULL,
    execution_type TEXT NOT NULL,        -- auto/manual/scheduled/api
    status TEXT NOT NULL,                -- pending/running/success/failed/timeout
    input_summary TEXT,                  -- 输入参数摘要
    output_summary TEXT,                 -- 输出结果摘要
    error_message TEXT,
    duration_ms INTEGER,
    started_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP
);

CREATE INDEX idx_plugin_executions_plugin ON plugin_executions(plugin_id);
CREATE INDEX idx_plugin_executions_archive ON plugin_executions(archive_id);
CREATE INDEX idx_plugin_executions_status ON plugin_executions(status);

-- 插件产生的标签记录（用于审计和回滚）
CREATE TABLE IF NOT EXISTS plugin_tags (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    archive_id TEXT NOT NULL REFERENCES archives(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    confidence REAL,                     -- 置信度 0.0-1.0
    approved INTEGER,                    -- NULL=待审核, 1=已批准, 0=已拒绝
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_plugin_tags_plugin ON plugin_tags(plugin_id);
CREATE INDEX idx_plugin_tags_archive ON plugin_tags(archive_id);
CREATE INDEX idx_plugin_tags_approved ON plugin_tags(approved);
```

### 8.2 与现有 AI 标签表的关系

现有的 `ai_generated_tags` 和 `ai_processing_queue` 表可以视为 Analyzer 类型插件的特化。
在插件系统完成后，AI 标签功能可以迁移为内置的 Analyzer 插件，复用 `plugin_tags` 和 `plugin_executions` 表。

---

## 9. API 设计

### 9.1 插件管理 API

```
# 插件列表
GET    /api/v1/plugins                           → 获取所有已安装插件
GET    /api/v1/plugins?type=metadata              → 按类型筛选
GET    /api/v1/plugins/:id                        → 获取单个插件详情

# 插件安装和管理
POST   /api/v1/plugins/install                    → 上传安装插件包（multipart/form-data, field: plugin）
DELETE /api/v1/plugins/:id                        → 卸载插件
PUT    /api/v1/plugins/:id/toggle                 → 启用/禁用插件
PUT    /api/v1/plugins/:id/config                 → 更新插件配置
GET    /api/v1/plugins/:id/config/schema          → 获取配置 schema

# 插件执行
POST   /api/v1/plugins/:id/execute                → 手动执行插件（通用）
POST   /api/v1/plugins/:id/execute/:archive_id    → 对特定档案执行插件

# 执行记录
GET    /api/v1/plugins/:id/executions             → 获取执行历史
GET    /api/v1/plugin-executions                  → 获取所有插件执行历史

# 插件标签审核
GET    /api/v1/plugin-tags?approved=null           → 获取待审核标签
PUT    /api/v1/plugin-tags/:id/approve             → 批准标签
PUT    /api/v1/plugin-tags/:id/reject              → 拒绝标签
POST   /api/v1/plugin-tags/batch-approve           → 批量审核

# Endpoint 插件的自定义路由
ANY    /api/v1/plugins/:id/api/*                   → 转发给 Endpoint 插件处理
```

### 9.2 请求/响应示例

**获取插件列表**:
```json
GET /api/v1/plugins

Response:
{
  "data": [
    {
      "id": "ehentai-metadata",
      "name": "E-Hentai Metadata",
      "version": "1.0.0",
      "type": "metadata",
      "description": "从 E-Hentai 获取档案元数据和标签",
      "author": "OtamoryX Community",
      "enabled": true,
      "execution_count": 142,
      "last_executed_at": "2026-03-01T10:30:00Z",
      "config": {
        "use_exhentai": false,
        "tag_languages": "japanese,chinese,english"
      }
    }
  ]
}
```

**手动执行 Metadata 插件**:
```json
POST /api/v1/plugins/ehentai-metadata/execute/archive-uuid-123

Body:
{
  "oneshot_param": "https://e-hentai.org/g/12345/abcdef/"
}

Response:
{
  "execution_id": "exec-uuid-456",
  "status": "success",
  "result": {
    "tags": [
      {"namespace": "artist", "name": "author_name", "confidence": null},
      {"namespace": "parody", "name": "series_name", "confidence": null},
      {"namespace": "character", "name": "char_name", "confidence": null}
    ],
    "title": "New Title From EH",
    "source_url": "https://e-hentai.org/g/12345/abcdef/"
  },
  "duration_ms": 1250
}
```

**更新插件配置**:
```json
PUT /api/v1/plugins/ehentai-metadata/config

Body:
{
  "config": {
    "use_exhentai": false,
    "tag_languages": "japanese,chinese,english"
  }
}
```

---

## 10. 前端 UI 设计

### 10.1 插件管理页面（Admin → Plugins）

现有的 `PluginsView.vue` 需要增强:

```
┌─────────────────────────────────────────────────┐
│  插件管理                          [上传插件]     │
├─────────────────────────────────────────────────┤
│  [全部] [Metadata] [Download] [Processor]       │
│  [Analyzer] [Script] [Endpoint]                 │
├─────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────┐    │
│  │ 🟢 E-Hentai Metadata         v1.0.0   │    │
│  │ 从 E-Hentai 获取档案元数据和标签         │    │
│  │ 已执行 142 次 · 上次: 2小时前            │    │
│  │ [配置] [执行记录] [禁用] [卸载]          │    │
│  └─────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────┐    │
│  │ ⚪ Image Optimizer            v1.2.0   │    │
│  │ 自动优化和压缩档案中的图像               │    │
│  │ 已禁用                                  │    │
│  │ [配置] [启用] [卸载]                    │    │
│  └─────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

### 10.2 插件配置弹窗

```
┌─────────────────────────────────────────────────┐
│  E-Hentai Metadata 配置                    [X]  │
├─────────────────────────────────────────────────┤
│                                                 │
│  使用 ExHentai                                  │
│  [□] 使用 ExHentai 而非 E-Hentai               │
│                                                 │
│  标签语言                                       │
│  [japanese,chinese,english          ]           │
│  优先使用的标签语言（逗号分隔）                    │
│                                                 │
│  获取封面                                       │
│  [□] 是否从 E-Hentai 下载封面替换本地封面        │
│                                                 │
│  最大重试次数                                    │
│  [ 3 ] (0-10)                                   │
│  API 请求失败时的最大重试次数                     │
│                                                 │
│  权限声明:                                       │
│  🌐 网络: api.e-hentai.org, exhentai.org       │
│  📖 数据库读: 是                                 │
│  ✏️  数据库写: tags, archive_tags                │
│                                                 │
│              [取消]  [保存配置]                   │
└─────────────────────────────────────────────────┘
```

### 10.3 档案详情页的插件集成

在 ReaderView 的 Info Panel 中，增加插件操作入口:

```
┌─────────────────────────┐
│  档案信息                │
│  标题: xxxxx            │
│  标签: artist:xx ...    │
│                         │
│  ── 插件操作 ──          │
│  [E-Hentai 获取标签]    │
│    URL: [输入框       ]  │  ← oneshot_param
│    [执行]               │
│                         │
│  [ComicInfo 提取]       │
│    [执行]               │
│                         │
│  上次插件操作:           │
│  EH Metadata: 成功      │
│  +5 标签, 更新了标题     │
└─────────────────────────┘
```

---

## 11. 插件发现与分发

### 11.1 本地安装

插件以 `.tar.gz` 包格式分发:

```
ehentai-metadata-1.0.0.tar.gz
├── plugin.toml
├── libehentai_metadata.so (Linux)
├── libehentai_metadata.dylib (macOS)
├── ehentai_metadata.dll (Windows)
└── README.md
```

安装方式:
1. **Web UI 上传**: 管理员通过插件管理页面上传
2. **API 上传**: `POST /api/v1/plugins/install` (multipart/form-data)
3. **手动放置**: 将解压后的目录放入 `data/plugins/` 并重启

### 11.2 插件目录结构（运行时）

```
data/
└── plugins/
    ├── ehentai-metadata/
    │   ├── plugin.toml
    │   ├── libehentai_metadata.so
    │   └── README.md
    ├── image-optimizer/
    │   ├── plugin.toml
    │   ├── libimage_optimizer.so
    │   └── README.md
    └── _temp/               # 上传和解压临时目录
```

### 11.3 未来: 插件市场（v2.0+）

暂不在 v1 版本实现，但预留接口:

```toml
# 未来在系统设置中可配置插件仓库源
[plugin_repository]
url = "https://plugins.otamoryx.dev"
```

```
GET  /api/v1/plugin-store/search?q=ehentai   → 搜索远程插件
POST /api/v1/plugin-store/install             → 从远程安装插件
GET  /api/v1/plugin-store/updates             → 检查更新
```

---

## 12. 开发者体验

### 12.1 快速开始模板

提供 `cargo-generate` 模板快速创建插件项目:

```bash
# 安装模板
cargo install cargo-generate

# 创建新插件项目
cargo generate --git https://github.com/otamoryx/plugin-template \
  --name my-metadata-plugin \
  --define type=metadata
```

生成的项目包含:
- 预配置的 `Cargo.toml`（引入 `otamoryx-plugin-sdk`）
- `plugin.toml` 模板
- `src/lib.rs` 骨架代码
- `build.sh` 构建脚本
- 基础测试

### 12.2 开发模式

```bash
# 在 OtamoryX 中启用插件开发模式
OTAMORYX_PLUGIN_DEV=1 ./otamoryx

# 开发模式特性:
# - 监视 data/plugins/ 目录变化，自动重载插件
# - 详细的 FFI 调用日志
# - 放宽超时限制（便于 debug）
# - 插件 stderr 输出转发到主系统日志
```

### 12.3 测试支持

SDK 提供 mock 工具:

```rust
#[cfg(test)]
mod tests {
    use otamoryx_plugin_sdk::testing::*;

    #[tokio::test]
    async fn test_metadata_extraction() {
        let plugin = MyMetadataPlugin::new(default_config()).unwrap();
        let ctx = MockPluginContext::new();
        let archive = MockArchiveInfo::builder()
            .title("[Artist] Title (Series)")
            .build();

        let result = plugin.get_tags(&ctx, &archive).await.unwrap();

        assert!(result.tags.iter().any(|t| t.namespace == "artist"));
        assert!(result.title.is_some());
    }
}
```

---

## 13. 实施路线图

### Phase 1: 基础框架（2-3 周）

| 任务 | 说明 | 涉及文件 |
|------|------|---------|
| 定义核心类型 | `PluginType`, `PluginManifest`, 各种 Result 类型 | `models/plugin.rs` |
| 实现 `plugin.toml` 解析 | TOML 解析 + 验证 | 新增 `services/plugin_manifest.rs` |
| 实现 PluginManager 基础 | 扫描、加载、卸载动态库 | 新增 `services/plugin_manager.rs` |
| 实现 FFI 调用层 | 安全的 FFI 调用包装（超时、panic 捕获） | 新增 `services/plugin_executor.rs` |
| 数据库迁移 | 创建新表，修改现有 plugins 表 | `migrations/` |
| 更新 API handlers | 完善 `handlers/plugins.rs` | `handlers/plugins.rs` |

### Phase 2: SDK 和首个内置插件（2-3 周）

| 任务 | 说明 |
|------|------|
| 创建 `otamoryx-plugin-sdk` crate | FFI 胶水代码、过程宏、类型定义 |
| 实现 Metadata 插件支持 | 完成 `MetadataPlugin` 流程，集成到 ProcessingPipeline |
| 实现 Script 插件支持 | 完成 `ScriptPlugin` 流程 |
| 内置 ComicInfo 提取插件 | 读取档案内的 `ComicInfo.xml` |
| 内置文件名解析插件 | 迁移现有 examples/metadata-extractor |

### Phase 3: 高级功能（2-3 周）

| 任务 | 说明 |
|------|------|
| 实现 Download 插件支持 | URL 匹配、下载流程 |
| 实现 Processor 插件支持 | 图像处理流程 |
| 实现 Analyzer 插件支持 | AI 分析流程，迁移现有 AI 标签功能 |
| 实现 Endpoint 插件支持 | 动态路由注册 |
| 安全框架 | 权限验证、运行时检查 |
| 定时调度 | Script 插件的 cron 调度 |

### Phase 4: 前端和打磨（1-2 周）

| 任务 | 说明 |
|------|------|
| 增强 PluginsView | 完整的插件管理 UI |
| 插件配置 UI | 基于 schema 自动生成配置表单 |
| ReaderView 集成 | 在档案详情中展示插件操作 |
| 插件执行记录 UI | 执行历史和日志查看 |
| 文档和模板 | 开发者文档、项目模板 |

---

## 14. 关键设计决策记录

### 14.1 为什么选择动态库而非 WASM？

| 维度 | 动态库 (.so/.dll) | WASM |
|------|-------------------|------|
| 性能 | 原生性能，无额外开销 | 有少量 runtime 开销 |
| 生态 | 可使用任何 Rust crate | 受 WASM 兼容性限制 |
| 安全 | 需要手动隔离 | 天然沙箱隔离 |
| 开发门槛 | 低，标准 Rust 开发 | 中等，需要 WASM 工具链 |
| 跨平台 | 需要每个平台编译 | 一次编译到处运行 |

**决策**: v1 采用动态库方案，因为:
- 与现有的 `libloading` 依赖和示例代码一致
- 对 AI/图像处理场景的性能更好
- 开发门槛更低
- v1 先采用 trusted plugin 模式，通过权限声明/审批/审计降低风险（不承诺强沙箱）

**未来考虑**: v2 可以增加 WASM 运行时支持，让不需要原生性能的简单插件（如正则解析、URL 匹配）运行在 WASM 沙箱中。

### 14.2 为什么用 C FFI + JSON 而非直接 Rust trait 对象？

Rust 没有稳定的 ABI，`#[repr(Rust)]` 的内存布局在不同编译器版本间可能不同。直接跨动态库传递 Rust trait 对象或复杂结构体是未定义行为。

C FFI + JSON 方案:
- ABI 稳定，插件和主程序可以用不同版本的 Rust 编译
- JSON 序列化/反序列化的性能开销对于插件调用频率来说可以忽略
- 调试友好，可以直接打印 FFI 传递的 JSON 内容

### 14.3 参数系统: 为什么选择 schema 驱动而非简单的 bool/string/int？

LANraragi 仅支持 3 种参数类型，虽然简单但限制了插件的配置能力。v1 采用 JSON Schema 子集作为统一模型，支持:
- `boolean`, `string`, `integer`, `number` 基础类型
- `enum` / `oneOf` 枚举类配置
- `minimum` / `maximum` 等约束
- `title` / `description` 文档字段

文档中的 `[params.xxx]` 写法视为 DSL 语法糖，最终都映射到 `config_schema`。  
这样前端可以根据 schema 自动生成配置表单，无需为每个插件编写自定义 UI。

---

## 15. 附录

### 附录 A: 完整的 Metadata 插件示例

```rust
// ehentai-metadata/src/lib.rs

use otamoryx_plugin_sdk::prelude::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    use_exhentai: bool,
    tag_languages: String,
    fetch_covers: bool,
    max_retries: i32,
}

pub struct EHentaiMetadata {
    config: Config,
}

#[otamoryx_plugin]
impl Plugin for EHentaiMetadata {
    fn new(config: serde_json::Value) -> Result<Self, PluginError> {
        let config: Config = serde_json::from_value(config)
            .unwrap_or_else(|_| Config {
                use_exhentai: false,
                tag_languages: "japanese,chinese,english".into(),
                fetch_covers: false,
                max_retries: 3,
            });
        Ok(Self { config })
    }
}

#[otamoryx_metadata]
#[async_trait]
impl MetadataPlugin for EHentaiMetadata {
    async fn get_tags(
        &self,
        ctx: &PluginContext,
        archive: &ArchiveInfo,
    ) -> Result<MetadataResult, PluginError> {
        // 1. 获取 gallery URL（从 oneshot 参数或通过标题搜索）
        let gallery_url = match &archive.oneshot_param {
            Some(url) => url.clone(),
            None => self.search_by_title(ctx, &archive.title).await?,
        };

        // 2. 获取 gallery 页面
        let base = if self.config.use_exhentai {
            "https://exhentai.org"
        } else {
            "https://api.e-hentai.org"
        };

        let response = ctx.http()
            .get(&format!("{}/api.php", base))
            .query(&[("gidlist", &gallery_url)])
            .send()
            .await?;

        // 3. 解析标签
        let gallery: EHGallery = response.json().await?;
        let tags = gallery.tags.iter().map(|t| {
            let parts: Vec<&str> = t.splitn(2, ':').collect();
            TagEntry {
                namespace: parts.get(0).unwrap_or(&"general").to_string(),
                name: parts.get(1).unwrap_or(parts[0]).to_string(),
                confidence: None,
            }
        }).collect();

        Ok(MetadataResult {
            tags,
            title: Some(gallery.title),
            summary: None,
            source_url: Some(gallery_url),
        })
    }
}

impl EHentaiMetadata {
    async fn search_by_title(
        &self,
        ctx: &PluginContext,
        title: &str,
    ) -> Result<String, PluginError> {
        // 通过标题搜索 gallery
        // ...
        Err(PluginError::MissingParam(
            "无法通过标题自动匹配，请提供 E-Hentai URL".into()
        ))
    }
}

#[derive(Deserialize)]
struct EHGallery {
    title: String,
    tags: Vec<String>,
}
```

### 附录 B: 与 LANraragi 插件类型的映射

| LANraragi | OtamoryX | 说明 |
|-----------|----------|------|
| Metadata | Metadata | 功能一致 |
| Login | — | 合并到 Metadata/Download 插件的 `plugin_dependencies.login_from` 配置中 |
| Download | Download | 功能一致 |
| Script | Script | 功能一致，增加了 cron 调度 |
| — | Processor | 新增，LANraragi 不支持图像处理 |
| — | Analyzer | 新增，面向 AI 内容分析 |
| — | Endpoint | 新增，LANraragi 不支持自定义 API |

LANraragi 的 Login 插件在 OtamoryX 中不作为独立类型存在。认证逻辑可以:
1. 内嵌在 Metadata/Download 插件中（简单场景）
2. 通过 `plugin_dependencies.login_from` 引用其他插件的认证结果（复杂场景，如 EH 的 cookie）

### 附录 C: PluginContext 完整定义

```rust
/// 插件运行上下文，由主系统提供
pub struct PluginContext {
    /// 日志记录器
    logger: PluginLogger,

    /// 临时目录（插件专用，自动清理）
    temp_dir: PathBuf,

    /// 插件数据目录（持久化存储）
    data_dir: PathBuf,

    /// 当前用户配置的参数
    params: HashMap<String, serde_json::Value>,

    /// 由主程序分配的请求上下文句柄（用于 callback）
    host_context_id: u64,
}
```

> 注: PluginContext 在 FFI 层面以 JSON 字符串传递，SDK 自动反序列化为上述结构。

#### Host callback ABI（v1 固定）

```c
// 插件创建时主程序传入 HostApi 指针，插件仅通过该表访问宿主能力
typedef struct {
  // 返回值约定:
  //   0 = success
  //  -1 = permission denied
  //  -2 = timeout
  //  -3 = invalid argument
  //  -4 = internal error
  int32_t (*http_request)(uint64_t ctx_id, const char* req_json, char** resp_json);
  int32_t (*db_query)(uint64_t ctx_id, const char* req_json, char** resp_json);
  int32_t (*fs_read)(uint64_t ctx_id, const char* req_json, char** resp_json);
  int32_t (*fs_write)(uint64_t ctx_id, const char* req_json, char** resp_json);
  void    (*free_string)(char* s);
} OtamoryxHostApiV1;
```

执行语义:
- 每次插件调用创建独立 `ctx_id`，调用结束后失效
- 所有 callback 都走统一权限检查（域名/路径/表）
- callback 返回的字符串由宿主分配，插件通过 `free_string` 释放
- SDK 负责将回调错误码映射为 `PluginError`

---

## 16. 插件事件/钩子系统

### 16.1 事件模型

插件不仅可以被主动调用，还可以**订阅系统事件**被动触发。这提供了比 LANraragi 更灵活的集成方式。

```rust
/// 系统事件枚举
#[derive(Debug, Clone, Serialize)]
pub enum PluginEvent {
    /// 新档案被添加到库中（扫描完成、缩略图已生成）
    ArchiveAdded { archive_id: String, file_path: String },

    /// 档案元数据被修改（标签增删、标题变更等）
    ArchiveUpdated { archive_id: String, changes: Vec<MetadataChange> },

    /// 档案被删除
    ArchiveDeleted { archive_id: String },

    /// 用户开始阅读档案
    ReadingStarted { archive_id: String, user_id: String },

    /// 用户阅读进度更新
    ReadingProgress { archive_id: String, user_id: String, page: i32, total: i32 },

    /// 系统启动完成
    SystemReady,

    /// 定时触发（cron 调度）
    Scheduled { plugin_id: String },

    /// 批量扫描完成
    ScanCompleted { total: usize, new: usize, updated: usize },
}
```

### 16.2 事件订阅声明

在 `plugin.toml` 中声明插件关注的事件:

```toml
[events]
# 订阅的事件列表
subscribe = ["archive_added", "archive_updated"]

# 事件过滤器（可选，减少不必要的触发）
[events.filters]
archive_added = { min_page_count = 5 }  # 仅关注页数 >= 5 的档案
```

### 16.3 事件分发机制

```rust
pub struct PluginEventBus {
    /// 事件 → 订阅该事件的插件列表
    subscriptions: HashMap<EventType, Vec<String>>,
}

impl PluginEventBus {
    /// 发布事件，异步通知所有订阅者
    pub async fn publish(&self, event: PluginEvent) {
        let event_type = event.event_type();
        if let Some(subscribers) = self.subscriptions.get(&event_type) {
            for plugin_id in subscribers {
                // 通过任务队列异步执行，不阻塞主流程
                self.task_queue.enqueue(PluginTask {
                    plugin_id: plugin_id.clone(),
                    event: event.clone(),
                    priority: TaskPriority::Low,
                }).await;
            }
        }
    }
}
```

**与 LANraragi 的对比**: LANraragi 仅在新文件添加时自动触发已启用的 Metadata 插件（`exec_enabled_plugins_on_file`），没有通用的事件系统。OtamoryX 的事件总线让任意类型的插件都能响应系统事件。

---

## 17. 错误处理与恢复

### 17.1 错误分类

```rust
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// 插件加载失败（动态库损坏、FFI 符号缺失等）
    #[error("插件加载失败: {0}")]
    LoadFailed(String),

    /// 插件初始化失败（配置无效、依赖缺失等）
    #[error("插件初始化失败: {0}")]
    InitFailed(String),

    /// 执行超时
    #[error("插件执行超时（{timeout_ms}ms）")]
    Timeout { timeout_ms: u64 },

    /// 插件内部 panic
    #[error("插件内部崩溃")]
    Panicked,

    /// 插件返回的业务错误
    #[error("插件错误: {0}")]
    PluginReturned(String),

    /// 缺少必要参数
    #[error("缺少参数: {0}")]
    MissingParam(String),

    /// 网络请求失败
    #[error("网络请求失败: {0}")]
    NetworkError(String),

    /// 权限不足
    #[error("权限不足: {0}")]
    PermissionDenied(String),

    /// 冷却中，稍后重试
    #[error("插件冷却中，{remaining_secs}秒后可再次执行")]
    Cooldown { remaining_secs: u32 },

    /// 序列化/反序列化错误
    #[error("数据格式错误: {0}")]
    SerdeError(String),
}
```

### 17.2 错误处理策略

| 错误类型 | 处理方式 | 对用户的影响 |
|---------|---------|------------|
| `LoadFailed` | 标记插件为 Disabled，记录错误日志 | 通知管理员，其他插件不受影响 |
| `InitFailed` | 同上 | 同上 |
| `Timeout` | 终止执行，记录到 `plugin_executions` | 该次执行失败，可手动重试 |
| `Panicked` | 卸载并重新加载插件实例 | 该次执行失败，自动恢复 |
| `PluginReturned` | 记录错误，标记执行为 failed | 该次执行失败，展示错误信息 |
| `Cooldown` | 拒绝执行，返回剩余冷却时间 | 提示用户稍后重试 |
| `PermissionDenied` | 拒绝操作，记录审计日志 | 提示权限不足 |

### 17.3 自动恢复机制

```rust
impl PluginManager {
    /// 处理插件 panic 后的自动恢复
    async fn handle_plugin_panic(&mut self, plugin_id: &str) {
        log::error!("插件 {} 发生 panic，尝试自动恢复...", plugin_id);

        // 1. 销毁旧实例
        self.unload_plugin(plugin_id).await;

        // 2. 记录连续 panic 次数
        let panic_count = self.increment_panic_count(plugin_id);

        // 3. 如果连续 panic 超过阈值，自动禁用
        if panic_count >= 3 {
            log::error!("插件 {} 连续 panic {}次，自动禁用", plugin_id, panic_count);
            self.set_enabled(plugin_id, false).await.ok();
            return;
        }

        // 4. 尝试重新加载
        match self.load_plugin(plugin_id).await {
            Ok(_) => log::info!("插件 {} 已恢复", plugin_id),
            Err(e) => {
                log::error!("插件 {} 恢复失败: {}，自动禁用", plugin_id, e);
                self.set_enabled(plugin_id, false).await.ok();
            }
        }
    }
}
```

### 17.4 批量执行的错误隔离

当对一个档案执行多个 Metadata 插件时，单个插件的失败不影响其他插件:

```rust
impl PluginManager {
    pub async fn exec_all_metadata(
        &self,
        archive_info: &ArchiveInfo,
    ) -> Result<Vec<(String, Result<MetadataResult, PluginError>)>> {
        let enabled = self.get_enabled_plugins(PluginType::Metadata);
        let mut results = Vec::new();

        for plugin in enabled {
            let result = self.exec_metadata(&plugin.manifest.id, archive_info).await;

            // 记录执行结果（无论成功或失败）
            self.record_execution(
                &plugin.manifest.id,
                &archive_info.id,
                &result,
            ).await;

            results.push((plugin.manifest.id.clone(), result));
        }

        Ok(results)
    }
}
```

---

## 18. 日志与可观测性

### 18.1 插件日志系统

每个插件拥有独立的日志通道:

```
data/logs/
├── otamoryx.log            # 主系统日志
└── plugins/
    ├── filename-parser.log # 各插件独立日志
    ├── comicinfo.log
    ├── ehentai-metadata.log
    └── ...
```

**日志级别**: `trace` / `debug` / `info` / `warn` / `error`

插件通过 SDK 写日志:
```rust
ctx.log().info("开始从 E-Hentai 获取标签...");
ctx.log().debug(&format!("请求 URL: {}", url));
ctx.log().error(&format!("API 返回错误: {}", err));
```

### 18.2 执行指标

`plugin_executions` 表提供以下可观测数据:

```
┌────────────────────────────────────────────────────┐
│  插件执行仪表板 (Admin → Plugins → 监控)             │
├────────────────────────────────────────────────────┤
│                                                    │
│  最近 24 小时:                                      │
│  ┌──────────────────┬────────┬──────┬─────┐       │
│  │ 插件              │ 成功   │ 失败  │ 平均耗时 │     │
│  ├──────────────────┼────────┼──────┼─────┤       │
│  │ filename-parser  │  127   │  0   │ 12ms│       │
│  │ comicinfo        │   89   │  3   │ 45ms│       │
│  │ ehentai-metadata │   42   │  7   │ 2.1s│       │
│  └──────────────────┴────────┴──────┴─────┘       │
│                                                    │
│  错误率最高: ehentai-metadata (14.3%)               │
│  最近错误: "API rate limit exceeded" (3次)          │
└────────────────────────────────────────────────────┘
```

### 18.3 健康检查

```rust
/// 插件健康状态
#[derive(Debug, Serialize)]
pub struct PluginHealth {
    pub name: String,
    pub status: HealthStatus,
    pub uptime_secs: u64,
    pub total_executions: u64,
    pub failed_executions: u64,
    pub error_rate: f64,         // 最近 100 次执行的错误率
    pub avg_duration_ms: u64,    // 平均执行时间
    pub last_error: Option<String>,
    pub consecutive_failures: u32,
}

#[derive(Debug, Serialize)]
pub enum HealthStatus {
    Healthy,            // 错误率 < 10%
    Degraded,           // 错误率 10%-50%
    Unhealthy,          // 错误率 > 50% 或连续失败 >= 3
    Disabled,
}
```

---

## 19. 标签审批工作流

### 19.1 标签来源与置信度

不同类型的插件产生的标签有不同的置信度特征:

| 来源 | 置信度范围 | 是否需要审核 |
|------|-----------|------------|
| 文件名解析 (filename-parser) | `null` (确定) | 否，直接应用 |
| ComicInfo.xml | `null` (确定) | 否，直接应用 |
| 内嵌 JSON 解析 (eze, koromo 等) | `null` (确定) | 否，直接应用 |
| 在线 API (E-Hentai, nHentai) | `null` (确定) | 可配置：自动应用或需审核 |
| AI Analyzer 插件 | `0.0-1.0` | 按置信度阈值决定 |

### 19.2 审批流程

```
插件产生标签
    │
    ▼
confidence == null?  ──YES──► 直接写入 archive_tags
    │                         （同时记录到 plugin_tags 用于审计）
    NO
    │
    ▼
confidence >= 阈值?  ──YES──► 自动批准，写入 archive_tags
    │                         （标记 approved = 1）
    NO
    │
    ▼
写入 plugin_tags (approved = NULL)
    │
    ▼
管理员在 UI 中审核
    │
    ├──批准──► 写入 archive_tags, 更新 approved = 1
    └──拒绝──► 更新 approved = 0
```

### 19.3 置信度阈值配置

```toml
# 系统设置中的全局阈值
[plugin_settings]
# 置信度 >= 此值的标签自动批准
auto_approve_threshold = 0.85

# 低于此值的标签自动拒绝（不进入审核队列）
auto_reject_threshold = 0.20

# 确定性标签（confidence = null）的处理策略
# "auto_apply" = 直接应用
# "require_review" = 也需要审核
certain_tag_policy = "auto_apply"
```

### 19.4 冲突处理

当多个插件为同一档案产生矛盾的标签时:

```rust
pub struct TagConflict {
    pub archive_id: String,
    pub tag_namespace: String,
    pub tag_name: String,
    pub sources: Vec<TagSource>,  // 多个插件声称不同的值
}

pub struct TagSource {
    pub plugin_id: String,
    pub value: String,
    pub confidence: Option<f32>,
}

// 冲突解决策略:
// 1. 优先级: 确定性标签 > 高置信度 > 低置信度
// 2. 如果多个确定性标签冲突 → 标记为需要人工审核
// 3. 用户手动修改的标签永远优先于插件产生的标签
```

---

## 20. 插件版本迁移

### 20.1 版本兼容性

```toml
# plugin.toml 中声明兼容性
[plugin]
id = "ehentai-metadata"
version = "2.0.0"
min_app_version = "0.6.5"   # 最低兼容的 OtamoryX 版本
manifest_version = 1
plugin_api_version = 1       # 必须与主程序支持的 ABI 版本匹配

[migration]
# 从旧版参数名映射到新版（类似 LANraragi 的 to_named_params）
from_v1 = { old_api_key = "api_key", old_use_proxy = "use_proxy" }
```

### 20.2 更新流程

```
用户上传新版插件
    │
    ▼
检查 plugin.toml 中的 id 是否匹配已安装的插件
    │
    ▼
检查新版 min_app_version 是否满足
    │
    ▼
检查新版 plugin_api_version 是否匹配
    │
    ▼
禁用旧版 → 备份旧版配置 → 替换文件 → 迁移配置 → 启用新版
    │                                                │
    │               (如果启用失败)                      │
    ▼                                                ▼
回滚到旧版，恢复配置                              更新完成
```

### 20.3 配置迁移

```rust
impl PluginManager {
    async fn migrate_config(
        &self,
        old_config: &serde_json::Value,
        old_manifest: &PluginManifest,
        new_manifest: &PluginManifest,
    ) -> serde_json::Value {
        let mut new_config = old_config.clone();

        // 应用 migration 映射
        if let Some(migration) = &new_manifest.migration {
            for (old_key, new_key) in migration {
                if let Some(value) = old_config.get(old_key) {
                    new_config[new_key] = value.clone();
                    new_config.as_object_mut().unwrap().remove(old_key);
                }
            }
        }

        // 按 schema 为新增字段填充默认值
        apply_schema_defaults(&mut new_config, &new_manifest.config_schema);

        new_config
    }
}
```

---

## 21. 内置插件目录与实现计划

本节参考 LANraragi 的 32 个内置插件，规划 OtamoryX 应随系统一起发布的内置插件。
内置插件与系统一同编译为同一二进制，不走 FFI 路径，直接实现 Rust trait。

### 21.1 LANraragi 内置插件全景

LANraragi 共有 **32 个内置插件**（22 Metadata + 4 Login + 3 Download + 3 Script）。
按照功能特征可分为三大类:

| 分类 | 数量 | 插件列表 |
|------|------|---------|
| **本地文件解析器** | 11 | Eze, ChaikaFile, ComicInfo, Koromo, Ksk, GalleryDL, HDoujin, HatH, Hentag, MEMS, EHDLInfo |
| **在线 API 对接** | 8 | EHentai+Login, nHentai+Login, Pixiv+Login, Hitomi, Chaika.moe, FAKKU+Login, HentagOnline |
| **工具类** | 6 | RegexParse(文件名解析), CopyTags, CopyArchiveTags, DateAdded, FolderToCat, SourceFinder, nHentaiSourceConverter |

### 21.2 OtamoryX 内置插件规划

将 LANraragi 的 32 个插件映射和重组为适合 OtamoryX 的版本。
分为 **内置（编译到主程序）** 和 **官方插件（独立发布但官方维护）** 两档。

#### 内置插件（随系统发布，零配置可用）

这些插件不需要网络访问，不依赖外部服务，是系统核心功能的一部分:

| # | 插件名 | 类型 | 对应 LANraragi | 优先级 | 说明 |
|---|--------|------|---------------|--------|------|
| 1 | `filename-parser` | Metadata | RegexParse | P0 | 从文件名解析标签，支持 `(Event) [Artist] Title (Series)` 等格式 |
| 2 | `comicinfo-parser` | Metadata | ComicInfo | P0 | 解析 ComicInfo.xml 标准格式 |
| 3 | `date-added` | Metadata | DateAdded | P0 | 添加 `date_added` 时间戳标签 |
| 4 | `tag-copier` | Metadata | CopyTags | P0 | 批量标签工具，将指定标签应用到档案 |
| 5 | `eze-parser` | Metadata | Eze | P1 | 解析 eze 浏览器扩展生成的 `info.json` |
| 6 | `gallerydl-parser` | Metadata | GalleryDL | P1 | 解析 gallery-dl 生成的 `info.json` |
| 7 | `koromo-parser` | Metadata | Koromo | P1 | 解析 Koromo/HitomiDownloader 的 `Info.json` |
| 8 | `hdoujin-parser` | Metadata | HDoujin | P2 | 解析 HDoujin Downloader 的 info.json/txt |
| 9 | `hentag-parser` | Metadata | Hentag | P2 | 解析 Hentag 的 `info.json` |
| 10 | `koushoku-parser` | Metadata | Ksk | P2 | 解析 Koushoku/Koharu 的 `info.yaml` |
| 11 | `chaika-parser` | Metadata | ChaikaFile | P2 | 解析 Chaika.moe 的 `api.json` |
| 12 | `hath-parser` | Metadata | HatH | P2 | 解析 HentaiAtHome 的 `galleryinfo.txt` |
| 13 | `ehdl-parser` | Metadata | EHDLInfo | P2 | 解析 EHDL 的 `info.txt` |
| 14 | `folder-to-category` | Script | FolderToCat | P1 | 按子文件夹自动创建分类 |
| 15 | `source-finder` | Script | SourceFinder | P1 | 通过 source 标签检测重复 |

#### 官方插件（独立发布，需要用户手动安装启用）

这些插件依赖外部网络服务，需要用户配置凭据:

| # | 插件名 | 类型 | 对应 LANraragi | 优先级 | 说明 |
|---|--------|------|---------------|--------|------|
| 16 | `ehentai-metadata` | Metadata | EHentai + EHentai Login | P0 | 从 E-Hentai/ExHentai 获取标签 |
| 17 | `nhentai-metadata` | Metadata | nHentai + nHentai CF Bypass | P0 | 从 nHentai 获取标签 |
| 18 | `pixiv-metadata` | Metadata | Pixiv + Pixiv Login | P1 | 从 Pixiv 获取作品标签 |
| 19 | `hitomi-metadata` | Metadata | Hitomi | P1 | 从 Hitomi.la 获取标签 |
| 20 | `chaika-metadata` | Metadata | Chaika.moe | P2 | 从 Chaika.moe 在线搜索标签 |
| 21 | `hentag-online` | Metadata | HentagOnline | P2 | 从 Hentag.com 在线搜索标签 |
| 22 | `ehentai-download` | Download | EHentai Download + Login | P1 | 从 E-Hentai 下载档案 |
| 23 | `pixiv-download` | Download | Pixiv Download + Login | P1 | 从 Pixiv 下载作品 |
| 24 | `image-optimizer` | Processor | — (OtamoryX 独有) | P2 | 图像优化与格式转换 |

> 注: LANraragi 的 FAKKU 相关插件（Metadata + Login）因 FAKKU 反爬措施频繁变化且用户群体较小，暂不实现。
> LANraragi 的 MEMS（Mayriad's EH Master Script）和 CopyArchiveTags 因使用场景过于小众，暂不实现。
> LANraragi 的 nHentaiSourceConverter 是一次性迁移工具，不作为标准插件提供。

### 21.3 各内置插件详细设计

#### P0 插件（必须首批实现）

---

##### 21.3.1 filename-parser（文件名解析器）

**对应 LANraragi**: RegexParse — LANraragi 中最核心的元数据插件之一

**功能描述**:
从档案文件名中提取结构化标签。支持同人志/漫画领域的多种常见命名格式。

**支持的文件名格式**:

| 格式 | 示例 | 提取结果 |
|------|------|---------|
| 标准同人志格式 | `(C99) [Circle (Artist)] Title (Series) [Chinese]` | event, group, artist, title, series, language |
| 简化格式 | `[Artist] Title (Series)` | artist, title, series |
| 带 ID 格式 | `{12345} Title` 或 `12345 Title` | source_id, title |
| Pixiv 格式 | `pixiv_98765432_Title` | pixiv_id, title |
| 纯标题 | `Some Manga Title v2` | title |

**参数**:

```toml
[params.use_custom_regex]
type = "bool"
label = "使用自定义正则"
default = false

[params.custom_regex]
type = "string"
label = "自定义正则表达式"
description = "使用命名捕获组: (?<artist>...) (?<series>...) (?<event>...) 等"
default = ""

[params.parse_curly_braces]
type = "bool"
label = "解析花括号标签"
description = "将文件名末尾 {tag1, tag2} 中的内容作为标签"
default = true

[params.default_language]
type = "string"
label = "默认语言"
description = "当文件名中未包含语言信息时使用的默认语言"
default = ""
```

**实现要点**:
```rust
/// 内置于主程序，直接实现 trait（不走 FFI）
pub struct FilenameParser {
    default_regex: Regex,
    custom_regex: Option<Regex>,
    config: FilenameParserConfig,
}

impl MetadataPlugin for FilenameParser {
    async fn get_tags(&self, ctx: &PluginContext, archive: &ArchiveInfo) -> Result<MetadataResult> {
        let filename = Path::new(&archive.file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&archive.title);

        let regex = self.custom_regex.as_ref().unwrap_or(&self.default_regex);
        let mut tags = Vec::new();

        if let Some(caps) = regex.captures(filename) {
            // 按命名捕获组提取
            for name in ["artist", "group", "series", "event", "language"] {
                if let Some(m) = caps.name(name) {
                    let value = m.as_str().trim();
                    if !value.is_empty() {
                        // 处理 "Circle (Artist)" 格式
                        if name == "artist" && value.contains('(') {
                            let (group, artist) = split_circle_artist(value);
                            tags.push(TagEntry::certain("group", &group));
                            tags.push(TagEntry::certain("artist", &artist));
                        } else {
                            tags.push(TagEntry::certain(name, value));
                        }
                    }
                }
            }
        }

        // 解析花括号标签
        if self.config.parse_curly_braces {
            if let Some(brace_tags) = extract_curly_brace_tags(filename) {
                for tag in brace_tags {
                    tags.push(TagEntry::certain("general", &tag));
                }
            }
        }

        Ok(MetadataResult {
            tags,
            title: None,  // 文件名解析不修改标题
            summary: None,
            source_url: None,
        })
    }
}
```

**默认正则**（参考 LANraragi 的 RegexParse）:
```regex
^(?:\((?P<event>[^)]+)\)\s*)?
 (?:\[(?P<artist>[^\]]+)\]\s*)?
 (?P<title>.+?)
 (?:\s*\((?P<series>[^)]+)\))?
 (?:\s*\[(?P<language>[^\]]+)\])?
 (?:\s*\{(?P<tags>[^}]+)\})?$
```

---

##### 21.3.2 comicinfo-parser（ComicInfo.xml 解析器）

**对应 LANraragi**: ComicInfo — 漫画管理领域的标准格式

**功能描述**:
解析档案内嵌的 `ComicInfo.xml` 文件。这是 ComicRack 定义的标准格式，被 Kavita、Komga、Calibre 等广泛支持。

**字段映射**:

| ComicInfo.xml 字段 | OtamoryX 标签/字段 |
|--------------------|--------------------|
| `<Title>` | 档案标题 |
| `<Series>` | `series:` 标签 |
| `<Writer>` | `group:` 标签 |
| `<Penciller>` | `artist:` 标签 |
| `<Genre>` | `genre:` 标签（逗号分隔） |
| `<Tags>` | `general:` 标签（逗号分隔） |
| `<Characters>` | `character:` 标签（逗号分隔） |
| `<Web>` | `source:` 标签 |
| `<LanguageISO>` | `language:` 标签 |
| `<Publisher>` | `publisher:` 标签 |
| `<Summary>` | 档案摘要 |
| `<PageCount>` | (信息性，不作为标签) |

**参数**: 无（零配置设计）

**实现要点**:
```rust
pub struct ComicInfoParser;

impl MetadataPlugin for ComicInfoParser {
    async fn get_tags(&self, ctx: &PluginContext, archive: &ArchiveInfo) -> Result<MetadataResult> {
        // 1. 在档案中查找 ComicInfo.xml（不区分大小写）
        let xml_data = ctx.archive()
            .extract_file(&archive.file_path, "ComicInfo.xml")
            .await?;

        let xml_data = match xml_data {
            Some(data) => data,
            None => return Err(PluginError::PluginReturned(
                "档案中未找到 ComicInfo.xml".into()
            )),
        };

        // 2. 解析 XML
        let comic_info: ComicInfo = quick_xml::de::from_reader(xml_data.as_slice())?;

        // 3. 映射为标签
        let mut tags = Vec::new();

        if let Some(genre) = &comic_info.genre {
            for g in genre.split(',') {
                tags.push(TagEntry::certain("genre", g.trim()));
            }
        }
        if let Some(writer) = &comic_info.writer {
            tags.push(TagEntry::certain("group", writer.trim()));
        }
        if let Some(penciller) = &comic_info.penciller {
            tags.push(TagEntry::certain("artist", penciller.trim()));
        }
        // ... 其他字段类似

        Ok(MetadataResult {
            tags,
            title: comic_info.title,
            summary: comic_info.summary,
            source_url: comic_info.web,
        })
    }
}
```

---

##### 21.3.3 date-added（时间戳标签）

**对应 LANraragi**: DateAdded

**功能描述**:
为档案添加 `date_added` 时间戳标签，用于按添加时间排序和筛选。

**时间来源选择**:

| 来源 | 说明 |
|------|------|
| 当前时间 | 使用档案入库时的时间（默认） |
| 文件修改时间 | 使用文件系统的 mtime |
| 自定义 | 通过 oneshot 参数指定 |

**参数**:
```toml
[params.time_source]
type = "select"
label = "时间来源"
default = "current"
options = [
    { value = "current", label = "当前时间" },
    { value = "file_mtime", label = "文件修改时间" },
]

[params.format]
type = "string"
label = "时间格式"
description = "时间戳格式（epoch 秒数或 ISO 8601）"
default = "epoch"
```

---

##### 21.3.4 tag-copier（批量标签工具）

**对应 LANraragi**: CopyTags

**功能描述**:
将指定标签批量应用到一个或多个档案。典型用途:
- 批量为一组英文翻译档案添加 `language:english`
- 批量为特定系列添加 `series:xxx`
- 在批处理模式下作为管道工具使用

**oneshot 参数**: 逗号分隔的标签列表，如 `language:english, translated`

---

##### 21.3.5 ehentai-metadata（E-Hentai 元数据）

**对应 LANraragi**: EHentai Metadata + EHentai Login

**功能描述**:
OtamoryX 中最重要的在线 Metadata 插件。从 E-Hentai/ExHentai 搜索并获取档案标签。

**搜索策略**（按优先级）:
1. 如果提供了 oneshot URL → 直接获取该 gallery
2. 如果档案有 `source:` 标签且包含 EH URL → 使用该 URL
3. 缩略图 SHA-1 反向搜索 → 通过封面匹配
4. 标题搜索 → 文本模糊匹配

**参数**:
```toml
[params.ipb_member_id]
type = "string"
label = "ipb_member_id"
description = "E-Hentai 账号 cookie（在浏览器开发者工具中获取）"
default = ""

[params.ipb_pass_hash]
type = "string"
label = "ipb_pass_hash"
description = "E-Hentai 密码 hash cookie"
default = ""

[params.igneous]
type = "string"
label = "igneous cookie"
description = "ExHentai 访问所需的 igneous cookie（可选）"
default = ""

[params.use_exhentai]
type = "bool"
label = "使用 ExHentai"
description = "搜索 ExHentai（需要有效的登录凭据）"
default = false

[params.search_by_thumbnail]
type = "bool"
label = "缩略图搜索"
description = "优先使用封面缩略图进行反向搜索"
default = true

[params.original_title]
type = "bool"
label = "使用原始标题"
description = "保存日文原始标题而非英文/罗马音标题"
default = false

[params.fetch_timestamp]
type = "bool"
label = "获取时间戳"
description = "获取上传时间和上传者信息"
default = false
```

**权限声明**:
```toml
[permissions]
network = ["api.e-hentai.org", "e-hentai.org", "exhentai.org"]
database_read = true
database_write = ["tags", "archive_tags"]
```

**冷却**: 4 秒（避免 EH API 速率限制）

**实现要点**:
- 使用 `reqwest` 构建带 cookie 的 HTTP 客户端
- 调用 E-Hentai JSON API (`api.e-hentai.org/api.php`) 获取 gallery 元数据
- 解析标签的 namespace（artist, group, parody, character, female, male, language, misc）
- 映射 EH 的 `female:` / `male:` 标签到通用的 `tag:female:xxx` 格式
- 错误处理: IP 封禁、速率限制、gallery 不存在、登录过期

---

##### 21.3.6 nhentai-metadata（nHentai 元数据）

**对应 LANraragi**: nHentai Metadata + nHentai CF Bypass Login

**功能描述**:
从 nHentai 搜索并获取档案标签。

**搜索策略**:
1. oneshot URL → 直接获取
2. `source:` 标签中的 nHentai URL → 直接获取
3. 文件名中的 `{ID}` 或 `ID Title` → 通过 ID 获取
4. 标题搜索

**参数**:
```toml
[params.user_agent]
type = "string"
label = "浏览器 User-Agent"
description = "必须与 cookie 来源的浏览器一致"
default = ""

[params.cf_clearance]
type = "string"
label = "cf_clearance cookie"
description = "Cloudflare 验证 cookie（在浏览器中获取）"
default = ""

[params.csrftoken]
type = "string"
label = "csrftoken cookie"
description = "nHentai CSRF token cookie"
default = ""

[params.fetch_date]
type = "bool"
label = "获取上传日期"
description = "获取 gallery 上传日期并添加为标签"
default = false
```

**权限声明**:
```toml
[permissions]
network = ["nhentai.net"]
database_read = true
database_write = ["tags", "archive_tags"]
```

---

#### P1 插件（第二批实现）

---

##### 21.3.7 eze-parser（eze info.json 解析器）

**对应 LANraragi**: Eze

**功能描述**:
解析 eze 浏览器扩展（用于从 E-Hentai 下载）生成的 `info.json` 文件。
这是最流行的 EH 下载方式之一。

**JSON 格式**:
```json
{
  "gallery_info": {
    "title": "...",
    "title_original": "...",
    "tags": {
      "artist": ["name1"],
      "group": ["circle1"],
      "parody": ["series1"],
      "character": ["char1"],
      "female": ["tag1", "tag2"],
      "male": ["tag3"],
      "language": ["chinese", "translated"],
      "misc": ["full color"]
    },
    "source": {
      "site": "e-hentai",
      "gid": 12345,
      "token": "abcdef"
    },
    "upload_date": [2024, 1, 15, 12, 30, 0]
  }
}
```

**参数**:
```toml
[params.original_title]
type = "bool"
label = "使用原始标题"
description = "优先使用日文原标题"
default = false

[params.fetch_extra]
type = "bool"
label = "提取额外信息"
description = "提取上传时间、上传者等附加信息作为标签"
default = false
```

---

##### 21.3.8 gallerydl-parser（gallery-dl info.json 解析器）

**对应 LANraragi**: GalleryDL

**功能描述**:
解析 gallery-dl（通用画廊下载工具）生成的 `info.json` 元数据。
gallery-dl 支持上百个网站，是目前最流行的下载工具之一。

**支持两种标签格式**:
```json
// 格式 1: Hash style
{ "tags": { "artist": ["name"], "language": ["chinese"] } }

// 格式 2: Array style
{ "tags": ["artist:name", "language:chinese"] }
```

---

##### 21.3.9 koromo-parser（Koromo/HitomiDownloader 解析器）

**对应 LANraragi**: Koromo

**功能描述**:
解析 Koromo / HitomiDownloader 生成的 `Info.json` 文件。

**JSON 格式**:
```json
{
  "Title": "...",
  "Tags": "artist:xxx, group:yyy, ...",
  "Artists": ["artist1"],
  "Characters": ["char1"],
  "Series": ["series1"],
  "Language": "Chinese",
  "URL": "https://hitomi.la/..."
}
```

---

##### 21.3.10 pixiv-metadata（Pixiv 元数据）

**对应 LANraragi**: Pixiv Metadata + Pixiv Login

**功能描述**:
从 Pixiv 获取插画/漫画作品的标签信息。

**ID 识别**:
- oneshot 参数: Pixiv 作品 URL 或 ID
- 文件名: `pixiv_12345678` 或 `12345678 Title`

**参数**:
```toml
[params.phpsessid]
type = "string"
label = "PHPSESSID cookie"
description = "Pixiv 登录 session cookie"
default = ""

[params.tag_languages]
type = "string"
label = "标签语言"
description = "标签翻译语言优先级（jp, en, zh），逗号分隔"
default = "jp,en"
```

**冷却**: 1 秒

---

##### 21.3.11 hitomi-metadata（Hitomi.la 元数据）

**对应 LANraragi**: Hitomi

**功能描述**:
从 Hitomi.la 获取标签。支持从文件名或 URL 中提取 gallery ID。

**特殊标签处理**:
- Hitomi 的 `male:` / `female:` 标签需要特殊映射

---

##### 21.3.12 ehentai-download（E-Hentai 下载器）

**对应 LANraragi**: E*Hentai Downloader + Login

**功能描述**:
给定 E-Hentai/ExHentai 的 gallery URL，通过 EH Archiver 系统下载档案。

**注意**: 使用 GP（Gallery Points），用户需要有足够的 GP 余额。

**URL 匹配**: `https?://(e-hentai|exhentai)\.org/g/\d+/[a-f0-9]+/`

**参数**:
```toml
[params.force_resample]
type = "bool"
label = "强制重采样"
description = "下载重采样版本而非原始质量（节省 GP）"
default = false
```

复用 `ehentai-metadata` 的登录凭据（通过 `plugin_dependencies.login_from`）。

---

##### 21.3.13 pixiv-download（Pixiv 下载器）

**对应 LANraragi**: Pixiv Downloader + Login

**功能描述**:
下载 Pixiv 插画/漫画作品，支持单图和多图。多图作品会打包为 ZIP。

**URL 匹配**: `https?://(?:www\.)?pixiv\.net/(?:[a-z]{2}/)?artworks/\d+`

复用 `pixiv-metadata` 的登录凭据。

---

##### 21.3.14 folder-to-category（文件夹转分类）

**对应 LANraragi**: FolderToCat — 非常实用的组织工具

**功能描述**:
扫描内容目录，按子文件夹结构自动创建 OtamoryX 的静态分类（Category）。

**示例**:
```
content/
├── 漫画/
│   ├── comic1.cbz   → 分类 "漫画"
│   └── comic2.cbz   → 分类 "漫画"
├── 同人志/
│   ├── doujin1.zip  → 分类 "同人志"
│   └── doujin2.zip  → 分类 "同人志"
└── Pixiv/
    └── works/
        └── art1.zip → 分类 "Pixiv" 或 "Pixiv/works"（取决于层级设置）
```

**参数**:
```toml
[params.clear_existing]
type = "bool"
label = "清除已有分类"
description = "运行前删除所有已有的静态分类"
default = false

[params.top_level_only]
type = "bool"
label = "仅顶层文件夹"
description = "只使用第一层子文件夹创建分类"
default = true
```

---

##### 21.3.15 source-finder（来源查重）

**对应 LANraragi**: SourceFinder

**功能描述**:
检查给定 URL 是否已存在于档案库中（通过 `source:` 标签匹配）。
特殊处理 E-Hentai/ExHentai 的 URL 互相转换。

用途: 在下载新档案前检查是否已有重复项。

---

#### P2 插件（第三批实现）

##### 21.3.16 - 21.3.21 剩余本地文件解析器

| 插件 | 对应 LANraragi | 解析文件 | 格式 |
|------|---------------|---------|------|
| `hdoujin-parser` | HDoujin | `info.json` / `info.txt` | JSON 或 key=value |
| `hentag-parser` | Hentag | `info.json` | JSON |
| `koushoku-parser` | Ksk | `info.yaml` / `koushoku.yaml` | YAML |
| `chaika-parser` | ChaikaFile | `api.json` | JSON |
| `hath-parser` | HatH | `galleryinfo.txt` | 文本 |
| `ehdl-parser` | EHDLInfo | `info.txt` | 文本 |

这些插件结构非常相似（都是从档案内嵌文件中读取元数据），可以用统一的框架实现:

```rust
/// 通用的"内嵌文件解析器"基础框架
pub struct EmbeddedFileParser {
    /// 要在档案中查找的文件名列表（按优先级排序）
    target_files: Vec<String>,
    /// 解析函数
    parser: Box<dyn Fn(&[u8]) -> Result<MetadataResult, PluginError>>,
}

impl MetadataPlugin for EmbeddedFileParser {
    async fn get_tags(&self, ctx: &PluginContext, archive: &ArchiveInfo) -> Result<MetadataResult> {
        for filename in &self.target_files {
            if let Some(data) = ctx.archive().extract_file(&archive.file_path, filename).await? {
                return (self.parser)(&data);
            }
        }
        Err(PluginError::PluginReturned(format!(
            "未找到目标文件: {:?}", self.target_files
        )))
    }
}
```

##### 21.3.22 chaika-metadata（Chaika.moe 在线搜索）

**对应 LANraragi**: Chaika.moe

**功能描述**:
从 Chaika.moe（E-Hentai 镜像站）在线搜索标签。当 E-Hentai 不可用时作为备选。

##### 21.3.23 hentag-online（Hentag.com 在线搜索）

**对应 LANraragi**: HentagOnline

**功能描述**:
从 Hentag.com 社区标签数据库搜索标签。支持多种搜索策略：URL 查找、已有 source 标签匹配、标题模糊搜索。

##### 21.3.24 image-optimizer（图像优化器）

**OtamoryX 独有** — LANraragi 无此功能

**功能描述**:
对档案内的图像进行优化处理：格式转换（WebP）、压缩、缩放、锐化等。
基于现有 `examples/plugins/image-processor` 的设计。

---

### 21.4 内置插件 vs 外部插件的区别

| 特征 | 内置插件 | 外部插件 |
|------|---------|---------|
| 编译方式 | 与主程序一同编译 | 独立编译为 `.so`/`.dll` |
| 调用方式 | 直接 Rust 函数调用 | 通过 C FFI + JSON |
| 性能 | 零额外开销 | 微量 JSON 序列化开销 |
| 更新方式 | 随系统版本更新 | 独立更新 |
| 安全隔离 | 完全信任 | 权限控制 + 运行时检查 |
| 代码位置 | `backend/src/plugins/builtin/` | `data/plugins/{name}/` |

**内置插件的模块结构**:
```
backend/src/plugins/
├── mod.rs              # 插件系统入口
├── builtin/
│   ├── mod.rs          # 内置插件注册
│   ├── filename_parser.rs
│   ├── comicinfo_parser.rs
│   ├── date_added.rs
│   ├── tag_copier.rs
│   ├── eze_parser.rs
│   ├── gallerydl_parser.rs
│   ├── koromo_parser.rs
│   ├── folder_to_category.rs
│   ├── source_finder.rs
│   └── embedded_file/   # 通用内嵌文件解析框架
│       ├── mod.rs
│       ├── hdoujin.rs
│       ├── hentag.rs
│       ├── koushoku.rs
│       ├── chaika.rs
│       ├── hath.rs
│       └── ehdl.rs
├── manager.rs          # PluginManager
├── executor.rs         # PluginExecutor
├── security.rs         # PluginSecurity
├── event_bus.rs        # PluginEventBus
└── ffi.rs              # FFI 加载层
```

### 21.5 内置插件执行顺序

当新档案入库时，内置 Metadata 插件按以下顺序执行:

```
档案入库
  │
  ▼
① filename-parser      ← 总是第一个，从文件名提取基础信息
  │
  ▼
② comicinfo-parser     ← 尝试读取 ComicInfo.xml
  │
  ▼
③ eze-parser           ← 尝试读取 eze info.json
  ├─(未找到)→ gallerydl-parser  ← 尝试读取 gallery-dl info.json
  ├─(未找到)→ koromo-parser     ← 尝试读取 Koromo Info.json
  ├─(未找到)→ ... (其他内嵌文件解析器)
  │
  ▼
④ date-added           ← 总是最后，添加时间戳
  │
  ▼
⑤ (如果启用) 在线 Metadata 插件按用户配置的顺序执行
```

**顺序配置**:

```toml
# 系统设置
[plugin_settings]
# 内嵌文件解析器的尝试顺序（按用户使用的下载工具调整）
embedded_parser_order = [
    "eze-parser",
    "gallerydl-parser",
    "koromo-parser",
    "hdoujin-parser",
    "hentag-parser",
    "koushoku-parser",
    "chaika-parser",
    "hath-parser",
    "ehdl-parser",
]

# 找到第一个匹配的内嵌文件后是否继续尝试其他解析器
stop_on_first_match = true
```

### 21.6 实现时间线

```
Phase 1: 插件框架 + P0 内置插件
├── Week 1-2: PluginManager + FFI 加载 + 数据库迁移
├── Week 3:   filename-parser + comicinfo-parser + date-added + tag-copier
└── Week 4:   集成测试 + API 完善

Phase 2: P1 插件 + SDK
├── Week 5-6: otamoryx-plugin-sdk crate + 过程宏
├── Week 7:   eze-parser + gallerydl-parser + koromo-parser
├── Week 8:   ehentai-metadata + nhentai-metadata (官方插件)
└── Week 9:   folder-to-category + source-finder + 前端 UI

Phase 3: P2 插件 + 高级功能
├── Week 10:  剩余内嵌文件解析器（统一框架批量实现）
├── Week 11:  pixiv-metadata + hitomi-metadata
├── Week 12:  ehentai-download + pixiv-download
├── Week 13:  chaika-metadata + hentag-online
└── Week 14:  image-optimizer + 事件系统 + 定时调度

Phase 4: 打磨
├── Week 15:  插件管理 UI 完善 + 标签审批 UI
├── Week 16:  开发者文档 + cargo-generate 模板
└── Week 17:  集成测试 + 性能调优 + 发布
```

---

## 22. 尚待完善的设计问题

以下问题在实现过程中需要进一步细化:

### 22.1 待解决

| # | 问题 | 影响范围 | 优先级 |
|---|------|---------|--------|
| 1 | **Login 插件是否需要独立类型**：当前设计将登录逻辑合并到 Metadata/Download 插件中，但 EH/Pixiv 的登录凭据被多个插件共享，是否应该恢复为独立的 Login 类型插件？ | 架构 | 高 |
| 2 | **内置插件的配置持久化**：内置插件的用户配置应该存在 `plugins` 表还是 `system_settings` 表？ | 数据库 | 中 |
| 3 | **插件间的标签去重**：多个插件可能为同一档案产生相同标签，去重应在何时发生（每个插件执行后 vs 全部执行后批量去重）？ | 执行流程 | 中 |
| 4 | **Host callback 扩展策略**：v1 已固定 `OtamoryxHostApiV1`，但后续如何做向后兼容扩展（新增函数、流式 IO）仍需细化 | SDK | 中 |
| 5 | **Windows/macOS 跨平台编译**：官方插件是否需要为每个平台提供预编译二进制？还是提供源码让用户自行编译？ | 分发 | 低 |
| 6 | **插件热重载的安全性**：在开发模式下卸载旧 `.so` 并加载新 `.so`，如何确保没有正在执行的 FFI 调用引用旧库？ | 开发体验 | 中 |
| 7 | **批量执行的并发控制**：对 1000 个档案批量执行在线 Metadata 插件时，如何控制并发度和速率？ | 性能 | 中 |

### 22.2 未来方向

| 方向 | 说明 | 版本目标 |
|------|------|---------|
| WASM 插件支持 | 为简单的文件名解析/正则类插件提供 WASM 沙箱运行时 | v2.0 |
| 插件市场 | 在线插件仓库、一键安装、自动更新 | v2.0 |
| 插件依赖管理 | 插件之间的依赖声明和自动安装 | v1.5 |
| GUI 插件开发工具 | 可视化的插件开发和调试工具 | v2.0 |
| 社区插件模板 | 针对常见场景的即用模板 | v1.5 |
