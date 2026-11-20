import pathlib
import tempfile
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import skill_templates


class NounTableTest(unittest.TestCase):
    def test_load_nouns_requires_all_tables(self):
        nouns = skill_templates.load_nouns(
            ROOT / "docs/versions/v0.1/product/content/card-nouns.md")
        self.assertEqual(nouns.taxon["animalia"], "动物")
        self.assertEqual(nouns.playstyle["machine"], "机械")
        self.assertEqual(nouns.special["scholar=true"], "学者")
        self.assertEqual(nouns.keyword_text["taunt"], "嘲讽")

    def test_parse_arg_spec_range_and_default(self):
        spec = skill_templates.parse_arg_spec("count", "count 1..3 default=1")
        self.assertEqual((spec.type, spec.minimum, spec.maximum, spec.default),
                         ("count", 1, 3, "1"))
        plain = skill_templates.parse_arg_spec("taxon", "taxon")
        self.assertEqual((plain.type, plain.minimum, plain.default), ("taxon", None, None))


class TemplateParseTest(unittest.TestCase):
    def test_parse_template_effect(self):
        doc = {
            "id": "on-death-summon", "kind": "effect", "trigger": "OnDeath",
            "op": "Summon", "selector": "self",
            "arg.token": "token", "arg.count": "count 1..3 default=1",
            "params": "token={token},count={count}",
            "text": "亡语：召唤{count.cn}个{token.stats}的{token.name}",
        }
        template = skill_templates.parse_template(doc, "on-death-summon.md")
        self.assertEqual(template.trigger, "OnDeath")
        self.assertEqual([a.name for a in template.args], ["token", "count"])

    def test_parse_template_field(self):
        calls = skill_templates.parse_template_field(
            "on-death-summon(token=termite,count=1); keyword-self(keyword=taunt)")
        self.assertEqual([c.template_id for c in calls],
                         ["on-death-summon", "keyword-self"])
        self.assertEqual(calls[1].args, {"keyword": "taunt"})

    def test_parse_template_rejects_unknown_field(self):
        with self.assertRaises(ValueError):
            skill_templates.parse_template(
                {"id": "x", "trigger": "OnDeath", "op": "Summon",
                 "selector": "self", "text": "a", "unexpected": "1"}, "x.md")

    def test_load_templates_reads_documents(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "skill-templates"
            path.mkdir()
            (path / "on-death-summon.md").write_text(
                "---\nid: on-death-summon\ntrigger: OnDeath\nop: Summon\n"
                "selector: self\narg.token: token\n"
                "params: token={token}\ntext: 亡语：召唤{token.name}\n---\n",
                encoding="utf-8")
            templates = skill_templates.load_templates(pathlib.Path(tmp))
            self.assertIn("on-death-summon", templates)
