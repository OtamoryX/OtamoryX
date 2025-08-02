# OtamoryX Requirements Document

**Version**: 1.0  
**Date**: July 31, 2025  
**Document Type**: Functional Requirements Specification

## 1. Project Overview

### 1.1 Project Purpose
OtamoryX is an open-source, self-deployable digital comic/manga reader and management platform designed to provide users with a modern, feature-rich alternative to existing solutions like LANraragi. The system enables users to organize, read, and manage their digital comic collections through both web and native desktop interfaces, while also serving as an OPDS v1.2 compliant server for integration with third-party comic reader applications.

### 1.2 Core Objectives
- **Self-deployment**: Enable users to run their own private comic library servers
- **Multi-platform access**: Support web browsers, native desktop applications, and third-party OPDS clients
- **Modern architecture**: Built with Rust backend and Vue.js frontend for performance and maintainability
- **Comprehensive management**: Advanced categorization, search, and metadata management
- **Reading experience**: Smooth, responsive comic reading interface with progress tracking
- **OPDS Integration**: Provide standard OPDS v1.2 server endpoints for compatibility with external comic readers

### 1.3 Target Audience
- Individual users managing personal comic collections
- Small communities sharing comic libraries
- Power users requiring advanced organization features
- Users of third-party comic reader applications seeking OPDS server integration
- Developers contributing to open-source digital library solutions

## 2. System Architecture

### 2.1 Technology Stack
- **Backend**: Rust with Axum web framework, SQLite database
- **Frontend**: Vue.js 3 with TypeScript, Tailwind CSS
- **Desktop**: Tauri framework for native applications
- **API**: RESTful JSON API with OPDS server protocol support for third-party clients

### 2.2 Deployment Models
- **Standalone Server**: Web-based access via browser with OPDS server endpoints for third-party clients
- **Desktop Application**: Native cross-platform client
- **Docker Container**: Containerized deployment option
- **OPDS Integration**: Backend serves as OPDS v1.2 compliant server for external comic reader applications

## 3. Functional Requirements

### 3.1 System Initialization and Security

#### 3.1.1 First-Run Setup
- **FR-001**: System must provide secure initialization flow for first-time deployment
- **FR-002**: First user created must be assigned administrative privileges

#### 3.1.2 User Authentication
- **FR-005**: System must support user registration with username, password, and optional email
- **FR-006**: System must authenticate users via username/password login
- **FR-007**: System must generate and manage API keys for authenticated users
- **FR-008**: System must provide secure logout functionality
- **FR-009**: API access must require authentication via Bearer token

### 3.2 Archive Management

#### 3.2.1 Archive Data Model
- **FR-010**: System must store archives with unique ID, title, file path, size, page count, and hash
- **FR-011**: System must track creation and modification timestamps for all archives
- **FR-012**: System can associate multiple tags with each archive
- **FR-013**: System must calculate and store content hash for duplicate detection

#### 3.2.2 File Scanning and Duplicate Detection
- **FR-014**: System must perform content hash calculation during initial file scanning
- **FR-015**: System must perform duplicate detection using content hash as primary method (strong detection)
- **FR-016**: System must perform duplicate detection using title comparison as secondary method (weak detection)
- **FR-017**: System must automatically assign "new" special tag to archives when no duplicates are found during scanning
- **FR-018**: System must skip processing and tag assignment for archives when hash or title matches are found
- **FR-019**: System must allow configuration of title similarity threshold for weak duplicate detection
- **FR-020**: System must process archives with "new" tag for metadata extraction and thumbnail generation
- **FR-021**: System must provide "new" as a reserved system tag that cannot be manually created or deleted by users

#### 3.2.3 Archive Operations
- **FR-022**: System must provide paginated archive listing via `/api/v1/archives`
- **FR-023**: System must retrieve individual archive details via `/api/v1/archives/:id`
- **FR-024**: System must serve archive page images via `/api/v1/archives/:id/pages/:page`
- **FR-025**: System must generate and serve thumbnail images via `/api/v1/archives/:id/thumbnail`
- **FR-026**: System must support query parameters for pagination (page, limit)
- **FR-027**: System must provide random archive retrieval via `/api/v1/archives/random`
- **FR-028**: Random archive retrieval must support configurable count parameter (default: 20, max: 50)
- **FR-029**: Random archive results must be affected by search parameters and filters
- **FR-030**: System must support batch deletion of archives via `/api/v1/archives/batch-delete`

