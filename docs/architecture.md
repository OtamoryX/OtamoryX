# OtamoryX 在线漫画阅读器技术架构文档

**版本**: 1.0  
**日期**: 2025年7月28日  
**技术栈**: Vue + Tauri + Rust

## 1. 项目概述

### 1.1 项目目标
OtamoryX是一个开源、可自部署的数字漫画阅读器和管理平台，旨在为用户提供现代化、功能丰富的替代方案（对标LANraragi等现有解决方案）。系统支持用户通过Web浏览器和原生桌面界面组织、阅读和管理其数字漫画收藏。采用现代化三层架构：
- **核心后端服务** - 数据管理、API服务、文件处理
- **Web前端界面** - 响应式现代化用户界面  
- **跨平台桌面客户端** - 原生桌面体验

### 1.2 核心目标
- **自部署**: 使用户能够运行自己的私人漫画图书馆服务器
- **多平台访问**: 支持Web浏览器和本机桌面应用程序
- **现代架构**: 采用Rust后端和Vue.js前端构建，确保性能和可维护性
- **全面管理**: 高级分类、搜索和元数据管理
- **阅读体验**: 流畅、响应式的漫画阅读界面，带有进度跟踪

### 1.3 技术栈
- **后端**: Rust with Axum web framework, SQLite数据库
- **前端**: Vue.js 3 with TypeScript, Tailwind CSS
- **桌面**: Tauri框架用于本机应用程序
- **API**: RESTful JSON API with OPDS支持

### 1.4 部署模型
- **独立服务器**: 通过浏览器进行基于Web的访问
- **桌面应用程序**: 本机跨平台客户端
- **Docker容器**: 容器化部署选项

**核心优势**: Rust生态协同效应 + 前端代码完全复用，一套Vue代码既可部署为网站，也可无缝打包成原生桌面应用。

### 1.5 插件系统架构
OtamoryX采用模块化插件架构，支持第三方扩展：
- **插件发现**: 自动扫描和注册插件
- **标准化API**: 统一的插件接口和钩子系统
- **安全沙盒**: 插件权限控制和安全验证
- **热加载**: 开发时支持插件热重载

### 1.6 AI自动标签系统（实验性）
集成AI模型进行内容分析和标签生成：
- **多模型支持**: 支持本地模型和云端API
- **后台处理**: 异步队列处理，不阻塞用户操作
- **智能标签**: 基于内容分析生成相关标签
- **用户审核**: 提供AI标签审核和反馈机制
## 2. 后端服务设计 (Rust)

后端是整个系统的核心，负责漫画管理、数据处理和API服务。

### 2.1 核心技术栈

| 组件 | 技术选型 | 版本要求 | 选型理由 |
|------|----------|----------|----------|
| **Web框架** | Axum | ^0.7 | tokio团队开发，异步生态无缝集成，模块化设计 |
| **数据库** | SQLite | latest | 轻量级嵌入式数据库，零配置 |
| **ORM** | SeaORM | latest | 异步SQL，编译时检查，SQLite支持 |
| **序列化** | serde + serde_json | ^1.0 | 高性能序列化，生态丰富 |
| **HTTP客户端** | reqwest | ^0.11 | 现代异步HTTP客户端 |
| **日志** | tracing + tracing-subscriber | ^0.1 | 结构化异步日志 |
| **配置管理** | config | ^0.14 | 多格式配置文件支持 |
| **认证** | jsonwebtoken | ^9.0 | JWT token生成和验证 |
| **加密** | bcrypt | ^0.15 | 密码哈希 |
| **文件处理** | zip, unrar, sevenz-rust | latest | 支持CBZ, CBR, CB7格式 |

### 2.2 项目结构

```
src/
├── main.rs                 # 应用入口点
├── config/
│   └── mod.rs             # 配置管理
├── models/
│   ├── mod.rs             # 数据模型定义
│   ├── archive.rs         # 漫画存档模型
│   ├── user.rs            # 用户模型
│   └── progress.rs        # 阅读进度模型
├── handlers/
│   ├── mod.rs             # API处理器
│   ├── archives.rs        # 漫画相关API
│   ├── auth.rs            # 认证API
│   ├── search.rs          # 搜索API
│   ├── progress.rs        # 阅读进度API
│   ├── categories.rs      # 分类管理API
│   ├── cache.rs           # 缓存管理API
│   ├── users.rs           # 用户管理API
│   ├── plugins.rs         # 插件管理API
│   ├── ai.rs              # AI自动标签API
│   ├── tags.rs            # 标签管理API
│   ├── health.rs          # 健康检查API
│   ├── opds.rs            # OPDS协议实现
│   └── settings.rs        # 设置API
├── services/
│   ├── mod.rs                    # 业务逻辑服务
│   ├── archive_service.rs        # 漫画处理服务
│   ├── archive_cache_service.rs  # 智能缓存服务
│   ├── archive_processing_service.rs # 存档处理服务
│   ├── auth_service.rs           # 认证服务
│   ├── search_service.rs         # 搜索服务
│   ├── random_service.rs         # 随机选择服务
│   └── processing_pipeline.rs    # 处理管道服务
├── utils/
│   ├── mod.rs             # 工具函数
│   ├── extractor.rs       # 压缩包解压
│   └── image.rs           # 图片处理
└── database/
    ├── mod.rs             # 数据库连接
    └── migrations/        # 数据库迁移文件
```

### 2.3 API设计规范

#### 2.3.1 RESTful API (主要接口)

**基础信息**
- **基础URL**: `http://localhost:3000/api/v1`
- **认证方式**: Bearer Token (`Authorization: Bearer <API_KEY>`)
- **数据格式**: JSON
- **字符编码**: UTF-8

**核心端点详细设计**

#### 系统管理 (`/api/v1/system`, `/health`)
| 方法 | 端点 | 描述 | 请求参数 | 响应格式 |
|------|------|------|----------|----------|
| `GET` | `/health` | 健康检查 | - | `HealthStatus` |
| `GET` | `/system/status` | 获取系统初始化状态 | - | `SystemStatus` |
| `POST` | `/system/initialize` | 首次运行系统初始化 | `InitializeSystemRequest` | `AuthResponse` |
| `GET` | `/settings` | 获取系统设置 | - | `SystemSettings` |
| `PUT` | `/settings` | 更新系统设置 | `SystemSettings` | `200 OK` |

#### 认证管理 (`/api/v1/auth`)
| 方法 | 端点 | 描述 | 请求参数 | 响应格式 |
|------|------|------|----------|----------|
| `POST` | `/auth/register` | 用户注册 | `CreateUserRequest` | `AuthResponse` |
| `POST` | `/auth/login` | 用户登录 | `LoginRequest` | `AuthResponse` |
| `POST` | `/auth/logout` | 用户登出 | - | `200 OK` |

#### 漫画管理 (`/api/v1/archives`)
| 方法 | 端点 | 描述 | 请求参数 | 响应格式 |
|------|------|------|----------|----------|
| `GET` | `/archives` | 获取漫画列表 | `page`, `limit`, `sort`, `filter` | `PaginatedResponse<Archive>` |
| `GET` | `/archives/random` | 获取随机漫画 | `count` (default: 20, max: 50) | `Array<Archive>` |
| `GET` | `/archives/{id}` | 获取漫画详情 | 路径参数: `id` | `Archive` |
| `GET` | `/archives/{id}/thumbnail` | 获取漫画缩略图 | 路径参数: `id` | 图片二进制数据 |
| `GET` | `/archives/{id}/pages/{page}` | 获取页面图片 | 路径参数: `id`, `page` | 图片二进制数据 |
| `DELETE` | `/archives/batch-delete` | 批量删除漫画 | `ArchiveIds[]` | `200 OK` |
| `GET` | `/archives/{id}/progress` | 获取阅读进度 | 路径参数: `id` | `ReadingProgress` |
| `POST` | `/archives/{id}/progress` | 更新阅读进度 | `UpdateProgressRequest` | `200 OK` |

