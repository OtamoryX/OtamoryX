# 部署指南

## Docker 部署（推荐）

### 从仓库构建并运行

仓库目前未提供可验证的预构建镜像，使用根目录的 `Dockerfile` 构建：

```bash
docker build -t otamoryx:latest .
export JWT_SECRET="replace-with-a-long-random-secret"
docker compose up -d
```

根目录的 `docker-compose.yml` 会将 `./data`、`./comics` 和缓存目录挂载到容器，并通过 Nginx 在 `3000` 提供 Web/API，后端进程在容器内部监听 `8080`。

### 使用 Docker Compose

如果镜像已经构建，项目根目录可以直接使用：

```bash
# 启动服务
docker compose up -d

# 查看日志
docker compose logs -f

# 停止服务
docker compose down
```

首次构建或代码更新后：

```bash
docker build -t otamoryx:latest .
docker compose up -d
```

## 本地部署

### 后端

```bash
cd backend
cargo run --release
```

默认监听地址：

```bash
http://127.0.0.1:8080
```

### 前端

```bash
cd frontend
pnpm build
# 部署 dist/ 目录到 web 服务器
```

### Tauri 桌面版

当前仓库尚未包含 `frontend/src-tauri/` 工程，桌面端仍处于规划阶段，暂不提供可执行构建步骤。

## AI 连接配置

AI 设置支持多个 OpenAI-compatible 连接配置，但同一时间只会将新任务加入当前启用的配置。已加入队列的任务会保留原配置，以便切换模型后仍能稳定重试。

Ollama 可通过其 OpenAI-compatible API 配置：

```text
配置名称: Ollama 本地
Base URL: http://localhost:11434/v1
模型: qwen3:8b
认证方式: 无认证
```

在 Docker 中运行 OtamoryX 且 Ollama 运行在宿主机时，`localhost` 指向容器自身。请改用宿主机可访问的地址，例如 `http://host.docker.internal:11434/v1`（取决于 Docker 平台），或将两个服务放到同一 Docker 网络中。

## 环境变量

```bash
# 数据库位置
DATABASE_URL=sqlite:./data/otamoryx.db

# JWT 签名密钥（生产环境必须设置长度足够的随机值）
JWT_SECRET=replace-with-a-long-random-secret

# 缓存目录
CACHE_PATH=./data/cache

# 服务监听地址
BIND_ADDRESS=0.0.0.0:8080

# 允许的跨域来源
CORS_ORIGINS=http://localhost:5173,https://yourdomain.com
```

漫画库路径由系统设置持久化管理；Docker Compose 默认将宿主机的 `./comics` 挂载到容器的 `/app/data/comics`。

## 健康检查

- 进程健康：`GET /health`
- 系统健康：`GET /api/v1/system/health`
