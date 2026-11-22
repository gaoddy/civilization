#!/usr/bin/env python3
"""协议文档链接测试：protocol.md 与两级索引的相对链接必须可解析。"""

import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
DOCS = ROOT / "docs"
LINK = re.compile(r"\]\(([^)#]+)(?:#[^)]*)?\)")


class DocsLinksTest(unittest.TestCase):
    def test_protocol_doc_and_index_links_resolve(self):
        for path in [DOCS / "current/technical/architecture/protocol.md",
                     DOCS / "current/technical/architecture/README.md",
                     DOCS / "current/technical/README.md"]:
            self.assertTrue(path.exists(), path)
            text = path.read_text()
            for target in LINK.findall(text):
                if target.startswith(("http://", "https://", "mailto:")):
                    continue
                resolved = (path.parent / target).resolve()
                self.assertTrue(resolved.exists(), f"{path.name} -> {target}")


if __name__ == "__main__":
    unittest.main()