#### 搜索和标签 (`/api/v1`)
| 方法 | 端点 | 描述 | 请求参数 | 响应格式 |
|------|------|------|----------|----------|
| `GET` | `/search` | 高级搜索漫画 | `query`, `tags`, `author`, `path`, `minPages`, `maxPages`, `minFileSize`, `maxFileSize`, `createdAfter`, `createdBefore`, `lastWeekRead`, `lastMonthRead`, `lastYearRead`, `sortBy`, `sortOrder`, `page`, `limit` | `PaginatedResponse<Archive>` |
| `GET` | `/tags` | 获取标签列表 | - | `Array<Tag>` |
| `DELETE` | `/tags/{id}/archives/batch-delete` | 批量删除标签下的漫画 | 路径参数: `id` | `200 OK` |
| `DELETE` | `/tags/prune` | 清理未使用的标签 | - | `200 OK` |

#### 分类管理 (`/api/v1/categories`)
| 方法 | 端点 | 描述 | 请求参数 | 响应格式 |
|------|------|------|----------|----------|
| `GET` | `/categories` | 获取所有分类 | - | `Array<Category>` |
| `POST` | `/categories` | 创建静态分类 | `CreateCategoryRequest` | `Category` |
| `POST` | `/categories/dynamic` | 创建动态分类 | `CreateDynamicCategoryRequest` | `DynamicCategory` |
| `PUT` | `/categories/{id}` | 更新分类信息 | `UpdateCategoryRequest` | `200 OK` |
| `DELETE` | `/categories/{id}` | 删除分类 | - | `200 OK` |
| `GET` | `/categories/{id}/archives` | 获取分类下的漫画 | `page`, `limit` | `PaginatedResponse<Archive>` |
| `POST` | `/categories/{id}/archives` | 向分类添加漫画 | `AddArchivesToCategoryRequest` | `200 OK` |
| `DELETE` | `/categories/{id}/archives` | 从分类移除漫画 | `RemoveArchivesFromCategoryRequest` | `200 OK` |
| `DELETE` | `/categories/{id}/archives/batch-delete` | 批量删除分类下的漫画 | 路径参数: `id` | `200 OK` |
| `DELETE` | `/categories/prune` | 清理空分类 | - | `200 OK` |

#### 用户管理 (`/api/v1/users`)
| 方法 | 端点 | 描述 | 请求参数 | 响应格式 |
|------|------|------|----------|----------|
| `GET` | `/users` | 获取用户列表（管理员）| - | `Array<User>` |
| `POST` | `/users` | 创建用户（管理员）| `CreateUserRequest` | `User` |
| `GET` | `/users/{id}` | 获取用户详情 | 路径参数: `id` | `User` |
| `PUT` | `/users/{id}` | 更新用户信息 | `UpdateUserRequest` | `200 OK` |
| `DELETE` | `/users/{id}` | 删除用户（管理员）| 路径参数: `id` | `200 OK` |
| `PUT` | `/users/{id}/paths` | 管理用户路径权限（管理员）| `UserPathsRequest` | `200 OK` |

#### 插件管理 (`/api/v1/plugins`)
| 方法 | 端点 | 描述 | 请求参数 | 响应格式 |
|------|------|------|----------|----------|
| `GET` | `/plugins` | 获取已安装插件列表 | - | `Array<Plugin>` |
| `POST` | `/plugins/install` | 安装插件 | `InstallPluginRequest` | `Plugin` |
| `PUT` | `/plugins/{id}/toggle` | 启用/禁用插件 | 路径参数: `id` | `200 OK` |
| `PUT` | `/plugins/{id}/config` | 配置插件 | `PluginConfigRequest` | `200 OK` |

#### AI自动标签 (`/api/v1/ai`, `/api/v1/settings/ai`)
| 方法 | 端点 | 描述 | 请求参数 | 响应格式 |
|------|------|------|----------|----------|
| `GET` | `/settings/ai` | 获取AI配置 | - | `AISettings` |
| `PUT` | `/settings/ai` | 更新AI配置 | `AISettings` | `200 OK` |
| `GET` | `/ai/status` | 获取AI处理状态 | - | `AIStatus` |
| `PUT` | `/ai/control` | 控制AI处理（暂停/恢复）| `AIControlRequest` | `200 OK` |

#### 2.3.2 OPDS v1.2 协议实现

**技术实现**
- **XML生成**: `atom_syndication` crate
- **基础URL**: `http://localhost:3000/opds`
- **内容类型**: `application/atom+xml;profile=opds-catalog`

**OPDS端点设计**

| 端点 | 类型 | 描述 |
|------|------|------|
| `/opds` | Navigation Feed | 根目录导航 |
| `/opds/recent` | Acquisition Feed | 最新漫画 |
| `/opds/all` | Acquisition Feed | 所有漫画 |
| `/opds/series` | Navigation Feed | 按系列分类 |
| `/opds/search?q={query}` | Acquisition Feed | 搜索结果 |

### 2.4 智能缓存系统

#### 2.4.1 缓存架构概述

OtamoryX实现了多层智能缓存系统，优化存档访问性能和用户体验：

**核心挑战**：
- 每次页面请求都解压整个存档会造成严重性能问题
- 需要在内存使用、响应速度和存储空间之间平衡
- 支持多种存档格式（CBZ、CBR、CB7、PDF等）

**解决方案**：
采用混合缓存策略，结合内存缓存、懒加载和智能预测

#### 2.4.2 缓存策略设计

**三种预设策略**：

| 策略 | 内存限制 | 缓存时间 | 预加载页数 | 适用场景 |
|------|----------|----------|------------|----------|
| **Conservative** | 128MB | 15分钟 | 前0后1页 | 低配置服务器、移动设备 |
| **Balanced** | 512MB | 1小时 | 前1后3页 | 个人服务器、家庭使用 |
| **Aggressive** | 2GB | 4小时 | 前5后10页 | 专用服务器、重度使用 |

**自定义配置参数**：
```rust
pub struct CustomCacheConfig {
    pub max_memory_mb: usize,              // 最大内存使用
    pub max_cached_archives: usize,        // 最大缓存存档数
    pub cache_ttl_hours: u32,              // 缓存生存时间
    pub preload_next_pages: u32,           // 预加载后续页数
    pub preload_prev_pages: u32,           // 预加载前面页数
    pub cleanup_threshold_percent: u32,     // 清理阈值百分比
    pub enable_background_preload: bool,    // 是否启用后台预加载
    pub max_concurrent_extractions: usize, // 最大并发解压数
}
```

#### 2.4.3 缓存生命周期管理

**缓存写入流程**：
1. 首次访问存档时，完整解压所有图片页面
2. 按自然顺序排序页面（使用natord crate）
3. 将所有页面数据存储在内存中
4. 记录访问时间和使用统计

**缓存淘汰策略**：
1. **TTL过期清理**：超过设定时间的条目自动清理
2. **LRU淘汰**：内存压力时清理最久未访问的条目
3. **容量限制**：超过最大存档数量时清理旧条目
4. **智能阈值**：内存使用达到阈值百分比时触发清理

**预加载机制**：
- **请求时预加载**：访问页面时异步预加载周围页面
- **背景预加载**：基于阅读模式预测下一步访问
- **并发控制**：限制同时进行的解压操作数量

#### 2.4.4 缓存管理API

**缓存状态监控**：
```
GET /api/v1/cache/status
返回：当前缓存使用情况、命中率、配置信息
```

