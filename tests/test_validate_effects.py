#!/usr/bin/env python3
"""validate_docs.py 的 effects DSL 校验单元测试（标准库 unittest）。

运行：python3 -m unittest discover -s tests -v
"""

import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / "scripts"))

import validate_docs as vd  # noqa: E402


class ParseEffectsTest(unittest.TestCase):
    def test_single_effect_parses(self):
        effects, errors = vd.parse_effects("OnPlay|Growth|friendly:single|times=2")
        self.assertEqual(errors, [])
        self.assertEqual(len(effects), 1)
        self.assertEqual(effects[0]["trigger"], "OnPlay")
        self.assertEqual(effects[0]["op"], "Growth")
        self.assertEqual(effects[0]["params"], {"times": "2"})

    def test_empty_bracket_means_no_effect(self):
        effects, errors = vd.parse_effects("[]")
        self.assertEqual(errors, [])
        self.assertEqual(effects, [])

    def test_multiple_effects_split_by_semicolon(self):
        raw = "OnActivate|RefreshShop|self|free=true;OnActivate|GainMinion|self|level=current,to=hand"
        effects, errors = vd.parse_effects(raw)
        self.assertEqual(errors, [])
        self.assertEqual([e["op"] for e in effects], ["RefreshShop", "GainMinion"])

    def test_unknown_trigger_rejected(self):
        effects, errors = vd.parse_effects("Bogus|Growth|self|times=1")
        self.assertEqual(effects, [])
        self.assertTrue(any("触发" in e for e in errors))

    def test_unknown_op_rejected(self):
        effects, errors = vd.parse_effects("OnPlay|Explode|self|")
        self.assertEqual(effects, [])
        self.assertTrue(any("原语" in e for e in errors))

    def test_missing_selector_rejected(self):
        effects, errors = vd.parse_effects("OnPlay|Growth")
        self.assertEqual(effects, [])
        self.assertTrue(any("段" in e for e in errors))

    def test_bad_selector_side_rejected(self):
        effects, errors = vd.parse_effects("OnPlay|Growth|nonsense:single|times=1")
        self.assertEqual(effects, [])
        self.assertTrue(any("选择器" in e for e in errors))

    def test_bad_duration_rejected(self):
        effects, errors = vd.parse_effects("OnPlay|ModifyStats|self|health=1,duration=forever")
        self.assertEqual(effects, [])
        self.assertTrue(any("duration" in e for e in errors))

    def test_text_prefix_must_match_trigger(self):
        errors = vd.check_text_trigger("战吼：使一个友方随从获得一次成长", "OnDeath|Growth|friendly:single|times=1")
        self.assertTrue(errors)

    def test_text_prefix_matching_trigger_ok(self):
        errors = vd.check_text_trigger("战吼：使一个友方随从获得一次成长", "OnPlay|Growth|friendly:single|times=1")
        self.assertEqual(errors, [])

    def test_deathrattle_prefix_maps_to_on_death(self):
        errors = vd.check_text_trigger("亡语：召唤一个 1/1 的白蚁", "OnDeath|Summon|self|card=baiyi,count=1")
        self.assertEqual(errors, [])

    def test_combat_start_text_must_use_combat_start_trigger(self):
        errors = vd.check_text_trigger("战斗开始时，对敌方所有随从造成 2 点伤害", "OnPlay|DealDamage|enemy:all|amount=2")
        self.assertTrue(errors)

    def test_delayed_battlecry_enemy_effect_may_use_combat_start(self):
        errors = vd.check_text_trigger("战吼：对随机一个敌方随从造成 1 点伤害", "OnCombatStart|DealDamage|enemy:random|amount=1")
        self.assertEqual(errors, [])

    def test_until_turn_end_suffix_does_not_skip_prefix_check(self):
        errors = vd.check_text_trigger(
            "战吼：使一个友方随从获得嘲讽直到回合结束",
            "OnDeath|GrantKeyword|friendly:single|keyword=taunt,duration=combat",
        )
        self.assertTrue(errors)

    def test_unknown_pick_rejected(self):
        effects, errors = vd.parse_effects("OnPlay|Growth|friendly:singel|times=1")
        self.assertEqual(effects, [])
        self.assertTrue(any("选取" in e for e in errors))

    def test_unknown_filter_key_rejected(self):
        effects, errors = vd.parse_effects("OnPlay|Growth|friendly:all:tirbe=machine|times=1")
        self.assertEqual(effects, [])
        self.assertTrue(any("筛选" in e for e in errors))

    def test_unknown_filter_value_rejected(self):
        effects, errors = vd.parse_effects("OnPlay|Growth|friendly:all:tribe=dragon|times=1")
        self.assertEqual(effects, [])
        self.assertTrue(any("tribe" in e for e in errors))

    def test_missing_required_param_rejected(self):
        effects, errors = vd.parse_effects("OnPlay|Summon|self|count=1")
        self.assertEqual(effects, [])
        self.assertTrue(any("token" in e or "card" in e for e in errors))

    def test_non_numeric_amount_rejected(self):
        effects, errors = vd.parse_effects("OnPlay|DealDamage|enemy:all|amount=two")
        self.assertEqual(effects, [])
        self.assertTrue(any("amount" in e for e in errors))

    def test_duplicate_param_key_rejected(self):
        effects, errors = vd.parse_effects("OnPlay|ModifyStats|self|attack=1,attack=2")
        self.assertEqual(effects, [])
        self.assertTrue(any("重复" in e for e in errors))

    def test_unknown_keyword_value_rejected(self):
        effects, errors = vd.parse_effects("OnPlay|GrantKeyword|friendly:all|keyword=nonexistent,duration=permanent")
        self.assertEqual(effects, [])
        self.assertTrue(any("keyword" in e for e in errors))

    def test_aura_cannot_deal_damage(self):
        effects, errors = vd.parse_effects("Aura|DealDamage|enemy:all|amount=1")
        self.assertEqual(effects, [])
        self.assertTrue(any("光环" in e for e in errors))

    def test_bad_condition_rejected(self):
        effects, errors = vd.parse_effects(
            "ConditionalAura|ModifyStats|self|attack=1,duration=permanent,condition=count:friendly:human:>=2"
        )
        self.assertEqual(effects, [])
        self.assertTrue(any("condition" in e for e in errors))

    def test_copy_accepts_card_type_filter(self):
        effects, errors = vd.parse_effects("OnPlayEffect|Copy|self|from=hand,type=minion,to=hand")
        self.assertEqual(errors, [])
        self.assertEqual(len(effects), 1)


