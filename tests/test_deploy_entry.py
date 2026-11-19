#!/usr/bin/env python3
"""发布入口断言：渲染输出只含清单，`ENV` 只接受 dev/prod。"""

import os
import pathlib
import stat
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]

MANIFEST = """---
apiVersion: v1
kind: ConfigMap
metadata:
  name: civilization-server-config
data:
  listen-addr: 0.0.0.0:8080
"""

HELM_STUB = f"""#!/bin/sh
cat <<'EOF'
{MANIFEST}EOF
"""

SSH_STUB = """#!/bin/sh
: > "$SSH_MARKER"
cat > "$SSH_STDIN"
"""


class DeployEntryTest(unittest.TestCase):
    def _run_make(self, args, env_extra=None):
        """在把 helm/ssh 替换成桩的 PATH 下运行 make，返回进程、ssh 标记与 stdin 留档。"""
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        stub_dir = pathlib.Path(tmp.name)
        for name, body in (("helm", HELM_STUB), ("ssh", SSH_STUB)):
            path = stub_dir / name
            path.write_text(body, encoding="utf-8")
            path.chmod(path.stat().st_mode | stat.S_IEXEC)
        marker = stub_dir / "ssh-called"
        stdin = stub_dir / "ssh-stdin"
        stdin.touch()
        env = dict(os.environ)
        env["PATH"] = f"{stub_dir}{os.pathsep}{env.get('PATH', '')}"
        env["SSH_MARKER"] = str(marker)
        env["SSH_STDIN"] = str(stdin)
        env.update(env_extra or {})
        proc = subprocess.run(
            ["make", *args],
            cwd=ROOT,
            env=env,
            capture_output=True,
            text=True,
            check=False,
            stdin=subprocess.DEVNULL,
        )
        return proc, marker, stdin

    def test_render_output_contains_only_manifests(self):
        """渲染流的第一行必须是清单分隔符，不能混入命令回显。"""
        proc, _, _ = self._run_make(["deploy-render", "ENV=dev"])
        self.assertEqual(proc.returncode, 0, proc.stderr)
        first = proc.stdout.splitlines()[0]
        self.assertEqual(first, "---", proc.stdout[:200])
        self.assertNotIn("helm template", proc.stdout)
        self.assertNotIn("Entering directory", proc.stdout)

    def test_deploy_apply_feeds_manifests_to_kubectl(self):
        """apply 通过管道喂给 kubectl 的内容同样只含清单。"""
        proc, marker, stdin = self._run_make(["deploy-apply", "ENV=dev", "SERVER=stub"])
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertTrue(marker.exists(), "ssh 未被调用")
        fed = stdin.read_text(encoding="utf-8")
        self.assertEqual(fed.splitlines()[0], "---", fed[:200])
        self.assertNotIn("helm template", fed)

    def test_unknown_env_is_rejected(self):
        """`ENV` 非 dev/prod 时直接报错，不能静默落到 dev。"""
        proc, marker, _ = self._run_make(
            ["deploy-rollout", "ENV=prodution", "SERVER=stub"]
        )
        self.assertNotEqual(proc.returncode, 0, proc.stdout)
        self.assertFalse(marker.exists(), "非法 ENV 仍执行了远端命令")
        self.assertIn("ENV", proc.stdout + proc.stderr)


if __name__ == "__main__":
    unittest.main()
