#!/usr/bin/env python3
"""协议字段表生成器测试：确定性、类型与字段覆盖、--check。"""

import pathlib
import subprocess
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import gen_protocol_docs as gen  # noqa: E402


class GenDocsTest(unittest.TestCase):
    def test_render_contains_every_type_and_required_fields(self):
        text = gen.render(ROOT / "protocol/schema/v1", gen.load_mapping())
        self.assertEqual(text, gen.render(ROOT / "protocol/schema/v1", gen.load_mapping()))
        for name in ("AuthRequest", "StateUpdate", "CombatResult", "Error"):
            self.assertIn(f"### {name}", text)
        self.assertIn("| 字段 | 类型 | 必填 | 说明 |", text)
        self.assertIn("clientIntentId", text)

    def test_check_mode_passes_on_committed_file(self):
        result = subprocess.run([sys.executable, str(ROOT / "scripts/gen_protocol_docs.py"), "--check"],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
