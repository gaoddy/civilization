#!/usr/bin/env python3
"""client_cache_copy.py：新 worktree 播种导入缓存的判定、过滤与回退。"""

import contextlib
import io
import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "client_cache_copy.py"


def run(*args, cwd=None):
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        cwd=cwd or ROOT,
        text=True,
        capture_output=True,
    )


def make_git_repo(base, name="repo"):
    repo = pathlib.Path(base) / name
    repo.mkdir()
    subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
    subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=repo, check=True)
    subprocess.run(["git", "config", "user.name", "Test"], cwd=repo, check=True)
    (repo / "README.md").write_text("test\n")
    subprocess.run(["git", "add", "README.md"], cwd=repo, check=True)
    subprocess.run(["git", "commit", "-qm", "init"], cwd=repo, check=True)
    return repo


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

    def test_sidecar_failure_leaves_no_partial_seed(self):
        sys.path.insert(0, str(ROOT / "scripts"))
        import client_cache_copy as module
        with tempfile.TemporaryDirectory() as tmp:
            main = self.make_checkout(tmp, "main")
            worktree = pathlib.Path(tmp) / "wt"
            (worktree / "client" / "assets").mkdir(parents=True)
            (main / "client" / ".godot" / "imported" / "tex.bin").write_bytes(b"x")
            (main / "client" / "assets" / "a.png").write_bytes(b"png")
            (main / "client" / "assets" / "a.png.import").write_text("sidecar")
            (worktree / "client" / "assets" / "a.png").write_bytes(b"png")

            real_copy_file = module.copy_file

            def failing_copy_file(src, dst, prefer_clone=True):
                raise OSError("模拟旁车复制失败")

            module.copy_file = failing_copy_file
            try:
                code = module.main(["--main", str(main), "--worktree", str(worktree)])
            finally:
                module.copy_file = real_copy_file
            self.assertEqual(code, 1)
            self.assertFalse((worktree / "client" / ".godot" / "imported").exists())

            code = module.main(["--main", str(main), "--worktree", str(worktree)])
            self.assertEqual(code, 0)
            self.assertTrue((worktree / "client" / ".godot" / "imported" / "tex.bin").is_file())
            self.assertTrue((worktree / "client" / "assets" / "a.png.import").is_file())

    def test_auto_detects_main_checkout(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = make_git_repo(tmp)
            imported = repo / "client" / ".godot" / "imported"
            imported.mkdir(parents=True)
            (imported / "tex.bin").write_bytes(b"x")
            result = run(cwd=repo)
            self.assertEqual(result.returncode, 0)
            self.assertIn("当前是主检出，无需复制缓存", result.stdout)

    def test_auto_detects_linked_worktree(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = make_git_repo(tmp)
            imported = repo / "client" / ".godot" / "imported"
            imported.mkdir(parents=True)
            (imported / "tex.bin").write_bytes(b"x")
            worktree = pathlib.Path(tmp) / "wt"
            subprocess.run(
                ["git", "worktree", "add", "--detach", str(worktree), "HEAD"],
                cwd=repo,
                check=True,
                capture_output=True,
            )
            result = run(cwd=worktree)
            self.assertEqual(result.returncode, 0)
            self.assertIn("播种", result.stdout)
            self.assertTrue((worktree / "client" / ".godot" / "imported" / "tex.bin").is_file())

    def test_clone_failure_falls_back(self):
        sys.path.insert(0, str(ROOT / "scripts"))
        import client_cache_copy as module

        with tempfile.TemporaryDirectory() as tmp:
            src = pathlib.Path(tmp) / "src"
            src.mkdir()
            (src / "a.bin").write_bytes(b"a")
            dst = pathlib.Path(tmp) / "dst"
            real_run = module.subprocess.run

            def failing_run(command, *args, **kwargs):
                if isinstance(command, list) and command and command[0] == "cp":
                    return subprocess.CompletedProcess(command, 1, "", "clone failed")
                return real_run(command, *args, **kwargs)

            module.subprocess.run = failing_run
            try:
                module.copy_tree(src, dst)
            finally:
                module.subprocess.run = real_run
            self.assertEqual((dst / "a.bin").read_bytes(), b"a")

    def test_unreadable_source_cache_is_handled(self):
        sys.path.insert(0, str(ROOT / "scripts"))
        import client_cache_copy as module

        with tempfile.TemporaryDirectory() as tmp:
            main = self.make_checkout(tmp, "main")
            worktree = pathlib.Path(tmp) / "wt"
            worktree.mkdir()
            source = (main / "client" / ".godot" / "imported").resolve()
            (source / "tex.bin").write_bytes(b"x")
            real_count = module.count_files

            def failing_count(path):
                if path.resolve() == source:
                    raise PermissionError("denied")
                return real_count(path)

            module.count_files = failing_count
            try:
                stderr = io.StringIO()
                with contextlib.redirect_stderr(stderr):
                    code = module.main(["--main", str(main), "--worktree", str(worktree)])
                self.assertEqual(code, 0)
                self.assertIn("跳过播种", stderr.getvalue())

                stderr = io.StringIO()
                with contextlib.redirect_stderr(stderr):
                    code = module.main(
                        ["--strict", "--main", str(main), "--worktree", str(worktree)]
                    )
                self.assertEqual(code, 1)
                self.assertIn("读取主检出导入缓存失败", stderr.getvalue())
            finally:
                module.count_files = real_count

    def test_non_git_environment_soft_skips(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = run(cwd=pathlib.Path(tmp))
            self.assertEqual(result.returncode, 0)
            self.assertIn("跳过播种", result.stderr)

            strict = run("--strict", cwd=pathlib.Path(tmp))
            self.assertEqual(strict.returncode, 1)

    def test_subdir_invocation_uses_repo_root(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = make_git_repo(tmp)
            (repo / "client" / "assets").mkdir(parents=True)
            (repo / "client" / "assets" / "a.png").write_bytes(b"png")
            main = self.make_checkout(tmp, "main")
            (main / "client" / ".godot" / "imported" / "tex.bin").write_bytes(b"x")
            (main / "client" / "assets" / "a.png").write_bytes(b"png")
            (main / "client" / "assets" / "a.png.import").write_text("sidecar")
            result = run("--main", str(main), cwd=repo / "client")
            self.assertEqual(result.returncode, 0)
            self.assertTrue((repo / "client" / "assets" / "a.png.import").is_file())
            self.assertFalse((repo / "client" / "client").exists())

    def test_quiet_suppresses_noop_messages(self):
        with tempfile.TemporaryDirectory() as tmp:
            main = self.make_checkout(tmp, "main")
            (main / "client" / ".godot" / "imported" / "tex.bin").write_bytes(b"x")

            result = run("--quiet", "--main", str(main), "--worktree", str(main))
            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stdout.strip(), "")

            worktree = pathlib.Path(tmp) / "wt"
            (worktree / "client" / ".godot" / "imported").mkdir(parents=True)
            (worktree / "client" / ".godot" / "imported" / "local.bin").write_bytes(b"y")
            result = run("--quiet", "--main", str(main), "--worktree", str(worktree))
            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stdout.strip(), "")

            seeded = pathlib.Path(tmp) / "wt2"
            seeded.mkdir()
            result = run("--quiet", "--main", str(main), "--worktree", str(seeded))
            self.assertEqual(result.returncode, 0)
            self.assertIn("播种", result.stdout)

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
