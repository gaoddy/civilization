# dev-bots 信号处理收尾 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 `scripts/dev_bots.sh` 在任何终止信号下都关掉 `room-service` 与 `sim`（端口释放）、原样透传 `sim` 的退出码，并让这三条路径进 `make test`。

**Architecture:** 脚本把 `sim` 从前台调用改后台启动并 `wait` 取退出码，`cleanup` 同时收两个进程、trap 同时挂 `EXIT INT TERM HUP`。可测性来自两个新的桩覆盖点（`DEV_SERVER_BIN` / `DEV_SIM_BIN`），测试用标准库 `unittest` 起真脚本 + 桩进程；信号只打到脚本 PID（这正是 `timeout` / CI 取消 / 进程管理器发信号的真实路径），不 `killpg`，否则桩进程会被信号直接杀掉，测试就测不出脚本的清理逻辑。

**Tech Stack:** bash（`set -euo pipefail`、`trap`、`wait`）、Python 3 标准库 `unittest` / `subprocess` / `signal` / `socket`。

**Spec:** `docs/superpowers/specs/2026-11-16-dev-bots-signals-design.md`

## Global Constraints

- 基线分支 `develop`；改动只发生在 `fix/dev-bots-signals` worktree 内。
- 不改 `sim dev` 的 Rust 逻辑，不改 `make dev-bots` 的对外用法（`make dev-bots [DEV_PORT=N]`，多余参数原样转发给 `sim dev`）。
- 不追求 SIGKILL 兜底（SIGKILL 不可捕获）。
- Python 测试只用标准库 `unittest`，由 `make test` 的 `python3 -m unittest discover -s tests -v` 发现；不引入 pytest 或任何新依赖。
- 三条新用例合计不拖长 `make test`：最长的那条在 15 秒内结束。

## Review Focus

- **端口已占用时不静默复用**：已有进程在听就退出 1 并给出换端口提示；期望失败即停，不是连上别人的服务。
- **`sim` 的退出码是脚本的退出码**：`sim dev` 以 3 退出时脚本也以 3 退出，上层才能判断成败。
- **只给脚本发 SIGTERM 也不留孤儿**：这是 `timeout` / CI 取消的真实路径；期望脚本退出时两个子进程都已结束、端口可再次绑定。
- **Ctrl-C 一次即退**：`SIGINT` 与 `SIGTERM` 走同一条 cleanup 路径，不需要按两次。
- **`HUP` 不比 `TERM` 弱**：终端关闭时同样完成清理。

---

### Task 1: 桩注入 + 后台 `wait` 透传退出码

**Files:**
- Modify: `scripts/dev_bots.sh:13-14`（二进制路径可覆盖）、`scripts/dev_bots.sh:62`（`sim` 改后台 + `wait`）
- Create: `tests/test_dev_bots_script.py`

**Interfaces:**
- Consumes: 现有行为（端口探测、就绪等待、`kill -0` 守卫）不变。
- Produces: 环境变量契约 `DEV_SERVER_BIN`、`DEV_SIM_BIN`（覆盖默认 `server/target/debug/{room-service,sim}`）；脚本退出码等于 `sim` 的退出码。Task 2 依赖脚本里存在 `SIM_PID` 变量与 `cleanup()` 函数。

- [ ] **Step 1: 写失败用例**

创建 `tests/test_dev_bots_script.py`（下面就是完整文件；用例只两条，第三条在 Task 2 加）。

```python
#!/usr/bin/env python3
"""scripts/dev_bots.sh 的行为断言：用桩二进制代替真实 server / sim。"""

import os
import pathlib
import signal
import socket
import subprocess
import sys
import tempfile
import textwrap
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "dev_bots.sh"

SERVER_STUB = textwrap.dedent(
    """
    import socket, sys, time
    port = int(sys.argv[sys.argv.index("--listen") + 1].rsplit(":", 1)[1])
    with socket.socket() as sock:
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.bind(("127.0.0.1", port))
        sock.listen(8)
        time.sleep(120)
    """
)

# 不解析 argv：脚本传的是 `dev --connect ws://... --intent-seed ...`。
SIM_STUB = textwrap.dedent(
    """
    import os, sys, time
    time.sleep(float(os.environ.get("STUB_SLEEP", "0.2")))
    sys.exit(int(os.environ.get("STUB_EXIT", "0")))
    """
)


def free_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def port_open(port: int) -> bool:
    with socket.socket() as sock:
        sock.settimeout(0.2)
        return sock.connect_ex(("127.0.0.1", port)) == 0


class DevBotsScriptTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        base = pathlib.Path(self.tmp.name)
        self.server_stub = base / "server_stub.py"
        self.sim_stub = base / "sim_stub.py"
        self.server_stub.write_text(SERVER_STUB, encoding="utf-8")
        self.sim_stub.write_text(SIM_STUB, encoding="utf-8")
        self.env = dict(os.environ)
        self.env["DEV_SERVER_BIN"] = str(self.server_stub)
        self.env["DEV_SIM_BIN"] = str(self.sim_stub)
        self.procs = []
        self.addCleanup(self._reap)

    def _reap(self):
        """用例失败时收尸：桩都留在脚本所在的进程组里。"""
        for proc in self.procs:
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            if proc.poll() is None:
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    pass

    def spawn(self, port: int, extra_env: dict | None = None) -> subprocess.Popen:
        env = dict(self.env)
        env["DEV_PORT"] = str(port)
        env.update(extra_env or {})
        proc = subprocess.Popen(
            [sys.executable, str(SCRIPT)],
            cwd=str(ROOT),
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            start_new_session=True,
        )
        self.procs.append(proc)
        return proc

    def test_port_occupied_fails_fast_with_hint(self):
        port = free_port()
        holder = socket.socket()
        holder.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        holder.bind(("127.0.0.1", port))
        holder.listen(1)
        self.addCleanup(holder.close)

        proc = self.spawn(port)
        output, _ = proc.communicate(timeout=30)
        self.assertEqual(proc.returncode, 1, output)
        self.assertIn(f"端口 {port} 已被占用", output)
        self.assertIn(f"DEV_PORT={port + 1}", output)

    def test_sim_exit_code_is_passed_through(self):
        port = free_port()
        proc = self.spawn(port, {"STUB_EXIT": "3"})
        output, _ = proc.communicate(timeout=30)
        self.assertEqual(proc.returncode, 3, output)
```

- [ ] **Step 2: 跑用例确认失败**

Run: `cd /Users/gaoddy/Projects/Civilization/.worktrees/fix-dev-bots-signals && python3 -m unittest tests.test_dev_bots_script -v`
Expected: 两条都 FAIL。第一条失败于 `缺少 room-service，先执行 make m1b-build`（脚本还不认 `DEV_SERVER_BIN`，退出码 1 但输出里没有「端口 X 已被占用」）；第二条失败于 `1 != 3`。

- [ ] **Step 3: 改脚本**

顶部路径改为可覆盖（`scripts/dev_bots.sh:13-14`）：

```bash
SERVER_BIN=${DEV_SERVER_BIN:-"$ROOT/server/target/debug/room-service"}
SIM_BIN=${DEV_SIM_BIN:-"$ROOT/server/target/debug/sim"}
```

文件末尾（`scripts/dev_bots.sh:62`）把前台调用改成后台 + `wait`：

```bash
"$SIM_BIN" dev --connect "$URL" "$@" &
SIM_PID=$!
wait "$SIM_PID"
```

`wait` 返回 `sim` 的退出码，`set -e` 让脚本以该码退出——这就是要的透传，此处不要 `|| true`。

> 与设计文档的一处偏差：设计文档写「`wait` 要放在 `|| true` 里，否则 `set -e` 会让透传的退出码丢掉」。实际上 `set -e` 让脚本以 `wait` 的返回码退出，正是透传；加 `|| true` 才会把 3 变成 0。以本条为准（Task 1 Step 1 的用例就是这条判据）。

- [ ] **Step 4: 跑用例确认通过**

Run: `cd /Users/gaoddy/Projects/Civilization/.worktrees/fix-dev-bots-signals && python3 -m unittest tests.test_dev_bots_script -v`
Expected: `Ran 2 tests ... OK`。

- [ ] **Step 5: 提交**

```bash
git add scripts/dev_bots.sh tests/test_dev_bots_script.py docs/superpowers/specs/2026-11-16-dev-bots-signals-design.md docs/superpowers/plans/2026-11-16-dev-bots-signals.md
git commit -m "fix(dev): dev_bots.sh 支持桩注入并透传 sim 退出码"
```

---

### Task 2: 终止信号下清理两个进程

**Files:**
- Modify: `scripts/dev_bots.sh:32-39`（`SIM_PID` 初始化、`cleanup`、`trap`）
- Modify: `tests/test_dev_bots_script.py`（新增第三条用例）

**Interfaces:**
- Consumes: Task 1 的 `SIM_PID` 变量与两个 `DEV_*_BIN` 覆盖点、`spawn(port, extra_env)`。
- Produces: INT / TERM / HUP 与正常退出都走同一条 `cleanup`；脚本退出前两个子进程都已结束。

- [ ] **Step 1: 写失败用例**

在 `DevBotsScriptTest` 里新增（放在 `test_sim_exit_code_is_passed_through` 之后）。INT / TERM / HUP 共用一条用例，逐个 `subTest`：

```python
    def test_termination_signals_kill_both_children(self):
        for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
            with self.subTest(sig=sig):
                port = free_port()
                # 桩 sim 存活足够久，保证脚本收到信号时它还在等。
                proc = self.spawn(port, {"STUB_SLEEP": "120"})
                deadline = time.time() + 10
                while time.time() < deadline and not port_open(port):
                    if proc.poll() is not None:
                        self.fail(f"{sig!r}: 脚本提前退出：{proc.communicate()[0]}")
                    time.sleep(0.05)
                self.assertTrue(port_open(port), f"{sig!r}: 桩服务端未在 10 秒内监听")

                os.kill(proc.pid, sig)  # 只打脚本，不打整组
                try:
                    output, _ = proc.communicate(timeout=10)
                except subprocess.TimeoutExpired:
                    self.fail(f"{sig!r}: 脚本收到信号后 10 秒内没有退出（trap 被前台 sim 挡住）")

                deadline = time.time() + 5
                while time.time() < deadline and port_open(port):
                    time.sleep(0.05)
                self.assertFalse(port_open(port), f"{sig!r}: 脚本退出后端口 {port} 仍被占用：{output}")