**动态配置更新**：
```
POST /api/v1/cache/configure
参数：strategy (conservative/balanced/aggressive/custom)
自定义配置：CustomCacheConfig对象
```

**缓存管理操作**：
```
DELETE /api/v1/cache/clear     # 清空缓存
GET /api/v1/cache/recommendations  # 获取配置推荐
```

#### 2.4.5 性能优化

**存档格式支持增强**：
- **RAR格式**：完全支持RAR解压（unrar crate）
- **7Z格式**：支持7Z解压（sevenz-rust crate）
- **ZIP格式**：高效ZIP解压（zip crate）
- **PDF格式**：PDF页面提取（预留接口）

**内存管理优化**：
- 实时内存使用监控
- 智能清理时机选择
- 分级缓存策略
- 异步清理避免阻塞

**并发优化**：
- 异步解压操作
- 并发限制防止资源竞争
- 智能队列管理
- 错误恢复机制

### 2.5 数据库设计

#### 2.5.1 表结构设计

```sql
-- 用户表
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    email TEXT,
    role TEXT NOT NULL DEFAULT 'user', -- 'admin' or 'user'
    api_key TEXT UNIQUE NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 用户路径权限表
CREATE TABLE user_paths (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT NOT NULL,
    path TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- 漫画存档表
CREATE TABLE archives (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    path TEXT UNIQUE NOT NULL,
    file_size INTEGER NOT NULL,
    page_count INTEGER NOT NULL,
    hash TEXT UNIQUE NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 标签表
CREATE TABLE tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    namespace TEXT DEFAULT 'general',
    UNIQUE(name, namespace)
);

-- 漫画标签关联表
CREATE TABLE archive_tags (
    archive_id TEXT,
    tag_id TEXT,
    PRIMARY KEY (archive_id, tag_id),
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

-- 阅读进度表
CREATE TABLE reading_progress (
    id TEXT PRIMARY KEY,
    archive_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    current_page INTEGER NOT NULL DEFAULT 0,
    total_pages INTEGER NOT NULL,
    progress_percentage REAL NOT NULL DEFAULT 0.0,
    last_read_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(archive_id, user_id),
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- 静态分类表
CREATE TABLE categories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    category_type TEXT NOT NULL DEFAULT 'static', -- 'static' or 'dynamic'
    search_criteria JSON, -- for dynamic categories
    owner_id TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
);

-- 分类漫画关联表（仅用于静态分类）
CREATE TABLE category_archives (
    category_id TEXT,
    archive_id TEXT,
    PRIMARY KEY (category_id, archive_id),
    FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE,
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE
);

-- 系统设置表
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value JSON NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 插件表
CREATE TABLE plugins (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    enabled BOOLEAN DEFAULT false,
    config JSON,
    installed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- AI生成标签表
CREATE TABLE ai_generated_tags (
    id TEXT PRIMARY KEY,
    archive_id TEXT NOT NULL,
    tag_id TEXT NOT NULL,
    confidence_score REAL NOT NULL,
    approved BOOLEAN DEFAULT NULL, -- NULL=pending, true=approved, false=rejected
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    reviewed_at DATETIME,
    reviewed_by TEXT,
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE,
    FOREIGN KEY (reviewed_by) REFERENCES users(id) ON DELETE SET NULL
);

-- AI处理队列表
CREATE TABLE ai_processing_queue (
    id TEXT PRIMARY KEY,
    archive_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'processing', 'completed', 'failed'
    priority INTEGER DEFAULT 0,
    attempts INTEGER DEFAULT 0,
    last_error TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    started_at DATETIME,
    completed_at DATETIME,
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE
);
```

### 2.5 文件扫描与处理系统

#### 2.5.1 文档扫描架构

**扫描触发机制**
- **实时文件监控**: 使用`notify` crate监控文件系统变化
- **定时扫描**: 基于cron表达式的周期性扫描
- **手动触发**: 用户主动触发的扫描操作
- **启动扫描**: 系统启动时的初始化扫描

**扫描策略**
```rust
pub struct ScanConfig {
    pub comic_paths: Vec<PathBuf>,
    pub recursive: bool,
    pub ignore_hidden: bool,
    pub file_extensions: HashSet<String>,
    pub duplicate_detection: DuplicateDetectionConfig,
}

pub struct DuplicateDetectionConfig {
    pub enable_hash_detection: bool,      // 强检测：内容哈希
    pub enable_title_detection: bool,     // 弱检测：标题相似度
    pub title_similarity_threshold: f32,  // 标题相似度阈值
}
```

#### 2.5.2 统一处理流水线

**处理流水线架构**
```rust
pub struct ProcessingPipeline {
    pub scanner: FileScanner,
    pub processor_pool: Arc<ProcessorPool>,
    pub task_queue: Arc<TaskQueue>,
    pub storage: Arc<dyn Storage>,
}

pub struct ProcessorPool {
    pub metadata_extractors: Vec<Box<dyn MetadataExtractor>>,
    pub thumbnail_generators: Vec<Box<dyn ThumbnailGenerator>>,
    pub ai_analyzers: Vec<Box<dyn AIAnalyzer>>,
}

#[derive(Debug, Clone)]
pub struct ProcessingTask {
    pub id: String,
    pub archive_id: String,
    pub task_type: TaskType,
    pub priority: i32,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub retry_count: i32,
}

#[derive(Debug, Clone)]
pub enum TaskType {
    InitialProcessing,    // 新文件的完整处理
    ThumbnailGeneration,  // 缩略图生成
    MetadataExtraction,   // 元数据提取
    AIAnalysis,          // AI分析标签
    Reprocessing,        // 重新处理
}
```

**处理流程设计**
```rust
impl ProcessingPipeline {
    pub async fn process_archive(&self, archive_path: &Path) -> Result<(), ProcessingError> {
        // 1. 文件扫描和重复检测
        let scan_result = self.scanner.scan_file(archive_path).await?;
        
        if scan_result.is_duplicate {
            // 跳过重复文件，不分配"new"标签
            return Ok(());
        }
        
        // 2. 创建存档记录并分配"new"标签
        let archive = self.create_archive_record(&scan_result).await?;
        self.assign_new_tag(&archive.id).await?;
        
        // 3. 提交处理任务到队列
        let tasks = vec![
            ProcessingTask::new(&archive.id, TaskType::MetadataExtraction, 1),
            ProcessingTask::new(&archive.id, TaskType::ThumbnailGeneration, 1),
            ProcessingTask::new(&archive.id, TaskType::AIAnalysis, 0), // 较低优先级
        ];
        
        for task in tasks {
            self.task_queue.enqueue(task).await?;
        }
        
        Ok(())
    }
}
```

#### 2.5.3 存档格式处理

**支持格式与处理方式**

| 格式 | 扩展名 | 处理库 | 图片格式支持 |
|------|--------|--------|-------------|
| **CBZ** | `.cbz`, `.zip` | `zip` crate | jpg, jpeg, png, webp |
| **CBR** | `.cbr`, `.rar` | `unrar` crate | jpg, jpeg, png, webp |
| **CB7** | `.cb7`, `.7z` | `sevenz-rust` | jpg, jpeg, png, webp |
| **标准压缩包** | `.zip`, `.rar` | 对应解压库 | jpg, jpeg, png, webp |

**格式检测与处理**
```rust
pub trait ArchiveExtractor: Send + Sync {
    fn supports_extension(&self, ext: &str) -> bool;
    async fn extract_pages(&self, path: &Path) -> Result<Vec<PageInfo>, ExtractorError>;
    async fn extract_page(&self, path: &Path, page_index: usize) -> Result<Vec<u8>, ExtractorError>;
    async fn get_page_count(&self, path: &Path) -> Result<usize, ExtractorError>;
}

pub struct PageInfo {
    pub index: usize,
    pub filename: String,
    pub size: usize,
    pub mime_type: String,
}
```

