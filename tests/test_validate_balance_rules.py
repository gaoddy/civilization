import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import validate_docs as vd  # noqa: E402


class EffectivePlaystylesTest(unittest.TestCase):
    def test_effect_returns_all_play_categories(self):
        doc = {"card_type": "effect",
               "categories": ["play:fish", "play:machine"]}
        self.assertEqual(vd.effective_playstyles(doc), ["fish", "machine"])

    def test_minion_returns_all_play_categories(self):
        doc = {"card_type": "minion",
               "categories": ["play:mammal", "play:bird", "prey:artiodactyla"]}
        self.assertEqual(vd.effective_playstyles(doc), ["mammal", "bird"])

    def test_without_play_returns_empty(self):
        doc = {"card_type": "minion", "categories": ["prey:artiodactyla"]}
        self.assertEqual(vd.effective_playstyles(doc), [])

    def test_missing_categories_returns_empty(self):
        self.assertEqual(vd.effective_playstyles({"card_type": "effect"}), [])


class CategoryErrorsTest(unittest.TestCase):
    def test_single_play_passes(self):
        self.assertEqual(vd.category_errors({"id": "c", "categories": ["play:mammal"]}), [])

    def test_multi_play_in_order_passes(self):
        doc = {"id": "c", "categories": ["play:mammal", "play:machine"]}
        self.assertEqual(vd.category_errors(doc), [])

    def test_missing_play_is_rejected(self):
        errors = vd.category_errors({"id": "c", "categories": ["prey:mammalia"]})
        self.assertTrue(any("至少要有 1 条 play" in e for e in errors), errors)

    def test_neutral_play_is_rejected(self):
        errors = vd.category_errors({"id": "c", "categories": ["play:neutral"]})
        self.assertTrue(any("13 个玩法" in e for e in errors), errors)

    def test_duplicate_category_is_rejected(self):
        errors = vd.category_errors(
            {"id": "c", "categories": ["play:mammal", "play:mammal"]})
        self.assertTrue(any("重复" in e for e in errors), errors)

    def test_play_order_is_enforced(self):
        errors = vd.category_errors(
            {"id": "c", "categories": ["play:bird", "play:mammal"]})
        self.assertTrue(any("顺序" in e for e in errors), errors)

    def test_unknown_prefix_is_rejected(self):
        errors = vd.category_errors({"id": "c", "categories": ["play:mammal", "boss:bee"]})
        self.assertTrue(any("前缀非法" in e for e in errors), errors)


class CategoryTaxonomyTest(unittest.TestCase):
    """加载分类树后覆盖 prey/host 的生物限制、节点存在性与同类顺序。"""

    NODES = {
        "animalia": {"parent": None, "rank": "kingdom", "name": "动物界"},
        "mammalia": {"parent": "animalia", "rank": "class", "name": "哺乳纲"},
        "artiodactyla": {"parent": "mammalia", "rank": "order", "name": "偶蹄目"},
        "craft": {"parent": None, "rank": "kingdom", "name": "造物"},
        "power-energy": {"parent": "craft", "rank": "technique", "name": "动力与能源"},
    }

    def setUp(self):
        self._nodes = vd.TAXONOMY_NODES
        self._order = vd.TAXONOMY_ORDER
        vd.TAXONOMY_NODES = dict(self.NODES)
        vd.TAXONOMY_ORDER = {node_id: index for index, node_id in enumerate(self.NODES)}

    def tearDown(self):
        vd.TAXONOMY_NODES = self._nodes
        vd.TAXONOMY_ORDER = self._order

    def test_non_biological_card_cannot_have_prey(self):
        doc = {"id": "c", "taxon": "power-energy",
               "categories": ["play:machine", "prey:artiodactyla"]}
        errors = vd.category_errors(doc)
        self.assertTrue(any("只允许生物卡" in e for e in errors), errors)

    def test_unknown_prey_node_is_rejected(self):
        doc = {"id": "c", "taxon": "mammalia",
               "categories": ["play:mammal", "prey:unknown-node"]}
        errors = vd.category_errors(doc)
        self.assertTrue(any("不在分类树" in e for e in errors), errors)

    def test_host_must_come_after_prey(self):
        doc = {"id": "c", "taxon": "mammalia",
               "categories": ["play:mammal", "host:animalia", "prey:artiodactyla"]}
        errors = vd.category_errors(doc)
        self.assertTrue(any("顺序" in e for e in errors), errors)


class SkillCountTest(unittest.TestCase):
    def setUp(self):
        self.skill_map = {
            "k1": {"keyword": "taunt"},
            "k2": {"keyword": "windfury"},
            "k3": {"keyword": "poison"},
            "p": {},
        }

    def test_one_two_star_allows_one(self):
        doc = {"level": 2, "categories": ["play:mammal"], "taxon": "hominidae",
               "is_scholar": False, "skills": ["k1", "k2"]}
        errors = vd.skill_count_errors(doc, self.skill_map)
        self.assertTrue(any("最多 1 个技能" in e for e in errors))

    def test_three_four_star_allows_two(self):
        doc = {"level": 4, "categories": ["play:mammal"], "taxon": "hominidae",
               "is_scholar": False, "skills": ["k1", "k2"]}
        self.assertEqual(vd.skill_count_errors(doc, self.skill_map), [])

    def test_five_seven_star_max_two_keywords(self):
        doc = {"level": 6, "categories": ["play:mammal"], "taxon": "hominidae",
               "is_scholar": False, "skills": ["k1", "k2", "k3", "p"]}
        errors = vd.skill_count_errors(doc, self.skill_map)
        self.assertTrue(any("最多 2 个登记关键词" in e for e in errors))

    def test_skillless_tool_card_allowed(self):
        doc = {"level": 3, "categories": ["play:machine"], "taxon": "life-medical",
               "is_scholar": False, "skills": []}
        self.assertEqual(vd.skill_count_errors(doc, self.skill_map), [])

    def test_skillless_non_tool_rejected(self):
        doc = {"level": 2, "categories": ["play:bird"], "taxon": "archaeopterygidae",
               "is_scholar": False, "skills": []}
        errors = vd.skill_count_errors(doc, self.skill_map)
        self.assertTrue(any("无技能" in e for e in errors))


if __name__ == "__main__":
    unittest.main()
