#!/usr/bin/env python3
"""validate_docs.py 的卡牌出处分节校验单元测试（标准库 unittest）。

运行：python3 -m unittest discover -s tests -v
"""

import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / "scripts"))

import validate_docs as vd  # noqa: E402


class SourceSectionTest(unittest.TestCase):
    def test_missing_section_returns_none(self):
        self.assertIsNone(vd.source_section("# 卡\n\n## 效果\n\n无\n"))

    def test_section_returns_body(self):
        text = "# 卡\n\n## 效果\n\n无\n\n## 出处\n\n史实考据。\n"
        self.assertEqual(vd.source_section(text), "史实考据。")

    def test_empty_section_returns_empty_string(self):
        text = "# 卡\n\n## 出处\n\n## 其他\n\n内容\n"
        self.assertEqual(vd.source_section(text), "")

    def test_section_stops_at_next_heading(self):
        text = "## 出处\n\n考据一句。\n\n## 其他\n\n不应包含\n"
        self.assertEqual(vd.source_section(text), "考据一句。")


if __name__ == "__main__":
    unittest.main()
