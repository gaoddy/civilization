#!/usr/bin/env python3
"""构建配置生成器测试：空配置、开发地址、正式构建校验与仓库外输出。"""

import contextlib
import io
import os
import pathlib
import sys
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import gen_build_config  # noqa: E402


class GenBuildConfigTest(unittest.TestCase):
    def _run(self, enves, argv=None, output=None):
        """在清空的环境变量下运行生成器，返回退出码、输出与生成文件内容。"""
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        target = pathlib.Path(output or tmp.name) / "build_config.gd"
        stdout = io.StringIO()
        stderr = io.StringIO()
        with mock.patch.dict(os.environ, enves, clear=True), \
                contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            code = gen_build_config.main(["--output", str(target), *(argv or [])])
        text = target.read_text(encoding="utf-8") if target.exists() else ""
        return code, stdout.getvalue() + stderr.getvalue(), text

    def test_empty_config_falls_back_to_local_url(self):
        """编辑器运行与本地调试包不传参数：地址为空，运行期回落本机。"""
        code, _, text = self._run({})
        self.assertEqual(code, 0)
        self.assertIn('const SERVER_URL := ""', text)
        self.assertIn("const STEAM_APP_ID := 0", text)
        self.assertIn('const BUILD_ENV := "local"', text)
        self.assertRegex(text, r"const GIT_DIRTY := (true|false)")

    def test_build_metadata_is_written(self):
        """构建环境与 git 版本写进配置，供产物与日志识别。"""
        code, _, text = self._run(
            {
                "SERVER_URL": "wss://civilization.satori.host/ws",
                "BUILD_ENV": "dev",
                "BUILD_TIME": "2026-11-25T00:00:00Z",
                "GIT_COMMIT": "63a1f2e",
                "GIT_DIRTY": "1",
            }
        )
        self.assertEqual(code, 0)
        self.assertIn('const BUILD_ENV := "dev"', text)
        self.assertIn('const GIT_COMMIT := "63a1f2e"', text)
        self.assertIn("const GIT_DIRTY := true", text)
        self.assertIn('const BUILD_TIME := "2026-11-25T00:00:00Z"', text)

    def test_steam_build_env_is_written(self):
        code, _, text = self._run(
            {
                "SERVER_URL": "wss://civilization.example/ws",
                "STEAM_APP_ID": "480",
                "BUILD_ENV": "steam",
                "GIT_COMMIT": "63a1f2e",
                "GIT_DIRTY": "0",
            },
            ["--require"],
        )
        self.assertEqual(code, 0)
        self.assertIn('const BUILD_ENV := "steam"', text)
        self.assertIn("const GIT_DIRTY := false", text)

    def test_dev_build_writes_configured_url(self):
        code, _, text = self._run({"SERVER_URL": "wss://civilization.satori.host/ws"})
        self.assertEqual(code, 0)
        self.assertIn('const SERVER_URL := "wss://civilization.satori.host/ws"', text)

    def test_require_server_rejects_missing_url(self):
        code, output, text = self._run({}, ["--require-server"])
        self.assertEqual(code, 1)
        self.assertIn("SERVER_URL", output)
        self.assertEqual(text, "")

    def test_require_server_rejects_plain_ws(self):
        code, output, text = self._run(
            {"SERVER_URL": "ws://civilization.satori.host/ws"}, ["--require-server"]
        )
        self.assertEqual(code, 1)
        self.assertIn("wss://", output)
        self.assertEqual(text, "")

    def test_require_server_accepts_wss_without_app_id(self):
        code, _, text = self._run(
            {"SERVER_URL": "wss://civilization.example/ws"}, ["--require-server"]
        )
        self.assertEqual(code, 0)
        self.assertIn('const SERVER_URL := "wss://civilization.example/ws"', text)
        self.assertIn("const STEAM_APP_ID := 0", text)

    def test_steam_require_needs_app_id(self):
        code, output, text = self._run(
            {"SERVER_URL": "wss://civilization.example/ws"}, ["--require"]
        )
        self.assertEqual(code, 1)
        self.assertIn("STEAM_APP_ID", output)
        self.assertEqual(text, "")

    def test_steam_require_writes_app_id(self):
        code, _, text = self._run(
            {"SERVER_URL": "wss://civilization.example/ws", "STEAM_APP_ID": "480"},
            ["--require"],
        )
        self.assertEqual(code, 0)
        self.assertIn("const STEAM_APP_ID := 480", text)

    def test_invalid_app_id_is_rejected(self):
        code, output, _ = self._run(
            {"SERVER_URL": "wss://civilization.example/ws", "STEAM_APP_ID": "abc"},
            ["--require"],
        )
        self.assertEqual(code, 1)
        self.assertIn("STEAM_APP_ID", output)

    def test_output_outside_repo_does_not_crash(self):
        """测试常写到临时目录，打印相对路径失败不能影响生成结果。"""
        code, output, text = self._run({"SERVER_URL": "wss://civilization.example/ws"})
        self.assertEqual(code, 0)
        self.assertIn("已生成", output)
        self.assertNotEqual(text, "")


if __name__ == "__main__":
    unittest.main()
