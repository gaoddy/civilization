import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import balance_report as br  # noqa: E402


def card(cid, plays, level, attack, health, skills=(), card_type="minion", cost=None,
         categories=None, taxon=None):
    """plays 为玩法 id 元组；需要 prey/host 时用 categories 传 kind:value 列表。"""
    if categories is None:
        categories = [f"play:{play}" for play in plays]
    return {
        "id": cid, "level": level, "attack": attack, "health": health,
        "skills": list(skills), "card_type": card_type, "cost": cost,
        "categories": [{"kind": kind, "value": value}
                       for kind, _, value in (item.partition(":") for item in categories)],
        "taxon": taxon,
        "is_scholar": False,
    }


class BandTest(unittest.TestCase):
    def test_inside_tolerance_passes(self):
        cards = [card(f"m{i}", ("mammal",), 3, 4, 5) for i in range(10)]
        self.assertEqual(br.violations(cards, ["mammal"]), [])

    def test_outside_tolerance_fails(self):
        cards = [card(f"m{i}", ("mammal",), 3, 6, 5) for i in range(10)]
        errors = br.violations(cards, ["mammal"])
        self.assertTrue(any("mammal" in e for e in errors))

    def test_spread_requires_020(self):
        cards = [card(f"m{i}", ("mammal",), 3, 4, 5) for i in range(10)]
        cards += [card(f"p{i}", ("plant",), 3, 3, 7) for i in range(10)]
        errors = br.violations(cards)
        self.assertTrue(any("攻血比差" in e for e in errors))

    def test_multi_play_card_counts_in_each_play(self):
        cards = [card("a", ("mammal", "machine"), 3, 4, 5)]
        stats = br.playstyle_stats(cards)
        self.assertEqual(stats["mammal"]["count"], 1)
        self.assertEqual(stats["machine"]["count"], 1)


class SkillDensityTest(unittest.TestCase):
    def test_three_four_star_needs_half(self):
        cards = [card(f"a{i}", ("mammal",), 3, 4, 5, ["x", "y"] if i < 4 else ["x"]) for i in range(10)]
        errors = br.violations(cards)
        self.assertTrue(any("3–4 星" in e for e in errors))

    def test_five_seven_star_needs_eighty_percent(self):
        cards = [card(f"b{i}", ("mammal",), 5, 8, 10, ["x", "y"] if i < 7 else ["x"]) for i in range(10)]
        errors = br.violations(cards)
        self.assertTrue(any("5–7 星" in e for e in errors))


class EffectTest(unittest.TestCase):
    def test_total_must_be_100(self):
        cards = [card(f"e{i}", ("machine",), 3, 0, 0, ["s"], "effect", 2) for i in range(99)]
        errors = br.violations(cards)
        self.assertTrue(any("效果牌总数" in e for e in errors))

    def test_level_share_cap(self):
        cards = [card(f"m{i}", ("mammal",), 1, 1, 3) for i in range(10)]
        cards += [card(f"e{i}", ("machine",), 1, 0, 0, ["s"], "effect", 1) for i in range(10)]
        errors = br.violations(cards)
        self.assertTrue(any("占比" in e for e in errors))

    def test_effect_playstyle_counts_are_multi(self):
        cards = [
            card("e1", ("machine",), 3, 0, 0, ["s"], "effect", 2),
            card("e2", ("machine", "mammal"), 3, 0, 0, ["s"], "effect", 2),
            card("e3", ("mammal",), 3, 0, 0, ["s"], "effect", 2),
        ]
        stats = br.effect_stats(cards)
        self.assertEqual(stats["by_playstyle"]["machine"], 2)
        self.assertEqual(stats["by_playstyle"]["mammal"], 2)
        self.assertEqual(stats["play_attributions"], 4)

    def test_effect_playstyle_mismatch_is_reported(self):
        cards = [card(f"e{i}", ("machine",), 3, 0, 0, ["s"], "effect", 2)
                 for i in range(100)]
        errors = br.violations(cards)
        self.assertTrue(any("machine" in e and "效果牌" in e for e in errors), errors)


def skill(keyword=None, effects=()):
    return {"keyword": keyword, "effects": list(effects)}


def effect(trigger, op, selector=None, **params):
    return {"trigger": trigger, "op": op, "params": params,
            "selector": selector or {"kind": "self"}}


SKILLS = {
    "k-taunt": skill("taunt"),
    "k-breed": skill("breed 1"),
    "k-armor": skill("armor 1"),
    "k-windfury": skill("windfury"),
    "s-activate-growth": skill(None, [effect("on_activate", "growth")]),
    "s-battlecry-growth": skill(None, [effect("on_play", "growth")]),
    "s-generic": skill(None, [effect("on_activate", "modify_stats", attack=1)]),
    "s-split": skill(None, [effect("on_death", "summon", token="termite-token")]),
    "s-worker": skill(None, [effect("on_play", "summon", token="worker-bee-token-2")]),
    "s-school": skill(None, [effect("aura", "modify_stats", attack=1, per_attack=1)]),
    "s-ignore": skill("ignore_taunt"),
    "s-contagion": skill(None, [effect("on_combat_start", "grant_keyword",
                                       keyword={"keyword": "contagion", "value": 1})]),
}


