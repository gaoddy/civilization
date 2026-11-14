#!/usr/bin/env python3
"""Make 入口测试：目标存在、protocol-check 只读且通过。"""

import pathlib
import subprocess
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]


class MakeTargetsTest(unittest.TestCase):
    def test_protocol_targets_exist(self):
        out = subprocess.run(["make", "-qp"], cwd=ROOT, capture_output=True, text=True).stdout
        for target in ("test:", "protocol:", "protocol-check:"):
            self.assertIn(target, out)

    def test_protocol_check_is_clean_and_read_only(self):
        before = subprocess.run(["git", "status", "--porcelain"], cwd=ROOT,
                                capture_output=True, text=True).stdout
        result = subprocess.run(["make", "protocol-check"], cwd=ROOT, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        after = subprocess.run(["git", "status", "--porcelain"], cwd=ROOT,
                               capture_output=True, text=True).stdout
        self.assertEqual(before, after)


if __name__ == "__main__":
    unittest.main()
