#!/usr/bin/env python3
"""客户端导出环境校验测试：ENV 只接受 dev/prod，正式构建必须有地址。"""

import os
import pathlib
import subprocess
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
CLEARED = (
    "ENV",
    "SERVER_URL",
    "CLIENT_DEV_SERVER_URL",
    "CLIENT_PROD_SERVER_URL",
    "STEAM_APP_ID",
    "IOS_DEV_SERVER_URL",
)


class ClientBuildEnvTest(unittest.TestCase):
    def _run_make(self, args):
        """清空构建相关环境变量后运行 make，避免父 shell 配置干扰断言。"""
        env = dict(os.environ)
        for name in CLEARED:
            env.pop(name, None)
        return subprocess.run(
            ["make", *args],
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=False,
            stdin=subprocess.DEVNULL,
        )

    def test_default_env_is_dev(self):
        proc = self._run_make(["client-env-check"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)

    def test_unknown_env_is_rejected(self):
        proc = self._run_make(["client-env-check", "ENV=production"])
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("ENV", proc.stdout + proc.stderr)

    def test_prod_requires_server_url(self):
        proc = self._run_make(["client-env-check", "ENV=prod"])
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("CLIENT_PROD_SERVER_URL", proc.stdout + proc.stderr)

    def test_prod_with_configured_url_passes(self):
        proc = self._run_make(
            ["client-env-check", "ENV=prod", "CLIENT_PROD_SERVER_URL=wss://civilization.example/ws"]
        )
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)

    def test_dry_run_stops_before_export_without_prod_url(self):
        proc = self._run_make(["-n", "client-export", "ENV=prod"])
        self.assertNotEqual(proc.returncode, 0)
        self.assertNotIn("--export-release", proc.stdout)

    def test_dry_run_bakes_dev_url(self):
        proc = self._run_make(["-n", "client-export", "ENV=dev"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("wss://civilization.satori.host/ws", proc.stdout)
        self.assertIn("scripts/gen_build_config.py", proc.stdout)

    def test_dry_run_uses_configured_prod_url(self):
        proc = self._run_make(
            ["-n", "client-export", "ENV=prod", "CLIENT_PROD_SERVER_URL=wss://civilization.example/ws"]
        )
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("wss://civilization.example/ws", proc.stdout)
        self.assertIn("--require-server", proc.stdout)


if __name__ == "__main__":
    unittest.main()
