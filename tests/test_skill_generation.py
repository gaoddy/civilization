import pathlib
import sys
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import audit_effects
import gen_card_data
import render_docs
import skill_templates
import validate_docs

CONTENT = ROOT / "docs/versions/v0.1/product/content"


def build_fixture(tmp):
    """构造最小内容目录：一条模板、一个技能实例、一张引用它的卡牌。"""
    content = pathlib.Path(tmp) / "content"
    (content / "skill-templates").mkdir(parents=True)
    (content / "tokens").mkdir()
    (content / "skills").mkdir()
    (content / "cards").mkdir()
    (content / "heroes").mkdir()
    (content / "taxonomy.md").write_text(
        "## 1. 科学分类树\n\n"
        "| id | 名称 | 上级 | 层级 | 备注 |\n"
        "| --- | --- | --- | --- | --- |\n"
        "| animalia | 动物界 | | 界 | |\n\n"
        "## 2. 玩法\n\n"
        "| id | 名称 | 机制 | 核心循环 |\n"
        "| --- | --- | --- | --- |\n"
        "| insect | 昆虫纲 | 繁衍 | 铺场 |\n",
        encoding="utf-8")

    (content / "skill-templates" / "on-death-summon.md").write_text(
        "---\nid: on-death-summon\nkind: effect\ntrigger: OnDeath\nop: Summon\n"
        "selector: self\narg.token: token\narg.count: count 1..3 default=1\n"
        "params: token={token},count={count}\n"
        "text: 亡语：召唤{count.cn}个 {token.stats} 的{token.name}\n---\n",
        encoding="utf-8")
    (content / "card-nouns.md").write_text(
        "## 1. 分类节点\n\n| 键 | 名词 |\n| --- | --- |\n| animalia | 动物 |\n\n"
        "## 2. 玩法\n\n| 键 | 名词 |\n| --- | --- |\n| insect | 昆虫纲 |\n\n"
        "## 3. 特殊筛选值\n\n| 键 | 名词 |\n| --- | --- |\n| scholar=true | 学者 |\n\n"
        "## 4. 关键词文案\n\n| 键 | 文案 |\n| --- | --- |\n"
        "| deathrattle | 亡语：死亡时触发 |\n| taunt | 嘲讽 |\n",
        encoding="utf-8")
    (content / "tokens" / "README.md").write_text(
        "## 2. 注册表\n\n| id | 名称 | 攻击力 | 生命值 | 关键词 | 玩法 | 科学分类 | 引用 |\n"
        "| --- | --- | --- | --- | --- | --- | --- | --- |\n"
        "| `baiyi` | 白蚁 | 1 | 2 | 无 | insect | animalia | 1 |\n",
        encoding="utf-8")
    (content / "skills" / "x.md").write_text(
        "---\nid: x\ntemplate: on-death-summon(token=baiyi,count=1)\n"
        "keyword: deathrattle\n---\n\n# x\n\n## 属性\n\n| 属性 | 值 |\n| --- | --- |\n"
        "| id | x |\n| 关键词 | 亡语 |\n| 确定性替换 | 不适用 |\n\n"
        "## 效果\n\n旧文案\n",
        encoding="utf-8")
    (content / "cards" / "c.md").write_text(
        "---\nid: c\ncard_type: minion\nname: 测试随从\ncategories: [play:insect]\n"
        "taxon: animalia\nsubtype: insect\nlevel: 1\n"
        "era: prehistoric\nattack: 1\nhealth: 2\nskills: [x]\nis_scholar: false\n---\n\n"
        "# 测试随从\n\n## 属性\n\n| 属性 | 值 |\n| --- | --- |\n"
        "| id | c |\n| 名称 | 测试随从 |\n| 关键词 | 无 |\n| 技能 | 无 |\n\n"
        "## 效果\n\n旧文案\n\n## 设计说明\n\n定位：测试。\n\n## 出处\n\n测试出处。\n",
        encoding="utf-8")
    (content / "heroes" / "c.md").write_text(
        "---\nid: c\nname: 测试随从\n"
        "categories: [play:insect]\nskills: [x]\n---\n\n"
        "# 测试随从\n\n## 属性\n\n| 属性 | 值 |\n| --- | --- |\n"
        "| id | c |\n| 名称 | 测试随从 |\n| 玩法 | 中立 |\n"
        "| 关联分类 | 无 |\n| 技能 | x |\n\n"
        "## 效果\n\n亡语：召唤一个 1/2 的白蚁\n",
        encoding="utf-8")
    return content


