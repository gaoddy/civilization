#!/usr/bin/env bash
# 本地联调：起服务端与 7 个机器人，打印房间码后等开发者在大厅加入。
#
# 端口被占用时直接失败，不复用已有服务：`/dev/tcp` 只能证明有进程在听，
# 证明不了对端是 room-service，误连会变成难懂的 WebSocket 错误。
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
PORT=${DEV_PORT:-8080}
URL="ws://127.0.0.1:${PORT}/ws"
LOG_DIR="$ROOT/output/dev-bots"
SERVER_LOG="$LOG_DIR/server.log"
SERVER_BIN="$ROOT/server/target/debug/room-service"
SIM_BIN="$ROOT/server/target/debug/sim"

for bin in "$SERVER_BIN" "$SIM_BIN"; do
  if [ ! -x "$bin" ]; then
    echo "缺少 $(basename "$bin")，先执行 make m1b-build"
    exit 1
  fi
done

if (exec 3<>"/dev/tcp/127.0.0.1/${PORT}") 2>/dev/null; then
  exec 3>&-
  echo "端口 ${PORT} 已被占用，用 make dev-bots DEV_PORT=$((PORT + 1)) 换端口"
  echo "换端口后客户端登录页的「服务器」字段也要改成 ws://127.0.0.1:$((PORT + 1))/ws"
  exit 1
fi

mkdir -p "$LOG_DIR"

"$SERVER_BIN" --listen "127.0.0.1:${PORT}" >"$SERVER_LOG" 2>&1 &
SERVER_PID=$!

cleanup() {
  kill "$SERVER_PID" 2>/dev/null || true
  wait "$SERVER_PID" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

ready=0
for _ in $(seq 1 100); do
  if (exec 3<>"/dev/tcp/127.0.0.1/${PORT}") 2>/dev/null; then
    exec 3>&-
    ready=1
    break
  fi
  sleep 0.1
done
if [ "$ready" -ne 1 ]; then
  echo "服务端未在 127.0.0.1:${PORT} 就绪"
  tail -n 20 "$SERVER_LOG"
  exit 1
fi
# 旧服务尚未完全退出时，新进程会绑定失败并立即退出，而端口探测仍然成功。
if ! kill -0 "$SERVER_PID" 2>/dev/null; then
  echo "服务端启动后立即退出（端口可能被其他进程占用）"
  tail -n 20 "$SERVER_LOG"
  exit 1
fi

"$SIM_BIN" dev --connect "$URL" "$@"
