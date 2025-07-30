# 部署指南

## Docker 部署

### 后端服务

```dockerfile
# Dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/otamoryx-server .
EXPOSE 3000
CMD ["./otamoryx-server"]
```

### Docker Compose

```yaml
version: '3.8'
services:
  otamoryx:
    build: .
    ports:
      - "3000:3000"
    volumes:
      - ./data:/app/data
      - ./comics:/app/comics:ro
    environment:
      - DATABASE_URL=sqlite:///app/data/otamoryx.db
      - COMICS_PATH=/app/comics
```

## 本地部署

### 后端

```bash
cd backend
cargo run --release
```

### 前端

```bash
cd frontend
pnpm build
# 部署 dist/ 目录到 web 服务器
```

### Tauri 桌面版

```bash
cd frontend
pnpm tauri build
# 安装包在 src-tauri/target/release/bundle/
```

## 环境变量

```bash
# 数据库位置
DATABASE_URL=sqlite://./data/otamoryx.db

# 漫画文件存储路径
COMICS_PATH=./comics

# API 访问密钥
API_KEY=your-secret-key

# 允许的跨域来源
CORS_ORIGINS=http://localhost:5173,https://yourdomain.com
```