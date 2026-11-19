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
import time
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
# 给了 SIM_PID_FILE 就把自身 PID 写进去，测试据此确认 sim 真的启动过；
# 给了 SIM_RUNS_FILE 就追加一行，测试据此数循环起过几局。
SIM_STUB = textwrap.dedent(
    """
    import os, sys, time
    pid_file = os.environ.get("SIM_PID_FILE")
    if pid_file:
        with open(pid_file, "w", encoding="utf-8") as handle:
            handle.write(str(os.getpid()))
    runs_file = os.environ.get("SIM_RUNS_FILE")
    if runs_file:
        with open(runs_file, "a", encoding="utf-8") as handle:
            handle.write("run\\n")
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


def process_alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    return True


class DevBotsScriptTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        base = pathlib.Path(self.tmp.name)
        self.server_stub = base / "server_stub.py"
        self.sim_stub = base / "sim_stub.py"
        self.server_stub.write_text(f"#!{sys.executable}\n{SERVER_STUB}", encoding="utf-8")
        self.sim_stub.write_text(f"#!{sys.executable}\n{SIM_STUB}", encoding="utf-8")
        self.server_stub.chmod(0o755)
        self.sim_stub.chmod(0o755)
        self.env = dict(os.environ)
        self.env["DEV_SERVER_BIN"] = str(self.server_stub)
        self.env["DEV_SIM_BIN"] = str(self.sim_stub)
        self.procs = []
        self.addCleanup(self._reap)

    def _reap(self):
        """用例失败时收尸：桩都留在脚本所在的进程组里。"""
        for proc in self.procs:
            try:
                # PID 可能已被复用；只有确认它仍是该组的组长才按组杀。
                if os.getpgid(proc.pid) == proc.pid:
                    os.killpg(proc.pid, signal.SIGKILL)
            except (ProcessLookupError, PermissionError):
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
            ["bash", str(SCRIPT)],
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

    def test_clean_game_starts_the_next_round(self):
        """一局结束后脚本应再起一次 sim，而不是退出。"""
        port = free_port()
        runs_file = pathlib.Path(self.tmp.name) / "sim-runs.log"
        proc = self.spawn(
            port,
            {
                "DEV_ROUNDS": "2",
                "STUB_SLEEP": "0.05",
                "SIM_RUNS_FILE": str(runs_file),
            },
        )
        output, _ = proc.communicate(timeout=30)
        self.assertEqual(proc.returncode, 0, output)
        runs = runs_file.read_text(encoding="utf-8").splitlines()
        self.assertEqual(len(runs), 2, output)
        self.assertIn("第 2 局", output)

    def test_invalid_rounds_fails_before_starting_the_sim(self):
        port = free_port()
        runs_file = pathlib.Path(self.tmp.name) / "sim-runs.log"
        proc = self.spawn(
            port, {"DEV_ROUNDS": "two", "SIM_RUNS_FILE": str(runs_file)}
        )
        output, _ = proc.communicate(timeout=30)
        self.assertEqual(proc.returncode, 1, output)
        self.assertIn("DEV_ROUNDS 必须是正整数", output)
        self.assertFalse(runs_file.exists(), output)

    def test_termination_signals_kill_both_children(self):
        for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
            with self.subTest(sig=sig):
                port = free_port()
                pid_file = pathlib.Path(self.tmp.name) / f"sim-{sig.value}.pid"
                # 桩 sim 存活足够久，且先把 PID 落盘：等到 pid 文件出现再发信号，
                # 才真的在考「脚本收到了 sim 的 PID 之后会不会收掉它」。
                proc = self.spawn(
                    port, {"STUB_SLEEP": "120", "SIM_PID_FILE": str(pid_file)}
                )
                deadline = time.time() + 10
                while time.time() < deadline and not (
                    port_open(port) and pid_file.exists() and pid_file.read_text()
                ):
                    if proc.poll() is not None:
                        self.fail(f"{sig!r}: 脚本提前退出，退出码 {proc.returncode}")
                    time.sleep(0.05)
                self.assertTrue(port_open(port), f"{sig!r}: 桩服务端未在 10 秒内监听")
                sim_pid = int(pid_file.read_text())

                os.kill(proc.pid, sig)  # 只打脚本，不打整组
                try:
                    output, _ = proc.communicate(timeout=10)
                except subprocess.TimeoutExpired:
                    self.fail(
                        f"{sig!r}: 脚本收到信号后 10 秒内没有退出"
                        f"（pid={proc.pid}，sim_pid={sim_pid}）"
                    )

                self.assertEqual(proc.returncode, 128 + sig.value, output)
                deadline = time.time() + 5
                while time.time() < deadline and (
                    port_open(port) or process_alive(sim_pid)
                ):
                    time.sleep(0.05)
                self.assertFalse(port_open(port), f"{sig!r}: 端口 {port} 仍被占用")
                self.assertFalse(process_alive(sim_pid), f"{sig!r}: sim 进程仍存活")


if __name__ == "__main__":
    unittest.main()
