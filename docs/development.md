# 开发指南

## 环境要求

- Rust 1.70+
- Node.js 18+
- pnpm

## 快速开始

### 安装依赖

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 Node.js (推荐使用 nvm)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 20
nvm use 20

# 安装 pnpm
npm install -g pnpm

# Tauri 开发 (可选)
cargo install tauri-cli
```

### 运行开发环境

```bash
# 启动后端 (终端1)
cd backend
cargo run

# 启动前端 (终端2)  
cd frontend
pnpm dev

# Tauri 桌面版 (终端3, 可选)
cd frontend
cargo tauri dev
```

## 项目结构

```
OtamoryX/
├── backend/           # Rust 后端 API
│   ├── src/
│   │   ├── main.rs
│   │   ├── handlers/  # API 处理器
│   │   ├── models/    # 数据模型
│   │   └── services/  # 业务逻辑
│   └── Cargo.toml
├── frontend/          # Vue.js 前端
│   ├── src/
│   │   ├── views/     # 页面组件
│   │   ├── components/# 可复用组件
│   │   ├── stores/    # 状态管理
│   │   └── composables/# 组合式函数
│   ├── src-tauri/     # Tauri 桌面应用
│   └── package.json
└── docs/              # 文档
```

## 开发规范

### Git 提交规范

```
feat: 新功能
fix: Bug 修复
docs: 文档更新
refactor: 重构
test: 测试相关
chore: 工具链更新
```

### 代码规范

- Rust: 使用 `cargo fmt` 和 `cargo clippy`
- TypeScript: 使用 ESLint 和 Prettier
- Vue: 使用组合式 API 和 `<script setup>`

## 测试

```bash
# 后端测试
cd backend
cargo test

# 前端测试
cd frontend
pnpm test
```

## 构建

```bash
# 后端构建
cd backend  
cargo build --release

# 前端构建
cd frontend
pnpm build

# Tauri 构建
cd frontend
pnpm tauri build
```