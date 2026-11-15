#!/usr/bin/env python3
"""GDScript 协议常量生成器测试：确定性、常量内容、--check 漂移检测。"""

import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import gen_protocol_gdscript as gen  # noqa: E402


class GenGdscriptTest(unittest.TestCase):
    def test_render_is_deterministic_and_contains_constants(self):
        first = gen.render(ROOT / "protocol/schema/v1", gen.load_mapping())
        self.assertEqual(first, gen.render(ROOT / "protocol/schema/v1", gen.load_mapping()))
        self.assertIn("const PROTOCOL_VERSION := 1", first)
        for expected in ('const MSG_AUTH_REQUEST := "AuthRequest"',
                         'const MSG_STATE_UPDATE := "StateUpdate"',
                         'const INTENT_BUY_MINION := "BuyMinion"',
                         'const ERROR_NOT_ENOUGH_GOLD := "NOT_ENOUGH_GOLD"',
                         'const PHASE_RECRUIT := "Recruit"',
                         'const EVENT_TRIPLE_FORMED := "TripleFormed"'):
            self.assertIn(expected, first)

    def test_check_mode_detects_drift(self):
        result = subprocess.run([sys.executable, str(ROOT / "scripts/gen_protocol_gdscript.py"), "--check"],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_client_output_matches_default_output(self):
        content = gen.render(ROOT / "protocol/schema/v1", gen.load_mapping())
        self.assertEqual(gen.CLIENT_OUTPUT, ROOT / "client" / "generated" / "protocol.gd")
        self.assertEqual(gen.CLIENT_OUTPUT.read_text(encoding="utf-8"), content)
        self.assertEqual(gen.CLIENT_OUTPUT.read_text(encoding="utf-8"),
                         gen.DEFAULT_OUTPUT.read_text(encoding="utf-8"))

    def test_explicit_output_writes_only_that_target(self):
        custom = pathlib.Path("/tmp/custom-protocol.gd")
        self.assertEqual(gen.output_targets(custom), [custom])
        self.assertEqual(gen.output_targets(custom, pathlib.Path("/tmp/client.gd")),
                         [custom, pathlib.Path("/tmp/client.gd")])
        self.assertEqual(gen.output_targets(), [gen.DEFAULT_OUTPUT, gen.CLIENT_OUTPUT])
        self.assertEqual(gen.output_targets(client_output=pathlib.Path("/tmp/client.gd")),
                         [gen.DEFAULT_OUTPUT, pathlib.Path("/tmp/client.gd")])

    def test_check_mode_fails_on_different_content(self):
        with tempfile.TemporaryDirectory() as tmp:
            target = pathlib.Path(tmp) / "protocol.gd"
            target.write_text("class_name Protocol\n")
            result = subprocess.run([sys.executable, str(ROOT / "scripts/gen_protocol_gdscript.py"),
                                     "--check", "--output", str(target)],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 1)


if __name__ == "__main__":
    unittest.main()
