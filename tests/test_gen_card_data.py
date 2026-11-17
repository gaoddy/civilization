import json, pathlib, re, subprocess, sys, unittest
ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import gen_card_data
class GenCardDataTest(unittest.TestCase):
    def test_counts_and_sorted_ids(self):
        data = json.loads(gen_card_data.build(ROOT)["cards.json"])
        self.assertEqual(len(data), 239)
        self.assertEqual(sum(c["card_type"] == "随从" for c in data), 203)
        self.assertEqual([c["id"] for c in data], sorted(c["id"] for c in data))
    def test_skills_and_tokens(self):
        out = gen_card_data.build(ROOT)
        self.assertEqual(len(json.loads(out["skills.json"])), 131)
        self.assertEqual([t["id"] for t in json.loads(out["tokens.json"])],
                         ["hela-cell-token", "seedling", "synthetic-cell-token"])
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