#### 3.2.4 Supported Formats
- **FR-031**: System must support CBZ (Comic Book ZIP) format
- **FR-032**: System must support CBR (Comic Book RAR) format  
- **FR-033**: System must support CB7 (Comic Book 7z) format
- **FR-034**: System must support standard ZIP and RAR archives containing images
- **FR-035**: System must support image formats is: jpg, jpeg, png, webp

### 3.3 Search and Discovery

#### 3.3.1 Basic Search
- **FR-028**: System must provide full-text search on archive titles via `/api/v1/search`
- **FR-029**: System must return paginated search results matching archive listing format
- **FR-030**: System must support case-insensitive title searching

#### 3.3.2 Advanced Search Parameters
- **FR-031**: System must filter archives by associated tags
- **FR-032**: System must filter archives by author
- **FR-033**: System must filter archives by path/folder
- **FR-034**: System must filter archives by page count range (minPages, maxPages)
- **FR-035**: System must filter archives by file size range (minFileSize, maxFileSize)
- **FR-036**: System must filter archives by creation date range (createdAfter, createdBefore)
- **FR-037**: System must filter archives by last read date (e.g., lastWeekRead, lastMonthRead, lastYearRead)

#### 3.3.3 Search Results and Sorting
- **FR-038**: System must support sorting by title, creation date, modification date, file size, page count
- **FR-039**: System must support ascending and descending sort order
- **FR-040**: System must combine multiple search parameters with AND logic
- **FR-041**: Search results must include total count and pagination metadata

#### 3.3.4 Search Management
- **FR-042**: System must allow administrators to configure default search result count
- **FR-043**: System must enforce maximum search result limits to prevent performance issues
- **FR-044**: Default search count must be configurable via system settings

### 3.4 Tag Management

#### 3.4.1 Tag Data Model
- **FR-045**: System must store tags with unique ID, name, and namespace
- **FR-046**: System must support tag namespaces (e.g., "author", "genre", "series")
- **FR-047**: System must maintain many-to-many relationships between archives and tags

#### 3.4.2 Tag Operations
- **FR-048**: System must provide complete tag listing via `/api/v1/tags`
- **FR-049**: System must allow tag-based filtering in search operations
- **FR-050**: System must automatically manage tag associations during archive operations, click tag can search archives with same tag
- **FR-051**: System must support batch deletion of all archives under specific tags via `/api/v1/tags/:id/archives/batch-delete`
- **FR-052**: System must provide prune functionality to remove tags with no associated archives via `/api/v1/tags/prune`

### 3.5 Category Management

#### 3.5.1 Static Categories
- **FR-053**: System must support user-created static categories for manual organization
- **FR-054**: System must allow adding/removing archives to/from static categories
- **FR-055**: System must track archive count per category
- **FR-056**: Static categories must support name and description metadata
- **FR-057**: System must support batch deletion of all archives within specific categories via `/api/v1/categories/:id/archives/batch-delete`

#### 3.5.2 Dynamic Categories
- **FR-058**: System must support dynamic categories based on search criteria, and in certain search results, users can save the current search results as a dynamic category.
- **FR-059**: Dynamic categories must automatically update based on stored search parameters
- **FR-060**: System must serialize and store search parameters as JSON for dynamic categories
- **FR-061**: Dynamic category contents must be computed on-demand

#### 3.5.3 Category Operations
- **FR-062**: System must list all categories via `/api/v1/categories`
- **FR-063**: System must create static categories via `/api/v1/categories`
- **FR-064**: System must create dynamic categories via `/api/v1/categories/dynamic`
- **FR-065**: System must update category metadata via `/api/v1/categories/:id`
- **FR-066**: System must delete categories via `/api/v1/categories/:id`
- **FR-067**: System must list archives within categories via `/api/v1/categories/:id/archives`
- **FR-068**: System must add archives to static categories via `/api/v1/categories/:id/archives`
- **FR-069**: System must remove archives from static categories via `/api/v1/categories/:id/archives`
- **FR-070**: System must provide prune functionality to remove categories with no associated archives via `/api/v1/categories/prune`

