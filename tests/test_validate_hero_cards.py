import pathlib
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / "scripts"))

import validate_docs as vd  # noqa: E402


SKILL = {
    "id": "light-speed-strike",
    "name": "光速出击",
    "text": "使一个友方随从本回合获得连击",
    "match_text": "同左",
}


def base_doc(**extra):
    doc = {
        "id": "einstein",
        "name": "爱因斯坦",
        "card_type": "minion",
        "tribe": "scholar",
        "subtype": "physics",
        "level": 5,
        "era": "modern",
        "attack": 0,
        "health": 16,
        "is_scholar": True,
        "skills": ["battlecry-all-friendly-growth-once"],
    }
    doc.update(extra)
    return doc


def hero_doc(**extra):
    doc = {
        "_file": "einstein.md",
        "id": "einstein",
        "name": "爱因斯坦",
        "card_type": "hero",
        "categories": ["play:machine"],
        "skills": ["light-speed-strike"],
    }
    doc.update(extra)
    return doc


def minion_doc(**extra):
    doc = {
        "_file": "einstein.md",
        "id": "einstein",
        "name": "爱因斯坦",
        "card_type": "minion",
        "categories": ["play:machine"],
        "skills": ["light-speed-strike"],
    }
    doc.update(extra)
    return doc


class CardAttrsTest(unittest.TestCase):
    def test_minion_has_no_hero_qualification_row(self):
        attrs = vd.expected_card_attrs(base_doc(), {})
        self.assertNotIn("英雄资格", attrs)
        for key in vd.LEGACY_HERO_FIELDS:
            self.assertNotIn(key, attrs)


class HeroAttrsTest(unittest.TestCase):
    def test_hero_attrs_keep_only_identity_and_skills(self):
        attrs = vd.expected_hero_attrs(
            hero_doc(categories=["play:machine"], skills=["light-speed-strike"]),
            {"light-speed-strike": SKILL},
        )
        self.assertEqual(attrs["id"], "einstein")
        self.assertEqual(attrs["名称"], "爱因斯坦")
        self.assertEqual(attrs["类型"], "英雄")
        self.assertEqual(attrs["技能"], "light-speed-strike")
        for key in ("星级", "攻击力", "生命值", "时代", "科学分类",
                    "英雄技能", "英雄技能费用", "英雄技能类型",
                    "每回合次数", "触发时机"):
            self.assertNotIn(key, attrs)


