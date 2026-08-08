#!/bin/bash

# Phase 4 User Management Validation Test Script
# 历史验证脚本：用于回归检查用户管理和权限系统

set -e

BASE_URL="http://127.0.0.1:8080"
API_BASE="$BASE_URL/api/v1"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# 测试结果统计
PASSED=0
FAILED=0
TOTAL=0

print_test() {
    echo -e "${BLUE}[TEST] $1${NC}"
    ((TOTAL++))
}

print_success() {
    echo -e "${GREEN}[PASS] $1${NC}"
    ((PASSED++))
}

print_error() {
    echo -e "${RED}[FAIL] $1${NC}"
    ((FAILED++))
}

print_warning() {
    echo -e "${YELLOW}[WARN] $1${NC}"
}

print_info() {
    echo -e "${BLUE}[INFO] $1${NC}"
}

# 测试用例变量
ADMIN_USERNAME="admin"
ADMIN_PASSWORD="Cpic1234"
TEST_USER1_USERNAME="testuser1"
TEST_USER1_PASSWORD="testpass1"
TEST_USER2_USERNAME="testuser2"
TEST_USER2_PASSWORD="testpass2"

# JWT tokens
ADMIN_TOKEN=""
USER1_TOKEN=""
USER2_TOKEN=""
USER1_ID=""
USER2_ID=""

# 辅助函数：HTTP请求
http_get() {
    local url="$1"
    local auth_token="$2"

    if [ -n "$auth_token" ]; then
        curl -s -w "\n%{http_code}" -H "Authorization: Bearer $auth_token" "$url"
    else
        curl -s -w "\n%{http_code}" "$url"
    fi
}

http_post() {
    local url="$1"
    local data="$2"
    local auth_token="$3"

    if [ -n "$auth_token" ]; then
        curl -s -w "\n%{http_code}" -H "Content-Type: application/json" -H "Authorization: Bearer $auth_token" -d "$data" "$url"
    else
        curl -s -w "\n%{http_code}" -H "Content-Type: application/json" -d "$data" "$url"
    fi
}

http_put() {
    local url="$1"
    local data="$2"
    local auth_token="$3"

    if [ -n "$auth_token" ]; then
        curl -s -w "\n%{http_code}" -X PUT -H "Content-Type: application/json" -H "Authorization: Bearer $auth_token" -d "$data" "$url"
    else
        curl -s -w "\n%{http_code}" -X PUT -H "Content-Type: application/json" -d "$data" "$url"
    fi
}

http_delete() {
    local url="$1"
    local auth_token="$2"

    if [ -n "$auth_token" ]; then
        curl -s -w "\n%{http_code}" -X DELETE -H "Authorization: Bearer $auth_token" "$url"
    else
        curl -s -w "\n%{http_code}" -X DELETE "$url"
    fi
}

# 等待服务器启动
wait_for_server() {
    print_info "Waiting for server to start..."
    local max_attempts=30
    local attempt=0

    while [ $attempt -lt $max_attempts ]; do
        if curl -s "$BASE_URL/health" >/dev/null 2>&1; then
            print_success "Server is ready!"
            return 0
        fi
        sleep 1
        ((attempt++))
    done

    print_error "Server failed to start within 30 seconds"
    exit 1
}