### 3.6 User Management

#### 3.6.1 User Administration
- **FR-071**: System must allow administrators to create new user accounts via `/api/v1/users`
- **FR-072**: System must allow administrators to delete user accounts via `/api/v1/users/:id`
- **FR-073**: System must support user role assignment (administrator, regular user)
- **FR-074**: System must maintain user creation and modification timestamps

#### 3.6.2 Path-Based Permissions
- **FR-075**: System must allow administrators to assign specific comic library paths to users
- **FR-076**: Regular users must only access archives within their assigned paths
- **FR-077**: Administrators must maintain access to all content regardless of path restrictions
- **FR-078**: System must validate user path permissions on all archive operations
- **FR-079**: User path assignments must be configurable via `/api/v1/users/:id/paths`

#### 3.6.3 User Management Operations
- **FR-080**: System must list all users via `/api/v1/users` (admin only)
- **FR-081**: System must retrieve user details via `/api/v1/users/:id`
- **FR-082**: System must update user permissions via `/api/v1/users/:id`
- **FR-083**: System must support bulk user operations for path assignment

### 3.7 Reading Progress Tracking

#### 3.7.1 Progress Data Model
- **FR-084**: System must track current page position for each user-archive combination
- **FR-085**: System must calculate and store progress percentage
- **FR-086**: System must record last read timestamp
- **FR-087**: System must store total page count for progress calculation

#### 3.7.2 Progress Operations
- **FR-088**: System must retrieve reading progress via `/api/v1/archives/:id/progress`
- **FR-089**: System must update reading progress via `/api/v1/archives/:id/progress`
- **FR-090**: Progress updates must only require current page number
- **FR-091**: System must automatically calculate progress percentage from page position
- **FR-092**: System must automatically remove "new" special tag when user reads beyond page 1 of an archive

### 3.8 System Configuration

#### 3.8.1 Settings Data Model
- **FR-092**: System must store one or more comics path configuration
- **FR-093**: System must configure image cache settings
  - **FR-093-1**: System must allow configuration of image cache storage path
  - **FR-093-2**: System must allow configuration of maximum cache size with automatic cleanup when limit is reached
  - **FR-093-3**: System must allow configuration of image quality compression level (1-100)
  - **FR-093-4**: System must allow configuration of output image format (JPEG, PNG, WebP)
  - **FR-093-5**: System must validate cache path exists and is writable
  - **FR-093-6**: System must validate maximum cache size is within reasonable limits (e.g., 100MB to 1TB)
  - **FR-093-7**: System must validate image quality is within acceptable range (1-100)
  - **FR-093-8**: System must provide default values for all cache settings
- **FR-094**: System must control automatic scanning behavior
  - **FR-094-1**: System must allow enabling/disabling automatic directory scanning
  - **FR-094-2**: System must allow configuration of whether to scan subdirectories recursively
  - **FR-094-3**: System must allow configuration of whether to ignore hidden files and directories
  - **FR-094-4**: System must support both real-time file system monitoring and scheduled interval scanning
  - **FR-094-5**: System must allow configuration of scan interval for scheduled scanning (cron schedule)
  - **FR-094-6**: System must support real-time monitoring of file system changes (file/directory creation, modification, deletion)
  - **FR-094-7**: System must allow enabling/disabling real-time monitoring and scheduled scanning independently
  - **FR-094-8**: System must allow manual triggering of scan operation
  - **FR-094-9**: System must support multiple comic library paths with individual scanning settings
  - **FR-094-10**: System must provide default scanning behavior when not configured

### 3.9 Plugin System

#### 3.9.1 Plugin Architecture
- **FR-095**: System must support external plugin loading and execution
- **FR-096**: System must provide a standardized plugin API interface
- **FR-097**: System must support plugin discovery and registration
- **FR-098**: System must allow plugins to extend core functionality through hooks

#### 3.9.2 Plugin Management
- **FR-099**: System must list all installed plugins via `/api/v1/plugins`
- **FR-100**: System must allow administrators to enable/disable plugins via `/api/v1/plugins/:id/toggle`
- **FR-101**: System must provide plugin configuration management via `/api/v1/plugins/:id/config`
- **FR-102**: System must support plugin installation from local files or repositories
- **FR-103**: System must validate plugin integrity and security before installation

