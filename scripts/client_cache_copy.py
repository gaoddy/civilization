#!/usr/bin/env python3
"""新 worktree 从主检出播种 Godot 导入缓存。

判定主检出、过滤无源旁车，优先写时复制（macOS clonefile / Linux reflink），
不支持时回退普通复制。对主检出只读，只写当前 worktree 的 client/.godot。
"""

import argparse
import os
import platform
import shutil
import subprocess
import sys
import time
from pathlib import Path

PREFIX = "client-cache-copy："


def resolve_main(worktree: Path) -> Path:
    """由 worktree 的 git 公共目录反推主检出根。"""
    result = subprocess.run(
        [
            "git",
            "-C",
            str(worktree),
            "rev-parse",
            "--path-format=absolute",
            "--git-common-dir",
        ],
        text=True,
        capture_output=True,
    )
    if result.returncode != 0:
        raise RuntimeError(f"不在 git 仓库内：{worktree}")
    common_dir = Path(result.stdout.strip())
    if not common_dir.is_absolute():
        common_dir = worktree / common_dir
    return common_dir.resolve().parent


def count_files(path: Path) -> int:
    if not path.is_dir():
        return 0
    return sum(1 for item in path.rglob("*") if item.is_file())


def copy_tree(src: Path, dst: Path, prefer_clone: bool = True) -> None:
    """把 src 整目录复制到尚不存在的 dst。"""
    dst.parent.mkdir(parents=True, exist_ok=True)
    if prefer_clone:
        system = platform.system()
        if system == "Darwin":
            command = ["cp", "-c", "-R", str(src), str(dst)]
        elif system == "Linux":
            command = ["cp", "-a", "--reflink=auto", str(src), str(dst)]
        else:
            command = None
        if command is not None:
            result = subprocess.run(command, capture_output=True, text=True)
            if result.returncode == 0:
                return
            if dst.exists():
                shutil.rmtree(dst, ignore_errors=True)
    shutil.copytree(src, dst, copy_function=shutil.copy2)


def copy_file(src: Path, dst: Path, prefer_clone: bool = True) -> None:
    dst.parent.mkdir(parents=True, exist_ok=True)
    if prefer_clone:
        system = platform.system()
        if system == "Darwin":
            command = ["cp", "-c", "-p", str(src), str(dst)]
        elif system == "Linux":
            command = ["cp", "--reflink=auto", "-p", str(src), str(dst)]
        else:
            command = None
        if command is not None:
            result = subprocess.run(command, capture_output=True, text=True)
            if result.returncode == 0:
                return
    shutil.copy2(src, dst)


def copy_sidecars(main_root: Path, worktree: Path) -> tuple[int, int]:
    """复制 .import 旁车，只保留源文件在当前 worktree 中仍存在的条目。"""
    main_client = main_root / "client"
    worktree_client = worktree / "client"
    copied = 0
    skipped = 0
    for sidecar in sorted(main_client.rglob("*.import")):
        relative = sidecar.relative_to(main_client)
        if ".godot" in relative.parts:
            continue
        source = worktree_client / relative.with_suffix("")
        if not source.is_file():
            skipped += 1
            continue
        copy_file(sidecar, worktree_client / relative)
        copied += 1
    return copied, skipped


def parse_args(argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="从主检出为当前 worktree 播种 Godot 导入缓存",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="主检出缺少导入缓存时失败；缺省只告警并跳过",
    )
    parser.add_argument("--main", help="主检出根目录（缺省由 git 公共目录反推）")
    parser.add_argument("--worktree", help="当前 worktree 根目录（缺省当前目录）")
    parser.add_argument("--quiet", action="store_true", help="成功路径不打印摘要")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    worktree = Path(args.worktree).resolve() if args.worktree else Path.cwd()
    try:
        main_root = Path(args.main).resolve() if args.main else resolve_main(worktree)
    except RuntimeError as error:
        print(f"{PREFIX}{error}", file=sys.stderr)
        return 1

    if main_root == worktree:
        if not args.quiet:
            print(f"{PREFIX}当前是主检出，无需复制缓存")
        return 0

    source = main_root / "client" / ".godot" / "imported"
    target = worktree / "client" / ".godot" / "imported"
    if count_files(target) > 0:
        if not args.quiet:
            print(f"{PREFIX}已有导入缓存，跳过播种")
        return 0

    if count_files(source) == 0:
        if args.strict:
            print(
                f"{PREFIX}主检出缺少导入缓存，请先执行：cd {main_root} && make client-import",
                file=sys.stderr,
            )
            return 1
        print(f"{PREFIX}主检出缺少导入缓存，跳过播种", file=sys.stderr)
        return 0

    seed = target.with_name("imported.seed")
    if seed.is_dir():
        shutil.rmtree(seed)
    elif seed.exists():
        seed.unlink()

    started = time.monotonic()
    try:
        copy_tree(source, seed)
    except OSError as error:
        shutil.rmtree(seed, ignore_errors=True)
        print(f"{PREFIX}复制导入缓存失败：{error}", file=sys.stderr)
        return 1

    # 旁车先复制、成功后才让 imported 生效：中断或失败不会留下
    # “缓存已就位、旁车缺失”的半套状态，下次运行仍会重新播种。
    try:
        copied, skipped = copy_sidecars(main_root, worktree)
    except OSError as error:
        shutil.rmtree(seed, ignore_errors=True)
        print(f"{PREFIX}复制 .import 旁车失败：{error}", file=sys.stderr)
        return 1

    try:
        if target.exists():
            shutil.rmtree(target)
        os.replace(seed, target)
    except OSError as error:
        shutil.rmtree(seed, ignore_errors=True)
        print(f"{PREFIX}导入缓存生效失败：{error}", file=sys.stderr)
        return 1
    if not args.quiet:
        elapsed = time.monotonic() - started
        print(
            f"{PREFIX}已从 {main_root} 播种 {count_files(target)} 个缓存文件"
            f"与 {copied} 个 .import 旁车（跳过 {skipped} 个无源旁车，耗时 {elapsed:.2f}s）"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
