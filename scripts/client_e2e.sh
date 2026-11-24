#!/usr/bin/env bash
# 客户端无头端到端：1 个 Godot 客户端建房 + 7 个 sim 机器人加入，打完一局。
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
PORT=${CIVILIZATION_E2E_PORT:-18080}
URL="ws://127.0.0.1:${PORT}/ws"
GODOT=${GODOT:-$(command -v godot 2>/dev/null || echo /Applications/Godot.app/Contents/MacOS/Godot)}
LOG_DIR="$ROOT/output/client-e2e"
CODE_FILE="$LOG_DIR/room-code.txt"
SERVER_LOG="$LOG_DIR/server.log"
CLIENT_LOG="$LOG_DIR/client.log"
SERVER_BIN="$ROOT/server/target/debug/room-service"
SIM_BIN="$ROOT/server/target/debug/sim"

mkdir -p "$LOG_DIR"
rm -f "$CODE_FILE" "$CLIENT_LOG"

"$SERVER_BIN" --listen "127.0.0.1:$PORT" >"$SERVER_LOG" 2>&1 &
SERVER_PID=$!
CLIENT_PID=""
BOT_PIDS=()

cleanup() {
  set +u
  for pid in "${BOT_PIDS[@]}"; do
    kill "$pid" 2>/dev/null || true
  done
  if [ -n "$CLIENT_PID" ]; then
    kill "$CLIENT_PID" 2>/dev/null || true
  fi
  kill "$SERVER_PID" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT

ready=0
for _ in $(seq 1 100); do
  if (exec 3<>"/dev/tcp/127.0.0.1/$PORT") 2>/dev/null; then
    exec 3>&-
    ready=1
    break
  fi
  sleep 0.1
done
if [ "$ready" -ne 1 ]; then
  echo "服务端未在 127.0.0.1:$PORT 就绪"
  tail -n 20 "$SERVER_LOG"
  exit 1
fi
if ! kill -0 "$SERVER_PID" 2>/dev/null; then
  echo "服务端启动后立即退出（端口可能被其他进程占用）"
  tail -n 20 "$SERVER_LOG"
  exit 1
fi

CIVILIZATION_E2E_URL="$URL" \
CIVILIZATION_E2E_CODE_FILE="$CODE_FILE" \
CIVILIZATION_E2E_HERO="${CIVILIZATION_E2E_HERO:-darwin}" \
CIVILIZATION_E2E_NAME="${CIVILIZATION_E2E_NAME:-GodotClient}" \
CIVILIZATION_E2E_TIMEOUT_SECONDS="${CIVILIZATION_E2E_TIMEOUT_SECONDS:-300}" \
  "$GODOT" --headless --path "$ROOT/client" res://tests/headless_game.tscn >"$CLIENT_LOG" 2>&1 &
CLIENT_PID=$!

for _ in $(seq 1 200); do
  if [ -s "$CODE_FILE" ]; then
    break
  fi
  if ! kill -0 "$CLIENT_PID" 2>/dev/null; then
    echo "客户端在取得房间码前退出"
    tail -n 40 "$CLIENT_LOG"
    exit 1
  fi
  sleep 0.1
done
if [ ! -s "$CODE_FILE" ]; then
  echo "超时：未取得房间码"
  tail -n 40 "$CLIENT_LOG"
  exit 1
fi

CODE=$(cat "$CODE_FILE")
echo "ROOM_CODE=$CODE"

HEROES=(tesla faraday galileo knuth marie-curie mendel pasteur)
for i in $(seq 1 7); do
  hero=${HEROES[$((i - 1))]}
  "$SIM_BIN" bot --connect "$URL" --room "$CODE" --name "Bot${i}" --hero "$hero" \
    >"$LOG_DIR/bot${i}.log" 2>&1 &
  BOT_PIDS+=("$!")
done

set +e
wait "$CLIENT_PID"
STATUS=$?
set -e
CLIENT_PID=""

echo "----- client log (tail) -----"
tail -n 20 "$CLIENT_LOG"

if [ "$STATUS" -ne 0 ]; then
  echo "客户端退出码 $STATUS"
  echo "----- server log (tail) -----"
  tail -n 40 "$SERVER_LOG"
  exit "$STATUS"
fi
if ! grep -q "CLIENT_E2E_GAME_OVER" "$CLIENT_LOG"; then
  echo "日志缺少 CLIENT_E2E_GAME_OVER"
  exit 1
fi
if grep -q "SCRIPT ERROR" "$CLIENT_LOG"; then
  echo "客户端出现 SCRIPT ERROR"
  exit 1
fi

RESULT=$(grep "CLIENT_E2E_GAME_OVER" "$CLIENT_LOG" | tail -n 1)
PLACE=$(echo "$RESULT" | sed -n 's/.*place=\([0-9]*\).*/\1/p')
SEED=$(echo "$RESULT" | sed -n 's/.*seed=\([^ ]*\).*/\1/p')
RANKING=$(echo "$RESULT" | sed -n 's/.*ranking=\([0-9]*\).*/\1/p')
INTENTS=$(echo "$RESULT" | sed -n 's/.*intents=\([0-9]*\).*/\1/p')
ERRORS=$(echo "$RESULT" | sed -n 's/.*errors=\([0-9]*\).*/\1/p')
SOFT=$(echo "$RESULT" | sed -n 's/.*soft=\([0-9]*\).*/\1/p')

if [ -z "$PLACE" ] || [ "$PLACE" -lt 1 ] || [ "$PLACE" -gt 8 ]; then
  echo "结算名次无效：${RESULT}"
  exit 1
fi
if [ "$RANKING" != "8" ]; then
  echo "结算排名应为 8 人，实际 ${RANKING}：${RESULT}"
  exit 1
fi
if [ -z "$SEED" ]; then
  echo "结算缺少种子：${RESULT}"
  exit 1
fi
if [ -z "$INTENTS" ] || [ "$INTENTS" -lt 20 ]; then
  echo "客户端发送意图过少（${INTENTS}）：${RESULT}"
  exit 1
fi
if [ "$ERRORS" != "$SOFT" ]; then
  echo "客户端收到未预期的协议错误（errors=${ERRORS} soft=${SOFT}）：${RESULT}"
  grep "CLIENT_E2E 收到错误" "$CLIENT_LOG" | tail -n 10
  exit 1
fi
if ! kill -0 "$SERVER_PID" 2>/dev/null; then
  echo "服务端在客户端结束后已退出"
  tail -n 40 "$SERVER_LOG"
  exit 1
fi
if grep -q "panicked" "$SERVER_LOG" "$LOG_DIR"/bot*.log 2>/dev/null; then
  echo "服务端或机器人出现 panic"
  grep -n "panicked" "$SERVER_LOG" "$LOG_DIR"/bot*.log 2>/dev/null | tail -n 5
  exit 1
fi

echo "CLIENT_E2E_OK place=$PLACE ranking=$RANKING intents=$INTENTS seed=$SEED soft_errors=$SOFT"