#### 3.9.3 Plugin Capabilities
- **FR-104**: Plugins must be able to add custom metadata fields to archives
- **FR-105**: Plugins must be able to synchronize metadata from external sources
- **FR-106**: Plugins must be able to extend search functionality with custom filters
- **FR-107**: Plugins must be able to add custom API endpoints under `/api/v1/plugins/:plugin_name/`
- **FR-108**: Plugins must be able to register scheduled tasks for background operations
- **FR-109**: Plugins must be able to modify archive processing workflows

#### 3.9.4 Plugin Development
- **FR-110**: System must provide SDK/API documentation for plugin development
- **FR-111**: Plugins must declare their required permissions and capabilities
- **FR-112**: System must provide plugin development templates and examples
- **FR-113**: Plugins must support hot-reloading for development purposes

### 3.10 AI Auto-Tagging (Experimental)

#### 3.10.1 AI Integration
- **FR-114**: System must support AI model integration for automatic archive analysis
- **FR-115**: System must support multiple AI providers (local models, cloud APIs)
- **FR-116**: AI processing must be configurable and optional for users
- **FR-117**: System must validate AI model availability before processing

#### 3.10.2 Background Processing
- **FR-118**: System must automatically queue new archives for AI analysis upon detection
- **FR-119**: AI processing must run as background tasks without blocking user operations
- **FR-120**: System must support configurable processing schedules (immediate, delayed, off-peak hours)
- **FR-121**: AI analysis must respect system resource limits and processing priorities

#### 3.10.3 Content Analysis
- **FR-122**: System must extract representative images from archives for AI analysis
- **FR-123**: AI models must analyze archive content and generate relevant tags automatically
- **FR-124**: System must support batch processing of existing archives for AI tagging
- **FR-125**: AI analysis must handle processing failures gracefully with retry mechanisms

#### 3.10.4 Tag Generation and Management
- **FR-126**: AI-generated tags must be clearly distinguished from user-created tags in the database
- **FR-127**: System must provide confidence scores for all AI-generated tags
- **FR-128**: Users must be able to review and approve/reject AI-generated tags via UI
- **FR-129**: System must support automatic application of high-confidence AI tags based on user settings
- **FR-130**: Users must be able to provide feedback to improve AI tag accuracy over time

#### 3.10.5 AI Configuration and Monitoring
- **FR-131**: System must allow administrators to configure AI processing settings via `/api/v1/settings/ai`
- **FR-132**: Configuration must include AI model selection, processing schedule, and resource limits
- **FR-133**: System must provide AI processing queue status and progress monitoring
- **FR-134**: System must log AI processing activities and maintain processing statistics
- **FR-135**: System must allow pausing/resuming AI processing without data loss

### 3.11 Health and Monitoring

#### 3.11.1 System Health
- **FR-131**: System must provide health status via `/health` endpoint
- **FR-132**: Health response must include service status, version, and timestamp
- **FR-133**: Health endpoint must not require authentication

## 4. Non-Functional Requirements

### 4.1 Performance Requirements

#### 4.1.1 Response Time
- **NFR-001**: API endpoints must respond within 2 seconds under normal load
- **NFR-002**: Image serving must support browser caching with appropriate headers
- **NFR-003**: Thumbnail generation must be optimized for fast display

#### 4.1.2 Scalability
- **NFR-004**: System must support libraries with up to 10,000 archives（设计目标，待验证）
- **NFR-005**: Pagination must maintain performance with large datasets
- **NFR-006**: Search operations must complete within 5 seconds for large libraries

### 4.2 Security Requirements

#### 4.2.1 Authentication and Authorization
- **NFR-007**: All API endpoints except health and system status must require authentication
- **NFR-008**: API keys must be generated with cryptographically secure randomness
- **NFR-009**: Passwords must be hashed using industry-standard algorithms
- **NFR-010**: System must prevent unauthorized access to archive files

#### 4.2.2 Data Protection
- **NFR-011**: File paths must be validated to prevent directory traversal attacks
- **NFR-012**: Archive extraction must be performed in isolated environments
- **NFR-013**: System must validate file types before processing

### 4.3 Compatibility Requirements

