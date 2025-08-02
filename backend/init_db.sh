#!/bin/bash

# 创建数据库并运行迁移
DB_FILE="otamoryx.db"

# 创建空数据库
touch "$DB_FILE"

# 运行迁移SQL文件
echo "Running migrations..."

# 创建migrations跟踪表
cat > temp_init.sql << 'EOF'
-- 创建migrations表来跟踪已执行的迁移
CREATE TABLE IF NOT EXISTS _migrations (
    version TEXT PRIMARY KEY,
    executed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
EOF

# 合并所有迁移文件
cat temp_init.sql migrations/20240101000001_init.sql migrations/20240101000002_add_page_count.sql migrations/20240101000003_add_categories_and_progress.sql > combined_migrations.sql

# 运行迁移
sqlite3 "$DB_FILE" < combined_migrations.sql

# 记录迁移执行
sqlite3 "$DB_FILE" << 'EOSQL'
INSERT OR IGNORE INTO _migrations (version) VALUES ('20240101000001_init');
INSERT OR IGNORE INTO _migrations (version) VALUES ('20240101000002_add_page_count');
INSERT OR IGNORE INTO _migrations (version) VALUES ('20240101000003_add_categories_and_progress');
EOSQL

# 清理临时文件
rm -f temp_init.sql combined_migrations.sql

echo "Database initialized successfully!"
echo "Listing tables:"
sqlite3 "$DB_FILE" ".tables"