#### 2.5.4 缩略图生成系统

**缩略图生成策略**
```rust
pub struct ThumbnailGenerator {
    pub image_processor: Arc<ImageProcessor>,
    pub cache_manager: Arc<CacheManager>,
    pub config: ThumbnailConfig,
}

pub struct ThumbnailConfig {
    pub sizes: Vec<ThumbnailSize>,
    pub quality: u8,                    // 1-100
    pub format: ImageFormat,            // JPEG, PNG, WebP
    pub cache_path: PathBuf,
    pub max_cache_size: u64,           // bytes
}

pub struct ThumbnailSize {
    pub name: String,    // "small", "medium", "large"
    pub width: u32,
    pub height: u32,
}
```

**生成流程**
1. **页面选择**: 选择第一页或封面页作为缩略图源
2. **图片解压**: 从存档中提取目标图片
3. **尺寸调整**: 按配置的尺寸规格调整图片
4. **格式转换**: 转换为指定格式（WebP推荐）
5. **质量压缩**: 根据质量设置进行压缩
6. **缓存存储**: 存储到缓存目录并管理缓存大小

#### 2.5.5 元数据提取系统

**元数据提取器接口**
```rust
pub trait MetadataExtractor: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> i32;
    async fn extract(&self, archive: &ArchiveInfo) -> Result<Metadata, MetadataError>;
    fn supported_formats(&self) -> &[String];
}

pub struct Metadata {
    pub title: Option<String>,
    pub series: Option<String>,
    pub volume: Option<String>,
    pub chapter: Option<String>,
    pub authors: Vec<String>,
    pub genres: Vec<String>,
    pub tags: Vec<String>,
    pub publisher: Option<String>,
    pub publish_date: Option<DateTime<Utc>>,
    pub language: Option<String>,
    pub page_count: Option<usize>,
    pub description: Option<String>,
}
```

**内置提取器**
1. **文件名解析器**: 从文件名提取标题、系列、卷号等
2. **ComicInfo.xml解析器**: 解析CBZ中的ComicInfo.xml元数据
3. **目录结构解析器**: 从文件夹层级结构推断元数据
4. **插件扩展器**: 通过插件系统扩展元数据来源

#### 2.5.6 处理任务调度

**任务队列管理**
```rust
pub struct TaskQueue {
    pub pending: Arc<Mutex<BTreeMap<i32, VecDeque<ProcessingTask>>>>, // 按优先级排序
    pub processing: Arc<Mutex<HashMap<String, ProcessingTask>>>,
    pub completed: Arc<Mutex<Vec<ProcessingTask>>>,
    pub failed: Arc<Mutex<Vec<ProcessingTask>>>,
    pub workers: Vec<TaskWorker>,
}

pub struct TaskWorker {
    pub id: String,
    pub worker_type: WorkerType,
    pub handle: JoinHandle<()>,
}

pub enum WorkerType {
    MetadataExtraction,
    ThumbnailGeneration, 
    AIAnalysis,
    General,
}
```

**调度策略**
- **优先级队列**: 高优先级任务（用户主动操作）优先处理
- **类型分离**: 不同类型的任务由专门的worker处理
- **负载均衡**: 动态调整worker数量和任务分配
- **错误重试**: 失败任务自动重试，带有退避机制
- **资源限制**: 控制并发任务数量，避免系统过载

#### 2.5.7 "new"标签管理

**特殊标签处理**
```rust
impl NewTagManager {
    // 分配"new"标签给新扫描的非重复文件
    pub async fn assign_new_tag(&self, archive_id: &str) -> Result<(), TagError> {
        let new_tag = self.get_or_create_new_tag().await?;
        self.tag_service.add_tag_to_archive(archive_id, &new_tag.id).await?;
        Ok(())
    }
    
    // 用户阅读超过第一页时自动移除"new"标签
    pub async fn remove_new_tag_on_read(&self, archive_id: &str, page: usize) -> Result<(), TagError> {
        if page > 1 {
            let new_tag = self.get_new_tag().await?;
            if let Some(tag) = new_tag {
                self.tag_service.remove_tag_from_archive(archive_id, &tag.id).await?;
            }
        }
        Ok(())
    }
}
```

**系统标签保护**
- "new"标签为系统保留标签，用户无法手动创建或删除
- 自动管理：扫描时添加，阅读时移除
- 过滤功能：用户可以通过"new"标签快速查看新添加的内容
## 3. Web前端设计 (Vue.js)

前端是用户直接交互的界面，注重响应式设计、美观和流畅的用户体验。

### 3.1 核心技术栈

| 组件 | 技术选型 | 版本要求 | 选型理由 |
|------|----------|----------|----------|
| **构建工具** | Vite | ^5.0 | 闪电般冷启动，即时HMR，现代前端标配 |
| **框架** | Vue 3 | ^3.4 | 组合式API，更好的逻辑复用和代码组织 |
| **CSS框架** | Tailwind CSS | ^3.4 | 功能优先，原子类快速构建，高开发效率 |
| **UI组件库** | Element Plus | ^2.4 | 高质量组件，与Tailwind良好结合 |
| **状态管理** | Pinia | ^2.1 | Vue官方推荐，简洁API，完美TypeScript支持 |
| **数据请求** | TanStack Query | ^5.0 | 异步状态管理，自动缓存、重试、后台刷新 |
| **路由** | Vue Router | ^4.2 | Vue官方路由，页面导航管理 |
| **TypeScript** | TypeScript | ^5.2 | 类型安全，更好的开发体验 |
| **图标** | Heroicons | ^2.0 | 美观的SVG图标库 |

### 3.2 项目结构

```
frontend/
├── public/                    # 静态资源
├── src/
│   ├── main.ts               # 应用入口
│   ├── App.vue               # 根组件
│   ├── assets/               # 资源文件
│   ├── components/           # 可复用组件
│   │   ├── ui/              # 基础UI组件
│   │   ├── layout/          # 布局组件
│   │   └── reader/          # 阅读器组件
│   ├── views/               # 页面组件
│   │   ├── Library.vue      # 书库页面
│   │   ├── Reader.vue       # 阅读器页面
│   │   ├── Details.vue      # 详情页面
│   │   └── Settings.vue     # 设置页面
│   ├── stores/              # Pinia状态管理
│   │   ├── auth.ts         # 认证状态
│   │   ├── reader.ts       # 阅读器状态
│   │   └── settings.ts     # 设置状态
│   ├── composables/         # 组合式函数
│   │   ├── useApi.ts       # API调用
│   │   ├── useReader.ts    # 阅读器逻辑
│   │   └── useInfiniteScroll.ts # 无限滚动
│   ├── types/               # TypeScript类型定义
│   │   ├── api.ts          # API类型
│   │   └── reader.ts       # 阅读器类型
│   └── utils/               # 工具函数
│       ├── request.ts      # HTTP请求工具
│       └── format.ts       # 格式化工具
├── tailwind.config.js       # Tailwind配置
├── vite.config.ts          # Vite配置
└── package.json            # 依赖管理
```

### 3.3 核心页面设计

#### 3.3.1 LibraryView - 主书库页面

**功能特性**
- 网格/列表视图切换
- 懒加载图片
- 无限滚动分页
- 实时搜索过滤
- 标签筛选
- 排序选项（标题、日期、大小）

**核心组件**
```vue
<template>
  <div class="library-container">
    <!-- 搜索栏 -->
    <SearchBar v-model="searchQuery" @search="handleSearch" />
    
    <!-- 过滤器 -->
    <FilterPanel 
      :tags="availableTags"
      :selected-tags="selectedTags"
      @update:tags="updateTags"
    />
    
    <!-- 视图切换 -->
    <ViewToggle v-model="viewMode" />
    
    <!-- 漫画网格 -->
    <ArchiveGrid 
      :archives="archives"
      :view-mode="viewMode"
      :loading="isLoading"
      @load-more="loadMore"
    />
  </div>
</template>
```

