#!/usr/bin/env python3
"""卡牌与服务端/客户端数据生成器测试：数量、展示费用、确定性与漂移检测。"""

import json
import pathlib
import re
import subprocess
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import gen_card_data  # noqa: E402
import gen_client_data  # noqa: E402


class GenCardDataTest(unittest.TestCase):
    def test_counts_and_sorted_ids(self):
        data = json.loads(gen_card_data.build(ROOT)["cards.json"])
        self.assertEqual(len(data), 340)
        self.assertEqual(sum(c["card_type"] == "minion" for c in data), 303)
        self.assertEqual([c["id"] for c in data], sorted(c["id"] for c in data))

    def test_skills_and_tokens(self):
        out = gen_card_data.build(ROOT)
        self.assertEqual(len(json.loads(out["skills.json"])), 193)
        tokens = json.loads(out["tokens.json"])
        ids = [t["id"] for t in tokens]
        self.assertEqual(ids, sorted(ids))
        self.assertGreaterEqual(len(ids), 3)
        self.assertLessEqual(
            {"hela-cell-token", "seedling", "synthetic-cell-token"}, set(ids))

    def test_skill_effects_are_structured(self):
        skills = json.loads(gen_card_data.build(ROOT)["skills.json"])
        skill = next(s for s in skills if s["id"] == "division-of-labor")
        self.assertIsInstance(skill["effects"], list)
        self.assertEqual(skill["effects"][0]["trigger"], "on_activate")
        self.assertEqual(skill["effects"][0]["selector"]["kind"], "side")

    def test_build_is_deterministic(self):
        self.assertEqual(gen_card_data.build(ROOT), gen_card_data.build(ROOT))

    def test_check_mode_detects_drift(self):
        self.assertEqual(subprocess.run([sys.executable, "scripts/gen_card_data.py", "--check"],
                         cwd=ROOT).returncode, 0)

    def test_config_matches_economy_doc(self):
        cfg = json.loads(gen_card_data.build(ROOT)["config.json"])
        text = (ROOT / "docs/versions/v0.1/product/systems/economy-and-numbers.md").read_text()
        costs = [int(n) for n in re.findall(r"\| [^|]+ → [^|]+ \| (\d+) \|", text)]
        self.assertEqual(costs, [5, 7, 9, 10, 11, 12])
        self.assertEqual(cfg["era_costs"], costs)
        for row in cfg["shop_weights"]:
            self.assertEqual(len(row), 7)


class GenClientDataTest(unittest.TestCase):
    def test_card_and_skill_counts(self):
        data = gen_client_data.build(ROOT)
        cards = json.loads(data["cards.json"])
        skills = json.loads(data["skills.json"])
        self.assertEqual(len(cards), 340)
        self.assertEqual(len(skills), 193)
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
        ids = [token["id"] for token in tokens]
        self.assertEqual(ids, sorted(ids))
        self.assertGreaterEqual(len(ids), 3)
        self.assertLessEqual(
            {"hela-cell-token", "seedling", "synthetic-cell-token"}, set(ids))
        for token in tokens:
            self.assertEqual(token["display_cost"], 3)

    def test_build_is_deterministic(self):
        self.assertEqual(gen_client_data.build(ROOT), gen_client_data.build(ROOT))

    def test_check_mode_detects_drift(self):
        self.assertEqual(subprocess.run([sys.executable, "scripts/gen_client_data.py", "--check"],
                                        cwd=ROOT).returncode, 0)


if __name__ == "__main__":
    unittest.main()
