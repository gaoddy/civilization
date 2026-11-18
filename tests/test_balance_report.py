import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import balance_report as br  # noqa: E402


def card(cid, playstyle, level, attack, health, skills=(), card_type="minion", cost=None, specialty=()):
    return {
        "id": cid, "playstyle": playstyle, "level": level, "attack": attack,
        "health": health, "skills": list(skills), "card_type": card_type,
        "cost": cost, "specialty": [{"kind": "play", "value": s[5:]} for s in specialty],
        "is_scholar": False, "is_hero": False,
    }


class BandTest(unittest.TestCase):
    def test_inside_tolerance_passes(self):
        cards = [card(f"m{i}", "mammal", 3, 4, 5) for i in range(10)]
        self.assertEqual(br.violations(cards, ["mammal"]), [])

    def test_outside_tolerance_fails(self):
        cards = [card(f"m{i}", "mammal", 3, 6, 5) for i in range(10)]
        errors = br.violations(cards, ["mammal"])
        self.assertTrue(any("mammal" in e for e in errors))

    def test_spread_requires_020(self):
        cards = [card(f"m{i}", "mammal", 3, 4, 5) for i in range(10)]
        cards += [card(f"p{i}", "plant", 3, 3, 7) for i in range(10)]
        errors = br.violations(cards)
        self.assertTrue(any("攻血比差" in e for e in errors))


class SkillDensityTest(unittest.TestCase):
    def test_three_four_star_needs_half(self):
        cards = [card(f"a{i}", "mammal", 3, 4, 5, ["x", "y"] if i < 4 else ["x"]) for i in range(10)]
        errors = br.violations(cards)
        self.assertTrue(any("3–4 星" in e for e in errors))

    def test_five_seven_star_needs_eighty_percent(self):
        cards = [card(f"b{i}", "mammal", 5, 8, 10, ["x", "y"] if i < 7 else ["x"]) for i in range(10)]
        errors = br.violations(cards)
        self.assertTrue(any("5–7 星" in e for e in errors))


class EffectTest(unittest.TestCase):
    def test_total_must_be_100(self):
        cards = [card(f"e{i}", "neutral", 3, 0, 0, ["s"], "effect", 2) for i in range(99)]
        errors = br.violations(cards)
        self.assertTrue(any("效果牌总数" in e for e in errors))

    def test_level_share_cap(self):
        cards = [card(f"m{i}", "mammal", 1, 1, 3) for i in range(10)]
        cards += [card(f"e{i}", "neutral", 1, 0, 0, ["s"], "effect", 1) for i in range(10)]
        errors = br.violations(cards)
        self.assertTrue(any("占比" in e for e in errors))

    def test_targeted_count(self):
        cards = [card(f"e{i}", "neutral", 3, 0, 0, ["s"], "effect", 2,
                      ["play:fish"] if i < 38 else []) for i in range(100)]
        errors = br.violations(cards)
        self.assertTrue(any("定向效果牌" in e for e in errors))

    def test_three_targeted_cards_per_playstyle(self):
        cards = [card(f"e{i}", "neutral", 3, 0, 0, ["s"], "effect", 2, ["play:fish"])
                 for i in range(39)]
        cards += [card(f"g{i}", "neutral", 3, 0, 0, ["s"], "effect", 2) for i in range(61)]
        errors = br.violations(cards)
        self.assertTrue(any("fish" in e and "3 张" in e for e in errors))


if __name__ == "__main__":
    unittest.main()
