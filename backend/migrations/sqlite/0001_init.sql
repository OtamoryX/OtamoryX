-- OtamoryX Database Initialization Script
-- This script creates all necessary tables and initial data

-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE,
    role TEXT NOT NULL DEFAULT 'user',
    password_hash TEXT NOT NULL,
    api_key TEXT UNIQUE NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create settings table
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create tags table
CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    namespace TEXT NOT NULL DEFAULT 'general',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(name, namespace)
);

-- Create archives table
CREATE TABLE IF NOT EXISTS archives (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    path TEXT NOT NULL,
    file_hash TEXT UNIQUE NOT NULL,
    file_size INTEGER NOT NULL,
    page_count INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create archive_tags table
CREATE TABLE IF NOT EXISTS archive_tags (
    archive_id TEXT NOT NULL,
    tag_id TEXT NOT NULL,
    PRIMARY KEY (archive_id, tag_id),
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

-- Create plugins table
CREATE TABLE IF NOT EXISTS plugins (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    manifest_version INTEGER NOT NULL DEFAULT 1,
    plugin_api_version INTEGER NOT NULL DEFAULT 1,
    plugin_type TEXT NOT NULL DEFAULT 'metadata' CHECK (plugin_type IN ('metadata', 'download', 'processor', 'analyzer', 'script', 'endpoint')),
    description TEXT,
    author TEXT,
    icon TEXT,
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    config TEXT,
    permissions TEXT,
    manifest TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_executed_at DATETIME,
    execution_count INTEGER NOT NULL DEFAULT 0
);

-- Create plugin_executions table for plugin execution history
CREATE TABLE IF NOT EXISTS plugin_executions (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    archive_id TEXT,
    execution_type TEXT NOT NULL CHECK (execution_type IN ('auto', 'manual', 'scheduled', 'api')),
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'success', 'failed', 'timeout')),
    input_summary TEXT,
    output_summary TEXT,
    error_message TEXT,
    duration_ms INTEGER,
    started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE,
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE SET NULL
);

-- Create plugin_tags table for plugin generated tag audit records
CREATE TABLE IF NOT EXISTS plugin_tags (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    archive_id TEXT NOT NULL,
    tag_id TEXT NOT NULL,
    confidence REAL,
    approved BOOLEAN,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE,
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

-- Create ai_generated_tags table
CREATE TABLE IF NOT EXISTS ai_generated_tags (
    id TEXT PRIMARY KEY,
    archive_id TEXT NOT NULL,
    tag_id TEXT,
    tag_name TEXT NOT NULL,
    namespace TEXT NOT NULL DEFAULT 'general',
    confidence REAL NOT NULL,
    approved BOOLEAN DEFAULT FALSE,
    reviewed_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE SET NULL
);

-- Create ai_processing_queue table
CREATE TABLE IF NOT EXISTS ai_processing_queue (
    id TEXT PRIMARY KEY,
    archive_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at DATETIME,
    completed_at DATETIME,
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE
);

-- Create categories table for static and dynamic categories
CREATE TABLE IF NOT EXISTS categories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    category_type TEXT NOT NULL CHECK (category_type IN ('static', 'dynamic')),
    search_criteria TEXT, -- JSON for dynamic categories
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create category_archives table for static category associations
CREATE TABLE IF NOT EXISTS category_archives (
    category_id TEXT NOT NULL,
    archive_id TEXT NOT NULL,
    PRIMARY KEY (category_id, archive_id),
    FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE,
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE
);

-- Create reading_progress table for user progress tracking
CREATE TABLE IF NOT EXISTS reading_progress (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    archive_id TEXT NOT NULL,
    current_page INTEGER NOT NULL DEFAULT 1,
    total_pages INTEGER NOT NULL DEFAULT 0,
    progress_percentage REAL NOT NULL DEFAULT 0.0,
    last_read_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, archive_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE
);

-- Create user_paths table for path-based permissions
CREATE TABLE IF NOT EXISTS user_paths (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    path TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Create system_settings table for configuration management
CREATE TABLE IF NOT EXISTS system_settings (
    id TEXT PRIMARY KEY DEFAULT 'default',
    comics_path TEXT NOT NULL DEFAULT './data/comics',
    supported_formats TEXT NOT NULL DEFAULT '["cbz","zip","cbr","rar","cb7","7z","pdf"]', -- JSON array
    max_file_size INTEGER NOT NULL DEFAULT 524288000, -- 500MB in bytes
    image_cache_size INTEGER NOT NULL DEFAULT 1073741824, -- 1GB in bytes
    image_cache_path TEXT NOT NULL DEFAULT './data/cache', -- Cache directory path
    image_cache_quality INTEGER NOT NULL DEFAULT 85,
    scan_on_startup BOOLEAN NOT NULL DEFAULT true,
    scan_settings TEXT NOT NULL DEFAULT '{"enabled":true,"recursive":true,"ignoreHidden":true,"realtimeMonitoring":false}', -- JSON object
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_archives_created_at ON archives(created_at);
CREATE INDEX IF NOT EXISTS idx_archive_tags_tag_id ON archive_tags(tag_id);
CREATE INDEX IF NOT EXISTS idx_tags_namespace ON tags(namespace);
CREATE INDEX IF NOT EXISTS idx_categories_type ON categories(category_type);
CREATE INDEX IF NOT EXISTS idx_reading_progress_user_id ON reading_progress(user_id);
CREATE INDEX IF NOT EXISTS idx_reading_progress_archive_id ON reading_progress(archive_id);
CREATE INDEX IF NOT EXISTS idx_reading_progress_last_read ON reading_progress(last_read_at);
CREATE INDEX IF NOT EXISTS idx_user_paths_user_id ON user_paths(user_id);
CREATE INDEX IF NOT EXISTS idx_category_archives_category_id ON category_archives(category_id);
CREATE INDEX IF NOT EXISTS idx_category_archives_archive_id ON category_archives(archive_id);
CREATE INDEX IF NOT EXISTS idx_plugin_executions_plugin ON plugin_executions(plugin_id);
CREATE INDEX IF NOT EXISTS idx_plugin_executions_archive ON plugin_executions(archive_id);
CREATE INDEX IF NOT EXISTS idx_plugin_executions_status ON plugin_executions(status);
CREATE INDEX IF NOT EXISTS idx_plugin_tags_plugin ON plugin_tags(plugin_id);
CREATE INDEX IF NOT EXISTS idx_plugin_tags_archive ON plugin_tags(archive_id);
CREATE INDEX IF NOT EXISTS idx_plugin_tags_approved ON plugin_tags(approved);

-- Insert initial data
-- Ensure the "new" special tag exists
INSERT OR IGNORE INTO tags (id, name, namespace) 
VALUES ('new-tag-id', 'new', 'system');

-- Insert default settings
INSERT OR IGNORE INTO system_settings (id) VALUES ('default');
