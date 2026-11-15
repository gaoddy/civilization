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


if __name__ == "__main__":
    unittest.main()
