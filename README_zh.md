# OtamoryX

**OtamoryX** 是一个开源、可自部署的数字漫画阅读器和管理平台，旨在为用户提供现代化、功能丰富的替代方案（对标 LANraragi 等现有解决方案）。

## ✨ 核心特性

- **🏠 自托管**: 运行您自己的私人漫画图书馆服务器
- **🌐 多平台**: 支持 Web 浏览器访问和原生桌面应用程序
- **📚 智能管理**: 高级分类、搜索和元数据管理
- **🔍 强大搜索**: 全文搜索，支持高级过滤选项
- **📖 流畅阅读**: 响应式漫画阅读界面，带有进度跟踪
- **🏷️ 智能标签**: 自动重复检测和智能标签系统
- **👥 多用户支持**: 用户管理与基于路径的权限控制
- **🔌 插件系统**: 可扩展架构，支持社区驱动的功能 *(v1.1.0+)*
- **🤖 AI 自动标签**: 实验性 AI 驱动的内容分析 *(v1.2.0+)*

## 🛠️ 技术栈

- **后端**: Rust + Axum Web 框架，SQLite 数据库
- **前端**: Vue.js 3 + TypeScript，Tailwind CSS
- **桌面端**: Tauri 框架，跨平台原生应用
- **API**: RESTful JSON API，支持 OPDS 协议

## 📋 支持格式

- **CBZ** (漫画书 ZIP 格式)
- **CBR** (漫画书 RAR 格式)
- **CB7** (漫画书 7z 格式)  
- **标准压缩包** (ZIP, RAR) 包含图片
- **图片格式**: JPG, JPEG, PNG, WebP

## 🚀 快速开始

### Docker 部署（推荐）
```bash
# 使用 Docker 运行
docker run -d \
  --name otamoryx \
  -p 3000:3000 \
  -v /path/to/comics:/data/comics \
  -v /path/to/config:/data/config \
  otamoryx/otamoryx:latest
```

### 独立二进制文件
```bash
# 下载并运行
wget https://github.com/username/otamoryx/releases/latest/download/otamoryx
chmod +x otamoryx
./otamoryx --config config.toml
```

### 桌面应用程序
从我们的 [发布页面](https://github.com/username/otamoryx/releases) 下载适用于 Windows、macOS 或 Linux 的安装程序。

## 📖 文档

详细信息请参考我们的完整文档：

- **[📋 需求文档](docs/requirements.md)** - 详细的功能需求和规范说明
- **[🏗️ 架构文档](docs/architecture.md)** - 技术架构和系统设计
- **[🛣️ 开发路线图](docs/roadmap.md)** - 开发路线图和计划功能
- **[🚀 部署指南](docs/deployment.md)** - 安装和部署说明 *(即将推出)*
- **[👩‍💻 开发指南](docs/development.md)** - 开发者设置和贡献指南 *(即将推出)*
- **[📚 API 参考](docs/api.md)** - 完整的 API 文档 *(即将推出)*

## 🤝 贡献

我们欢迎社区贡献！请查看我们的 [贡献指南](CONTRIBUTING.md) 了解如何开始。

## 📄 许可证

本项目采用 GNU 通用公共许可证 v3.0 - 详情请参阅 [LICENSE](LICENSE) 文件。

## 🌟 支持

如果您觉得 OtamoryX 有用，请考虑在 GitHub 上给它一个星标！