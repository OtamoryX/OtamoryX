#!/bin/bash
# OtamoryX 开发环境一键启动脚本

set -e

# 颜色
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

ROOT="$(cd "$(dirname "$0")" && pwd)"
BACKEND_LOG="$ROOT/.dev-backend.log"
FRONTEND_LOG="$ROOT/.dev-frontend.log"
PID_FILE="$ROOT/.dev.pid"

# 清理函数
cleanup() {
  echo ""
  echo -e "${YELLOW}正在关闭开发服务...${NC}"
  if [ -f "$PID_FILE" ]; then
    while read -r pid; do
      kill "$pid" 2>/dev/null && echo -e "  ${RED}✗${NC} 已停止进程 $pid" || true
    done < "$PID_FILE"
    rm -f "$PID_FILE"
  fi
  echo -e "${GREEN}已退出${NC}"
  exit 0
}

trap cleanup SIGINT SIGTERM

# 检查是否已有进程在跑
if [ -f "$PID_FILE" ]; then
  echo -e "${YELLOW}检测到已有 dev 进程，先关闭...${NC}"
  while read -r pid; do
    kill "$pid" 2>/dev/null || true
  done < "$PID_FILE"
  rm -f "$PID_FILE"
  sleep 1
fi

echo -e "${CYAN}========================================${NC}"
echo -e "${CYAN}   OtamoryX 开发环境启动${NC}"
echo -e "${CYAN}========================================${NC}"

# 启动后端
echo -e "\n${YELLOW}[1/2]${NC} 启动后端 (Rust/Axum)..."
cd "$ROOT/backend"
cargo build 2>&1 | tail -3
cargo run > "$BACKEND_LOG" 2>&1 &
BACKEND_PID=$!
echo "$BACKEND_PID" > "$PID_FILE"

# 等待后端就绪
echo -n "  等待后端启动"
for i in $(seq 1 30); do
  if curl -s http://localhost:8080/health > /dev/null 2>&1; then
    echo -e " ${GREEN}✓${NC}"
    break
  fi
  echo -n "."
  sleep 1
  if [ $i -eq 30 ]; then
    echo -e " ${RED}超时${NC}"
    echo -e "${RED}后端启动失败，查看日志: $BACKEND_LOG${NC}"
    cat "$BACKEND_LOG" | tail -20
    cleanup
  fi
done

# 启动前端
echo -e "${YELLOW}[2/2]${NC} 启动前端 (Vite/Vue3)..."
cd "$ROOT/frontend"
pnpm dev --host 0.0.0.0 > "$FRONTEND_LOG" 2>&1 &
FRONTEND_PID=$!
echo -e "$BACKEND_PID\n$FRONTEND_PID" > "$PID_FILE"

sleep 2

echo -e "\n${GREEN}========================================${NC}"
echo -e "${GREEN}   开发环境已就绪！${NC}"
echo -e "${GREEN}========================================${NC}"
echo -e "  前端: ${CYAN}http://localhost:5173${NC}"
echo -e "  后端: ${CYAN}http://localhost:8080${NC}"
echo -e "  健康: ${CYAN}http://localhost:8080/health${NC}"
echo -e "\n  日志: tail -f $BACKEND_LOG"
echo -e "        tail -f $FRONTEND_LOG"
echo -e "\n  ${YELLOW}按 Ctrl+C 停止所有服务${NC}\n"

# 保持运行，等待中断
wait
