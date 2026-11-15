#!/usr/bin/env python3
"""客户端边界断言：不引用规则实现，只通过服务端视图驱动界面。"""

import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
CLIENT = ROOT / "client"
FORBIDDEN = re.compile(r"rule[-_]engine|resolve_combat|start_game|cargo|server/data")


class ClientBoundaryTest(unittest.TestCase):
    def test_client_exists(self):
        self.assertTrue((CLIENT / "project.godot").is_file())

    def test_autoloads_declared(self):
        project = (CLIENT / "project.godot").read_text(encoding="utf-8")
        for name in ("GameStore", "Net", "CardDB"):
            self.assertRegex(project, rf"(?m)^{name}=\"")

    def test_no_rule_engine_reference(self):
        offenders = []
        for path in sorted(CLIENT.rglob("*.gd")):
            for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                if FORBIDDEN.search(line):
                    offenders.append(f"{path.relative_to(ROOT)}:{number}: {line.strip()}")
        self.assertEqual(offenders, [], "\n".join(offenders))


if __name__ == "__main__":
    unittest.main()