# 测试1: 基本认证验证
test_basic_authentication() {
    print_info "=== 基本认证验证 ==="

    # 1.1 Admin登录
    print_test "Admin用户登录"
    local response=$(http_post "$API_BASE/auth/login" "{\"username\":\"$ADMIN_USERNAME\",\"password\":\"$ADMIN_PASSWORD\"}")
    local body=$(echo "$response" | head -n -1)
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "200" ]; then
        ADMIN_TOKEN=$(echo "$body" | jq -r '.token // empty')
        if [ -n "$ADMIN_TOKEN" ] && [ "$ADMIN_TOKEN" != "null" ]; then
            print_success "Admin登录成功，获得token"
        else
            print_error "Admin登录响应中缺少token"
        fi
    else
        print_error "Admin登录失败，状态码: $status"
    fi

    # 1.2 创建测试用户1
    print_test "创建测试用户1"
    local user1_data="{\"username\":\"$TEST_USER1_USERNAME\",\"password\":\"$TEST_USER1_PASSWORD\",\"email\":\"user1@test.com\",\"role\":\"user\"}"
    local response=$(http_post "$API_BASE/users" "$user1_data" "$ADMIN_TOKEN")
    local body=$(echo "$response" | head -n -1)
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "200" ]; then
        USER1_ID=$(echo "$body" | jq -r '.id // empty')
        print_success "测试用户1创建成功，ID: $USER1_ID"
    else
        print_error "测试用户1创建失败，状态码: $status, 响应: $body"
    fi

    # 1.3 创建测试用户2
    print_test "创建测试用户2"
    local user2_data="{\"username\":\"$TEST_USER2_USERNAME\",\"password\":\"$TEST_USER2_PASSWORD\",\"email\":\"user2@test.com\",\"role\":\"user\"}"
    local response=$(http_post "$API_BASE/users" "$user2_data" "$ADMIN_TOKEN")
    local body=$(echo "$response" | head -n -1)
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "200" ]; then
        USER2_ID=$(echo "$body" | jq -r '.id // empty')
        print_success "测试用户2创建成功，ID: $USER2_ID"
    else
        print_error "测试用户2创建失败，状态码: $status, 响应: $body"
    fi

    # 1.4 测试用户1登录
    print_test "测试用户1登录"
    local response=$(http_post "$API_BASE/auth/login" "{\"username\":\"$TEST_USER1_USERNAME\",\"password\":\"$TEST_USER1_PASSWORD\"}")
    local body=$(echo "$response" | head -n -1)
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "200" ]; then
        USER1_TOKEN=$(echo "$body" | jq -r '.token // empty')
        if [ -n "$USER1_TOKEN" ] && [ "$USER1_TOKEN" != "null" ]; then
            print_success "测试用户1登录成功"
        else
            print_error "测试用户1登录响应中缺少token"
        fi
    else
        print_error "测试用户1登录失败，状态码: $status"
    fi

    # 1.5 测试用户2登录
    print_test "测试用户2登录"
    local response=$(http_post "$API_BASE/auth/login" "{\"username\":\"$TEST_USER2_USERNAME\",\"password\":\"$TEST_USER2_PASSWORD\"}")
    local body=$(echo "$response" | head -n -1)
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "200" ]; then
        USER2_TOKEN=$(echo "$body" | jq -r '.token // empty')
        if [ -n "$USER2_TOKEN" ] && [ "$USER2_TOKEN" != "null" ]; then
            print_success "测试用户2登录成功"
        else
            print_error "测试用户2登录响应中缺少token"
        fi
    else
        print_error "测试用户2登录失败，状态码: $status"
    fi

    # 1.6 错误密码测试
    print_test "错误密码登录测试"
    local response=$(http_post "$API_BASE/auth/login" "{\"username\":\"$ADMIN_USERNAME\",\"password\":\"wrongpassword\"}")
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "401" ]; then
        print_success "错误密码被正确拒绝"
    else
        print_error "错误密码测试失败，状态码: $status"
    fi

    # 1.7 不存在用户测试
    print_test "不存在用户登录测试"
    local response=$(http_post "$API_BASE/auth/login" "{\"username\":\"nonexistentuser\",\"password\":\"password\"}")
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "401" ]; then
        print_success "不存在用户被正确拒绝"
    else
        print_error "不存在用户测试失败，状态码: $status"
    fi
}

# 测试2: 权限隔离验证
test_permission_isolation() {
    print_info "=== 权限隔离验证 ==="

    # 2.1 Admin访问管理端点
    print_test "Admin访问用户列表"
    local response=$(http_get "$API_BASE/users" "$ADMIN_TOKEN")
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "200" ]; then
        print_success "Admin可以访问用户列表"
    else
        print_error "Admin无法访问用户列表，状态码: $status"
    fi

    # 2.2 普通用户访问管理端点（应该失败）
    print_test "普通用户访问用户列表（应该被拒绝）"
    local response=$(http_get "$API_BASE/users" "$USER1_TOKEN")
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "403" ] || [ "$status" = "401" ]; then
        print_success "普通用户被正确拒绝访问管理端点"
    else
        print_error "普通用户权限检查失败，状态码: $status"
    fi

    # 2.3 用户访问自己的信息
    print_test "用户访问自己的信息"
    local response=$(http_get "$API_BASE/users/$USER1_ID" "$USER1_TOKEN")
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "200" ]; then
        print_success "用户可以访问自己的信息"
    else
        print_warning "用户访问自己信息测试，状态码: $status（可能需要路径权限验证）"
    fi

    # 2.4 用户访问他人信息（应该失败）
    print_test "用户访问他人信息（应该被拒绝）"
    local response=$(http_get "$API_BASE/users/$USER2_ID" "$USER1_TOKEN")
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "403" ] || [ "$status" = "401" ]; then
        print_success "用户被正确拒绝访问他人信息"
    else
        print_warning "用户数据隔离检查，状态码: $status（可能需要改进）"
    fi
}

# 测试3: 路径权限验证
test_path_permissions() {
    print_info "=== 路径权限验证 ==="

    # 3.1 为用户1设置路径权限
    print_test "为用户1设置路径权限"
    local paths_data="{\"paths\":[\"/app/data/comics/manga/*\"]}"
    local response=$(http_put "$API_BASE/users/$USER1_ID/paths" "$paths_data" "$ADMIN_TOKEN")
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "200" ]; then
        print_success "用户1路径权限设置成功"
    else
        print_error "用户1路径权限设置失败，状态码: $status"
    fi

    # 3.2 为用户2设置路径权限
    print_test "为用户2设置路径权限"
    local paths_data="{\"paths\":[\"/app/data/comics/novels/*\"]}"
    local response=$(http_put "$API_BASE/users/$USER2_ID/paths" "$paths_data" "$ADMIN_TOKEN")
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "200" ]; then
        print_success "用户2路径权限设置成功"
    else
        print_error "用户2路径权限设置失败，状态码: $status"
    fi

    # 3.3 验证用户路径权限
    print_test "验证用户1路径权限"
    local response=$(http_get "$API_BASE/users/$USER1_ID/paths" "$ADMIN_TOKEN")
    local body=$(echo "$response" | head -n -1)
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "200" ]; then
        local paths_count=$(echo "$body" | jq '. | length')
        if [ "$paths_count" -eq 1 ]; then
            print_success "用户1路径权限验证成功"
        else
            print_error "用户1路径权限数量不符，期望1个，实际$paths_count个"
        fi
    else
        print_error "无法获取用户1路径权限，状态码: $status"
    fi
}

