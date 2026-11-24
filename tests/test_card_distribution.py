import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import validate_docs as vd  # noqa: E402


def minion(cid, level, attack, health, play, skills=()):
    return {"id": cid, "card_type": "minion", "level": level, "attack": attack,
            "health": health, "categories": [f"play:{play}"], "skills": list(skills),
            "in_shop_pool": True}


class StatDistributionTest(unittest.TestCase):
    def test_three_same_stats_rejected(self):
        cards = [minion(f"a{i}", 2, 3, 3, "mammal") for i in range(3)]
        errors = vd.distribution_errors(cards)
        self.assertTrue(any("play:mammal" in e and "3/3" in e for e in errors), errors)

    def test_two_same_stats_pass(self):
        cards = [minion(f"a{i}", 2, 3, 3, "mammal") for i in range(2)]
        self.assertEqual(vd.distribution_errors(cards), [])

    def test_attack_zero_exempt_from_stats(self):
        cards = [minion(f"z{i}", 5, 0, 16, "mammal", ["skill-x"]) for i in range(10)]
        self.assertFalse([e for e in vd.distribution_errors(cards) if "身材" in e])


class SkillSetDistributionTest(unittest.TestCase):
    def test_duplicate_effect_skill_set_rejected(self):
        cards = [minion("a", 3, 3, 5, "plant", ["keyword-taunt", "battlecry-friendly-growth-once"]),
                 minion("b", 3, 4, 5, "plant", ["battlecry-friendly-growth-once"])]
        errors = vd.distribution_errors(cards)
        self.assertTrue(any("效果技能" in e for e in errors), errors)

    def test_keyword_only_cards_need_unique_body(self):
        cards = [minion("a", 2, 2, 5, "bird", ["keyword-taunt"]),
                 minion("b", 2, 2, 5, "bird", ["keyword-taunt"])]
        errors = vd.distribution_errors(cards)
        self.assertTrue(any("只带关键词" in e for e in errors), errors)

    def test_attack_zero_full_skill_set_must_differ(self):
        cards = [minion("a", 4, 0, 12, "mammal", ["skill-x"]),
                 minion("b", 4, 0, 12, "mammal", ["skill-x"])]
        errors = vd.distribution_errors(cards)
        self.assertTrue(any("攻 0" in e for e in errors), errors)