class PlaystyleMechanicsTest(unittest.TestCase):
    """新关键词、原语与原语玩法归属的校验；只依赖合成 YAML。"""

    def _yaml(self, body):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "doc.md"
            path.write_text("---\n" + body + "\n---\n", encoding="utf-8")
            return vd.parse_front_matter(path)

    def test_armor_and_detonate_skill_accepted(self):
        skill = self._yaml(
            "id: test-skill\n"
            "text: 战斗开始时：引爆\n"
            "keyword: armor 2\n"
            "effects: OnCombatStart|Detonate|enemy:single|amount=2,threshold=3"
        )
        self.assertEqual(skill["keyword"], "armor 2")
        effects, errors = vd.parse_effects(skill["effects"])
        self.assertEqual(errors, [])
        self.assertEqual(effects[0]["op"], "Detonate")
        granted, errors = vd.parse_effects("OnDeath|GrantKeyword|self|keyword=armor 2")
        self.assertEqual(errors, [])
        self.assertEqual(granted[0]["params"]["keyword"], "armor 2")

    def test_poison_rejected_for_mammal(self):
        bad = self._yaml("id: bad-poison\nplaystyle: mammal\nskills: [test-skill]")
        self.assertTrue(vd.keyword_errors(bad, "poison"))
        good = self._yaml("id: ok-poison\nplaystyle: reptile\nskills: [test-skill]")
        self.assertEqual(vd.keyword_errors(good, "poison"), [])

    def test_mimic_rejected_outside_cephalopod(self):
        skill_map = {
            "test-mimic": self._yaml(
                "id: test-mimic\neffects: OnPlay|Mimic|enemy:single|duration=combat"
            )
        }
        bad = self._yaml("id: bad-mimic\nplaystyle: mammal\nskills: [test-mimic]")
        self.assertTrue(vd.mechanism_errors(bad, skill_map))
        good = self._yaml("id: ok-mimic\nplaystyle: cephalopod\nskills: [test-mimic]")
        self.assertEqual(vd.mechanism_errors(good, skill_map), [])


if __name__ == "__main__":
    unittest.main()
