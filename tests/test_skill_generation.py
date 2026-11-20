import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import gen_card_data
import skill_templates


class GeneratorTemplateTest(unittest.TestCase):
    def setUp(self):
        self.renderer = skill_templates.Renderer()

    def test_skill_entry_template_matches_legacy(self):
        legacy = {"id": "x", "keyword": "deathrattle",
                  "effects": "OnDeath|Summon|self|token=baiyi,count=1"}
        templated = {"id": "x", "keyword": "deathrattle",
                     "template": "on-death-summon(token=baiyi,count=1)"}
        self.assertEqual(gen_card_data.skill_entry(legacy, self.renderer),
                         gen_card_data.skill_entry(templated, self.renderer))

    def test_skill_text_template_matches_renderer(self):
        rendered = self.renderer.instance_text(
            {"id": "x", "template": "on-death-summon(token=baiyi,count=1)"})
        self.assertEqual(rendered, "亡语：召唤一个 2/2 的白蚁")


if __name__ == "__main__":
    unittest.main()