#### 3.3.2 ReaderView - 漫画阅读器

**功能特性**
- 单页/双页模式
- 缩放控制
- 页面预加载
- 快捷键导航
- 阅读进度自动保存
- 全屏模式

**核心组件**
```vue
<template>
  <div class="reader-container" @keydown="handleKeydown">
    <!-- 工具栏 -->
    <ReaderToolbar
      :current-page="currentPage"
      :total-pages="totalPages"
      :reading-mode="readingMode"
      @update:page="goToPage"
      @update:mode="setReadingMode"
    />
    
    <!-- 图片显示区域 -->
    <ImageViewer
      :images="currentImages"
      :zoom="zoomLevel"
      @zoom="handleZoom"
      @next="nextPage"
      @prev="prevPage"
    />
    
    <!-- 进度条 -->
    <ProgressBar :progress="readingProgress" />
  </div>
</template>
```

#### 3.3.3 状态管理设计

**认证状态 (auth.ts)**
```typescript
export const useAuthStore = defineStore('auth', () => {
  const apiKey = ref<string>('')
  const isAuthenticated = computed(() => !!apiKey.value)
  
  const login = async (key: string) => {
    // 验证API Key
    apiKey.value = key
    localStorage.setItem('apiKey', key)
  }
  
  const logout = () => {
    apiKey.value = ''
    localStorage.removeItem('apiKey')
  }
  
  return { apiKey, isAuthenticated, login, logout }
})
```

**阅读器状态 (reader.ts)**
```typescript
export const useReaderStore = defineStore('reader', () => {
  const currentArchive = ref<Archive | null>(null)
  const currentPage = ref(0)
  const readingMode = ref<'single' | 'double'>('single')
  const zoomLevel = ref(1)
  
  const setArchive = (archive: Archive) => {
    currentArchive.value = archive
    currentPage.value = 0
  }
  
  const nextPage = () => {
    if (currentPage.value < (currentArchive.value?.pageCount ?? 0) - 1) {
      currentPage.value++
    }
  }
  
  return { currentArchive, currentPage, readingMode, zoomLevel, setArchive, nextPage }
})
```

### 3.4 响应式设计

**断点设计**
```css
/* Tailwind CSS 断点 */
sm: 640px   /* 平板 */
md: 768px   /* 小桌面 */
lg: 1024px  /* 大桌面 */
xl: 1280px  /* 超大桌面 */
2xl: 1536px /* 4K显示器 */
```

**响应式策略**
- 移动端：单列网格，底部导航
- 平板：双列网格，侧边栏导航
- 桌面：多列网格，顶部导航
## 4. 桌面客户端设计 (Tauri)

Tauri将完成的Vue Web应用封装成高性能的本地桌面应用。

### 4.1 核心架构

**工作原理**
- **Rust后端进程** - 管理系统资源和原生功能
- **原生WebView** - 渲染Vue应用界面
- **IPC通信** - 前端与原生功能交互桥梁

### 4.2 技术配置

#### 4.2.1 依赖配置

**Cargo.toml 核心依赖**
```toml
[dependencies]
tauri = { version = "2.0", features = ["shell-open"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
dirs = "5.0"
notify = "6.0"
```

#### 4.2.2 Tauri配置 (tauri.conf.json)

```json
{
  "productName": "OtamoryX",
  "version": "1.0.0",
  "build": {
    "distDir": "../frontend/dist",
    "devPath": "http://localhost:5173",
    "withGlobalTauri": false
  },
  "app": {
    "windows": [
      {
        "title": "OtamoryX - 漫画阅读器",
        "width": 1200,
        "height": 800,
        "minWidth": 800,
        "minHeight": 600,
        "resizable": true,
        "fullscreenable": true
      }
    ],
    "security": {
      "csp": "default-src 'self'; img-src 'self' asset: https://asset.localhost"
    }
  },
  "allowlist": {
    "fs": {
      "all": false,
      "readFile": true,
      "readDir": true,
      "scope": ["$APPDATA/otamoryx/*", "$HOME/Pictures/*"]
    },
    "dialog": {
      "all": false,
      "open": true,
      "save": true
    },
    "notification": {
      "all": true
    },
    "shell": {
      "all": false,
      "open": true
    }
  }
}
```

### 4.3 原生功能集成

#### 4.3.1 文件系统操作

```rust
#[tauri::command]
async fn import_local_folder(path: String) -> Result<Vec<String>, String> {
    use std::fs;
    
    let entries = fs::read_dir(&path)
        .map_err(|e| format!("无法读取目录: {}", e))?;
    
    let mut comic_files = Vec::new();
    
    for entry in entries {
        let entry = entry.map_err(|e| format!("读取条目错误: {}", e))?;
        let path = entry.path();
        
        if let Some(ext) = path.extension() {
            if matches!(ext.to_str(), Some("cbz" | "cbr" | "zip" | "rar")) {
                if let Some(path_str) = path.to_str() {
                    comic_files.push(path_str.to_string());
                }
            }
        }
    }
    
    Ok(comic_files)
}
```

#### 4.3.2 系统通知

```rust
#[tauri::command] 
async fn show_notification(title: String, body: String) -> Result<(), String> {
    use tauri::api::notification::Notification;
    
    Notification::new("com.otamoryx.app")
        .title(&title)
        .body(&body)
        .show()
        .map_err(|e| format!("通知发送失败: {}", e))?;
    
    Ok(())
}
```

#### 4.3.3 窗口管理

```rust
#[tauri::command]
async fn toggle_fullscreen(window: tauri::Window) -> Result<(), String> {
    let is_fullscreen = window.is_fullscreen()
        .map_err(|e| format!("获取全屏状态失败: {}", e))?;
    
    window.set_fullscreen(!is_fullscreen)
        .map_err(|e| format!("设置全屏失败: {}", e))?;
    
    Ok(())
}

#[tauri::command]
async fn set_window_title(window: tauri::Window, title: String) -> Result<(), String> {
    window.set_title(&title)
        .map_err(|e| format!("设置标题失败: {}", e))?;
    
    Ok(())
}
```

### 4.4 Vue前端集成

#### 4.4.1 Tauri API调用

```typescript
// composables/useTauri.ts
import { invoke } from '@tauri-apps/api/tauri'
import { open } from '@tauri-apps/api/dialog'
import { sendNotification } from '@tauri-apps/api/notification'

export const useTauri = () => {
  // 导入本地文件夹
  const importLocalFolder = async (): Promise<string[]> => {
    const selected = await open({
      multiple: false,
      directory: true,
      title: '选择漫画文件夹'
    })
    
    if (selected && typeof selected === 'string') {
      return await invoke('import_local_folder', { path: selected })
    }
    
    return []
  }
  
  // 显示系统通知
  const showNotification = async (title: string, body: string) => {
    if ('__TAURI__' in window) {
      await invoke('show_notification', { title, body })
    } else {
      // Web环境降级处理
      if (Notification.permission === 'granted') {
        new Notification(title, { body })
      }
    }
  }
  
  // 切换全屏
  const toggleFullscreen = async () => {
    if ('__TAURI__' in window) {
      await invoke('toggle_fullscreen')
    } else {
      // Web环境降级处理
      if (document.fullscreenElement) {
        document.exitFullscreen()
      } else {
        document.documentElement.requestFullscreen()
      }
    }
  }
  
  return {
    importLocalFolder,
    showNotification,
    toggleFullscreen
  }
}
```

#### 4.4.2 环境检测

