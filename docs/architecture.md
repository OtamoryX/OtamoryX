# OtamoryX 在线漫画阅读器技术架构文档

**版本**: 1.0  
**日期**: 2025年7月28日  
**技术栈**: Vue + Tauri + Rust

## 1. 项目概述

### 1.1 项目目标
本项目旨在开发一个功能强大、性能卓越的在线漫画阅读和管理工具，对标 LANraragi 等现有解决方案。系统采用现代化三层架构：
- **核心后端服务** - 数据管理与API服务
- **Web前端界面** - 响应式现代化用户界面  
- **跨平台桌面客户端** - 原生桌面体验

### 1.2 核心技术选型
为实现高性能、高安全性和卓越的开发体验，采用以下技术栈：

| 组件 | 技术 | 选型理由 |
|------|------|----------|
| **后端服务** | Rust | 内存安全、高并发性能、系统底层控制力 |
| **Web前端** | Vue.js v3 | 渐进式框架、优秀生态、卓越开发体验 |
| **桌面客户端** | Tauri | 现代化构建、原生WebView、极小体积 |

**核心优势**: Rust生态协同效应 + 前端代码完全复用，一套Vue代码既可部署为网站，也可无缝打包成原生桌面应用。
## 2. 后端服务设计 (Rust)

后端是整个系统的核心，负责漫画管理、数据处理和API服务。

### 2.1 核心技术栈

| 组件 | 技术选型 | 版本要求 | 选型理由 |
|------|----------|----------|----------|
| **Web框架** | Axum | ^0.7 | tokio团队开发，异步生态无缝集成，模块化设计 |
| **数据库** | SeaORM | latest | 异步SQL，编译时检查，轻量级嵌入式 |
| **序列化** | serde + serde_json | ^1.0 | 高性能序列化，生态丰富 |
| **HTTP客户端** | reqwest | ^0.11 | 现代异步HTTP客户端 |
| **日志** | tracing + tracing-subscriber | ^0.1 | 结构化异步日志 |
| **配置管理** | config | ^0.14 | 多格式配置文件支持 |

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
│   ├── opds.rs            # OPDS协议实现
│   └── settings.rs        # 设置API
├── services/
│   ├── mod.rs             # 业务逻辑服务
│   ├── archive_service.rs # 漫画处理服务
│   ├── auth_service.rs    # 认证服务
│   └── search_service.rs  # 搜索服务
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

#### 系统初始化 (`/api/v1/system`)
| 方法 | 端点 | 描述 | 请求参数 | 响应格式 |
|------|------|------|----------|----------|
| `GET` | `/system/status` | 获取系统状态 | - | `SystemStatus` |
| `POST` | `/system/initialize` | 初始化系统管理员 | `InitializeSystemRequest` | `AuthResponse` |

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
| `GET` | `/archives/{id}` | 获取漫画详情 | 路径参数: `id` | `Archive` |
| `GET` | `/archives/{id}/pages/{page}` | 获取页面图片 | 路径参数: `id`, `page` | 图片二进制数据 |
| `GET` | `/archives/{id}/progress` | 获取阅读进度 | 路径参数: `id` | `ReadingProgress` |
| `POST` | `/archives/{id}/progress` | 更新阅读进度 | `UpdateProgressRequest` | `200 OK` |

#### 搜索和标签 (`/api/v1`)
| 方法 | 端点 | 描述 | 请求参数 | 响应格式 |
|------|------|------|----------|----------|
| `GET` | `/search` | 高级搜索漫画 | `query`, `tags`, `minPages`, `maxPages`, `minFileSize`, `maxFileSize`, `sortBy`, `sortOrder`, `page`, `limit` | `PaginatedResponse<Archive>` |
| `GET` | `/tags` | 获取标签列表 | - | `Array<Tag>` |

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
| `DELETE` | `/categories/{id}/archives` | 从分类移除漫画 | `AddArchivesToCategoryRequest` | `200 OK` |

#### 系统设置 (`/api/v1/settings`)
| 方法 | 端点 | 描述 | 请求参数 | 响应格式 |
|------|------|------|----------|----------|
| `GET` | `/settings` | 获取系统设置 | - | `SystemSettings` |
| `PUT` | `/settings` | 更新系统设置 | `SystemSettings` | `200 OK` |

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

### 2.4 数据库设计

#### 2.4.1 表结构设计

```sql
-- 漫画存档表
CREATE TABLE archives (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
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
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    namespace TEXT DEFAULT 'general'
);

-- 漫画标签关联表
CREATE TABLE archive_tags (
    archive_id INTEGER,
    tag_id INTEGER,
    PRIMARY KEY (archive_id, tag_id),
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

-- 阅读进度表
CREATE TABLE reading_progress (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    archive_id INTEGER NOT NULL,
    user_id TEXT NOT NULL,
    current_page INTEGER NOT NULL DEFAULT 0,
    total_pages INTEGER NOT NULL,
    progress_percentage REAL NOT NULL DEFAULT 0.0,
    last_read_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE
);
```

### 2.5 存档处理

#### 2.5.1 支持格式与处理方式

| 格式 | 扩展名 | 处理库 | 技术要求 |
|------|--------|--------|----------|
| **CBZ** | `.cbz`, `.zip` | `zip` crate | 纯Rust实现，无外部依赖 |
| **CBR** | `.cbr`, `.rar` | `unrar` crate | 需要系统安装unrar库 |
| **CB7** | `.cb7`, `.7z` | `sevenz-rust` | 纯Rust实现 |
| **PDF** | `.pdf` | `pdf-extract` | 图片提取支持 |

#### 2.5.2 图片处理流程

```rust
// 核心处理流程示例
async fn extract_page_image(
    archive_path: &Path,
    page_number: usize,
) -> Result<Vec<u8>, ExtractorError> {
    // 1. 识别存档格式
    // 2. 使用对应解压器获取图片
    // 3. 图片格式转换（可选）
    // 4. 压缩优化（可选）
    // 5. 返回二进制数据
}
```
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
