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
    "BUILD_ENV",
    "BUILD_TIME",
    "GIT_COMMIT",
    "GIT_DIRTY",
    "PLATFORM",
    "DEVICE",
    "WIDTH",
    "HEIGHT",
    "LOCAL_SERVER",
    "AAB",
    "IOS_DEVICE",
    "IOS_DEVICE_TYPE",
    "IOS_LAUNCH",
)
DEV_URL = "wss://civilization.satori.host/ws"
# 固定构建时间戳：命令行变量覆盖 make/client.mk 的时间戳赋值，让跨两次 make
# 的整行比较不受时钟影响。
BUILD_TIME_PIN = "CLIENT_BUILD_TIME=2026-01-01T00:00:00Z"


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
        # 构建时间戳取生成时刻（make/client.mk 的 CLIENT_BUILD_TIME），两次 make 之间
        # 跨过秒边界就会得到不同的 BUILD_TIME。这里比的是 ENV/env 展开是否等价，
        # 把时间戳钉死才有确定性。
        upper = self._run_make(["-n", "client-export", "ENV=dev", BUILD_TIME_PIN])
        lower = self._run_make(["-n", "client-export", "env=dev", BUILD_TIME_PIN])
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

    def test_run_ios_rejects_unknown_device_type(self):
        proc = self._run_make(["-n", "client-run", "PLATFORM=ios", "IOS_DEVICE_TYPE=bogus"])
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("IOS_DEVICE_TYPE 只能是", proc.stdout + proc.stderr)

    def test_ios_device_env_alias_selects_device_and_env(self):
        proc = self._run_make(["-n", "run-ios-ipad-dev"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        line = self._gen_line(proc)
        self.assertIn(DEV_URL, line)
        self.assertIn('--require-url', line)
        self.assertIn('IOS_DEVICE_TYPE="ipad"', proc.stdout)

    def test_ios_device_env_alias_local_keeps_empty_config(self):
        proc = self._run_make(["-n", "run-ios-iphone-local"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn('SERVER_URL=""', self._gen_line(proc))
        self.assertIn('IOS_DEVICE_TYPE="iphone"', proc.stdout)

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

    def test_local_bakes_empty_url_with_metadata(self):
        """local 调试包仍写空地址，但会写入构建环境与 git 版本供产物识别。"""
        proc = self._run_make(["-n", "client-export", "PLATFORM=local"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        line = self._gen_line(proc)
        self.assertIn('SERVER_URL=""', line)
        self.assertIn('BUILD_ENV="local"', line)
        self.assertNotIn("--require", line)

    def test_exports_name_platform_in_artifact_path(self):
        """产物统一在 output/client/，文件名带平台；复用判定写入戳记目录。"""
        cases = (
            ("macos", [], "Civilization-macos-$id.app"),
            ("ios", [], "Civilization-ios-$id.ipa"),
            ("android", [], "Civilization-android-$id.$format"),
            ("android", ["AAB=1"], "Civilization-android-$id.$format"),
        )
        for platform, extra, artifact in cases:
            with self.subTest(platform=platform, extra=extra):
                proc = self._run_make(["-n", "client-export", f"PLATFORM={platform}", *extra])
                self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
                self.assertIn("output/client/" + artifact, proc.stdout)
                self.assertIn("output/client/stamps/", proc.stdout)
                self.assertIn("scripts/client_build.py reuse", proc.stdout)

    def test_run_ios_install_uses_build_directory(self):
        """真机安装路径与导出的构建号目录一致，导出被复用时直接安装已有产物。"""
        proc = self._run_make(["-n", "run-ios-ipad-dev"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("output/client/build/ios/$id", proc.stdout)
        self.assertIn('IOS_APP=', proc.stdout)

    def test_android_aab_uses_aab_script(self):
        proc = self._run_make(["-n", "client-export", "PLATFORM=android", "AAB=1"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn('format="aab"', proc.stdout)
        self.assertIn('android_export.sh "$format"', proc.stdout)

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
        self.assertIn('format="aab"', proc.stdout)
        self.assertIn('android_export.sh "$format"', proc.stdout)

    def test_alias_export_ios_passes_env(self):
        proc = self._run_make(["-n", "export-ios", "ENV=dev"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        line = self._gen_line(proc)
        self.assertIn(DEV_URL, line)
        self.assertIn("--require-url", line)

    def test_alias_export_ipad_routes_to_ios_preset(self):
        cases = (
            ("local", []),
            ("dev", []),
            ("prod", ["CLIENT_PROD_SERVER_URL=wss://civilization.example/ws"]),
        )
        for env_name, extra in cases:
            with self.subTest(env=env_name):
                proc = self._run_make(["-n", f"export-ipad-{env_name}", *extra])
                self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
                self.assertIn('--export-release "iOS"', proc.stdout)

    def test_alias_platform_overrides_conflicting_argument(self):
        proc = self._run_make(["-n", "export-ios", "PLATFORM=bogus"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn('--export-release "iOS"', proc.stdout)

    def test_env_alias_dev_uses_dev_url(self):
        for platform in ("macos", "ios", "android"):
            with self.subTest(platform=platform):
                proc = self._run_make(["-n", f"export-{platform}-dev"])
                self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
                line = self._gen_line(proc)
                self.assertIn(DEV_URL, line)
                self.assertIn("--require-url", line)

    def test_env_alias_local_uses_empty_config(self):
        for platform in ("macos", "ios", "android"):
            with self.subTest(platform=platform):
                proc = self._run_make(["-n", f"export-{platform}-local"])
                self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
                line = self._gen_line(proc)
                self.assertIn('SERVER_URL=""', line)
                self.assertNotIn("--require", line)

    def test_env_alias_prod_requires_url(self):
        for platform in ("macos", "ios", "android"):
            with self.subTest(platform=platform):
                proc = self._run_make(["-n", f"export-{platform}-prod"])
                self.assertNotEqual(proc.returncode, 0)
                self.assertIn("CLIENT_PROD_SERVER_URL", proc.stdout + proc.stderr)

    def test_env_alias_prod_with_url(self):
        proc = self._run_make(
            [
                "-n",
                "export-android-prod",
                "CLIENT_PROD_SERVER_URL=wss://civilization.example/ws",
            ]
        )
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        line = self._gen_line(proc)
        self.assertIn("wss://civilization.example/ws", line)
        self.assertIn("--require-server", line)

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
