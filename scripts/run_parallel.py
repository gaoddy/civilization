#!/usr/bin/env python3
"""并行执行互不依赖的 shell 命令。

用法：
  python3 scripts/run_parallel.py [--jobs N] "cmd-a" "cmd-b" ...

- 每条命令由独立子进程执行；任一命令失败不阻止其余命令继续执行。
- 每个命令的输出整体缓冲后打印，避免并发写入交错。
- 单个命令输出超过 MAX_OUTPUT 字符时保留末尾，失败信息通常位于结尾。
- 任一命令退出码非 0 时，脚本以 1 退出，并汇总失败命令与退出码。
"""
from __future__ import annotations

import argparse
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed

MAX_OUTPUT = 12_000


def default_jobs() -> int:
    return max(1, min(os.cpu_count() or 1, 8))


def run_one(index: int, command: str) -> tuple[int, str, int, str]:
    proc = subprocess.run(
        command,
        shell=True,
        capture_output=True,
        text=True,
        errors="replace",
    )
    return index, command, proc.returncode, (proc.stdout or "") + (proc.stderr or "")


def format_output(output: str) -> str:
    text = output.rstrip()
    if len(text) <= MAX_OUTPUT:
        return text
    dropped = len(text) - MAX_OUTPUT
    return f"…（已截断前 {dropped} 字符）\n{text[-MAX_OUTPUT:]}"


def report(index: int, total: int, command: str, code: int, output: str) -> None:
    print(f"==> [{index + 1}/{total}] {command}", flush=True)
    text = format_output(output)
    if text:
        print(text, flush=True)
    print(f"<== [{index + 1}/{total}] 退出 {code}", flush=True)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="并行执行互不依赖的 shell 命令")
    parser.add_argument(
        "--jobs",
        type=int,
        default=default_jobs(),
        help=f"并行度，默认 {default_jobs()}（CPU 数，上限 8）",
    )
    parser.add_argument("commands", nargs="+", help="要执行的 shell 命令")
    args = parser.parse_args(argv)
    if args.jobs < 1:
        parser.error("--jobs 必须是正整数")
    if not args.commands:
        parser.error("至少需要一条命令")

    total = len(args.commands)
    failures: list[tuple[int, str, int]] = []

    if args.jobs == 1 or total == 1:
        for index, command in enumerate(args.commands):
            i, command, code, output = run_one(index, command)
            report(i, total, command, code, output)
            if code != 0:
                failures.append((i, command, code))
    else:
        with ThreadPoolExecutor(max_workers=min(args.jobs, total)) as pool:
            futures = [
                pool.submit(run_one, index, command)
                for index, command in enumerate(args.commands)
            ]
            for future in as_completed(futures):
                i, command, code, output = future.result()
                report(i, total, command, code, output)
                if code != 0:
                    failures.append((i, command, code))

    if failures:
        print(f"失败 {len(failures)} 项：", file=sys.stderr)
        for index, command, code in sorted(failures):
            print(f"  [{index + 1}/{total}] {command}（退出 {code}）", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
