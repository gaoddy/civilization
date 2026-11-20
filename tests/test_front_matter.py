#!/usr/bin/env python3
"""validate_docs.py 的 front matter 解析单元测试（标准库 unittest）。

运行：python3 -m unittest discover -s tests -v
"""

import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / "scripts"))

import validate_docs as vd  # noqa: E402


class FrontMatterTest(unittest.TestCase):
    def _write(self, text):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        path = pathlib.Path(tmp.name) / "card.md"
        path.write_text(text, encoding="utf-8")
        return path

    def test_duplicate_key_is_rejected(self):
        # 重复键会被 dict 静默覆盖，掩盖「skills: [] 写进了 level 的值」这类损坏。
        path = self._write("---\nid: a\nlevel: 1\nlevel: 1\n---\n")

        with self.assertRaisesRegex(ValueError, "键重复"):
            vd.parse_front_matter(path)

    def test_plain_values_still_parse(self):
        path = self._write("---\nid: a\nlevel: 1\nskills: []\nname: 测试\n---\n")

        doc = vd.parse_front_matter(path)

        self.assertEqual(doc["id"], "a")
        self.assertEqual(doc["level"], 1)
        self.assertEqual(doc["skills"], [])
        self.assertEqual(doc["name"], "测试")
