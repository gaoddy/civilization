#!/usr/bin/env python3
"""协议文档全局一致性测试：旧名清除、联网架构与协议文档对齐。"""

import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs"


class DocsConsistencyTest(unittest.TestCase):
    def test_no_state_delta_anywhere(self):
        # 只扫描 versions 与 current；本地忽略的 .superpowers/ 过程记录不参与检索。
        offenders = [str(p) for base in (DOCS / "versions", DOCS / "current")
                     for p in base.rglob("*.md") if "StateDelta" in p.read_text()]
        self.assertEqual(offenders, [])

    def test_network_architecture_matches_protocol(self):
        text = (DOCS / "current/technical/architecture/network-architecture.md").read_text()
        for needle in ('"v": 1', '"replyTo"', "StateUpdate", "Snapshot", "不递增"):
            self.assertIn(needle, text)

    def test_protocol_links_from_architecture_index(self):
        text = (DOCS / "current/technical/architecture/README.md").read_text()
        self.assertIn("protocol.md", text)
        self.assertIn("protocol-messages.md", text)


if __name__ == "__main__":
    unittest.main()
