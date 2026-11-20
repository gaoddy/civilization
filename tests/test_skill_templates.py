import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import skill_templates


class NounTableTest(unittest.TestCase):
    def test_load_nouns_requires_all_tables(self):
        nouns = skill_templates.load_nouns(
            ROOT / "docs/versions/v0.1/product/content/card-nouns.md")
        self.assertEqual(nouns.taxon["animalia"], "动物")
        self.assertEqual(nouns.playstyle["machine"], "机械")
        self.assertEqual(nouns.special["scholar=true"], "学者")
        self.assertEqual(nouns.keyword_text["taunt"], "嘲讽")

    def test_parse_arg_spec_range_and_default(self):
        spec = skill_templates.parse_arg_spec("count", "count 1..3 default=1")
        self.assertEqual((spec.type, spec.minimum, spec.maximum, spec.default),
                         ("count", 1, 3, "1"))
        plain = skill_templates.parse_arg_spec("taxon", "taxon")
        self.assertEqual((plain.type, plain.minimum, plain.default), ("taxon", None, None))


if __name__ == "__main__":
    unittest.main()
