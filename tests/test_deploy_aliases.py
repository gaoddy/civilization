#!/usr/bin/env python3
"""部署快捷别名测试：`deploy-<环境>` 与 `deploy-<步骤>-<环境>` 的环境透传。"""

import os
import pathlib
import subprocess
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
CLEARED = ("ENV", "env", "SERVER")


class DeployAliasTest(unittest.TestCase):
    def _run_make(self, args):
        """清空部署相关环境变量后运行 make，避免父 shell 配置干扰断言。"""
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

    def test_status_alias_uses_prod_namespace(self):
        proc = self._run_make(["-n", "deploy-status-prod"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("-n civilization ", proc.stdout)
        self.assertNotIn("civilization-dev", proc.stdout)

    def test_status_alias_uses_dev_namespace(self):
        proc = self._run_make(["-n", "deploy-status-dev"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("-n civilization-dev ", proc.stdout)

    def test_image_alias_uses_env_tag(self):
        proc = self._run_make(["-n", "deploy-image-prod"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("civilization-server:prod", proc.stdout)

    def test_render_alias_uses_env_values(self):
        proc = self._run_make(["-n", "deploy-render-dev"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("values.dev.yaml", proc.stdout)
        self.assertIn("--namespace civilization-dev", proc.stdout)

    def test_alias_passes_server_override(self):
        proc = self._run_make(["-n", "deploy-status-prod", "SERVER=root@example"])
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
        self.assertIn("ssh root@example", proc.stdout)

    def test_full_alias_maps_to_deploy(self):
        for env in ("dev", "prod"):
            with self.subTest(env=env):
                proc = self._run_make(["-n", f"deploy-{env}", "MAKE=/usr/bin/true"])
                self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)
                self.assertIn(f"deploy ENV={env}", proc.stdout)

    def test_env_agnostic_step_has_no_alias(self):
        proc = self._run_make(["-n", "deploy-lint-dev"])
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("No rule to make target", proc.stdout + proc.stderr)


if __name__ == "__main__":
    unittest.main()
