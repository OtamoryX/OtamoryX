# OtamoryX

[English](README.md) | [中文 (Chinese)](README_zh.md)

**OtamoryX** is an open-source, self-deployable digital comic/manga reader and management platform designed to provide users with a modern, feature-rich alternative to existing solutions like LANraragi.

### ✨ Key Features

- **🏠 Self-hosted**: Run your own private comic library server
- **🌐 Multi-platform**: Web browser access today, desktop packaging planned
- **📚 Smart Management**: Advanced categorization, search, and metadata management
- **🔍 Powerful Search**: Full-text search with advanced filtering options
- **📖 Smooth Reading**: Responsive comic reading interface with progress tracking
- **🏷️ Intelligent Tagging**: Automatic duplicate detection and smart tag system
- **👥 Multi-user Support**: User management with path-based permissions
- **🔌 Plugin System**: Built-in plugins, plugin management, manifests, permissions, and execution history are available; external dynamic-plugin Runtime is still in development
- **🤖 AI Features**: AI connection profiles and title translation/language-detection queues are available; personalized content curation is planned

### 🛠️ Tech Stack

- **Backend**: Rust with Axum web framework, SQLite database
- **Frontend**: Vue.js 3 with TypeScript, Tailwind CSS  
- **Desktop (planned)**: Tauri framework for future native cross-platform applications
- **API**: RESTful JSON API with OPDS support

### 📋 Supported Formats

- **CBZ** (Comic Book ZIP)
- **CBR** (Comic Book RAR) 
- **CB7** (Comic Book 7z)
- **Standard archives** (ZIP, RAR) containing images
- **Image formats**: JPG, JPEG, PNG, WebP

### 🚀 Quick Start

#### Docker (Recommended)

The current `master` build is published to GitHub Container Registry as
`ghcr.io/otamoryx/otamoryx:main-unstable-latest`.

```bash
# Prepare persistent data directories
mkdir -p data comics cache

export JWT_SECRET="replace-with-a-long-random-secret"

# Run the published image
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

The web application is exposed at `http://localhost:3000`. The backend listens on `8080` inside the container.

For Compose, set `JWT_SECRET` and run `docker compose up -d`; the included Compose file uses the same published image. To use a stable release after one is published, set `OTAMORYX_IMAGE=ghcr.io/otamoryx/otamoryx:latest`.

To build locally instead, run `docker build -t otamoryx:local .` and start Compose with `OTAMORYX_IMAGE=otamoryx:local docker compose up -d`.

See the [Deployment Guide](docs/deployment.md) for volume and configuration details.

#### Local Development
```bash
cp backend/.env.example backend/.env
cd backend && cargo run

# In another terminal
cd frontend && pnpm install && pnpm dev
```

#### Desktop Application
The Tauri desktop project has not been added to the repository yet. Desktop packaging remains a roadmap item and no installer is currently available.

### 📖 Documentation

For detailed information, please refer to our comprehensive documentation:

- **[📋 Requirements](docs/requirements.md)** - Detailed functional requirements and specifications
- **[🏗️ Architecture](docs/architecture.md)** - Technical architecture and system design  
- **[🛣️ Roadmap](docs/roadmap.md)** - Development roadmap and planned features
- **[🚀 Deployment Guide](docs/deployment.md)** - Current installation and deployment notes
- **[👩‍💻 Development Guide](docs/development.md)** - Current developer setup and workflow
- API reference - not yet published as a standalone document

### 🤝 Contributing

We welcome contributions from the community! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details on how to get started.

### 📄 License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

### 🌟 Support

If you find OtamoryX useful, please consider giving it a star on GitHub!
