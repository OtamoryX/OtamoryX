# OtamoryX

**OtamoryX** 是一个开源、可自部署的数字漫画阅读器和管理平台，旨在为用户提供现代化、功能丰富的替代方案（对标 LANraragi 等现有解决方案）。

## ✨ 核心特性

- **🏠 自托管**: 运行您自己的私人漫画图书馆服务器
- **🌐 多平台**: 当前支持 Web 浏览器访问，桌面端封装规划中
- **📚 智能管理**: 高级分类、搜索和元数据管理
- **🔍 强大搜索**: 全文搜索，支持高级过滤选项
- **📖 流畅阅读**: 响应式漫画阅读界面，带有进度跟踪
- **🏷️ 智能标签**: 自动重复检测和智能标签系统
- **👥 多用户支持**: 用户管理与基于路径的权限控制
- **🔌 插件系统**: 已提供内置插件、插件管理、manifest、权限和执行历史；外部动态库插件 Runtime 仍在开发中
- **🤖 AI 功能**: 已提供 AI 连接配置、标题翻译与语言检测队列；个性化内容净化与推荐仍在规划中

## 🛠️ 技术栈

- **后端**: Rust + Axum Web 框架，SQLite 数据库
- **前端**: Vue.js 3 + TypeScript，Tailwind CSS
- **桌面端（规划中）**: 后续基于 Tauri 提供跨平台原生应用
- **API**: RESTful JSON API，支持 OPDS 协议

## 📋 支持格式

- **CBZ** (漫画书 ZIP 格式)
- **CBR** (漫画书 RAR 格式)
- **CB7** (漫画书 7z 格式)  
- **标准压缩包** (ZIP, RAR) 包含图片
- **图片格式**: JPG, JPEG, PNG, WebP

## 🚀 快速开始

### Docker 部署（推荐）

当前 `master` 分支构建的镜像已发布到 GitHub Container Registry：
`ghcr.io/otamoryx/otamoryx:main-unstable-latest`。

```bash
# 准备持久化目录
mkdir -p data comics cache

export JWT_SECRET="replace-with-a-long-random-secret"

# 运行已发布的镜像
docker run -d \
  --name otamoryx \
  --restart unless-stopped \
  -p 3000:3000 \
  -e JWT_SECRET="$JWT_SECRET" \
  -e DATABASE_URL=sqlite:/app/data/otamoryx.db \
  -e COMICS_PATH=/app/data/comics \
  -v "$PWD/data:/app/data" \
  -v "$PWD/comics:/app/data/comics" \
  -v "$PWD/cache:/app/data/cache" \
  ghcr.io/otamoryx/otamoryx:main-unstable-latest
```

Web 应用地址为 `http://localhost:3000`，容器内后端监听 `8080`。

使用 Compose 时，设置 `JWT_SECRET` 后执行 `docker compose up -d` 即可；仓库内的 Compose 文件默认使用相同的已发布镜像。稳定版本发布后，可以设置 `OTAMORYX_IMAGE=ghcr.io/otamoryx/otamoryx:latest` 切换到稳定镜像。

如果要从源码构建，请执行 `docker build -t otamoryx:local .`，然后运行 `OTAMORYX_IMAGE=otamoryx:local docker compose up -d`。

卷挂载和配置说明请参阅[部署指南](docs/deployment.md)。

### 本地开发
```bash
cp backend/.env.example backend/.env
cd backend && cargo run

# 另一个终端
cd frontend && pnpm install && pnpm dev
```

### 桌面应用程序
仓库目前尚未包含 Tauri 桌面工程，桌面打包仍属于路线图内容，暂没有可下载的安装程序。

## 📖 文档

详细信息请参考我们的完整文档：

- **[📋 需求文档](docs/requirements.md)** - 详细的功能需求和规范说明
- **[🏗️ 架构文档](docs/architecture.md)** - 技术架构和系统设计
- **[🛣️ 开发路线图](docs/roadmap.md)** - 开发路线图和计划功能
- **[🚀 部署指南](docs/deployment.md)** - 当前安装与部署说明
- **[👩‍💻 开发指南](docs/development.md)** - 当前开发环境与工作流说明
- API 参考文档 - 暂未单独发布

## 🤝 贡献

我们欢迎社区贡献！请查看我们的 [贡献指南](CONTRIBUTING.md) 了解如何开始。

## 📄 许可证

本项目采用 GNU 通用公共许可证 v3.0 - 详情请参阅 [LICENSE](LICENSE) 文件。

## 🌟 支持

如果您觉得 OtamoryX 有用，请考虑在 GitHub 上给它一个星标！
