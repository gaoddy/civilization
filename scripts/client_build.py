#!/usr/bin/env python3
"""客户端构建标识：git 版本、工作区指纹与产物复用判定。

子命令：

- `id`：打印构建号 `<提交短号>[-dirty]-<环境>[-aab]`；
- `stamp`：写产物戳记 JSON（提交、指纹、环境、配置与产物清单）；
- `reuse`：比对戳记与当前源码状态，命中时退出 0 且不重复导出，否则退出 1。

构建号里的 dirty 取 `--watch` 路径的 `git status --porcelain` 结果；指纹把
HEAD、status 文本与每个变更文件的大小/mtime 一起哈希，因此 dirty 工作区也能
安全复用：源码一变指纹就变，同名产物会被重建覆盖。
"""

from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCHEMA = 1


def _git(args: list[str], root: pathlib.Path) -> str:
    try:
        done = subprocess.run(
            ["git", *args], cwd=root, capture_output=True, text=True, check=False
        )
    except FileNotFoundError:
        return ""
    return done.stdout if done.returncode == 0 else ""


def git_value(args: list[str], root: pathlib.Path) -> str:
    return _git(args, root).strip()


def status_text(root: pathlib.Path, watch: list[str]) -> str:
    paths = watch or ["client"]
    return _git(["status", "--porcelain", "--untracked-files=normal", "--", *paths], root)


def _status_paths(text: str) -> list[str]:
    """把 porcelain 文本拆成受影响的仓库相对路径，重命名两侧都算。"""
    paths: list[str] = []
    for line in text.splitlines():
        if len(line) < 4:
            continue
        rest = line[3:].strip()
        if " -> " in rest:
            old, new = rest.split(" -> ", 1)
            paths.extend([new.strip('"'), old.strip('"')])
        elif rest:
            paths.append(rest.strip('"'))
    return paths


CONTENT_HASH_LIMIT = 2 * 1024 * 1024


def _file_token(root: pathlib.Path, relative: str, target: pathlib.Path) -> str:
    """小文件按内容哈希，大文件按大小与 mtime：避免导出自身触碰的临时改写干扰指纹。"""
    stat = target.stat()
    if stat.st_size <= CONTENT_HASH_LIMIT:
        digest = hashlib.sha256(target.read_bytes()).hexdigest()
        return f"{relative}:{digest}"
    return f"{relative}:{stat.st_size}:{stat.st_mtime_ns}"


def fingerprint(root: pathlib.Path, watch: list[str]) -> tuple[str, bool]:
    """返回（工作区指纹，是否 dirty）；watch 为空时只看 client。"""
    text = status_text(root, watch)
    commit = git_value(["rev-parse", "HEAD"], root) or "nogit"
    parts = [commit, text]
    for relative in _status_paths(text):
        target = root / relative
        try:
            if target.is_file():
                parts.append(_file_token(root, relative, target))
            elif target.exists():
                parts.append(f"{relative}:dir")
            else:
                parts.append(f"{relative}:missing")
        except OSError:
            parts.append(f"{relative}:missing")
    digest = hashlib.sha256("\n".join(parts).encode("utf-8")).hexdigest()
    return digest, bool(text.strip())


def build_id(root: pathlib.Path, env: str, watch: list[str], fmt: str) -> str:
    commit = git_value(["rev-parse", "--short", "HEAD"], root) or "nogit"
    _, dirty = fingerprint(root, watch)
    parts = [commit]
    if dirty:
        parts.append("dirty")
    if env:
        parts.append(env)
    if fmt == "aab":
        parts.append("aab")
    return "-".join(parts)


def _relative(root: pathlib.Path, path: str) -> str:
    resolved = pathlib.Path(path)
    if not resolved.is_absolute():
        resolved = root / resolved
    try:
        return str(resolved.resolve().relative_to(root.resolve()))
    except ValueError:
        return str(resolved)


def _artifact_exists(root: pathlib.Path, relative: str) -> bool:
    target = root / relative
    return target.is_file() and target.stat().st_size > 0


def _utc_now() -> str:
    return datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def build_stamp(
    root: pathlib.Path,
    platform: str,
    env: str,
    identity: str,
    fmt: str,
    watch: list[str],
    artifacts: list[str],
    server_url: str,
    steam_app_id: int,
    godot_version: str,
) -> dict:
    digest, dirty = fingerprint(root, watch)
    return {
        "schema": SCHEMA,
        "platform": platform,
        "env": env,
        "build_id": identity,
        "format": fmt,
        "git": {
            "commit": git_value(["rev-parse", "--short", "HEAD"], root),
            "commit_full": git_value(["rev-parse", "HEAD"], root),
            "branch": git_value(["rev-parse", "--abbrev-ref", "HEAD"], root),
            "dirty": dirty,
        },
        "worktree": {"paths": watch or ["client"], "fingerprint": digest},
        "config": {
            "server_url": server_url,
            "steam_app_id": steam_app_id,
            "godot_version": godot_version,
        },
        "artifacts": [_relative(root, item) for item in artifacts],
        "built_at": _utc_now(),
    }


