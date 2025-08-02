-- Add system_settings table for configuration management
CREATE TABLE IF NOT EXISTS system_settings (
    id TEXT PRIMARY KEY DEFAULT 'default',
    comics_path TEXT NOT NULL DEFAULT './comics',
    supported_formats TEXT NOT NULL DEFAULT '["cbz","zip","cbr","rar","cb7","7z","pdf"]', -- JSON array
    max_file_size INTEGER NOT NULL DEFAULT 524288000, -- 500MB in bytes
    image_cache_size INTEGER NOT NULL DEFAULT 1073741824, -- 1GB in bytes
    scan_on_startup BOOLEAN NOT NULL DEFAULT true,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Insert default settings
INSERT OR IGNORE INTO system_settings (id) VALUES ('default');