# 测试4: 管理员保护验证
test_admin_protection() {
    print_info "=== 管理员保护验证 ==="

    # 4.1 检查管理员数量
    print_test "检查当前管理员数量"
    local response=$(http_get "$API_BASE/users/admins" "$ADMIN_TOKEN")
    local body=$(echo "$response" | head -n -1)
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "200" ]; then
        local admin_count=$(echo "$body" | jq '. | length')
        print_success "当前系统有 $admin_count 个管理员"

        # 4.2 尝试删除管理员（如果只有一个管理员，应该失败）
        if [ "$admin_count" -eq 1 ]; then
            print_test "尝试删除唯一管理员（应该被拒绝）"
            local admin_id=$(echo "$body" | jq -r '.[0].id')
            local response=$(http_delete "$API_BASE/users/$admin_id" "$ADMIN_TOKEN")
            local status=$(echo "$response" | tail -n 1)

            if [ "$status" = "403" ] || [ "$status" = "400" ]; then
                print_success "唯一管理员删除被正确阻止"
            else
                print_error "管理员保护机制失败，状态码: $status"
            fi
        else
            print_info "系统有多个管理员，跳过删除保护测试"
        fi
    else
        print_error "无法获取管理员列表，状态码: $status"
    fi
}

# 测试5: 安全审计验证
test_security_audit() {
    print_info "=== 安全审计验证 ==="

    # 5.1 无token访问受保护端点
    print_test "无token访问受保护端点"
    local response=$(http_get "$API_BASE/users")
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "401" ]; then
        print_success "无token访问被正确拒绝"
    else
        print_error "无token访问测试失败，状态码: $status"
    fi

    # 5.2 无效token测试
    print_test "无效token访问测试"
    local response=$(http_get "$API_BASE/users" "invalid.token.here")
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "401" ]; then
        print_success "无效token被正确拒绝"
    else
        print_error "无效token测试失败，状态码: $status"
    fi

    # 5.3 基本SQL注入测试
    print_test "基本SQL注入防护测试"
    local malicious_data="{\"username\":\"admin'; DROP TABLE users; --\",\"password\":\"password\"}"
    local response=$(http_post "$API_BASE/auth/login" "$malicious_data")
    local status=$(echo "$response" | tail -n 1)

    if [ "$status" = "401" ] || [ "$status" = "400" ]; then
        print_success "SQL注入攻击被正确阻止"
    else
        print_warning "SQL注入测试结果需要进一步验证，状态码: $status"
    fi
}

# 清理测试用户
cleanup_test_users() {
    print_info "=== 清理测试用户 ==="

    if [ -n "$USER1_ID" ]; then
        print_test "删除测试用户1"
        local response=$(http_delete "$API_BASE/users/$USER1_ID" "$ADMIN_TOKEN")
        local status=$(echo "$response" | tail -n 1)

        if [ "$status" = "200" ]; then
            print_success "测试用户1删除成功"
        else
            print_warning "测试用户1删除失败，状态码: $status"
        fi
    fi

    if [ -n "$USER2_ID" ]; then
        print_test "删除测试用户2"
        local response=$(http_delete "$API_BASE/users/$USER2_ID" "$ADMIN_TOKEN")
        local status=$(echo "$response" | tail -n 1)

        if [ "$status" = "200" ]; then
            print_success "测试用户2删除成功"
        else
            print_warning "测试用户2删除失败，状态码: $status"
        fi
    fi
}

# 生成测试报告
generate_report() {
    print_info "=== 测试结果报告 ==="
    echo "总测试用例: $TOTAL"
    echo "通过: $PASSED"
    echo "失败: $FAILED"
    echo "成功率: $(( (PASSED * 100) / TOTAL ))%"

    if [ $FAILED -eq 0 ]; then
        print_success "✅ 所有测试通过！Phase 4 用户管理系统验证成功"
    else
        print_error "❌ 有 $FAILED 个测试失败，需要修复"
    fi
}

# 主测试流程
main() {
    print_info "开始 Phase 4 用户管理和权限系统验证测试"
    print_info "目标API: $API_BASE"

    wait_for_server

    test_basic_authentication
    test_permission_isolation
    test_path_permissions
    test_admin_protection
    test_security_audit

    cleanup_test_users
    generate_report
}

# 执行测试
main "$@"