def reuse_reason(
    root: pathlib.Path,
    stamp_path: pathlib.Path,
    platform: str,
    env: str,
    identity: str,
    fmt: str,
    watch: list[str],
    artifacts: list[str],
    server_url: str,
    steam_app_id: int,
    godot_version: str,
) -> str:
    """命中复用返回空串，否则返回未复用的原因。"""
    if not stamp_path.is_file():
        return f"缺少戳记 {stamp_path}"
    try:
        stamp = json.loads(stamp_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        return f"戳记无法解析：{error}"
    expected = {
        "schema": SCHEMA,
        "platform": platform,
        "env": env,
        "build_id": identity,
        "format": fmt,
        "fingerprint": fingerprint(root, watch)[0],
        "server_url": server_url,
        "steam_app_id": steam_app_id,
        "godot_version": godot_version,
    }
    stored = {
        "schema": stamp.get("schema"),
        "platform": stamp.get("platform"),
        "env": stamp.get("env"),
        "build_id": stamp.get("build_id"),
        "format": stamp.get("format"),
        "fingerprint": (stamp.get("worktree") or {}).get("fingerprint"),
        "server_url": (stamp.get("config") or {}).get("server_url"),
        "steam_app_id": (stamp.get("config") or {}).get("steam_app_id"),
        "godot_version": (stamp.get("config") or {}).get("godot_version"),
    }
    for key, value in expected.items():
        if stored[key] != value:
            return f"戳记不匹配（{key}：记录 {stored[key]!r}，当前 {value!r}）"
    wanted = [_relative(root, item) for item in artifacts]
    if sorted(stamp.get("artifacts") or []) != sorted(wanted):
        return "产物清单与戳记不一致"
    for relative in wanted:
        if not _artifact_exists(root, relative):
            return f"产物缺失或为空 {relative}"
    return ""


def _parse_app_id(raw: str) -> int:
    if not raw:
        return 0
    return int(raw)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="客户端构建标识与产物复用")
    sub = parser.add_subparsers(dest="command", required=True)

    def add_common(target: argparse.ArgumentParser) -> None:
        target.add_argument("--root", type=pathlib.Path, default=ROOT, help="仓库根目录")
        target.add_argument("--watch", action="append", default=[], help="参与 dirty 与指纹判定的路径，可重复")

    identify = sub.add_parser("id", help="打印构建号")
    add_common(identify)
    identify.add_argument("--env", default="local")
    identify.add_argument("--format", choices=["", "apk", "aab"], default="")

    stamp_cmd = sub.add_parser("stamp", help="写产物戳记")
    add_common(stamp_cmd)
    stamp_cmd.add_argument("--stamp", required=True, type=pathlib.Path)
    stamp_cmd.add_argument("--platform", required=True)
    stamp_cmd.add_argument("--env", default="local")
    stamp_cmd.add_argument("--id", dest="identity", required=True)
    stamp_cmd.add_argument("--format", choices=["", "apk", "aab"], default="")
    stamp_cmd.add_argument("--artifact", action="append", default=[], help="产物路径，可重复")
    stamp_cmd.add_argument("--server-url", default="")
    stamp_cmd.add_argument("--steam-app-id", default="0")
    stamp_cmd.add_argument("--godot-version", default="")

    reuse_cmd = sub.add_parser("reuse", help="判断产物能否复用")
    add_common(reuse_cmd)
    reuse_cmd.add_argument("--stamp", required=True, type=pathlib.Path)
    reuse_cmd.add_argument("--platform", required=True)
    reuse_cmd.add_argument("--env", default="local")
    reuse_cmd.add_argument("--id", dest="identity", required=True)
    reuse_cmd.add_argument("--format", choices=["", "apk", "aab"], default="")
    reuse_cmd.add_argument("--artifact", action="append", default=[], help="产物路径，可重复")
    reuse_cmd.add_argument("--server-url", default="")
    reuse_cmd.add_argument("--steam-app-id", default="0")
    reuse_cmd.add_argument("--godot-version", default="")

    args = parser.parse_args(argv)
    watch = args.watch or ["client"]
    root = args.root

    if args.command == "id":
        print(build_id(root, args.env, watch, args.format))
        return 0

    try:
        app_id = _parse_app_id(args.steam_app_id)
    except ValueError:
        print(f"client_build: STEAM_APP_ID 不是整数：{args.steam_app_id}", file=sys.stderr)
        return 2

    if args.command == "stamp":
        if not args.identity:
            print("client_build: 缺少构建号", file=sys.stderr)
            return 2
        stamp = build_stamp(
            root,
            args.platform,
            args.env,
            args.identity,
            args.format,
            watch,
            args.artifact,
            args.server_url,
            app_id,
            args.godot_version,
        )
        args.stamp.parent.mkdir(parents=True, exist_ok=True)
        args.stamp.write_text(json.dumps(stamp, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        return 0

    reason = reuse_reason(
        root,
        args.stamp,
        args.platform,
        args.env,
        args.identity,
        args.format,
        watch,
        args.artifact,
        args.server_url,
        app_id,
        args.godot_version,
    )
    if reason:
        print(f"未复用：{reason}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
