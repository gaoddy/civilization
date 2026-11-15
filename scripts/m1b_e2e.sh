#!/usr/bin/env bash
# v0.1 M1b 无头端到端：1 个 Godot 客户端建房 + 7 个 sim 机器人加入，打完一局。
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
PORT=${M1B_PORT:-18080}
URL="ws://127.0.0.1:${PORT}/ws"
GODOT=${GODOT:-$(command -v godot 2>/dev/null || echo /Applications/Godot.app/Contents/MacOS/Godot)}
LOG_DIR="$ROOT/output/m1b-e2e"
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

M1B_URL="$URL" \
M1B_CODE_FILE="$CODE_FILE" \
M1B_HERO="${M1B_HERO:-darwin}" \
M1B_NAME="${M1B_NAME:-GodotClient}" \
M1B_TIMEOUT_SECONDS="${M1B_TIMEOUT_SECONDS:-300}" \
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
if ! grep -q "M1B_GAME_OVER" "$CLIENT_LOG"; then
  echo "日志缺少 M1B_GAME_OVER"
  exit 1
fi
if grep -q "SCRIPT ERROR" "$CLIENT_LOG"; then
  echo "客户端出现 SCRIPT ERROR"
  exit 1
fi

echo "M1B_E2E_OK"