class RenderDocsTest(unittest.TestCase):
    def test_render_skill_attrs_and_body(self):
        with tempfile.TemporaryDirectory() as tmp:
            content = build_fixture(tmp)
            render_docs.render(content, write=True)
            text = (content / "skills" / "x.md").read_text(encoding="utf-8")
            self.assertIn("| 模板 | on-death-summon |", text)
            self.assertIn("## 效果\n\n亡语：召唤一个 1/2 的白蚁", text)

    def test_render_preserves_handwritten_sections(self):
        with tempfile.TemporaryDirectory() as tmp:
            content = build_fixture(tmp)
            render_docs.render(content, write=True)
            text = (content / "cards" / "c.md").read_text(encoding="utf-8")
            self.assertIn("## 设计说明\n\n定位：测试。", text)
            self.assertIn("## 出处\n\n测试出处。", text)

    def test_render_hero_attrs_uses_categories(self):
        with tempfile.TemporaryDirectory() as tmp:
            content = build_fixture(tmp)
            render_docs.render(content, write=True)
            text = (content / "heroes" / "c.md").read_text(encoding="utf-8")
            self.assertIn("| 玩法 | 昆虫纲 |", text)
            self.assertNotIn("关联分类", text)

    def test_render_check_reports_drift_without_writing(self):
        with tempfile.TemporaryDirectory() as tmp:
            content = build_fixture(tmp)
            path = content / "skills" / "x.md"
            path.write_text(path.read_text(encoding="utf-8").replace("旧文案", "旧内容"),
                            encoding="utf-8")
            drift = render_docs.render(content, write=False)
            self.assertTrue(drift)
            self.assertIn("旧内容", path.read_text(encoding="utf-8"))


class GeneratorTemplateTest(unittest.TestCase):
    def setUp(self):
        self.renderer = skill_templates.Renderer()

    def test_skill_entry_requires_template(self):
        legacy = {"id": "x", "keyword": "deathrattle",
                  "effects": "OnDeath|Summon|self|token=baiyi,count=1"}
        with self.assertRaisesRegex(ValueError, "template"):
            gen_card_data.skill_entry(legacy, self.renderer)

    def test_skill_text_template_matches_renderer(self):
        rendered = self.renderer.instance_text(
            {"id": "x", "template": "on-death-summon(token=baiyi,count=1)"})
        self.assertEqual(rendered, "亡语：召唤一个 2/2 的白蚁")


class AuditTemplateTest(unittest.TestCase):
    def test_audit_expands_template_instance(self):
        with tempfile.TemporaryDirectory() as tmp:
            content = build_fixture(tmp)
            report = audit_effects.audit(content / "skills")
            self.assertEqual(report.violations, [])
            self.assertEqual(report.matrix, {("OnDeath", "Summon"): 1})


class CollectSkillErrorsTest(unittest.TestCase):
    def test_check_skills_reports_template_error_without_crashing(self):
        skill = {"id": "bad", "_file": "README.md", "template": "missing-template"}
        saved = validate_docs.errors
        validate_docs.errors = []
        try:
            with mock.patch.object(validate_docs, "load", return_value=[skill]):
                validate_docs.check_skills()
            messages = list(validate_docs.errors)
        finally:
            validate_docs.errors = saved
        self.assertTrue(any("模板不存在" in message for message in messages))

    def test_check_skills_rejects_effects_field(self):
        skill = {"id": "bad", "_file": "README.md",
                 "template": "on-death-summon(token=baiyi)",
                 "effects": "OnDeath|Summon|self|token=baiyi"}
        saved = validate_docs.errors
        validate_docs.errors = []
        try:
            with mock.patch.object(validate_docs, "load", return_value=[skill]):
                validate_docs.check_skills()
            messages = list(validate_docs.errors)
        finally:
            validate_docs.errors = saved
        self.assertTrue(any("未知字段" in message and "effects" in message
                            for message in messages))

    def test_check_skills_reports_template_load_failure(self):
        saved = validate_docs.errors
        validate_docs.errors = []
        try:
            with mock.patch.object(validate_docs.skill_templates, "load_templates",
                                   side_effect=ValueError("坏模板")):
                validate_docs.check_skills()
            messages = list(validate_docs.errors)
        finally:
            validate_docs.errors = saved
        self.assertTrue(any("技能模板加载失败" in message for message in messages))


class ValidateTemplateTest(unittest.TestCase):
    def setUp(self):
        self.renderer = skill_templates.Renderer()

    def test_expected_attrs_include_templates(self):
        skill = {"id": "x", "template": "on-death-summon(token=termite,count=1)"}
        attrs = validate_docs.expected_skill_attrs(skill)
        self.assertEqual(attrs["模板"], "on-death-summon")

    def test_missing_template_reported(self):
        errors = validate_docs.validate_skill_template(
            {"id": "x", "template": "not-a-template", "_file": "x.md"}, {},
            self.renderer)
        self.assertTrue(any("模板不存在" in e for e in errors))


if __name__ == "__main__":
    unittest.main()
