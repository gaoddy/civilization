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
        self.assertEqual(len(data), 432)
        self.assertEqual(sum(c["card_type"] == "minion" for c in data), 332)
        self.assertEqual([c["id"] for c in data], sorted(c["id"] for c in data))

    def test_skills_and_non_pool_cards(self):
        out = gen_card_data.build(ROOT)
        self.assertNotIn("tokens.json", out)
        skills_dir = ROOT / "docs/current/product/content/skills"
        expected = sum(1 for path in skills_dir.glob("*.md") if path.name != "README.md")
        self.assertEqual(len(json.loads(out["skills.json"])), expected)
        cards = json.loads(out["cards.json"])
        non_pool = {c["id"] for c in cards if not c["in_shop_pool"]}
        self.assertLessEqual(
            {"hela-cell-token", "seedling", "synthetic-cell-token", "triple-reward"},
            non_pool)

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
        text = (ROOT / "docs/current/product/systems/economy-and-numbers.md").read_text()
        costs = [int(n) for n in re.findall(r"\| [^|]+ → [^|]+ \| (\d+) \|", text)]
        self.assertEqual(costs, [5, 7, 9, 10, 11, 12])
        self.assertEqual(cfg["era_costs"], costs)
        for row in cfg["shop_weights"]:
            self.assertEqual(len(row), 7)


class GenClientDataTest(unittest.TestCase):
    def _bundle(self):
        return json.loads(gen_client_data.build(ROOT)[gen_client_data.FILE])

    def test_card_and_skill_counts(self):
        bundle = self._bundle()
        cards = bundle["cards"]
        skills = bundle["skills"]
        self.assertEqual(len(cards), 432)
        self.assertEqual(len(skills), 255)
        heroes = bundle["heroes"]
        self.assertEqual(len(heroes), 32)
        self.assertEqual([hero["id"] for hero in heroes],
                         sorted(hero["id"] for hero in heroes))
        self.assertEqual([card["id"] for card in cards], sorted(card["id"] for card in cards))

    def test_sections_match_server_tables(self):
        """下发内容与规则数据同源：只有 cards 额外附展示费用。"""
        bundle = self._bundle()
        tables = gen_card_data.build(ROOT)
        for name in ("heroes", "skills", "taxonomy"):
            self.assertEqual(bundle[name], json.loads(tables[f"{name}.json"]), name)
        server_cards = json.loads(tables["cards.json"])
        stripped = [
            {key: value for key, value in card.items() if key != "display_cost"}
            for card in bundle["cards"]
        ]
        self.assertEqual(stripped, server_cards)

    def test_version_is_content_hash(self):
        bundle = self._bundle()
        sections = {name: bundle[name] for name in gen_client_data.SECTIONS}
        self.assertEqual(bundle["version"], gen_client_data.content_version(sections))
        self.assertRegex(bundle["version"], r"^[0-9a-f]{16}$")

    def test_display_cost(self):
        cards = self._bundle()["cards"]
        for card in cards:
            if card["card_type"] == "minion":
                self.assertEqual(card["display_cost"], 3, card["id"])
            else:
                self.assertIsInstance(card["display_cost"], int, card["id"])

    def test_non_pool_cards_have_display_cost(self):
        bundle = self._bundle()
        non_pool = [
            card for card in bundle["cards"]
            if not card["in_shop_pool"] and card["card_type"] == "minion"
        ]
        self.assertLessEqual(
            {"hela-cell-token", "seedling", "synthetic-cell-token"}, {c["id"] for c in non_pool})
        for card in non_pool:
            self.assertEqual(card["display_cost"], 3)

    def test_build_is_deterministic(self):
        self.assertEqual(gen_client_data.build(ROOT), gen_client_data.build(ROOT))

    def test_check_mode_detects_drift(self):
        self.assertEqual(subprocess.run([sys.executable, "scripts/gen_client_data.py", "--check"],
                                        cwd=ROOT).returncode, 0)


if __name__ == "__main__":
    unittest.main()
