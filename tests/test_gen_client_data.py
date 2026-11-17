#!/usr/bin/env python3
"""客户端卡牌展示表生成器测试：数量、展示费用、确定性与漂移检测。"""

import json
import pathlib
import subprocess
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import gen_client_data  # noqa: E402


class GenClientDataTest(unittest.TestCase):
    def test_card_and_skill_counts(self):
        data = gen_client_data.build(ROOT)
        cards = json.loads(data["cards.json"])
        skills = json.loads(data["skills.json"])
        self.assertEqual(len(cards), 239)
        self.assertEqual(len(skills), 131)
        self.assertEqual(sum(card["is_hero"] for card in cards), 32)
        self.assertEqual([card["id"] for card in cards], sorted(card["id"] for card in cards))

    def test_display_cost(self):
        cards = json.loads(gen_client_data.build(ROOT)["cards.json"])
        for card in cards:
            if card["card_type"] == "minion":
                self.assertEqual(card["display_cost"], 3, card["id"])
            else:
                self.assertIsInstance(card["display_cost"], int, card["id"])

    def test_tokens_have_display_cost(self):
        tokens = json.loads(gen_client_data.build(ROOT)["tokens.json"])
        self.assertEqual([token["id"] for token in tokens],
                         ["hela-cell-token", "seedling", "synthetic-cell-token"])
        for token in tokens:
            self.assertEqual(token["display_cost"], 3)

    def test_skill_effects_are_structured(self):
        skills = json.loads(gen_client_data.build(ROOT)["skills.json"])
        skill = next(s for s in skills if s["id"] == "division-of-labor")
        self.assertIsInstance(skill["effects"], list)
        self.assertEqual(skill["effects"][0]["trigger"], "on_activate")

    def test_build_is_deterministic(self):
        self.assertEqual(gen_client_data.build(ROOT), gen_client_data.build(ROOT))

    def test_check_mode_detects_drift(self):
        self.assertEqual(subprocess.run([sys.executable, "scripts/gen_client_data.py", "--check"],
                                        cwd=ROOT).returncode, 0)


if __name__ == "__main__":
    unittest.main()
