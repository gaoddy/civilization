#!/usr/bin/env python3
"""客户端构建号与产物复用测试：dirty 标记、指纹变化、戳记比对。"""

import contextlib
import io
import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import client_build  # noqa: E402


class ClientBuildTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = pathlib.Path(self.tmp.name)
        (self.root / "client").mkdir()
        (self.root / "client" / "app.txt").write_text("v1", encoding="utf-8")
        self._git("init", "-q")
        self._git("add", "client")
        self._git(
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-q",
            "-m",
            "init",
        )

    def _git(self, *args):
        subprocess.run(["git", *args], cwd=self.root, check=True, capture_output=True, text=True)

    def _run(self, argv):
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr), contextlib.redirect_stdout(io.StringIO()):
            code = client_build.main([*argv, "--root", str(self.root)])
        return code, stderr.getvalue()

    def _identity(self, env="dev", fmt=""):
        return client_build.build_id(self.root, env, ["client"], fmt)

    def _artifact(self, name="app.bin", content="artifact"):
        target = self.root / "out" / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
        return str(target.relative_to(self.root))

    def _stamp_args(self, stamp, identity, artifact, server_url="", godot_version="4.7.2"):
        return [
            "--stamp",
            str(stamp),
            "--platform",
            "ios",
            "--env",
            "dev",
            "--id",
            identity,
            "--watch",
            "client",
            "--artifact",
            artifact,
            "--server-url",
            server_url,
            "--godot-version",
            godot_version,
        ]

    def test_clean_id_has_commit_and_env(self):
        identity = self._identity()
        commit = client_build.git_value(["rev-parse", "--short", "HEAD"], self.root)
        self.assertTrue(identity.startswith(commit))
        self.assertTrue(identity.endswith("-dev"))
        self.assertNotIn("dirty", identity)

    def test_dirty_id_and_fingerprint_track_edits(self):
        clean_id = self._identity()
        _, clean_dirty = client_build.fingerprint(self.root, ["client"])
        self.assertFalse(clean_dirty)

        (self.root / "client" / "app.txt").write_text("v2", encoding="utf-8")
        dirty_id = self._identity()
        digest, dirty = client_build.fingerprint(self.root, ["client"])
        self.assertTrue(dirty)
        self.assertIn("dirty", dirty_id)
        self.assertNotEqual(dirty_id, clean_id)
        self.assertTrue(digest)

    def test_dirty_fingerprint_changes_after_second_edit(self):
        (self.root / "client" / "app.txt").write_text("v2", encoding="utf-8")
        first = client_build.fingerprint(self.root, ["client"])[0]
        (self.root / "client" / "app.txt").write_text("v3", encoding="utf-8")
        second = client_build.fingerprint(self.root, ["client"])[0]
        self.assertNotEqual(first, second)

    def test_aab_build_id_has_suffix(self):
        identity = self._identity(fmt="aab")
        self.assertTrue(identity.endswith("-aab"))
        self.assertTrue(identity.endswith("-dev-aab"))

    def test_reuse_accepts_matching_stamp(self):
        identity = self._identity()
        artifact = self._artifact()
        stamp = self.root / "output" / "stamps" / "ios.json"
        args = self._stamp_args(stamp, identity, artifact)
        self.assertEqual(self._run(["stamp", *args])[0], 0)
        self.assertEqual(self._run(["reuse", *args])[0], 0)

    def test_reuse_rejects_missing_artifact(self):
        identity = self._identity()
        artifact = self._artifact()
        stamp = self.root / "output" / "stamps" / "ios.json"
        args = self._stamp_args(stamp, identity, artifact)
        self.assertEqual(self._run(["stamp", *args])[0], 0)
        (self.root / artifact).unlink()
        code, stderr = self._run(["reuse", *args])
        self.assertEqual(code, 1)
        self.assertIn("产物缺失", stderr)

    def test_reuse_rejects_changed_source_even_when_dirty(self):
        (self.root / "client" / "app.txt").write_text("v2", encoding="utf-8")
        identity = self._identity()
        artifact = self._artifact()
        stamp = self.root / "output" / "stamps" / "ios.json"
        args = self._stamp_args(stamp, identity, artifact)
        self.assertEqual(self._run(["stamp", *args])[0], 0)
        self.assertEqual(self._run(["reuse", *args])[0], 0)

        (self.root / "client" / "app.txt").write_text("v3", encoding="utf-8")
        code, stderr = self._run(["reuse", *args])
        self.assertEqual(code, 1)
        self.assertIn("fingerprint", stderr)

    def test_reuse_rejects_changed_config(self):
        identity = self._identity()
        artifact = self._artifact()
        stamp = self.root / "output" / "stamps" / "ios.json"
        self.assertEqual(self._run(["stamp", *self._stamp_args(stamp, identity, artifact)])[0], 0)
        code, stderr = self._run(
            ["reuse", *self._stamp_args(stamp, identity, artifact, server_url="wss://other.example/ws")]
        )
        self.assertEqual(code, 1)
        self.assertIn("server_url", stderr)

    def test_reuse_rejects_missing_stamp(self):
        code, stderr = self._run(
            ["reuse", *self._stamp_args(self.root / "output" / "stamps" / "none.json", self._identity(), self._artifact())]
        )
        self.assertEqual(code, 1)
        self.assertIn("缺少戳记", stderr)


if __name__ == "__main__":
    unittest.main()