class SkillAttrsTest(unittest.TestCase):
    def test_active_skill_adds_cost_and_uses_rows(self):
        attrs = vd.expected_skill_attrs(
            {
                "id": "shout",
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
                "id": "observation",
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

    def test_real_hero_cards_reference_existing_skills(self):
        skills = {}
        for path in sorted((vd.ROOT / "skills").glob("*.md")):
            if path.name == "README.md":
                continue
            doc = vd.parse_front_matter(path)
            skills[doc["id"]] = doc

        heroes = []
        for path in sorted((vd.ROOT / "heroes").glob("*.md")):
            if path.name == "README.md":
                continue
            doc = vd.parse_front_matter(path)
            heroes.append(doc)
            self.assertEqual(doc["card_type"], "hero", doc["id"])
            self.assertTrue(vd.card_skills(doc), doc["id"])
            for sid in vd.card_skills(doc):
                self.assertIn(sid, skills, f"{doc['id']} 引用了不存在的技能 {sid}")

        self.assertEqual(len(heroes), 32)
        # 13 张原英雄各有一个专属技能，扩到 32 张后这些技能仍被引用。
        referenced = {sid for doc in heroes for sid in vd.card_skills(doc)}
        self.assertLessEqual(HERO_SKILLS, referenced)

        active = [sid for sid in HERO_SKILLS if "cost" in skills[sid]]
        passive = [sid for sid in HERO_SKILLS if "cost" not in skills[sid]]
        self.assertEqual(len(active), 10)
        self.assertEqual(len(passive), 3)
        for sid in active:
            self.assertIn("uses_per_turn", skills[sid], sid)
        for sid in passive:
            self.assertNotIn("uses_per_turn", skills[sid], sid)

    def test_hero_and_minion_pools_share_ids(self):
        hero_ids = {
            vd.parse_front_matter(path)["id"]
            for path in (vd.ROOT / "heroes").glob("*.md")
            if path.name != "README.md"
        }
        card_ids = {
            vd.parse_front_matter(path)["id"]
            for path in (vd.ROOT / "cards").glob("*.md")
            if path.name != "README.md"
        }
        self.assertEqual(hero_ids, hero_ids & card_ids, "每张英雄卡都应有同名随从卡")


class ActiveSkillFieldTest(unittest.TestCase):
    def errors(self, **fields):
        skill = {"_file": "shout.md", "id": "shout",
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

    def test_named_passive_skill_without_match_text_is_rejected(self):
        skill = {"_file": "observation.md", "id": "observation", "name": "观测",
                 "text": "每回合首次刷新时额外出现一张当前时代的随机随从"}
        messages = vd.skill_field_errors(skill)
        self.assertTrue(any("有 name 的技能必须填写 match_text" in m
                            for m in messages), messages)


class HeroSkillSemanticsTest(unittest.TestCase):
    def errors(self, skill):
        return vd.hero_skill_errors(hero_doc(), skill)

    def test_active_skill_needs_on_activate(self):
        messages = self.errors({"id": "shout", "cost": 1,
                                "_rendered_effects": "OnPlay|Growth|target|times=1"})
        self.assertTrue(any("必须有 OnActivate 效果" in m for m in messages), messages)

    def test_active_skill_with_on_activate_passes(self):
        self.assertEqual(self.errors({"id": "shout", "cost": 1,
                                      "_rendered_effects": "OnActivate|Growth|target|times=1"}), [])

    def test_active_skill_rejects_passive_trigger(self):
        messages = self.errors({
            "id": "shout", "cost": 1,
            "_rendered_effects": "OnActivate|Growth|target|times=1;"
                                 "Aura|ModifyStats|friendly:all|health=1,duration=permanent",
        })
        self.assertTrue(any("英雄主动技能 shout 不得有触发" in m for m in messages), messages)

    def test_passive_skill_rejects_battlecry(self):
        messages = self.errors({"id": "observation",
                                "_rendered_effects": "OnPlay|RefreshShop|self"})
        self.assertTrue(any("不得有触发" in m for m in messages), messages)

    def test_passive_skill_rejects_on_activate(self):
        messages = self.errors({"id": "observation",
                                "_rendered_effects": "OnActivate|Growth|self|times=1"})
        self.assertTrue(any("不得有触发" in m for m in messages), messages)

    def test_passive_skill_needs_passive_trigger(self):
        messages = self.errors({"id": "observation", "_rendered_effects": "[]"})
        self.assertTrue(any("缺少被动触发" in m for m in messages), messages)

    def test_passive_skill_with_aura_passes(self):
        self.assertEqual(self.errors({
            "id": "observation",
            "_rendered_effects": "Aura|ModifyStats|friendly:all|health=1,duration=permanent",
        }), [])

    def test_skill_without_rendered_effects_is_skipped(self):
        self.assertEqual(self.errors({"id": "observation"}), [])


class SkillRefsTest(unittest.TestCase):
    def test_unknown_skill_reference_is_rejected(self):
        doc = base_doc(_file="einstein.md", skills=["no-such-skill"])
        errors = vd.skill_ref_errors(doc, {"battlecry-all-friendly-growth-once": {}})
        self.assertEqual(errors, ["einstein.md: 引用的技能不存在 'no-such-skill'"])

    def test_known_skill_reference_passes(self):
        doc = base_doc(_file="einstein.md", skills=["battlecry-all-friendly-growth-once"])
        self.assertEqual(vd.skill_ref_errors(doc, {"battlecry-all-friendly-growth-once": {}}), [])


class HeroCardsTest(unittest.TestCase):
    """`check_hero_cards` 从 content 目录读英雄卡，这里用临时目录注入。"""

    def run_check(self, heroes, minions=None, cards=None, effects=(), skills=None):
        minions = list(minions if minions is not None else [minion_doc()])
        cards = list(cards if cards is not None else minions) + list(effects)
        if skills is None:
            skills = {"light-speed-strike": SKILL}
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            for kind, docs in (("cards", cards), ("heroes", heroes)):
                directory = root / kind
                directory.mkdir()
                for doc in docs:
                    (directory / doc["_file"]).write_text("# 标题\n", encoding="utf-8")
            vd.errors.clear()
            with mock.patch.object(vd, "ROOT", root), \
                    mock.patch.object(vd, "load", return_value=heroes):
                vd.check_hero_cards(cards, minions, list(effects), skills)
            return list(vd.errors)

    def test_thirty_two_heroes_pass(self):
        heroes = [hero_doc(_file=f"h{i}.md", id=f"h{i}", name=f"h{i}")
                  for i in range(32)]
        minions = [minion_doc(_file=f"h{i}.md", id=f"h{i}", name=f"h{i}")
                   for i in range(32)]
        self.assertEqual(self.run_check(heroes, minions, cards=minions), [])

    def test_thirty_one_heroes_are_rejected(self):
        heroes = [hero_doc(_file=f"h{i}.md", id=f"h{i}", name=f"h{i}")
                  for i in range(31)]
        minions = [minion_doc(_file=f"h{i}.md", id=f"h{i}", name=f"h{i}")
                   for i in range(32)]
        errors = self.run_check(heroes, minions, cards=minions)
        self.assertTrue(any("期望 32" in e for e in errors), errors)

    def test_hero_without_named_minion_is_rejected(self):
        heroes = [hero_doc(_file="ghost.md", id="ghost", name="ghost")]
        errors = self.run_check(heroes)
        self.assertTrue(any("没有对应的随从卡" in e for e in errors), errors)

    def test_hero_skills_may_differ_from_named_minion(self):
        heroes = [hero_doc(_file=f"h{i}.md", id=f"h{i}", name=f"h{i}", skills=[])
                  for i in range(32)]
        minions = [minion_doc(_file=f"h{i}.md", id=f"h{i}", name=f"h{i}")
                   for i in range(32)]
        self.assertEqual(self.run_check(heroes, minions, cards=minions), [])

    def test_hero_skill_with_keyword_is_rejected(self):
        heroes = [hero_doc()]
        skills = {"light-speed-strike": dict(SKILL, keyword="battlecry")}
        errors = self.run_check(heroes, skills=skills)
        self.assertTrue(any("不得登记关键词" in e for e in errors), errors)

    def test_hero_category_mismatch_is_rejected(self):
        heroes = [hero_doc(categories=["play:bird"])]
        errors = self.run_check(heroes)
        self.assertTrue(any("categories 与随从卡" in e for e in errors), errors)

    def test_minion_with_is_hero_field_is_rejected(self):
        minions = [minion_doc(is_hero=False)]
        errors = self.run_check([hero_doc()], minions, cards=minions)
        self.assertTrue(any("不得再有 is_hero" in e for e in errors), errors)

    def test_hero_with_minion_only_field_is_rejected(self):
        heroes = [hero_doc(attack=3)]
        errors = self.run_check(heroes)
        self.assertTrue(any("英雄卡不得有字段" in e for e in errors), errors)

    def test_hero_with_wrong_card_type_is_rejected(self):
        heroes = [hero_doc(card_type="minion")]
        errors = self.run_check(heroes)
        self.assertTrue(any("card_type 必须为 hero" in e for e in errors), errors)


if __name__ == "__main__":
    unittest.main()