```typescript
// utils/platform.ts
export const isPlatform = {
  tauri: typeof window !== 'undefined' && '__TAURI__' in window,
  web: typeof window !== 'undefined' && !('__TAURI__' in window),
  desktop: typeof window !== 'undefined' && '__TAURI__' in window,
  mobile: /Android|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(
    typeof navigator !== 'undefined' ? navigator.userAgent : ''
  )
}

export const getBaseUrl = () => {
  if (isPlatform.tauri) {
    // Tauri环境从配置读取
    return localStorage.getItem('serverUrl') || 'http://localhost:3000'
  }
  // Web环境使用相对路径
  return ''
}
```

### 4.5 构建与打包

#### 4.5.1 开发模式

```bash
# 启动后端服务
cd backend && cargo run

# 启动前端开发服务器
cd frontend && pnpm dev

# 启动Tauri开发模式（新终端）
cd frontend && cargo tauri dev
```

#### 4.5.2 生产构建

```bash
# 构建前端
cd frontend && pnpm build

# 构建Tauri应用
cd frontend && cargo tauri build
```

**输出文件位置**
```
frontend/src-tauri/target/release/bundle/
├── deb/              # Linux .deb包
├── appimage/         # Linux AppImage
├── msi/              # Windows安装包  
├── nsis/             # Windows NSIS安装包
└── macos/            # macOS .app和.dmg
```
## 5. 开发指南与规范

### 5.1 开发环境搭建

#### 5.1.1 系统要求

| 组件 | 最低要求 | 推荐配置 |
|------|----------|----------|
| **操作系统** | Windows 10/macOS 10.15/Ubuntu 20.04 | 最新稳定版 |
| **Rust** | 1.70+ | 1.75+ |
| **Node.js** | 18.0+ | 20.0+ LTS |
| **内存** | 8GB | 16GB+ |
| **磁盘空间** | 10GB | 20GB+ |

#### 5.1.2 开发工具链安装

```bash
# 1. 安装Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. 安装Node.js (推荐使用nvm)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 20
nvm use 20

# 3. 安装pnpm
npm install -g pnpm

# 4. 安装Tauri CLI
cargo install tauri-cli

# 5. 系统依赖 (Linux)
sudo apt update
sudo apt install libwebkit2gtk-4.0-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev
```

### 5.2 开发工作流程

#### 5.2.1 项目启动顺序

```bash
# 终端1: 启动后端服务
cd backend
cargo run --release

# 终端2: 启动前端开发服务器
cd frontend  
pnpm dev

# 终端3: 启动Tauri开发模式 (可选)
cd frontend
cargo tauri dev
```

#### 5.2.2 Git工作流

**分支策略**
- `main` - 主分支，稳定版本
- `develop` - 开发分支，集成最新功能
- `feature/*` - 功能分支
- `hotfix/*` - 紧急修复分支

**提交规范 (Conventional Commits)**
```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**类型说明**
- `feat` - 新功能
- `fix` - 修复Bug
- `docs` - 文档更新
- `style` - 代码格式调整
- `refactor` - 重构
- `test` - 测试相关
- `chore` - 构建/工具链更新

### 5.3 代码规范

#### 5.3.1 Rust代码规范

**基础规范**
```rust
// 使用标准格式化工具
// cargo fmt

// 使用Clippy进行静态检查
// cargo clippy -- -D warnings

// 文档注释示例
/// 从存档中提取指定页面的图片
/// 
/// # Arguments
/// * `archive_path` - 存档文件路径
/// * `page_number` - 页面编号 (从0开始)
/// 
/// # Returns
/// * `Result<Vec<u8>, ExtractorError>` - 图片二进制数据或错误
/// 
/// # Examples
/// ```
/// let image_data = extract_page_image(&path, 0).await?;
/// ```
pub async fn extract_page_image(
    archive_path: &Path,
    page_number: usize,
) -> Result<Vec<u8>, ExtractorError> {
    // 实现逻辑
}
```

### 2.6 插件系统架构

#### 2.6.1 插件框架设计

**核心组件**
- **插件管理器**: 负责插件的加载、卸载和生命周期管理
- **钩子系统**: 提供系统扩展点，允许插件注入自定义逻辑
- **权限控制**: 插件权限声明和运行时验证
- **通信接口**: 插件与核心系统的数据交换

**插件生命周期**
```rust
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn initialize(&self, context: &PluginContext) -> Result<(), PluginError>;
    fn shutdown(&self) -> Result<(), PluginError>;
    fn get_capabilities(&self) -> Vec<PluginCapability>;
}

pub enum PluginCapability {
    MetadataExtraction,
    CustomEndpoint,
    ScheduledTask,
    ArchiveProcessing,
    SearchExtension,
}
```

#### 2.6.2 插件API接口

**元数据扩展接口**
```rust
pub trait MetadataExtractor: Plugin {
    async fn extract_metadata(&self, archive_path: &Path) -> Result<HashMap<String, Value>, PluginError>;
    fn supported_formats(&self) -> Vec<String>;
}
```

**自定义端点接口**
```rust
pub trait EndpointProvider: Plugin {
    fn register_routes(&self, router: &mut Router) -> Result<(), PluginError>;
    fn endpoint_prefix(&self) -> String; // e.g., "/api/v1/plugins/my-plugin"
}
```

**定时任务接口**
```rust
pub trait ScheduledTaskProvider: Plugin {
    fn get_schedule(&self) -> CronSchedule;
    async fn execute_task(&self, context: &TaskContext) -> Result<(), PluginError>;
}
```

#### 2.6.3 插件安全模型

**权限声明**
```toml
# plugin.toml
[plugin]
name = "metadata-sync"
version = "1.0.0"
description = "Synchronize metadata from external sources"

[permissions]
network = true
filesystem_read = ["/path/to/comics"]
database_read = true
database_write = ["tags", "archives.metadata"]
custom_endpoints = true
scheduled_tasks = true
```

**沙盒限制**
- 文件系统访问限制在声明的路径范围内
- 网络访问需要明确权限声明
- 数据库操作限制在允许的表和字段
- API端点注册需要权限验证

#### 2.6.4 插件开发SDK

**插件模板**
```rust
use otamoryx_plugin_api::*;

#[derive(Default)]
pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn name(&self) -> &str { "my-plugin" }
    fn version(&self) -> &str { "1.0.0" }
    
    fn initialize(&self, context: &PluginContext) -> Result<(), PluginError> {
        // 插件初始化逻辑
        Ok(())
    }
    
    fn get_capabilities(&self) -> Vec<PluginCapability> {
        vec![PluginCapability::MetadataExtraction]
    }
}

impl MetadataExtractor for MyPlugin {
    async fn extract_metadata(&self, archive_path: &Path) -> Result<HashMap<String, Value>, PluginError> {
        // 元数据提取逻辑
        Ok(HashMap::new())
    }
    
    fn supported_formats(&self) -> Vec<String> {
        vec!["cbz".to_string(), "cbr".to_string()]
    }
}

// 插件导出宏
plugin_export!(MyPlugin);
```

### 2.6 AI自动标签系统（集成到统一流水线）

#### 2.6.1 AI分析器接口

**AI分析器抽象**
```rust
pub trait AIAnalyzer: Send + Sync {
    fn name(&self) -> &str;
    fn model_type(&self) -> AIModelType;
    async fn analyze_archive(&self, archive: &ArchiveInfo) -> Result<AIAnalysisResult, AIError>;
    async fn health_check(&self) -> Result<bool, AIError>;
    fn supports_format(&self, format: &str) -> bool;
}

pub enum AIModelType {
    LocalModel(String),    // 本地模型路径
    CloudAPI(String),      // API端点
    Plugin(String),        // 插件提供的AI服务
}

