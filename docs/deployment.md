# 部署指南

## Docker 部署（推荐）

### 使用已发布镜像

`master` 分支的构建镜像发布在 GitHub Container Registry：
`ghcr.io/otamoryx/otamoryx:main-unstable-latest`。

```bash
mkdir -p data comics cache
export JWT_SECRET="replace-with-a-long-random-secret"
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

根目录的 `docker-compose.yml` 默认使用同一个镜像，并将 `./data`、`./comics` 和缓存目录挂载到容器。Web/API 通过 Nginx 在 `3000` 提供，后端进程在容器内部监听 `8080`。

### 使用 Docker Compose

项目根目录可以直接使用：

```bash
# 启动服务
docker compose up -d

# 查看日志
docker compose logs -f

# 停止服务
docker compose down
```

更新到最新的 master 镜像：

```bash
docker compose pull
docker compose up -d
```

### 从源码构建

```bash
docker build -t otamoryx:local .
OTAMORYX_IMAGE=otamoryx:local docker compose up -d
```

稳定版本发布后，可使用 `OTAMORYX_IMAGE=ghcr.io/otamoryx/otamoryx:latest` 切换镜像。

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

AI 设置支持多个 OpenAI-compatible 连接配置。当前选中的配置优先处理新任务；当网络超时、限流或服务端错误发生时，任务会按配置列表顺序切换到其他已启用的配置。已切换的任务会保留最后成功选中的配置作为后续重试的首选。

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
