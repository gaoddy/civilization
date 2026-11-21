#!/usr/bin/env python3
"""生成客户端构建配置 `client/generated/build_config.gd`。

开发构建不写任何环境变量，生成空配置，客户端回落到本机调试地址；发布构建由
`make client-export-steam` 传入 `SERVER_URL` 与 `STEAM_APP_ID`，缺任一值或地址不是
`wss://` 时直接失败，不产出客户端。
"""

from __future__ import annotations

import argparse
import os
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = ROOT / "client" / "generated" / "build_config.gd"

TEMPLATE = """class_name BuildConfig
# 本文件由 scripts/gen_build_config.py 生成，不要手改。

const SERVER_URL := "{server_url}"
const STEAM_APP_ID := {steam_app_id}
"""


def _parse_app_id(raw: str) -> int:
    try:
        value = int(raw)
    except ValueError:
        raise ValueError(f"STEAM_APP_ID 必须是正整数：{raw!r}") from None
    if value <= 0:
        raise ValueError(f"STEAM_APP_ID 必须是正整数：{raw!r}")
    return value


def render(server_url: str, steam_app_id: int) -> str:
    return TEMPLATE.format(server_url=server_url, steam_app_id=steam_app_id)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="生成 client/generated/build_config.gd")
    parser.add_argument(
        "--require",
        action="store_true",
        help="发布构建：SERVER_URL 与 STEAM_APP_ID 必须提供，且地址必须是 wss://",
    )
    parser.add_argument("--output", type=pathlib.Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args(argv)

    server_url = os.environ.get("SERVER_URL", "").strip()
    raw_app_id = os.environ.get("STEAM_APP_ID", "").strip()

    try:
        if args.require:
            if not server_url:
                raise ValueError("发布构建需要 SERVER_URL")
            if not server_url.startswith("wss://"):
                raise ValueError(f"SERVER_URL 必须是 wss:// 地址：{server_url}")
            if not raw_app_id:
                raise ValueError("发布构建需要 STEAM_APP_ID")
        steam_app_id = _parse_app_id(raw_app_id) if raw_app_id else 0
    except ValueError as error:
        print(f"gen_build_config: {error}", file=sys.stderr)
        return 1

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(render(server_url, steam_app_id), encoding="utf-8")
    print(f"已生成 {args.output.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