pub struct AIAnalysisResult {
    pub suggested_tags: Vec<SuggestedTag>,
    pub confidence_summary: f32,
    pub processing_time: Duration,
    pub model_version: String,
}

pub struct SuggestedTag {
    pub name: String,
    pub namespace: String,
    pub confidence: f32,  // 0.0 - 1.0
    pub reasoning: Option<String>,
}
```

#### 2.6.2 统一流水线中的AI处理

**AI任务集成**
```rust
impl ProcessingPipeline {
    async fn process_ai_analysis(&self, task: &ProcessingTask) -> Result<(), ProcessingError> {
        let archive = self.storage.get_archive(&task.archive_id).await?;
        
        // 1. 选择合适的AI分析器
        let analyzer = self.select_ai_analyzer(&archive).await?;
        
        // 2. 检查AI服务可用性
        if !analyzer.health_check().await? {
            return Err(ProcessingError::AIServiceUnavailable);
        }
        
        // 3. 执行AI分析
        let analysis_result = analyzer.analyze_archive(&archive).await?;
        
        // 4. 处理分析结果
        self.process_ai_results(&task.archive_id, analysis_result).await?;
        
        Ok(())
    }
    
    async fn process_ai_results(&self, archive_id: &str, result: AIAnalysisResult) -> Result<(), ProcessingError> {
        for suggested_tag in result.suggested_tags {
            // 创建或获取标签
            let tag = self.get_or_create_tag(&suggested_tag.name, &suggested_tag.namespace).await?;
            
            // 存储AI生成的标签建议
            let ai_tag = AIGeneratedTag {
                id: Uuid::new_v4().to_string(),
                archive_id: archive_id.to_string(),
                tag_id: tag.id,
                confidence_score: suggested_tag.confidence,
                approved: None, // 等待用户审核
                created_at: Utc::now(),
                reviewed_at: None,
                reviewed_by: None,
            };
            
            self.storage.save_ai_generated_tag(ai_tag).await?;
            
            // 如果置信度足够高，自动应用标签
            if suggested_tag.confidence >= self.config.auto_apply_threshold {
                self.auto_apply_ai_tag(archive_id, &tag.id).await?;
            }
        }
        
        Ok(())
    }
}
```

#### 2.6.3 AI模型实现示例

**本地模型实现**
```rust
pub struct LocalImageAnalyzer {
    model_path: String,
    runtime: Arc<ort::Session>,
    preprocessor: ImagePreprocessor,
}

impl AIAnalyzer for LocalImageAnalyzer {
    fn name(&self) -> &str { "Local Image Classifier" }
    
    fn model_type(&self) -> AIModelType {
        AIModelType::LocalModel(self.model_path.clone())
    }
    
    async fn analyze_archive(&self, archive: &ArchiveInfo) -> Result<AIAnalysisResult, AIError> {
        // 1. 提取代表性图像（前几页）
        let sample_images = self.extract_sample_images(archive, 3).await?;
        
        // 2. 预处理图像
        let processed_images = self.preprocessor.process_batch(sample_images).await?;
        
        // 3. 模型推理
        let predictions = self.runtime.run(processed_images).await?;
        
        // 4. 后处理和标签映射
        let suggested_tags = self.postprocess_predictions(predictions)?;
        
        Ok(AIAnalysisResult {
            suggested_tags,
            confidence_summary: self.calculate_overall_confidence(&suggested_tags),
            processing_time: start_time.elapsed(),
            model_version: "v1.0.0".to_string(),
        })
    }
}
```

**云端API实现**
```rust
pub struct CloudVisionAnalyzer {
    api_client: Arc<reqwest::Client>,
    api_key: String,
    endpoint: String,
}

impl AIAnalyzer for CloudVisionAnalyzer {
    async fn analyze_archive(&self, archive: &ArchiveInfo) -> Result<AIAnalysisResult, AIError> {
        // 1. 提取并编码图像
        let cover_image = self.extract_cover_image(archive).await?;
        let encoded_image = base64::encode(cover_image);
        
        // 2. 构建API请求
        let request = CloudVisionRequest {
            image: encoded_image,
            features: vec!["label_detection", "text_detection"],
            max_results: 10,
        };
        
        // 3. 调用云端API
        let response = self.api_client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;
        
        // 4. 解析响应并转换为标签
        let cloud_result: CloudVisionResponse = response.json().await?;
        let suggested_tags = self.convert_to_tags(cloud_result)?;
        
        Ok(AIAnalysisResult {
            suggested_tags,
            confidence_summary: self.calculate_confidence(&cloud_result),
            processing_time: start_time.elapsed(),
            model_version: "cloud-api-v1".to_string(),
        })
    }
}
```

#### 2.6.4 AI配置和管理

**配置结构**
```rust
pub struct AIConfig {
    pub enabled: bool,
    pub auto_apply_threshold: f32,          // 自动应用标签的置信度阈值
    pub processing_schedule: AISchedule,    // 处理调度配置
    pub resource_limits: AIResourceLimits, // 资源限制
    pub enabled_analyzers: Vec<String>,     // 启用的分析器列表
}

pub struct AISchedule {
    pub immediate: bool,        // 立即处理
    pub batch_processing: bool, // 批处理模式
    pub off_peak_hours: Option<Vec<u8>>, // 非高峰时段（小时）
}

pub struct AIResourceLimits {
    pub max_concurrent_tasks: usize,
    pub max_memory_usage: u64,      // bytes
    pub timeout_seconds: u64,
    pub max_retries: u32,
}
```

#### 2.6.5 用户交互和审核

**审核界面设计**
```rust
pub struct AITagReviewService {
    storage: Arc<dyn Storage>,
    user_service: Arc<UserService>,
}

impl AITagReviewService {
    // 获取待审核的AI标签
    pub async fn get_pending_ai_tags(&self, user_id: &str, limit: usize) -> Result<Vec<AITagReview>, ServiceError> {
        let tags = self.storage.get_pending_ai_tags(user_id, limit).await?;
        
        Ok(tags.into_iter().map(|tag| AITagReview {
            id: tag.id,
            archive_title: tag.archive_title,
            tag_name: tag.tag_name,
            namespace: tag.namespace,
            confidence: tag.confidence_score,
            preview_images: vec![], // 可以添加预览图
            created_at: tag.created_at,
        }).collect())
    }
    
    // 批量审核AI标签
    pub async fn review_ai_tags(&self, user_id: &str, reviews: Vec<AITagDecision>) -> Result<(), ServiceError> {
        for decision in reviews {
            match decision.action {
                ReviewAction::Approve => {
                    self.approve_ai_tag(&decision.tag_id, user_id).await?;
                }
                ReviewAction::Reject => {
                    self.reject_ai_tag(&decision.tag_id, user_id).await?;
                }
                ReviewAction::Edit => {
                    self.edit_and_approve_ai_tag(&decision.tag_id, user_id, &decision.edited_name).await?;
                }
            }
        }
        
        Ok(())
    }
}

pub struct AITagDecision {
    pub tag_id: String,
    pub action: ReviewAction,
    pub edited_name: Option<String>,
}

pub enum ReviewAction {
    Approve,
    Reject,
    Edit,
}
```

#### 2.6.6 AI处理监控和统计

**处理状态监控**
```rust
pub struct AIProcessingMonitor {
    pub queue_size: usize,
    pub processing_count: usize,
    pub completed_today: usize,
    pub failed_today: usize,
    pub average_processing_time: Duration,
    pub active_models: Vec<String>,
}
```

**统计数据**
- AI标签生成统计
- 用户审核行为分析
- 模型性能指标
- 资源使用情况
- 错误率和重试统计

**错误处理**
```rust
// 使用自定义错误类型
#[derive(Debug, thiserror::Error)]
pub enum ExtractorError {
    #[error("文件不存在: {path}")]
    FileNotFound { path: String },
    
