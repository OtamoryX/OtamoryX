-- Add categories table for static and dynamic categories
CREATE TABLE IF NOT EXISTS categories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    category_type TEXT NOT NULL CHECK (category_type IN ('static', 'dynamic')),
    search_criteria TEXT, -- JSON for dynamic categories
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Add category_archives table for static category associations
CREATE TABLE IF NOT EXISTS category_archives (
    category_id TEXT NOT NULL,
    archive_id TEXT NOT NULL,
    PRIMARY KEY (category_id, archive_id),
    FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE CASCADE,
    FOREIGN KEY (archive_id) REFERENCES archives(id) ON DELETE CASCADE
);

-- Add reading_progress table for user progress tracking
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

-- Add user_paths table for path-based permissions
CREATE TABLE IF NOT EXISTS user_paths (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    path TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Create useful indexes
CREATE INDEX IF NOT EXISTS idx_categories_type ON categories(category_type);
CREATE INDEX IF NOT EXISTS idx_reading_progress_user_id ON reading_progress(user_id);
CREATE INDEX IF NOT EXISTS idx_reading_progress_archive_id ON reading_progress(archive_id);
CREATE INDEX IF NOT EXISTS idx_reading_progress_last_read ON reading_progress(last_read_at);
CREATE INDEX IF NOT EXISTS idx_user_paths_user_id ON user_paths(user_id);
CREATE INDEX IF NOT EXISTS idx_category_archives_category_id ON category_archives(category_id);
CREATE INDEX IF NOT EXISTS idx_category_archives_archive_id ON category_archives(archive_id);