#### 4.3.1 Browser Support
- **NFR-014**: Frontend must support modern browsers (Chrome 90+, Firefox 90+, Safari 14+)
- **NFR-015**: Interface must be responsive for desktop and tablet devices
- **NFR-016**: System must function with JavaScript enabled

#### 4.3.2 Platform Support
- **NFR-017**: Backend must compile and run on Linux, Windows, and macOS
- **NFR-018**: Desktop application must support cross-platform deployment
- **NFR-019**: Docker containers must support AMD64 and ARM64 architectures

### 4.4 Reliability Requirements

#### 4.4.1 Data Integrity
- **NFR-020**: Database operations must be atomic and consistent
- **NFR-021**: Archive files must not be modified by the system
- **NFR-022**: System must handle corrupt archive files gracefully

#### 4.4.2 Error Handling
- **NFR-023**: API must return appropriate HTTP status codes for all scenarios
- **NFR-024**: Error responses must include helpful error messages
- **NFR-025**: System must log errors for debugging purposes

## 5. User Roles and Permissions

### 5.1 User Types
- **Administrator**: Full system access, created during initialization
- **Regular User**: Archive access, personal progress tracking
- **Guest**: Read-only access (future consideration)

### 5.2 Permission Matrix

| Operation | Administrator | Regular User |
|-----------|---------------|--------------|
| System initialization | ✓ | ✗ |
| User management | ✓ | ✗ |
| Archive upload | ✓ | ✓* |
| Archive viewing | ✓ | ✓** |
| Category management | ✓ | ✓ |
| Progress tracking | ✓ | ✓ |
| System settings | ✓ | ✗ |
| Batch operations | ✓ | ✗ |
| Prune operations | ✓ | ✗ |
| Plugin management | ✓ | ✗ |
| AI auto-tagging | ✓ | ✓*** |

*Subject to administrator configuration  
**Restricted to assigned library paths  
***Subject to AI feature enablement and resource limits

## 6. API Specification Summary

### 6.1 Endpoint Categories

#### System Management (4 endpoints)
- `GET /health` - Health check
- `GET /api/v1/system/status` - Initialization status
- `POST /api/v1/system/initialize` - First-run setup
- `GET /api/v1/settings`, `PUT /api/v1/settings` - Configuration

#### Authentication (3 endpoints)
- `POST /api/v1/auth/register` - User registration
- `POST /api/v1/auth/login` - User login
- `POST /api/v1/auth/logout` - User logout

#### User Management (6 endpoints)
- `GET /api/v1/users` - List users (admin)
- `POST /api/v1/users` - Create user (admin)
- `GET /api/v1/users/:id` - User details
- `PUT /api/v1/users/:id` - Update user
- `DELETE /api/v1/users/:id` - Delete user (admin)
- `PUT /api/v1/users/:id/paths` - Manage user paths (admin)

#### Archive Management (6 endpoints)
- `GET /api/v1/archives` - List archives
- `GET /api/v1/archives/random` - Random archives
- `GET /api/v1/archives/:id` - Archive details
- `GET /api/v1/archives/:id/thumbnail` - Thumbnail image
- `GET /api/v1/archives/:id/pages/:page` - Page image
- `DELETE /api/v1/archives/batch-delete` - Batch delete archives

#### Search and Tags (4 endpoints)
- `GET /api/v1/search` - Advanced search
- `GET /api/v1/tags` - Tag listing
- `DELETE /api/v1/tags/:id/archives/batch-delete` - Batch delete tag archives
- `DELETE /api/v1/tags/prune` - Remove unused tags

#### Categories (9 endpoints)
- `GET /api/v1/categories` - List categories
- `POST /api/v1/categories` - Create static category
- `POST /api/v1/categories/dynamic` - Create dynamic category
- `PUT /api/v1/categories/:id` - Update category
- `DELETE /api/v1/categories/:id` - Delete category
- `GET /api/v1/categories/:id/archives` - Category contents
- `POST/DELETE /api/v1/categories/:id/archives` - Manage category membership
- `DELETE /api/v1/categories/:id/archives/batch-delete` - Batch delete category archives
- `DELETE /api/v1/categories/prune` - Remove empty categories

