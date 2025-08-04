# Contributing to OtamoryX

[English](#english) | [中文 (Chinese)](#中文)

---

## English

Thank you for your interest in contributing to OtamoryX! We welcome contributions from the community and are excited to see what you can bring to this project.

### 🤝 How to Contribute

There are many ways to contribute to OtamoryX:

- 🐛 **Report bugs** - Help us identify and fix issues
- 💡 **Suggest features** - Share ideas for new functionality
- 📝 **Improve documentation** - Help make our docs clearer and more comprehensive
- 💻 **Submit code** - Fix bugs, implement features, or optimize performance
- 🌐 **Translate** - Help make OtamoryX accessible in more languages
- 🧪 **Test** - Help test new features and releases

### 🛠️ Development Setup

#### Prerequisites

Make sure you have the following installed:

- **Rust** 1.70+ ([Install Rust](https://rustup.rs/))
- **Node.js** 18.0+ ([Install Node.js](https://nodejs.org/))
- **pnpm** ([Install pnpm](https://pnpm.io/installation))
- **Git** ([Install Git](https://git-scm.com/))

#### Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/your-username/otamoryx.git
   cd otamoryx
   ```

3. **Set up the backend**:
   ```bash
   cd backend
   cargo build
   ```

4. **Set up the frontend**:
   ```bash
   cd frontend
   pnpm install
   ```

5. **Run the development servers**:
   ```bash
   # Terminal 1: Backend
   cd backend
   cargo run

   # Terminal 2: Frontend
   cd frontend
   pnpm dev

   # Terminal 3: Desktop app (optional)
   cd frontend
   cargo tauri dev
   ```

### 📋 Development Guidelines

#### Code Style

**Rust Code:**
- Follow the official [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- Use `cargo fmt` to format code
- Use `cargo clippy` to catch common mistakes
- Write documentation comments for public APIs

**TypeScript/Vue Code:**
- Use ESLint and Prettier for consistent formatting
- Follow Vue 3 Composition API best practices
- Use TypeScript for type safety
- Write clear, descriptive variable and function names

#### Commit Messages

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Build process or auxiliary tool changes

**Examples:**
```
feat(api): add archive search endpoint
fix(reader): resolve page navigation issue
docs: update API documentation
```

### 🔄 Pull Request Process

1. **Create a branch** for your feature/fix:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following our coding guidelines

3. **Test your changes**:
   ```bash
   # Run backend tests
   cd backend && cargo test

   # Run frontend tests
   cd frontend && pnpm test

   # Run linting
   cargo clippy
   pnpm lint
   ```

4. **Commit your changes** with conventional commit messages

5. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

6. **Open a Pull Request** on GitHub with:
   - Clear title and description
   - Reference to any related issues
   - Screenshots for UI changes
   - Test results if applicable

### 🐛 Reporting Bugs

When reporting bugs, please include:

- **Clear description** of the issue
- **Steps to reproduce** the problem
- **Expected behavior** vs actual behavior
- **Environment information** (OS, browser, versions)
- **Screenshots** or error logs if applicable

Use our [bug report template](.github/ISSUE_TEMPLATE/bug_report.md) when creating issues.

### 💡 Feature Requests

For feature requests, please:

- **Check existing issues** to avoid duplicates
- **Describe the feature** clearly and concisely
- **Explain the use case** and benefits
- **Consider implementation** if you have ideas

Use our [feature request template](.github/ISSUE_TEMPLATE/feature_request.md).

### 📚 Documentation

Help improve our documentation by:

- Fixing typos or unclear explanations
- Adding examples or use cases
- Translating documentation
- Creating tutorials or guides

### 🧪 Testing

We appreciate help with testing:

- **Manual testing** of new features
- **Writing automated tests** for code coverage
- **Performance testing** with large libraries
- **Cross-platform testing** on different OS

### 🌐 Translation

Help make OtamoryX accessible worldwide:

- Translate UI strings in the frontend
- Translate documentation
- Review existing translations
- Add support for new languages

### 📞 Getting Help

If you need help contributing:

- 💬 **Discussions** - Use [GitHub Discussions](https://github.com/username/otamoryx/discussions)
- 🐛 **Issues** - Create an issue for bugs or feature requests
- 📧 **Email** - Contact maintainers directly

### 📜 Code of Conduct

Please note that this project is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you agree to abide by its terms.

### 📄 License

By contributing to OtamoryX, you agree that your contributions will be licensed under the GNU General Public License v3.0.

---

## 中文

感谢您对 OtamoryX 项目的贡献兴趣！我们欢迎社区贡献，并期待看到您为这个项目带来的价值。

### 🤝 如何贡献

有多种方式可以为 OtamoryX 做出贡献：

- 🐛 **报告错误** - 帮助我们识别和修复问题
- 💡 **建议功能** - 分享新功能的想法
- 📝 **改进文档** - 帮助使我们的文档更清晰、更全面
- 💻 **提交代码** - 修复错误、实现功能或优化性能
- 🌐 **翻译** - 帮助 OtamoryX 支持更多语言
- 🧪 **测试** - 帮助测试新功能和版本

### 🛠️ 开发环境搭建

#### 系统要求

确保已安装以下软件：

- **Rust** 1.70+ ([安装 Rust](https://rustup.rs/))
- **Node.js** 18.0+ ([安装 Node.js](https://nodejs.org/))
- **pnpm** ([安装 pnpm](https://pnpm.io/installation))
- **Git** ([安装 Git](https://git-scm.com/))

#### 开始开发

1. **Fork 仓库** 在 GitHub 上
2. **克隆到本地**:
   ```bash
   git clone https://github.com/your-username/otamoryx.git
   cd otamoryx
   ```

3. **设置后端**:
   ```bash
   cd backend
   cargo build
   ```

4. **设置前端**:
   ```bash
   cd frontend
   pnpm install
   ```

5. **运行开发服务器**:
   ```bash
   # 终端 1: 后端
   cd backend
   cargo run

   # 终端 2: 前端
   cd frontend
   pnpm dev

   # 终端 3: 桌面应用 (可选)
   cd frontend
   cargo tauri dev
   ```

### 📋 开发指南

#### 代码风格

**Rust 代码:**
- 遵循官方 [Rust 风格指南](https://doc.rust-lang.org/nightly/style-guide/)
- 使用 `cargo fmt` 格式化代码
- 使用 `cargo clippy` 检查常见错误
- 为公共 API 编写文档注释

**TypeScript/Vue 代码:**
- 使用 ESLint 和 Prettier 保持格式一致
- 遵循 Vue 3 组合式 API 最佳实践
- 使用 TypeScript 确保类型安全
- 使用清晰、描述性的变量和函数名

#### 提交信息

我们遵循 [约定式提交](https://www.conventionalcommits.org/) 规范：

```
<类型>[可选范围]: <描述>

[可选正文]

[可选脚注]
```

**类型:**
- `feat`: 新功能
- `fix`: 错误修复
- `docs`: 文档更改
- `style`: 代码样式更改（格式化等）
- `refactor`: 代码重构
- `test`: 添加或更新测试
- `chore`: 构建过程或辅助工具更改

**示例:**
```
feat(api): 添加漫画搜索端点
fix(reader): 解决页面导航问题
docs: 更新 API 文档
```

### 🔄 Pull Request 流程

1. **创建分支** 用于您的功能/修复：
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **进行更改** 遵循我们的编码指南

3. **测试您的更改**:
   ```bash
   # 运行后端测试
   cd backend && cargo test

   # 运行前端测试
   cd frontend && pnpm test

   # 运行代码检查
   cargo clippy
   pnpm lint
   ```

4. **提交更改** 使用约定式提交信息

5. **推送到您的 fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

6. **在 GitHub 上开启 Pull Request**，包含：
   - 清晰的标题和描述
   - 引用相关问题
   - UI 更改的截图
   - 适用的测试结果

### 🐛 报告错误

报告错误时，请包含：

- **清晰的问题描述**
- **重现问题的步骤**
- **预期行为** vs 实际行为
- **环境信息**（操作系统、浏览器、版本）
- **截图** 或错误日志（如适用）

创建问题时请使用我们的 [错误报告模板](.github/ISSUE_TEMPLATE/bug_report.md)。

### 💡 功能请求

对于功能请求，请：

- **检查现有问题** 避免重复
- **清晰简洁地描述功能**
- **解释用例** 和好处
- **考虑实现方案**（如果您有想法）

请使用我们的 [功能请求模板](.github/ISSUE_TEMPLATE/feature_request.md)。

### 📚 文档

通过以下方式帮助改进我们的文档：

- 修复拼写错误或不清楚的解释
- 添加示例或用例
- 翻译文档
- 创建教程或指南

### 🧪 测试

我们欢迎测试方面的帮助：

- **手动测试** 新功能
- **编写自动化测试** 提高代码覆盖率
- **性能测试** 大型图书馆
- **跨平台测试** 不同操作系统

### 🌐 翻译

帮助 OtamoryX 走向世界：

- 翻译前端 UI 字符串
- 翻译文档
- 审查现有翻译
- 添加新语言支持

### 📞 获取帮助

如果您在贡献过程中需要帮助：

- 💬 **讨论** - 使用 [GitHub 讨论](https://github.com/username/otamoryx/discussions)
- 🐛 **问题** - 为错误或功能请求创建问题
- 📧 **邮件** - 直接联系维护者

### 📜 行为准则

请注意，本项目受我们的 [行为准则](CODE_OF_CONDUCT.md) 约束。参与即表示您同意遵守其条款。

### 📄 许可证

通过为 OtamoryX 做出贡献，您同意您的贡献将在 GNU 通用公共许可证 v3.0 下许可。