class EffectCostLevelTest(unittest.TestCase):
    def test_four_cost_effect_must_be_six_or_seven(self):
        cards = [card("e5", ("machine",), 5, 0, 0, ["s"], "effect", 4)]
        self.assertTrue(any("4 费" in e for e in br.effect_cost_errors(cards)))

    def test_four_cost_effect_at_six_and_seven_passes(self):
        cards = [card("e6", ("machine",), 6, 0, 0, ["s"], "effect", 4),
                 card("e7", ("machine",), 7, 0, 0, ["s"], "effect", 4)]
        self.assertEqual(br.effect_cost_errors(cards), [])

    def test_three_cost_effect_is_not_checked(self):
        cards = [card("e3", ("machine",), 3, 0, 0, ["s"], "effect", 3)]
        self.assertEqual(br.effect_cost_errors(cards), [])


class VocabularyTest(unittest.TestCase):
    def test_three_four_second_skill_must_be_vocabulary(self):
        cards = [card("a", ("mammal",), 3, 3, 4, ["k-taunt", "s-generic"])]
        errors = br.vocabulary_errors(cards, SKILLS)
        self.assertTrue(any("a" in e and "第二个技能" in e for e in errors))

    def test_three_four_second_skill_in_vocabulary_passes(self):
        cards = [card("b", ("mammal",), 3, 3, 4, ["k-taunt", "s-activate-growth"])]
        self.assertEqual(br.vocabulary_errors(cards, SKILLS), [])

    def test_five_seven_allows_one_generic_with_one_signature(self):
        cards = [card("c", ("mammal",), 5, 7, 9, ["k-taunt", "s-generic"])]
        self.assertEqual(br.vocabulary_errors(cards, SKILLS), [])

    def test_five_seven_rejects_two_generics(self):
        cards = [card("d", ("mammal",), 5, 7, 9,
                      ["s-generic", "s-ignore", "s-activate-growth"])]
        errors = br.vocabulary_errors(cards, SKILLS)
        self.assertTrue(any("d" in e and "表外技能" in e for e in errors))

    def test_five_seven_rejects_no_signature(self):
        cards = [card("e", ("mammal",), 5, 7, 9, ["s-generic", "s-worker"])]
        errors = br.vocabulary_errors(cards, SKILLS)
        self.assertTrue(any("e" in e and "签名机制" in e for e in errors))

    def test_multi_play_uses_union_of_vocabulary(self):
        # 第二个技能「最优攻击」属于机械词汇，哺乳纲本身不含；多玩法并集后通过。
        cards = [card("f", ("mammal", "machine"), 3, 3, 4, ["k-taunt", "s-ignore"])]
        self.assertEqual(br.vocabulary_errors(cards, SKILLS), [])

    def test_no_play_and_scholar_are_skipped(self):
        no_play = card("n", (), 3, 3, 4, ["s-generic", "s-generic"], categories=[])
        scholar = card("s", ("mammal",), 3, 0, 6, ["s-generic", "s-generic"])
        scholar["is_scholar"] = True
        self.assertEqual(br.vocabulary_errors([no_play, scholar], SKILLS), [])

    def test_craft_tool_taxon_skips_vocabulary_but_automation_is_checked(self):
        tool = card("tool", ("machine",), 5, 7, 9,
                    ["s-generic", "s-worker", "s-generic"], taxon="power-energy")
        automation = card("auto", ("machine",), 5, 7, 9,
                          ["s-generic", "s-worker"], taxon="automation")
        errors = br.vocabulary_errors([tool, automation], SKILLS)
        self.assertFalse(any("tool" in e for e in errors))
        self.assertTrue(any("auto" in e and "签名机制" in e for e in errors))

    def test_skill_terms_recognizes_mechanisms(self):
        self.assertEqual(br.skill_terms(SKILLS["s-activate-growth"]), {"成长"})
        self.assertEqual(br.skill_terms(SKILLS["s-battlecry-growth"]),
                         {"成长", "战吼成长"})
        self.assertIn("分裂", br.skill_terms(SKILLS["s-split"]))
        self.assertIn("召唤工蜂", br.skill_terms(SKILLS["s-worker"]))
        self.assertIn("群聚", br.skill_terms(SKILLS["s-school"]))
        self.assertIn("最优攻击", br.skill_terms(SKILLS["s-ignore"]))
        self.assertIn("传染", br.skill_terms(SKILLS["s-contagion"]))
        self.assertEqual(br.skill_terms(SKILLS["k-breed"]), {"繁衍"})
        self.assertEqual(br.skill_terms(SKILLS["k-armor"]), {"减伤"})
        self.assertEqual(br.skill_terms(SKILLS["k-windfury"]), {"连击"})
        self.assertEqual(br.skill_terms(SKILLS["k-taunt"]), {"嘲讽"})


if __name__ == "__main__":
    unittest.main()
