#!/usr/bin/env python3
"""生成客户端构建配置 `client/generated/build_config.gd`。

三档用法：

- 编辑器运行、`local` 档与本地调试包不传参数，生成空配置，客户端按平台回落到本机调试地址；
- `make client-export ENV=dev|prod` / `make client-export PLATFORM=ios ENV=dev|prod` 要求 `SERVER_URL` 非空：
  `dev` 传 `--require-url`，允许 `ws://` 局域网地址；`prod` 传 `--require-server`，必须是 `wss://`；
- `make client-export PLATFORM=steam` 传 `--require`，在地址之外还要求 `STEAM_APP_ID`。

除服务端地址外，配置里还写入构建环境与 git 版本，供产物与日志识别：

- `BUILD_ENV` 取同环境变量，缺省 `local`；`local` 调试包写 `local`，Steam 包写 `steam`；
- `GIT_COMMIT` / `GIT_DIRTY` 缺省由脚本读取 `git rev-parse` 与 `git status -- client`，可用同名环境变量覆盖；
- `BUILD_TIME` 缺省取生成时刻（UTC），批量构建时由 Make 传入同一时间戳。
"""

from __future__ import annotations

import argparse
import datetime
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = ROOT / "client" / "generated" / "build_config.gd"

TEMPLATE = """class_name BuildConfig
# 本文件由 scripts/gen_build_config.py 生成，不要手改。

const SERVER_URL := "{server_url}"
const STEAM_APP_ID := {steam_app_id}
const BUILD_ENV := "{build_env}"
const GIT_COMMIT := "{git_commit}"
const GIT_DIRTY := {git_dirty}
const BUILD_TIME := "{build_time}"
"""


def _git(args: list[str]) -> str:
    try:
        done = subprocess.run(
            ["git", *args], cwd=ROOT, capture_output=True, text=True, check=False
        )
    except FileNotFoundError:
        return ""
    return done.stdout if done.returncode == 0 else ""


def git_state() -> tuple[str, bool]:
    """返回（提交短号，工作区是否有未提交改动）；非 git 环境返回空提交。"""
    commit = _git(["rev-parse", "--short", "HEAD"]).strip()
    status = _git(["status", "--porcelain", "--untracked-files=normal", "--", "client"])
    return commit, bool(status.strip())


def _utc_now() -> str:
    return datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _gd_string(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')


def _parse_app_id(raw: str) -> int:
    try:
        value = int(raw)
    except ValueError:
        raise ValueError(f"STEAM_APP_ID 必须是正整数：{raw!r}") from None
    if value <= 0:
        raise ValueError(f"STEAM_APP_ID 必须是正整数：{raw!r}")
    return value


def render(
    server_url: str,
    steam_app_id: int,
    build_env: str,
    git_commit: str,
    git_dirty: bool,
    build_time: str,
) -> str:
    return TEMPLATE.format(
        server_url=_gd_string(server_url),
        steam_app_id=steam_app_id,
        build_env=_gd_string(build_env),
        git_commit=_gd_string(git_commit),
        git_dirty="true" if git_dirty else "false",
        build_time=_gd_string(build_time),
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="生成 client/generated/build_config.gd")
    parser.add_argument(
        "--require",
        action="store_true",
        help="Steam 发布构建：SERVER_URL 与 STEAM_APP_ID 必须提供，且地址必须是 wss://",
    )
    parser.add_argument(
        "--require-url",
        action="store_true",
        help="dev 构建：SERVER_URL 必须提供，允许 ws:// 局域网地址",
    )
    parser.add_argument(
        "--require-server",
        action="store_true",
        help="prod 构建：SERVER_URL 必须提供且是 wss://，不要求 STEAM_APP_ID",
    )
    parser.add_argument("--output", type=pathlib.Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args(argv)

    server_url = os.environ.get("SERVER_URL", "").strip()
    raw_app_id = os.environ.get("STEAM_APP_ID", "").strip()
    build_env = os.environ.get("BUILD_ENV", "").strip() or "local"
    build_time = os.environ.get("BUILD_TIME", "").strip() or _utc_now()
    git_commit = os.environ.get("GIT_COMMIT", "").strip()
    raw_dirty = os.environ.get("GIT_DIRTY", "").strip()
    if not git_commit or raw_dirty == "":
        detected_commit, detected_dirty = git_state()
        git_commit = git_commit or detected_commit
        git_dirty = raw_dirty == "1" or (raw_dirty == "" and detected_dirty)
    else:
        git_dirty = raw_dirty == "1"

    try:
        if args.require or args.require_server or args.require_url:
            if not server_url:
                raise ValueError("dev/prod 构建需要 SERVER_URL")
        if args.require or args.require_server:
            if not server_url.startswith("wss://"):
                raise ValueError(f"SERVER_URL 必须是 wss:// 地址：{server_url}")
        if args.require and not raw_app_id:
            raise ValueError("Steam 构建需要 STEAM_APP_ID")
        steam_app_id = _parse_app_id(raw_app_id) if raw_app_id else 0
    except ValueError as error:
        print(f"gen_build_config: {error}", file=sys.stderr)
        return 1

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        render(server_url, steam_app_id, build_env, git_commit, git_dirty, build_time),
        encoding="utf-8",
    )
    try:
        shown = args.output.relative_to(ROOT)
    except ValueError:
        shown = args.output
    print(f"已生成 {shown}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
