import csv, io, pathlib, re, sys, unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import gen_text, i18n_terms

_CJK = re.compile(r"[一-鿿]")


def _keys(text):
    rows = list(csv.reader(io.StringIO(text)))
    return [row[0] for row in rows[1:]]


class TextTableTest(unittest.TestCase):
    def test_csv_header_and_unique_keys(self):
        rows = list(csv.reader(io.StringIO(gen_text.build(ROOT))))
        self.assertEqual(rows[0], ["keys", "zh_CN"])
        keys = [row[0] for row in rows[1:]]
        self.assertEqual(keys, sorted(keys))
        self.assertEqual(len(keys), len(set(keys)))
        self.assertEqual([row for row in rows[1:] if not row[1]], [])

    def test_content_keys_cover_cards_skills_tokens(self):
        keys = _keys(gen_text.build(ROOT))
        self.assertIn("card.adam-smith.name", keys)
        self.assertIn("skill.division-of-labor.text", keys)
        self.assertIn("token.seedling.name", keys)

    def test_generated_prefix_keys_are_replaced(self):
        existing = {"ui.test.only": "只用于测试", "card.old.name": "旧卡"}
        merged = gen_text.merge(existing, {"card.new.name": "新卡"})
        self.assertEqual(merged["ui.test.only"], "只用于测试")
        self.assertNotIn("card.old.name", merged)

    def test_unknown_prefix_is_rejected(self):
        with self.assertRaises(ValueError):
            gen_text.merge({"page.title": "标题"}, {})
