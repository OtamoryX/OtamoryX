# 数据目录结构

OtamoryX 在运行时会使用以下数据目录结构：

```
/app/data/
├── otamoryx.db          # SQLite 数据库文件
├── comics/              # 漫画文件存储目录
│   └── [漫画文件...]
└── cache/               # 缓存目录
    ├── thumbnails/      # 缩略图缓存
    └── pages/           # 页面图片缓存
```

## Docker 挂载点

在使用 Docker 运行时，建议将 `/app/data` 目录挂载为外部卷：

```bash
docker run -v /path/to/your/data:/app/data otamoryx
```

这样只需要挂载一个数据目录，就包含了所有需要持久化的数据。

## 环境变量

可以通过环境变量覆盖默认设置：

- `DATABASE_URL`: 数据库连接字符串（默认：`sqlite:./data/otamoryx.db`）
- `CACHE_PATH`: 缓存目录路径（默认：`./data/cache`）
- 其他配置可通过应用程序设置界面修改

## 权限要求

确保 Docker 容器内的进程对 `/app/data` 目录有读写权限。

## 数据持久化

- 数据库文件：存储所有应用程序数据（用户、漫画元数据等）
- 漫画文件：存储在 `data/comics/` 目录中
- 缓存文件：存储在 `data/cache/` 目录中（缩略图、页面缓存等）
- 应用程序会自动创建必要的子目录