#!/usr/bin/env bash
# iOS 真机开发运行：把 client-export-ios 的产物装到 iPhone 并启动。
# 默认选第一台已连接的 iPhone；IOS_DEVICE 用设备名或 UDID 指定其他设备，
# IOS_LAUNCH=0 只安装不启动（设备锁屏时启动会失败，安装不受影响）。
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
APP="$ROOT/output/ios/build/Build/Products/Release-iphoneos/Civilization.app"
BUNDLE_ID=com.civilization.banner

if [ ! -d "$APP" ]; then
  echo "缺少 ${APP}，先执行 make client-export-ios" >&2
  exit 1
fi

DEVICES_JSON=$(mktemp)
trap 'rm -f "$DEVICES_JSON"' EXIT
xcrun devicectl list devices --json-output "$DEVICES_JSON" >/dev/null

DEVICE=${IOS_DEVICE:-}
if [ -z "$DEVICE" ]; then
  # 新版 Xcode 把硬件与连接信息移到 properties，旧字段已标记废弃：优先读新字段并回退。
  SELECTED=$(/usr/bin/python3 - "$DEVICES_JSON" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    devices = json.load(handle)["result"]["devices"]

for device in devices:
    properties = device.get("properties") or {}
    hardware = properties.get("hardware") or device.get("hardwareProperties") or {}
    connection = properties.get("connection") or device.get("connectionProperties") or {}
    if hardware.get("deviceType") != "iPhone":
        continue
    if (connection.get("state") or connection.get("tunnelState")) != "connected":
        continue
    udid = hardware.get("udid") or device.get("identifier")
    if not udid:
        continue
    name = (device.get("deviceProperties") or {}).get("name") or hardware.get("marketingName") or udid
    print("%s\t%s" % (udid, name))
    break
PY
)
  if [ -z "$SELECTED" ]; then
    echo "没有已连接的 iPhone：先接线并在 Xcode → Window → Devices and Simulators 中信任设备，" >&2
    echo "或用 IOS_DEVICE=<设备名|UDID> 指定。当前设备：" >&2
    xcrun devicectl list devices >&2
    exit 1
  fi
  # 输出为「UDID<TAB>设备名」；用 cut 取列，设备名可能含空格。
  DEVICE=$(printf '%s' "$SELECTED" | cut -f1)
  DEVICE_NAME=$(printf '%s' "$SELECTED" | cut -f2)
else
  DEVICE_NAME=$DEVICE
fi

echo "安装到 ${DEVICE_NAME}（${DEVICE}）"
xcrun devicectl device install app --device "$DEVICE" "$APP"

if [ "${IOS_LAUNCH:-1}" = "0" ]; then
  echo "已跳过启动（IOS_LAUNCH=0）"
else
  xcrun devicectl device process launch --device "$DEVICE" "$BUNDLE_ID"
fi
