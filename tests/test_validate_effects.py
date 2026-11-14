#!/usr/bin/env python3
"""validate_docs.py 的 effects DSL 校验单元测试（标准库 unittest）。

运行：python3 -m unittest discover -s tests -v
"""

import pathlib
import sys
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


if __name__ == "__main__":
    unittest.main()
