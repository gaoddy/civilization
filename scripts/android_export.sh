#!/usr/bin/env bash
# Android 导出：APK（侧载）与 AAB（Google Play）。
#
# 签名参数只从环境变量读取，不写入仓库：ANDROID_KEYSTORE_PATH / ANDROID_KEYSTORE_USER /
# ANDROID_KEYSTORE_PASSWORD。Godot 只从工程内的 export_presets.cfg 读签名配置，因此
# 导出期间临时把参数写进预设，退出时（包括失败）用备份恢复，仓库文件不留口令。
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FORMAT="${1:-}"
case "$FORMAT" in
  apk) PRESET="Android"; EXTENSION="apk" ;;
  aab) PRESET="Android AAB"; EXTENSION="aab" ;;
  *)
    echo "用法：scripts/android_export.sh apk|aab" >&2
    exit 2
    ;;
esac

GODOT="${GODOT:-godot}"
TEMPLATES="${GODOT_TEMPLATE_DIR:-$HOME/Library/Application Support/Godot/export_templates/4.7.2.stable}"
OUTPUT="${ANDROID_OUTPUT:-$ROOT/output/android}"
case "$OUTPUT" in
  /*) ;;
  *) OUTPUT="$ROOT/$OUTPUT" ;;
esac
PRESETS="$ROOT/client/export_presets.cfg"

fail() {
  echo "$*" >&2
  exit 1
}

[ -f "$TEMPLATES/android_release.apk" ] || fail "缺少 Android 导出模板：$TEMPLATES/android_release.apk"
if [ "$FORMAT" = "aab" ]; then
  [ -f "$TEMPLATES/android_source.zip" ] || fail "缺少 Android 构建模板：$TEMPLATES/android_source.zip"
fi
[ -x "$GODOT" ] || command -v "$GODOT" >/dev/null 2>&1 || fail "找不到 Godot 可执行文件：$GODOT"
[ -f "$PRESETS" ] || fail "缺少客户端导出预设：$PRESETS"
[ -n "${ANDROID_KEYSTORE_PATH:-}" ] || fail "需要 ANDROID_KEYSTORE_PATH（release keystore 路径）"
[ -n "${ANDROID_KEYSTORE_USER:-}" ] || fail "需要 ANDROID_KEYSTORE_USER（keystore alias）"
[ -n "${ANDROID_KEYSTORE_PASSWORD:-}" ] || fail "需要 ANDROID_KEYSTORE_PASSWORD（keystore 口令）"
[ -f "$ANDROID_KEYSTORE_PATH" ] || fail "Android keystore 不存在：$ANDROID_KEYSTORE_PATH"
command -v java >/dev/null 2>&1 || fail "需要 JDK（java / keytool）才能签名 Android 产物"

if [ -z "${JAVA_HOME:-}" ]; then
  JAVA_HOME="$(/usr/libexec/java_home 2>/dev/null || true)"
  [ -n "$JAVA_HOME" ] && export JAVA_HOME
fi
if [ -z "${ANDROID_HOME:-}" ] && [ -z "${ANDROID_SDK_ROOT:-}" ]; then
  default_sdk="$HOME/Library/Android/sdk"
  if [ -d "$default_sdk" ]; then
    export ANDROID_HOME="$default_sdk"
    export ANDROID_SDK_ROOT="$default_sdk"
  fi
fi

backup="$(mktemp "${TMPDIR:-/tmp}/civilization-export-presets.XXXXXX")"
cp "$PRESETS" "$backup"
android_dir_existed=0
if [ -d "$ROOT/client/android" ]; then
  android_dir_existed=1
fi
cleanup() {
  cp "$backup" "$PRESETS"
  rm -f "$backup"
  # Gradle 构建模板由本次导出安装时，导出结束后移除，避免把生成目录留在工程里。
  if [ "$android_dir_existed" = "0" ] && [ -d "$ROOT/client/android" ]; then
    rm -rf "$ROOT/client/android"
  fi
}
trap cleanup EXIT

python3 - "$PRESETS" "$PRESET" "$ANDROID_KEYSTORE_PATH" "$ANDROID_KEYSTORE_USER" "$ANDROID_KEYSTORE_PASSWORD" <<'PY'
import re
import sys

path, preset, keystore, user, password = sys.argv[1:6]
text = open(path, encoding="utf-8").read()

block_re = re.compile(r"(?ms)^\[preset\.(\d+)\]\s*\n(.*?)(?=^\[preset\.|\Z)")
match = None
for candidate in block_re.finditer(text):
    if f'name="{preset}"' in candidate.group(2).splitlines():
        match = candidate
        break
if match is None:
    raise SystemExit(f"export_presets.cfg 中找不到预设：{preset}")

index = match.group(1)
options_header = f"[preset.{index}.options]"
options_re = re.compile(r"(?ms)^" + re.escape(options_header) + r"\s*\n(.*?)(?=^\[|\Z)")
options_match = options_re.search(text)
if options_match is None:
    raise SystemExit(f"预设 {preset} 缺少 options 段")

def quoted(value):
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'

options = options_match.group(1)
options = re.sub(
    r"(?m)^keystore/(?:debug|debug_user|debug_password|release|release_user|release_password)=.*\n?",
    "",
    options,
)
options = options.rstrip("\n") + "\n" + (
    f"keystore/release={quoted(keystore)}\n"
    f"keystore/release_user={quoted(user)}\n"
    f"keystore/release_password={quoted(password)}\n"
)
text = text[: options_match.start(1)] + options + text[options_match.end(1):]
open(path, "w", encoding="utf-8").write(text)
PY

mkdir -p "$OUTPUT"
export_args=(--headless --path "$ROOT/client")
if [ "$FORMAT" = "aab" ] && [ ! -d "$ROOT/client/android/build" ]; then
  export_args+=(--install-android-build-template)
fi
"$GODOT" "${export_args[@]}" --export-release "$PRESET" "$OUTPUT/Civilization.$EXTENSION"
test -f "$OUTPUT/Civilization.$EXTENSION"
echo "Android 产物：$OUTPUT/Civilization.$EXTENSION"
