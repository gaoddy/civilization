import collections
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / "scripts"))

import validate_docs as vd  # noqa: E402


SKILL = {
    "id": "hero-light-speed-strike",
    "name": "光速出击",
    "text": "使一个友方随从本回合获得连击",
    "match_text": "同左",
}


def base_doc(**extra):
    doc = {
        "id": "einstein",
        "name": "爱因斯坦",
        "card_type": "随从",
        "tribe": "学者",
        "subtype": "物理",
        "level": 5,
        "era": "现代",
        "attack": 0,
        "health": 16,
        "is_scholar": True,
        "skills": ["battlecry-all-friendly-growth-once"],
    }
    doc.update(extra)
    return doc


class HeroCardAttrsTest(unittest.TestCase):
    def test_non_hero_has_no_hero_rows(self):
        attrs = vd.expected_card_attrs(base_doc(), {})
        self.assertEqual(attrs["英雄资格"], "否")
        for key in vd.LEGACY_HERO_FIELDS:
            self.assertNotIn(key, attrs)

    def test_hero_card_has_no_hero_skill_rows(self):
        doc = base_doc(is_hero=True, skills=["hero-light-speed-strike"])
        attrs = vd.expected_card_attrs(doc, {"hero-light-speed-strike": SKILL})
        self.assertEqual(attrs["英雄资格"], "是")
        self.assertEqual(attrs["技能"], "hero-light-speed-strike")
        for key in ("英雄技能", "英雄技能费用", "英雄技能类型", "每回合次数", "触发时机"):
            self.assertNotIn(key, attrs)


class SkillAttrsTest(unittest.TestCase):
    def test_active_skill_adds_cost_and_uses_rows(self):
        attrs = vd.expected_skill_attrs(
            {
                "id": "hero-shout",
                "name": "呐喊",
                "text": "使一个友方随从获得一次成长",
                "match_text": "同左",
                "cost": 1,
                "uses_per_turn": 2,
            }
        )
        self.assertEqual(attrs["费用"], "1")
        self.assertEqual(attrs["每回合次数"], "2")

    def test_passive_skill_omits_active_rows(self):
        attrs = vd.expected_skill_attrs(
            {
                "id": "hero-observation",
                "name": "观测",
                "text": "每回合首次刷新时额外出现一张当前时代的随机随从",
                "match_text": "同左",
            }
        )
        self.assertNotIn("费用", attrs)
        self.assertNotIn("每回合次数", attrs)


HERO_SKILLS = {
    "alternating-current", "division-of-labor", "efficient-algorithm",
    "evolution-discovery", "faraday-cage", "heredity", "invention-factory",
    "light-speed-strike", "microbiology", "observation", "radiation",
    "shout", "universal-manuscript",
}


class RealHeroCardsTest(unittest.TestCase):
    def test_no_skill_id_keeps_hero_prefix(self):
        for path in sorted((vd.ROOT / "skills").glob("*.md")):
            if path.name == "README.md":
                continue
            doc = vd.parse_front_matter(path)
            self.assertFalse(doc["id"].startswith("hero-"), doc["id"])

    def test_real_hero_cards_reference_renamed_skills(self):
        skills = {}
        for path in sorted((vd.ROOT / "skills").glob("*.md")):
            if path.name == "README.md":
                continue
            doc = vd.parse_front_matter(path)
            skills[doc["id"]] = doc

        heroes, referenced = [], collections.Counter()
        for path in sorted((vd.ROOT / "cards").glob("*.md")):
            if path.name == "README.md":
                continue
            doc = vd.parse_front_matter(path)
            if doc.get("is_hero") is True:
                heroes.append(doc)
                refs = [sid for sid in vd.card_skills(doc) if sid in HERO_SKILLS]
                self.assertEqual(len(refs), 1, doc["id"])
                referenced[refs[0]] += 1

        self.assertEqual(len(heroes), 13)
        self.assertEqual(set(referenced), HERO_SKILLS)
        self.assertEqual(set(referenced.values()), {1})

        active = [sid for sid in HERO_SKILLS if "cost" in skills[sid]]
        passive = [sid for sid in HERO_SKILLS if "cost" not in skills[sid]]
        self.assertEqual(len(active), 10)
        self.assertEqual(len(passive), 3)
        for sid in active:
            self.assertIn("uses_per_turn", skills[sid], sid)
        for sid in passive:
            self.assertNotIn("uses_per_turn", skills[sid], sid)


class ActiveSkillFieldTest(unittest.TestCase):
    def errors(self, **fields):
        skill = {"_file": "hero-shout.md", "id": "hero-shout",
                 "name": "呐喊", "text": "使一个友方随从获得一次成长",
                 "match_text": "同左"}
        skill.update(fields)
        return vd.skill_field_errors(skill)

    def test_cost_without_uses_is_rejected(self):
        messages = self.errors(cost=1)
        self.assertTrue(any("必须成对出现" in m for m in messages), messages)

    def test_uses_without_cost_is_rejected(self):
        messages = self.errors(uses_per_turn=1)
        self.assertTrue(any("必须成对出现" in m for m in messages), messages)

    def test_cost_out_of_range_is_rejected(self):
        messages = self.errors(cost=3, uses_per_turn=1)
        self.assertTrue(any("技能费用越界" in m for m in messages), messages)

    def test_uses_out_of_range_is_rejected(self):
        messages = self.errors(cost=1, uses_per_turn=3)
        self.assertTrue(any("每回合次数非法" in m for m in messages), messages)

    def test_boolean_cost_is_rejected(self):
        messages = self.errors(cost=True, uses_per_turn=1)
        self.assertTrue(any("技能费用越界" in m for m in messages), messages)

    def test_legal_active_skill_passes(self):
        self.assertEqual(self.errors(cost=1, uses_per_turn=2), [])


class HeroCardsTest(unittest.TestCase):
    HERO_IDS = ["adam-smith", "darwin", "davinci", "edison", "einstein",
                "faraday", "galileo", "knuth", "luxun", "marie-curie",
                "mendel", "pasteur", "tesla"]

    def run_check(self, count, extra_minions=(), extra_effects=()):
        heroes = [{"_file": f"{card_id}.md", "id": card_id, "name": card_id,
                   "skills": [], "is_hero": True}
                  for card_id in self.HERO_IDS[:count]]
        minions = list(heroes) + list(extra_minions)
        effects = list(extra_effects)
        vd.errors.clear()
        vd.check_hero_cards(minions + effects, minions, effects)
        return list(vd.errors)

    def test_thirteen_heroes_pass(self):
        self.assertEqual(self.run_check(13), [])

    def test_twelve_heroes_are_rejected(self):
        errors = self.run_check(12)
        self.assertTrue(any("期望 13" in e for e in errors), errors)

    def test_non_hero_minion_with_is_hero_field_is_rejected(self):
        errors = self.run_check(13, extra_minions=[
            {"_file": "luxun.md", "id": "fake", "name": "fake",
             "skills": [], "is_hero": False},
        ])
        self.assertTrue(any("非英雄随从不得有 is_hero" in e for e in errors), errors)

    def test_effect_card_with_is_hero_is_rejected(self):
        errors = self.run_check(13, extra_effects=[
            {"_file": "luxun.md", "id": "effect", "name": "effect",
             "skills": [], "is_hero": True, "card_type": "效果"},
        ])
        self.assertTrue(any("效果牌不得有 is_hero" in e for e in errors), errors)


if __name__ == "__main__":
    unittest.main()
