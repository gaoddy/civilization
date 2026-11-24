#!/usr/bin/env python3
"""client_cache_copy.py：新 worktree 播种导入缓存的判定、过滤与回退。"""

import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "client_cache_copy.py"


def run(*args):
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        cwd=ROOT,
        text=True,
        capture_output=True,
    )


class ClientCacheCopyTest(unittest.TestCase):
    def make_checkout(self, base, name):
        root = pathlib.Path(base) / name
        (root / "client" / ".godot" / "imported").mkdir(parents=True)
        (root / "client" / "assets").mkdir(parents=True)
        return root

    def test_main_checkout_is_noop(self):
        with tempfile.TemporaryDirectory() as tmp:
            main = self.make_checkout(tmp, "main")
            (main / "client" / ".godot" / "imported" / "tex.bin").write_bytes(b"x")
            result = run("--main", str(main), "--worktree", str(main))
            self.assertEqual(result.returncode, 0)
            self.assertIn("当前是主检出，无需复制缓存", result.stdout)

    def test_worktree_seeds_imported_and_existing_source_sidecars(self):
        with tempfile.TemporaryDirectory() as tmp:
            main = self.make_checkout(tmp, "main")
            worktree = pathlib.Path(tmp) / "wt"
            (worktree / "client" / "assets").mkdir(parents=True)
            (main / "client" / ".godot" / "imported" / "tex.bin").write_bytes(b"x")
            (main / "client" / "assets" / "a.png").write_bytes(b"png")
            (main / "client" / "assets" / "a.png.import").write_text("sidecar")
            (main / "client" / "assets" / "gone.png.import").write_text("orphan")
            (worktree / "client" / "assets" / "a.png").write_bytes(b"png")
            result = run("--main", str(main), "--worktree", str(worktree))
            self.assertEqual(result.returncode, 0)
            self.assertTrue((worktree / "client" / ".godot" / "imported" / "tex.bin").is_file())
            self.assertTrue((worktree / "client" / "assets" / "a.png.import").is_file())
            self.assertFalse((worktree / "client" / "assets" / "gone.png.import").exists())
            self.assertIn("1 个 .import 旁车", result.stdout)
            self.assertIn("跳过 1 个无源旁车", result.stdout)

    def test_existing_cache_is_skipped(self):
        with tempfile.TemporaryDirectory() as tmp:
            main = self.make_checkout(tmp, "main")
            worktree = pathlib.Path(tmp) / "wt"
            (worktree / "client" / ".godot" / "imported").mkdir(parents=True)
            (worktree / "client" / ".godot" / "imported" / "local.bin").write_bytes(b"y")
            (main / "client" / ".godot" / "imported" / "tex.bin").write_bytes(b"x")
            result = run("--main", str(main), "--worktree", str(worktree))
            self.assertEqual(result.returncode, 0)
            self.assertIn("已有导入缓存，跳过播种", result.stdout)
            self.assertFalse((worktree / "client" / ".godot" / "imported" / "tex.bin").exists())

    def test_strict_fails_without_main_cache(self):
        with tempfile.TemporaryDirectory() as tmp:
            main = pathlib.Path(tmp) / "main"
            (main / "client").mkdir(parents=True)
            worktree = pathlib.Path(tmp) / "wt"
            worktree.mkdir()
            result = run("--strict", "--main", str(main), "--worktree", str(worktree))
            self.assertEqual(result.returncode, 1)
            self.assertIn("请先执行", result.stderr)

    def test_soft_skips_without_main_cache(self):
        with tempfile.TemporaryDirectory() as tmp:
            main = pathlib.Path(tmp) / "main"
            (main / "client").mkdir(parents=True)
            worktree = pathlib.Path(tmp) / "wt"
            worktree.mkdir()
            result = run("--main", str(main), "--worktree", str(worktree))
            self.assertEqual(result.returncode, 0)
            self.assertIn("跳过播种", result.stderr)

    def test_interrupted_seed_is_replaced(self):
        with tempfile.TemporaryDirectory() as tmp:
            main = self.make_checkout(tmp, "main")
            worktree = pathlib.Path(tmp) / "wt"
            (worktree / "client" / ".godot" / "imported.seed").mkdir(parents=True)
            (worktree / "client" / ".godot" / "imported.seed" / "junk.bin").write_bytes(b"junk")
            (main / "client" / ".godot" / "imported" / "tex.bin").write_bytes(b"x")
            result = run("--main", str(main), "--worktree", str(worktree))
            self.assertEqual(result.returncode, 0)
            self.assertTrue((worktree / "client" / ".godot" / "imported" / "tex.bin").is_file())
            self.assertFalse((worktree / "client" / ".godot" / "imported.seed").exists())

    def test_copy_tree_falls_back_without_clone(self):
        sys.path.insert(0, str(ROOT / "scripts"))
        import client_cache_copy as module
        with tempfile.TemporaryDirectory() as tmp:
            src = pathlib.Path(tmp) / "src"
            (src / "nested").mkdir(parents=True)
            (src / "a.bin").write_bytes(b"a")
            (src / "nested" / "b.bin").write_bytes(b"b")
            dst = pathlib.Path(tmp) / "dst"
            module.copy_tree(src, dst, prefer_clone=False)
            self.assertEqual((dst / "a.bin").read_bytes(), b"a")
            self.assertEqual((dst / "nested" / "b.bin").read_bytes(), b"b")
