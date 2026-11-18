import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import validate_docs as vd  # noqa: E402


class EffectivePlaystyleTest(unittest.TestCase):
    def test_effect_uses_play_association(self):
        doc = {"card_type": "effect", "playstyle": "neutral", "specialty": ["play:fish"]}
        self.assertEqual(vd.effective_playstyle(doc), "fish")

    def test_effect_without_association_is_neutral(self):
        doc = {"card_type": "effect", "playstyle": "neutral", "specialty": []}
        self.assertEqual(vd.effective_playstyle(doc), "neutral")

    def test_minion_keeps_own_playstyle(self):
        doc = {"card_type": "minion", "playstyle": "bird", "specialty": ["play:fish"]}
        self.assertEqual(vd.effective_playstyle(doc), "bird")


class SkillCountTest(unittest.TestCase):
    def setUp(self):
        self.skill_map = {
            "k1": {"keyword": "taunt"},
            "k2": {"keyword": "windfury"},
            "k3": {"keyword": "poison"},
            "p": {},
        }

    def test_one_two_star_allows_one(self):
        doc = {"level": 2, "playstyle": "mammal", "taxon": "hominidae",
               "is_scholar": False, "skills": ["k1", "k2"]}
        errors = vd.skill_count_errors(doc, self.skill_map)
        self.assertTrue(any("最多 1 个技能" in e for e in errors))

    def test_three_four_star_allows_two(self):
        doc = {"level": 4, "playstyle": "mammal", "taxon": "hominidae",
               "is_scholar": False, "skills": ["k1", "k2"]}
        self.assertEqual(vd.skill_count_errors(doc, self.skill_map), [])

    def test_five_seven_star_max_two_keywords(self):
        doc = {"level": 6, "playstyle": "mammal", "taxon": "hominidae",
               "is_scholar": False, "skills": ["k1", "k2", "k3", "p"]}
        errors = vd.skill_count_errors(doc, self.skill_map)
        self.assertTrue(any("最多 2 个登记关键词" in e for e in errors))

    def test_skillless_tool_card_allowed(self):
        doc = {"level": 3, "playstyle": "neutral", "taxon": "life-medical",
               "is_scholar": False, "skills": []}
        self.assertEqual(vd.skill_count_errors(doc, self.skill_map), [])

    def test_skillless_non_tool_rejected(self):
        doc = {"level": 2, "playstyle": "bird", "taxon": "archaeopterygidae",
               "is_scholar": False, "skills": []}
        errors = vd.skill_count_errors(doc, self.skill_map)
        self.assertTrue(any("无技能" in e for e in errors))


if __name__ == "__main__":
    unittest.main()