#### Reading Progress (2 endpoints)
- `GET /api/v1/archives/:id/progress` - Get progress
- `POST /api/v1/archives/:id/progress` - Update progress

#### Plugin Management (4 endpoints)
- `GET /api/v1/plugins` - List installed plugins
- `POST /api/v1/plugins/install` - Install plugin
- `PUT /api/v1/plugins/:id/toggle` - Enable/disable plugin
- `PUT /api/v1/plugins/:id/config` - Configure plugin

#### AI Auto-Tagging (4 endpoints)
- `GET /api/v1/settings/ai` - Get AI configuration
- `PUT /api/v1/settings/ai` - Update AI configuration
- `GET /api/v1/ai/status` - Get AI processing status
- `PUT /api/v1/ai/control` - Control AI processing (pause/resume)

**总计: 69 API 端点（已实现），5 OPDS 端点（规划中 - Phase 6）**

### 6.2 Data Exchange Format
- **Request/Response**: JSON
- **Authentication**: Bearer token in Authorization header
- **Pagination**: Standardized format with page, limit, total, hasNext
- **Error Responses**: HTTP status codes with JSON error details

## 7. Data Relationships
- **Archive ↔ Tag**: Many-to-many relationship
- **User ↔ Archive**: Many-to-many through ReadingProgress
- **Category ↔ Archive**: Many-to-many (static categories only)
- **User ↔ Category**: One-to-many (category ownership)

## 8. Deployment and Installation Requirements

### 8.1 System Requirements

#### Minimum Requirements
- **RAM**: 512MB available memory
- **Storage**: 1GB for application, additional space for comic libraries
- **CPU**: Modern 64-bit processor
- **Network**: HTTP/HTTPS access for web interface

#### Recommended Requirements
- **RAM**: 2GB for optimal performance
- **Storage**: SSD for database and cache, network storage for archives
- **CPU**: Multi-core processor for concurrent operations

### 8.2 Deployment Options

#### Standalone Binary
- Single executable with embedded web interface
- SQLite database co-located with application
- Suitable for personal use and small deployments

#### Docker Container
- Multi-stage build for optimized image size
- Volume mounts for persistent data and comic libraries
- Environment variable configuration
- Health check integration

#### Desktop Application
- Tauri-based native application
- Embedded web server for local access
- System tray integration
- Cross-platform installers

### 8.3 Configuration Management
- **Environment Variables**: Runtime configuration
- **Configuration Files**: Structured settings (TOML/JSON)
- **Database Schema**: Automatic migrations
- **Default Settings**: Sensible defaults for new installations

## 9. Success Criteria

### 9.1 Functional Success
- **Complete API Implementation**: 69 endpoints fully functional, 5 OPDS endpoints planned
- **Archive Format Support**: CBZ, CBR, CB7 formats working
- **Search Functionality**: Fast, accurate search with all specified parameters
- **Category System**: Both static and dynamic categories operational
- **Progress Tracking**: Reliable reading position synchronization

### 9.2 Performance Success
- **Response Time**: 95% of API calls under 2 seconds
- **Library Scale**: Support for 10,000+ archive libraries（设计目标，待性能测试验证）
- **Concurrent Users**: Support for 10+ simultaneous users
- **Memory Usage**: Stable memory footprint under extended use

### 9.3 Deployment Success
- **Easy Installation**: One-command deployment via Docker
- **Documentation**: Complete setup and user guides
- **Cross-Platform**: Working on Linux, Windows, macOS
- **Self-Contained**: No external dependencies for basic operation

## 10. Future Considerations

### 10.1 Planned Features
- **OPDS Protocol**: Standard digital library protocol support
- **Plugin Ecosystem**: Community-driven plugin marketplace
- **AI Enhancement**: Advanced content analysis and recommendation systems
- **Multi-User Libraries**: Shared libraries with user isolation
- **Advanced Reading Features**: Bookmarks, annotations, reading lists

### 10.2 Scalability Planning
- **Database Migration**: Path to PostgreSQL for larger deployments
- **Distributed Storage**: Support for object storage backends
- **Load Balancing**: Multi-instance deployment capability
- **Caching Layer**: Redis integration for improved performance

---

*This requirements document serves as the definitive specification for OtamoryX functionality based on the current codebase implementation and architecture design.*