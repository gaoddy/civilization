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
SERVER_BIN=${DEV_SERVER_BIN:-"$ROOT/server/target/debug/room-service"}
SIM_BIN=${DEV_SIM_BIN:-"$ROOT/server/target/debug/sim"}

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

SIM_PID=""
SERVER_PID=""

cleanup() {
  # 也收 jobs -p：信号可能落在 `cmd &` 与 `PID=$!` 之间，那时 PID 还是空的。
  for pid in "$SIM_PID" "$SERVER_PID" $(jobs -p); do
    if [ -n "$pid" ]; then
      kill "$pid" 2>/dev/null || true
    fi
  done
  # 裸 wait 收该 shell 的全部子进程：上面的 PID 还没赋值时也不会漏掉。
  wait 2>/dev/null || true
}

# 终止信号必须收完子进程就退出：如果只 cleanup 不 exit，脚本会继续往下起
# `sim`，然后干等到对局结束——信号就白发了。trap 也要在拉起任何子进程之前
# 装好，否则信号落在启动与安装之间，服务端就成了孤儿。
trap cleanup EXIT
trap 'cleanup; exit 130' INT
trap 'cleanup; exit 143' TERM
trap 'cleanup; exit 129' HUP

# 监听全部网卡：手机与开发机同一局域网时才连得上；桌面端仍连 127.0.0.1。
DEV_LISTEN=${DEV_LISTEN:-0.0.0.0}
"$SERVER_BIN" --listen "${DEV_LISTEN}:${PORT}" >"$SERVER_LOG" 2>&1 &
SERVER_PID=$!

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

LAN_HOST=$(scutil --get LocalHostName 2>/dev/null || true)
if [ -n "$LAN_HOST" ]; then
  echo "手机端默认地址 ws://${LAN_HOST}.local:${PORT}/ws（桌面端 ${URL}）"
fi

# 一局结束后自动准备下一局：服务端保持运行（重启会让客户端令牌失效），
# 由机器人重新建房，房间码按服务端计数器递增。DEV_ROUNDS 为空时一直循环。
DEV_ROUNDS=${DEV_ROUNDS:-}
if [ -n "$DEV_ROUNDS" ]; then
  case "$DEV_ROUNDS" in
    *[!0-9]* | 0)
      echo "DEV_ROUNDS 必须是正整数，实际为 ${DEV_ROUNDS}"
      exit 1
      ;;
  esac
fi

# 每局的机器人意图种子按局递增，避免重开时机器人的决策序列一成不变；
# DEV_INTENT_SEED 是第 1 局的起始种子，缺省 0。脚本统一接管 --intent-seed，
# 转发给 sim dev 的参数里不要再带它，给了也会被这里的值覆盖。
DEV_INTENT_SEED=${DEV_INTENT_SEED:-0}
case "$DEV_INTENT_SEED" in
  '' | *[!0-9]*)
    echo "DEV_INTENT_SEED 必须是非负整数，实际为 ${DEV_INTENT_SEED}"
    exit 1
    ;;
esac

round=1
while :; do
  if [ -n "$DEV_ROUNDS" ] && [ "$round" -gt "$DEV_ROUNDS" ]; then
    break
  fi
  round_seed=$((DEV_INTENT_SEED + round - 1))
  echo "第 ${round} 局（意图种子 ${round_seed}）：起 7 个机器人建房；Ctrl-C 结束全部进程"
  "$SIM_BIN" dev --connect "$URL" "$@" --intent-seed "$round_seed" &
  SIM_PID=$!
  status=0
  wait "$SIM_PID" || status=$?
  SIM_PID=""
  if [ "$status" -ne 0 ]; then
    exit "$status"
  fi
  round=$((round + 1))
  echo "本局结束，下一局自动起新房间；在客户端结算页点「确定」回大厅，输入新房间码继续。"
done