    #[error("不支持的格式: {format}")]
    UnsupportedFormat { format: String },
    
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),
}

// 使用Result<T, E>进行错误传播
pub fn process_archive(path: &Path) -> Result<Archive, ExtractorError> {
    let metadata = std::fs::metadata(path)
        .map_err(|_| ExtractorError::FileNotFound { 
            path: path.to_string_lossy().to_string() 
        })?;
    
    // 处理逻辑...
    Ok(archive)
}
```

#### 5.3.2 Vue.js代码规范

**组合式API规范**
```typescript
// 使用<script setup>语法
<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import type { Archive } from '@/types/api'

// Props定义
interface Props {
  archives: Archive[]
  loading?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  loading: false
})

// 响应式状态
const selectedId = ref<string | null>(null)
const searchQuery = ref('')

// 计算属性
const filteredArchives = computed(() => {
  if (!searchQuery.value) return props.archives
  
  return props.archives.filter(archive =>
    archive.title.toLowerCase().includes(searchQuery.value.toLowerCase())
  )
})

// 方法
const selectArchive = (id: string) => {
  selectedId.value = id
}

// 生命周期
onMounted(() => {
  // 初始化逻辑
})
</script>
```

**TypeScript类型定义**
```typescript
// types/api.ts
export interface Archive {
  readonly id: string
  readonly title: string
  readonly path: string
  readonly pageCount: number
  readonly fileSize: number
  readonly createdAt: string
  readonly updatedAt: string
  readonly tags: Tag[]
}

export interface Tag {
  readonly id: string
  readonly name: string
  readonly namespace: string
}

export interface PaginatedResponse<T> {
  readonly data: T[]
  readonly page: number
  readonly limit: number
  readonly total: number
  readonly hasNext: boolean
}
```

### 5.4 测试策略

#### 5.4.1 后端测试

```rust
// 单元测试
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_extract_page_image() {
        let temp_dir = TempDir::new().unwrap();
        let archive_path = temp_dir.path().join("test.cbz");
        
        // 创建测试文件...
        
        let result = extract_page_image(&archive_path, 0).await;
        assert!(result.is_ok());
    }
}

// 集成测试
#[tokio::test]
async fn test_api_get_archives() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/archives")
                .header("Authorization", "Bearer test-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}
```

#### 5.4.2 前端测试

```typescript
// 组件测试 (Vitest + Vue Test Utils)
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import ArchiveCard from '@/components/ArchiveCard.vue'

describe('ArchiveCard', () => {
  it('renders archive title correctly', () => {
    const archive = {
      id: '1',
      title: '测试漫画',
      pageCount: 20,
      // ... 其他属性
    }
    
    const wrapper = mount(ArchiveCard, {
      props: { archive }
    })
    
    expect(wrapper.text()).toContain('测试漫画')
  })
})

// E2E测试 (Playwright)
import { test, expect } from '@playwright/test'

test('library page loads and displays archives', async ({ page }) => {
  await page.goto('/library')
  
  // 等待数据加载
  await page.waitForSelector('[data-testid="archive-grid"]')
  
  // 检查是否显示漫画列表
  const archiveCards = page.locator('[data-testid="archive-card"]')
  await expect(archiveCards).toHaveCountGreaterThan(0)
})
```

### 5.5 性能优化指南

#### 5.5.1 后端性能优化

```rust
// 1. 使用连接池
use sqlx::postgres::PgPoolOptions;

let pool = PgPoolOptions::new()
    .max_connections(20)
    .connect(&database_url)
    .await?;

// 2. 缓存策略
use moka::future::Cache;

let image_cache = Cache::builder()
    .max_capacity(1000)
    .time_to_live(Duration::from_secs(3600))
    .build();

// 3. 异步处理
use tokio::task;

let handle = task::spawn(async move {
    // 耗时操作
    process_archive(path).await
});
```

#### 5.5.2 前端性能优化

```typescript
// 1. 组件懒加载
const ReaderView = defineAsyncComponent(() => 
  import('@/views/ReaderView.vue')
)

// 2. 图片懒加载
<img 
  :src="imageSrc"
  loading="lazy"
  :data-src="fullImageSrc"
  @load="handleImageLoad"
/>

// 3. 虚拟滚动
import { VirtualList } from '@tanstack/vue-virtual'

<VirtualList 
  :data="archives"
  :itemSize="200"
  :containerHeight="600"
>
  <template #default="{ item }">
    <ArchiveCard :archive="item" />
  </template>
</VirtualList>
```

### 5.6 调试与故障排除

#### 5.6.1 常见问题解决

**编译错误**
```bash
# Rust编译失败
cargo clean && cargo build

# 前端构建失败  
pnpm install --force
rm -rf node_modules/.vite
pnpm build
```

**运行时错误**
```bash
# 检查后端日志
RUST_LOG=debug cargo run

# 检查前端控制台
# 浏览器开发者工具 -> Console

# Tauri调试
cargo tauri dev --debug
```

#### 5.6.2 性能分析

```bash
# Rust性能分析
cargo install cargo-profiler
cargo profiler callgrind --bin otamoryx-server

# 前端性能分析
# Chrome DevTools -> Performance
# Vue DevTools -> Performance
```

## 6. 部署架构

### 6.1 部署环境设计

#### 6.1.1 生产环境架构
```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Load Balancer │    │   Web Server    │    │  Application    │
│    (Nginx)      │───▶│    (Nginx)      │───▶│   (Rust API)    │
│                 │    │                 │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Static Files  │    │   Vue Frontend  │    │   SQLite DB     │
│   (CDN/S3)      │    │   (SPA)         │    │   (File)        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

#### 6.1.2 容器化部署策略
- **后端服务**: Docker 容器 + 多阶段构建
- **前端静态文件**: Nginx 容器或 CDN
- **数据持久化**: 卷挂载 SQLite 文件
- **配置管理**: 环境变量 + 配置文件

### 6.2 扩展性考虑

#### 6.2.1 水平扩展
- 多后端实例负载均衡
- 共享存储 (NFS/S3) 替代本地文件
- PostgreSQL 替代 SQLite
- Redis 缓存层

#### 6.2.2 性能优化
- CDN 静态资源分发
- 图片缓存和压缩
- API 响应缓存
- 数据库连接池

---

*详细的部署配置请参考 [deployment.md](./deployment.md)*

## 7. 安全架构

### 7.1 安全设计原则

#### 7.1.1 多层防护
- **网络层**: HTTPS、防火墙、DDoS 防护
- **应用层**: API Key 认证、输入验证、CSRF 防护
- **数据层**: 敏感数据脱敏、安全日志

#### 7.1.2 最小权限原则
- API Key 权限分级
- 文件系统访问限制
- 容器运行时权限控制

### 7.2 威胁模型

| 威胁类型 | 风险等级 | 主要防护措施 |
|----------|----------|--------------|
| 未授权访问 | 高 | API Key 认证 |
| 路径遍历 | 高 | 路径验证和沙盒 |
| XSS攻击 | 中 | CSP + 输入过滤 |
| 文件上传攻击 | 中 | 类型验证 + 扫描 |

### 7.3 认证与授权

#### 7.3.1 认证流程
```
Client ──(API Key)──▶ Middleware ──(Validation)──▶ Handler
   │                      │                          │
   │                      ▼                          │
   │              ┌─────────────┐                    │
   └─────(401)────│ Auth Failed │                    │
                  └─────────────┘                    │
                                                     ▼
                                              ┌─────────────┐
                                              │  Response   │
                                              └─────────────┘
```

#### 7.3.2 权限模型
- **只读权限**: 浏览漫画、搜索
- **读写权限**: 上传文件、修改元数据
- **管理权限**: 系统设置、用户管理
