# OtamoryX

[English](README.md) | [中文 (Chinese)](README_zh.md)

**OtamoryX** is an open-source, self-deployable digital comic/manga reader and management platform designed to provide users with a modern, feature-rich alternative to existing solutions like LANraragi.

### ✨ Key Features

- **🏠 Self-hosted**: Run your own private comic library server
- **🌐 Multi-platform**: Web browser access and native desktop applications  
- **📚 Smart Management**: Advanced categorization, search, and metadata management
- **🔍 Powerful Search**: Full-text search with advanced filtering options
- **📖 Smooth Reading**: Responsive comic reading interface with progress tracking
- **🏷️ Intelligent Tagging**: Automatic duplicate detection and smart tag system
- **👥 Multi-user Support**: User management with path-based permissions
- **🔌 Plugin System**: Extensible architecture for community-driven features *(v1.1.0+)*
- **🤖 AI Auto-tagging**: Experimental AI-powered content analysis *(v1.2.0+)*

### 🛠️ Tech Stack

- **Backend**: Rust with Axum web framework, SQLite database
- **Frontend**: Vue.js 3 with TypeScript, Tailwind CSS  
- **Desktop**: Tauri framework for native cross-platform applications
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
# Run with Docker
docker run -d \
  --name otamoryx \
  -p 3000:3000 \
  -v /path/to/comics:/data/comics \
  -v /path/to/config:/data/config \
  otamoryx/otamoryx:latest
```

#### Standalone Binary
```bash
# Download and run
wget https://github.com/username/otamoryx/releases/latest/download/otamoryx
chmod +x otamoryx
./otamoryx --config config.toml
```

#### Desktop Application
Download the appropriate installer from our [releases page](https://github.com/username/otamoryx/releases) for Windows, macOS, or Linux.

### 📖 Documentation

For detailed information, please refer to our comprehensive documentation:

- **[📋 Requirements](docs/requirements.md)** - Detailed functional requirements and specifications
- **[🏗️ Architecture](docs/architecture.md)** - Technical architecture and system design  
- **[🛣️ Roadmap](docs/roadmap.md)** - Development roadmap and planned features
- **[🚀 Deployment Guide](docs/deployment.md)** - Installation and deployment instructions *(Coming Soon)*
- **[👩‍💻 Development Guide](docs/development.md)** - Developer setup and contribution guidelines *(Coming Soon)*
- **[📚 API Reference](docs/api.md)** - Complete API documentation *(Coming Soon)*

### 🤝 Contributing

We welcome contributions from the community! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details on how to get started.

### 📄 License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

### 🌟 Support

If you find OtamoryX useful, please consider giving it a star on GitHub!