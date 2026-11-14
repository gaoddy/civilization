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


class RealHeroCardsTest(unittest.TestCase):
    def test_real_hero_cards_reference_one_hero_skill(self):
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
                hero_refs = [
                    sid for sid in vd.card_skills(doc)
                    if sid.startswith(vd.HERO_SKILL_PREFIX)
                ]
                self.assertEqual(len(hero_refs), 1, doc["id"])
                referenced[hero_refs[0]] += 1

        self.assertEqual(len(heroes), 13)
        hero_skills = {sid for sid in skills if sid.startswith(vd.HERO_SKILL_PREFIX)}
        self.assertEqual(set(referenced), hero_skills)
        self.assertEqual(set(referenced.values()), {1})

        active = [sid for sid in hero_skills if "cost" in skills[sid]]
        passive = [sid for sid in hero_skills if "cost" not in skills[sid]]
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


class HeroSkillRefsTest(unittest.TestCase):
    HERO_IDS = ["adam-smith", "darwin", "davinci", "edison", "einstein",
                "faraday", "galileo", "knuth", "luxun", "marie-curie",
                "mendel", "pasteur", "tesla"]

    def run_check(self, skills_by_card, skill_ids):
        heroes = []
        for card_id, skills in zip(self.HERO_IDS, skills_by_card):
            heroes.append({"_file": f"{card_id}.md", "id": card_id,
                           "name": card_id, "skills": skills, "is_hero": True})
        skill_map = {sid: {"_file": f"{sid}.md", "id": sid,
                           "name": sid, "text": "", "match_text": "同左"}
                     for sid in skill_ids}
        vd.errors.clear()
        vd.check_hero_cards(heroes, heroes, [], skill_map)
        return list(vd.errors)

    def test_card_with_two_hero_skills_is_rejected(self):
        ids = [f"hero-s{i}" for i in range(13)]
        skills = [[sid] for sid in ids]
        skills[0] = [ids[0], ids[1]]
        errors = self.run_check(skills, ids)
        self.assertTrue(any("恰好引用 1 个 hero-* 技能" in e for e in errors), errors)

    def test_card_with_no_hero_skill_is_rejected(self):
        ids = [f"hero-s{i}" for i in range(13)]
        skills = [[sid] for sid in ids]
        skills[0] = []
        errors = self.run_check(skills, ids)
        self.assertTrue(any("恰好引用 1 个 hero-* 技能" in e for e in errors), errors)

    def test_unreferenced_hero_skill_is_rejected(self):
        ids = [f"hero-s{i}" for i in range(13)]
        skills = [[sid] for sid in ids]
        errors = self.run_check(skills, ids + ["hero-orphan"])
        self.assertTrue(any("hero-orphan" in e and "恰好被 1 张英雄卡引用" in e
                            for e in errors), errors)

    def test_valid_mapping_passes(self):
        ids = [f"hero-s{i}" for i in range(13)]
        self.assertEqual(self.run_check([[sid] for sid in ids], ids), [])


if __name__ == "__main__":
    unittest.main()
