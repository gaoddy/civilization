#!/usr/bin/env python3
"""客户端构建环境测试：local/dev/prod 三档、`ENV` 与 `env` 等价、地址写入规则。"""

import os
import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
CLEARED = (
    "ENV",
    "env",
    "SERVER_URL",
    "CLIENT_DEV_SERVER_URL",
    "CLIENT_PROD_SERVER_URL",
    "STEAM_APP_ID",
    "PLATFORM",
    "DEVICE",
    "WIDTH",
    "HEIGHT",
    "LOCAL_SERVER",
    "AAB",
    "IOS_DEVICE",
    "IOS_LAUNCH",
)
DEV_URL = "wss://civilization.satori.host/ws"


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

    def _gen_line(self, proc):
        """取干跑输出中生成构建配置的那一行。"""
        lines = [line for line in proc.stdout.splitlines() if "gen_build_config.py" in line]
        self.assertTrue(lines, proc.stdout + proc.stderr)
        return lines[0]

    def test_default_env_is_local(self):
        proc = self._run_make(["client-env-check"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)

    def test_unknown_env_is_rejected(self):
        for name in ("ENV", "env"):
            with self.subTest(name=name):
                proc = self._run_make(["client-env-check", f"{name}=production"])
                self.assertNotEqual(proc.returncode, 0)
                self.assertIn("local / dev / prod", proc.stdout + proc.stderr)

    def test_dev_requires_server_url(self):
        proc = self._run_make(["client-env-check", "ENV=dev", "CLIENT_DEV_SERVER_URL="])
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("CLIENT_DEV_SERVER_URL", proc.stdout + proc.stderr)

    def test_prod_requires_server_url(self):
        proc = self._run_make(["client-env-check", "ENV=prod"])
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("CLIENT_PROD_SERVER_URL", proc.stdout + proc.stderr)

    def test_prod_with_configured_url_passes(self):
        proc = self._run_make(
            ["client-env-check", "ENV=prod", "CLIENT_PROD_SERVER_URL=wss://civilization.example/ws"]
        )
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)

    def test_conflicting_env_values_are_rejected(self):
        proc = self._run_make(["-n", "client-export", "ENV=dev", "env=prod"])
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("ENV 与 env", proc.stdout + proc.stderr)

    def test_dry_run_local_bakes_empty_config(self):
        proc = self._run_make(["-n", "client-export"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        line = self._gen_line(proc)
        self.assertIn('SERVER_URL=""', line)
        self.assertNotIn("--require-server", line)

    def test_dry_run_bakes_dev_url(self):
        proc = self._run_make(["-n", "client-export", "ENV=dev"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        line = self._gen_line(proc)
        self.assertIn(DEV_URL, line)
        self.assertIn("--require-url", line)

    def test_lowercase_env_matches_uppercase(self):
        upper = self._run_make(["-n", "client-export", "ENV=dev"])
        lower = self._run_make(["-n", "client-export", "env=dev"])
        self.assertEqual(upper.returncode, 0, upper.stdout + upper.stderr)
        self.assertEqual(lower.returncode, 0, lower.stdout + lower.stderr)
        self.assertEqual(self._gen_line(upper), self._gen_line(lower))

    def test_same_value_in_both_cases_passes(self):
        proc = self._run_make(["-n", "client-export", "ENV=dev", "env=dev"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn(DEV_URL, self._gen_line(proc))

    def test_dry_run_uses_configured_prod_url(self):
        proc = self._run_make(
            ["-n", "client-export", "ENV=prod", "CLIENT_PROD_SERVER_URL=wss://civilization.example/ws"]
        )
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        line = self._gen_line(proc)
        self.assertIn("wss://civilization.example/ws", line)
        self.assertIn("--require-server", line)

    def test_ios_dry_run_uses_dev_url(self):
        proc = self._run_make(["-n", "client-export", "PLATFORM=ios", "ENV=dev"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        line = self._gen_line(proc)
        self.assertIn(DEV_URL, line)
        self.assertNotIn("cooter.local", line)

    def test_run_ios_defaults_to_local(self):
        proc = self._run_make(["-n", "client-run", "PLATFORM=ios"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        line = self._gen_line(proc)
        self.assertIn('SERVER_URL=""', line)
        self.assertNotIn("cooter.local", line)

    def test_android_dry_run_bakes_dev_url(self):
        proc = self._run_make(["-n", "client-export", "PLATFORM=android", "ENV=dev"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        line = self._gen_line(proc)
        self.assertIn(DEV_URL, line)
        self.assertIn("--require-url", line)

    def test_unknown_platform_is_rejected(self):
        proc = self._run_make(["-n", "client-export", "PLATFORM=bogus"])
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("PLATFORM 只能是", proc.stdout + proc.stderr)

    def test_local_and_steam_reject_env(self):
        for platform in ("local", "steam"):
            with self.subTest(platform=platform):
                proc = self._run_make(["-n", "client-export", f"PLATFORM={platform}", "ENV=dev"])
                self.assertNotEqual(proc.returncode, 0)
                self.assertIn("不支持 ENV", proc.stdout + proc.stderr)

    def test_local_does_not_bake_build_config(self):
        proc = self._run_make(["-n", "client-export", "PLATFORM=local"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertNotIn("gen_build_config.py", proc.stdout + proc.stderr)

    def test_android_aab_uses_aab_script(self):
        proc = self._run_make(["-n", "client-export", "PLATFORM=android", "AAB=1"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("android_export.sh aab", proc.stdout)

    def test_android_invalid_aab_is_rejected(self):
        proc = self._run_make(["-n", "client-export", "PLATFORM=android", "AAB=2"])
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("AAB 只能是 0 或 1", proc.stdout + proc.stderr)

    def test_run_iphone_uses_preset_size(self):
        proc = self._run_make(["-n", "client-run", "DEVICE=iphone"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn('CIVILIZATION_WINDOW_SIZE="2622x1206"', proc.stdout)
        self.assertIn('CIVILIZATION_DEVICE="phone"', proc.stdout)
        self.assertIn('CIVILIZATION_LOCAL_SERVER="1"', proc.stdout)

    def test_run_without_device_keeps_server_off(self):
        proc = self._run_make(["-n", "client-run"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn('CIVILIZATION_LOCAL_SERVER="0"', proc.stdout)
        self.assertNotIn("cargo build -q -p room-service", proc.stdout)

    def test_run_unknown_device_is_rejected(self):
        proc = self._run_make(["-n", "client-run", "DEVICE=bogus"])
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("DEVICE 只能是", proc.stdout + proc.stderr)

    def test_run_requires_both_width_and_height(self):
        proc = self._run_make(["-n", "client-run", "WIDTH=960"])
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("WIDTH 与 HEIGHT 需同时传入正整数", proc.stdout + proc.stderr)

    def test_alias_run_iphone_matches_device_preset(self):
        proc = self._run_make(["-n", "run-iphone"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn('CIVILIZATION_WINDOW_SIZE="2622x1206"', proc.stdout)
        self.assertIn('CIVILIZATION_DEVICE="phone"', proc.stdout)
        self.assertIn('CIVILIZATION_LOCAL_SERVER="1"', proc.stdout)

    def test_alias_export_aab_uses_aab_script(self):
        proc = self._run_make(["-n", "export-aab"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("android_export.sh aab", proc.stdout)

    def test_alias_export_ios_passes_env(self):
        proc = self._run_make(["-n", "export-ios", "ENV=dev"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        line = self._gen_line(proc)
        self.assertIn(DEV_URL, line)
        self.assertIn("--require-url", line)

    def test_alias_platform_overrides_conflicting_argument(self):
        proc = self._run_make(["-n", "export-ios", "PLATFORM=bogus"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn('--export-release "iOS"', proc.stdout)

    def test_generator_dev_allows_ws_url(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = pathlib.Path(tmp) / "build_config.gd"
            proc = subprocess.run(
                [sys.executable, "scripts/gen_build_config.py", "--require-url", "--output", str(output)],
                cwd=ROOT,
                env={**os.environ, "SERVER_URL": "ws://127.0.0.1:8081/ws", "STEAM_APP_ID": ""},
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
            self.assertIn("ws://127.0.0.1:8081/ws", output.read_text(encoding="utf-8"))

    def test_generator_prod_rejects_ws_url(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = pathlib.Path(tmp) / "build_config.gd"
            proc = subprocess.run(
                [sys.executable, "scripts/gen_build_config.py", "--require-server", "--output", str(output)],
                cwd=ROOT,
                env={**os.environ, "SERVER_URL": "ws://127.0.0.1:8081/ws", "STEAM_APP_ID": ""},
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(proc.returncode, 0)
            self.assertIn("wss://", proc.stdout + proc.stderr)


if __name__ == "__main__":
    unittest.main()
