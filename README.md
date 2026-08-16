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
```bash
# Build the image from this repository
docker build -t otamoryx:latest .

# The Compose file requires a JWT secret
export JWT_SECRET="replace-with-a-long-random-secret"
docker compose up -d
```

The web application is exposed at `http://localhost:3000`. The backend listens on `8080` inside the container. See the [Deployment Guide](docs/deployment.md) for volume and configuration details.

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
