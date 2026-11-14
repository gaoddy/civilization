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
        self.assertNotIn("英雄技能", attrs)

    def test_hero_card_adds_hero_rows(self):
        doc = base_doc(
            is_hero=True,
            hero_skill="hero-light-speed-strike",
            hero_skill_cost=1,
            hero_skill_type="主动",
            hero_skill_uses_per_turn=1,
            hero_skill_trigger="主动使用",
        )
        attrs = vd.expected_card_attrs(doc, {"hero-light-speed-strike": SKILL})
        self.assertEqual(attrs["英雄资格"], "是")
        self.assertEqual(attrs["英雄技能"], "hero-light-speed-strike")
        self.assertEqual(attrs["英雄技能费用"], "1")
        self.assertEqual(attrs["英雄技能类型"], "主动")
        self.assertEqual(attrs["每回合次数"], "1")
        self.assertEqual(attrs["触发时机"], "主动使用")

    def test_hero_skill_text(self):
        self.assertEqual(
            vd.hero_skill_text(SKILL),
            "光速出击：使一个友方随从本回合获得连击",
        )


class RealHeroCardsTest(unittest.TestCase):
    def test_real_hero_cards_reference_valid_skills(self):
        skills = {}
        for path in sorted((vd.ROOT / "skills").glob("*.md")):
            if path.name == "README.md":
                continue
            doc = vd.parse_front_matter(path)
            skills[doc["id"]] = doc
        heroes = []
        for path in sorted((vd.ROOT / "cards").glob("*.md")):
            if path.name == "README.md":
                continue
            doc = vd.parse_front_matter(path)
            if doc.get("is_hero") is True:
                heroes.append(doc)

        self.assertEqual(len(heroes), 13)
        self.assertEqual(len({doc["hero_skill"] for doc in heroes}), 13)
        for doc in heroes:
            skill = skills.get(doc["hero_skill"])
            self.assertIsNotNone(skill, f"{doc['id']}: hero_skill 不存在")
            self.assertTrue(skill.get("name"), f"{doc['id']}: 英雄技能缺少 name")
            self.assertTrue(skill.get("match_text"), f"{doc['id']}: 英雄技能缺少 match_text")


if __name__ == "__main__":
    unittest.main()
