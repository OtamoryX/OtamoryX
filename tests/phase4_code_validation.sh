#!/bin/bash

# Phase 4 Backend Integration Test
# 历史验证脚本：检查用户管理和权限系统代码状态

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_test() {
    echo -e "${BLUE}[TEST] $1${NC}"
}

print_success() {
    echo -e "${GREEN}[PASS] $1${NC}"
}

print_error() {
    echo -e "${RED}[FAIL] $1${NC}"
}

print_info() {
    echo -e "${BLUE}[INFO] $1${NC}"
}

cd /home/sober/OtamoryX

print_info "=== Phase 4 用户管理系统验证 ==="
print_info "检查历史用户管理功能的代码与迁移状态"

# 1. 检查关键文件是否存在
print_test "检查用户管理相关文件结构"

key_files=(
    "backend/src/handlers/users.rs"
    "backend/src/middleware/auth.rs"
    "backend/src/middleware/admin.rs"
    "backend/src/middleware/path_permission.rs"
    "backend/src/services/access_control_service.rs"
    "backend/src/services/admin_service.rs"
    "backend/src/services/auth_service.rs"
)

all_files_exist=true
for file in "${key_files[@]}"; do
    if [ -f "$file" ]; then
        echo "  ✅ $file"
    else
        echo "  ❌ $file (缺失)"
        all_files_exist=false
    fi
done

if [ "$all_files_exist" = true ]; then
    print_success "所有关键用户管理文件存在"
else
    print_error "部分关键文件缺失"
fi

# 2. 运行后端编译测试
print_test "后端代码编译测试"
cd backend

if cargo check --quiet 2>/dev/null; then
    print_success "后端代码编译成功"
else
    print_error "后端代码编译失败"
    print_info "尝试详细编译检查..."
    cargo check
fi

# 3. 运行单元测试
print_test "运行单元测试"
if cargo test --quiet 2>/dev/null; then
    print_success "单元测试通过"
else
    print_info "单元测试详细输出:"
    cargo test
fi

# 4. 检查数据库迁移
print_test "检查数据库迁移文件"
migration_files=(
    "migrations/sqlite/0001_init.sql"
    "migrations/postgres/0001_init.sql"
)

migration_valid=true
for file in "${migration_files[@]}"; do
    if [ -f "$file" ]; then
        echo "  ✅ $file"
        # 检查迁移文件中是否包含用户管理相关表
        if grep -Eq "CREATE TABLE( IF NOT EXISTS)? users" "$file" && grep -Eq "CREATE TABLE( IF NOT EXISTS)? user_paths" "$file"; then
            echo "  ✅ 包含用户管理表结构"
        else
            echo "  ❌ 用户管理表结构不完整"
            migration_valid=false
        fi
    else
        echo "  ❌ $file (缺失)"
        migration_valid=false
    fi
done

if [ "$migration_valid" = true ]; then
    print_success "数据库迁移文件验证通过"
else
    print_error "数据库迁移文件存在问题"
fi

# 5. 验证关键功能实现
print_test "验证关键功能实现"

# 检查认证中间件
if grep -q "auth_middleware" src/middleware/auth.rs; then
    echo "  ✅ JWT认证中间件已实现"
else
    echo "  ❌ JWT认证中间件未找到"
fi

# 检查管理员中间件
if grep -q "admin_middleware" src/middleware/admin.rs; then
    echo "  ✅ 管理员权限中间件已实现"
else
    echo "  ❌ 管理员权限中间件未找到"
fi

# 检查路径权限
if grep -q "has_path_permission" src/middleware/path_permission.rs; then
    echo "  ✅ 路径权限验证已实现"
else
    echo "  ❌ 路径权限验证未找到"
fi

# 检查用户管理handlers
if grep -q "create_user\|delete_user\|update_user" src/handlers/users.rs; then
    echo "  ✅ 用户CRUD操作已实现"
else
    echo "  ❌ 用户CRUD操作不完整"
fi

# 检查管理员保护
if grep -q "admin_count.*<=.*admins_to_delete_count" src/handlers/users.rs; then
    echo "  ✅ 管理员删除保护已实现"
else
    echo "  ❌ 管理员删除保护未找到"
fi

print_success "关键功能实现检查完成"

# 6. 检查API路由配置
print_test "检查API路由配置"
if [ -f "src/main.rs" ] || [ -f "src/lib.rs" ]; then
    if rg -n "users.*route|route\\(.*/api/v1/users|UserHandler::" src >/dev/null; then
        echo "  ✅ 用户管理路由已配置"
    else
        echo "  ❌ 用户管理路由配置未找到"
    fi

    if rg -n "auth_middleware|admin_middleware" src >/dev/null; then
        echo "  ✅ 认证中间件已应用"
    else
        echo "  ❌ 认证中间件应用未找到"
    fi
else
    print_error "主程序文件未找到"
fi

# 7. 生成验证报告
print_info "=== Phase 4 验证报告 ==="

cd /home/sober/OtamoryX

print_info "根据 roadmap.md Phase 4 完成状态："
echo "✅ 用户管理系统：CRUD操作已实现"
echo "✅ 认证系统：JWT中间件已实现"
echo "✅ 权限系统：管理员和路径权限已实现"
echo "✅ 安全保护：管理员删除保护已实现"
echo "⚠️  实际测试：需要启动服务器进行完整验证"

print_info "下一步建议："
echo "1. 手动启动服务器: cd backend && cargo run"
echo "2. 运行完整测试: ./tests/phase4_validation_test.sh"
echo "3. 验证多用户场景和数据隔离"
echo "4. 结合最新 roadmap 决定该历史模块是否继续增强"

print_success "Phase 4 代码结构验证完成 ✅"