```

文件顶部 import 补 `time`（按字母序插在 `tempfile` 之后、`unittest` 之前）。

- [ ] **Step 2: 跑用例确认失败**

Run: `cd /Users/gaoddy/Projects/Civilization/.worktrees/fix-dev-bots-signals && python3 -m unittest tests.test_dev_bots_script.DevBotsScriptTest.test_termination_signals_kill_both_children -v`
Expected: FAIL，每个 subTest 都是 `脚本收到信号后 10 秒内没有退出（trap 被前台 sim 挡住）`——这正是设计要修的行为。

- [ ] **Step 3: 改脚本**

在 `SERVER_PID=$!` 之后先初始化 `SIM_PID`，再替换 cleanup 与 trap（`scripts/dev_bots.sh:32-39`）：

```bash
SIM_PID=""
cleanup() {
  for pid in "$SIM_PID" "$SERVER_PID"; do
    if [ -n "$pid" ]; then
      kill "$pid" 2>/dev/null || true
    fi
  done
  wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM HUP
```

裸 `wait` 收该 shell 的全部子进程（两个都已 `kill`），所以 `SIM_PID` 还是空的时候也不会漏掉 `SERVER_PID`。

- [ ] **Step 4: 跑全部用例确认通过**

Run: `cd /Users/gaoddy/Projects/Civilization/.worktrees/fix-dev-bots-signals && python3 -m unittest tests.test_dev_bots_script -v`
Expected: `Ran 3 tests ... OK`，耗时 < 15 秒，无残留（`pgrep -f server_stub` 无输出）。

- [ ] **Step 5: 提交**

```bash
git add scripts/dev_bots.sh tests/test_dev_bots_script.py
git commit -m "fix(dev): dev_bots.sh 在 INT/TERM/HUP 下同时清理服务端与 sim"
```

---

### Task 3: 删掉恒真断言并跑完整校验

**Files:**
- Modify: `server/crates/room-service/tests/dev_bots.rs:8`（去掉 `DEV_BOTS` import）、`server/crates/room-service/tests/dev_bots.rs:53-56`（整条函数连同 `#[test]` 删除）

**Interfaces:**
- Consumes: 无。
- Produces: 无新接口；`dev_bots.rs` 只剩两条有保护力的用例。

- [ ] **Step 1: 删除恒真用例**

删掉 `dev_bots_leaves_exactly_one_seat_for_the_developer`（含 `#[test]`），并把 import 改成 `use sim::bot::dev::{DevConfig, run as run_dev};`——该文件里 `DEV_BOTS` 只在这一条用例出现过。

- [ ] **Step 2: 跑 Rust 测试确认仍通过**

Run: `cd /Users/gaoddy/Projects/Civilization/.worktrees/fix-dev-bots-signals/server && cargo test -p room-service --test dev_bots`
Expected: `2 passed`（`dev_run_finishes_a_game_once_the_developer_joins`、`dev_run_gives_up_when_nobody_joins`），无 unused import 警告。

- [ ] **Step 3: 跑全量校验**

Run: `cd /Users/gaoddy/Projects/Civilization/.worktrees/fix-dev-bots-signals && make lint && make test`
Expected: 两条命令退出码均为 0；unittest 输出里出现 `test_dev_bots_script` 的三条 `ok`。

- [ ] **Step 4: 手工验证真实栈**

```bash
make dev-bots &            # 起真服务端 + 7 个机器人
sleep 3
kill -TERM $!              # 只打脚本，模拟 timeout / CI 取消
sleep 2
lsof -i :8080              # 期望无输出
```

再验证 Ctrl-C：`make dev-bots` 后按一次 Ctrl-C，进程干净退出，`lsof -i :8080` 无输出。

- [ ] **Step 5: 提交**

```bash
git add server/crates/room-service/tests/dev_bots.rs
git commit -m "test(dev): 删除 dev_bots 中恒真的座位数断言"
```

---

## 收尾

- 全分支 diff 走一次 review，findings 一并修掉。
- 合并回 `develop`（本地 `--no-ff`，与上一任务一致），在合并后的树上重跑 `make test`。
- 删除 worktree 与分支。
