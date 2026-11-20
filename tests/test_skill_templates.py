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


class RenderTest(unittest.TestCase):
    def setUp(self):
        self.renderer = skill_templates.Renderer()
        self.summon_one = skill_templates.TemplateCall(
            "on-death-summon", {"token": "baiyi", "count": "1"})
        self.summon_two = skill_templates.TemplateCall(
            "on-death-summon", {"token": "baiyi", "count": "2"})

    def test_expand_effect(self):
        self.assertEqual(
            self.renderer.expand(self.summon_one),
            ("OnDeath|Summon|self|token=baiyi,count=1",))

    def test_expand_omits_missing_arg(self):
        call = skill_templates.TemplateCall(
            "aura-adjacent-playstyle-stats",
            {"playstyle": "mammal", "attack": "2", "health": "2"})
        self.assertEqual(self.renderer.expand(call),
                         ("Aura|ModifyStats|friendly:adjacent:playstyle=mammal|attack=2,health=2",))

    def test_text_count_and_stats(self):
        self.assertEqual(self.renderer.text(self.summon_two),
                         "亡语：召唤两个 2/2 的白蚁")

    def test_keyword_text_substitutes_level(self):
        call = skill_templates.TemplateCall("keyword-self", {"keyword": "armor 2"})
        self.assertEqual(self.renderer.text(call), "减伤 2：受到伤害时，减免 2 点伤害")

    def test_duration_suffix(self):
        template = skill_templates.Template(
            "t", "effect", "OnPlayEffect", "GrantKeyword", "friendly:single",
            "keyword={keyword},duration={duration}",
            "使一个友方随从获得{keyword}{duration.suffix}",
            (skill_templates.ArgSpec("keyword", "keyword"),
             skill_templates.ArgSpec("duration", "duration")))
        self.renderer.templates["t"] = template
        call = skill_templates.TemplateCall(
            "t", {"keyword": "taunt", "duration": "combat"})
        self.assertEqual(self.renderer.text(call), "使一个友方随从获得嘲讽直到回合结束")

    def test_multiple_calls_bind_args_per_template(self):
        doc = {"template": "on-activate-refresh-shop; on-activate-gain-minion"}
        self.assertEqual(self.renderer.instance_segments(doc), (
            "OnActivate|RefreshShop|self|free=true",
            "OnActivate|GainMinion|self|from=shop,level=current,to=hand",
        ))

    def test_effect_params_split_keyword_components(self):
        template = skill_templates.Template(
            "t", "effect", "OnPlayEffect", "GrantKeyword", "friendly:single",
            "keyword={keyword.name},value={keyword.level}", "",
            (skill_templates.ArgSpec("keyword", "keyword"),))
        self.renderer.templates["t"] = template
        call = skill_templates.TemplateCall("t", {"keyword": "armor 2"})
        self.assertEqual(self.renderer.expand(call),
                         ("OnPlayEffect|GrantKeyword|friendly:single|keyword=armor,value=2",))

    def test_effect_params_drop_empty_component(self):
        template = skill_templates.Template(
            "t", "effect", "OnPlayEffect", "GrantKeyword", "friendly:single",
            "keyword={keyword.name},value={keyword.level}", "",
            (skill_templates.ArgSpec("keyword", "keyword"),))
        self.renderer.templates["t"] = template
        call = skill_templates.TemplateCall("t", {"keyword": "taunt"})
        self.assertEqual(self.renderer.expand(call),
                         ("OnPlayEffect|GrantKeyword|friendly:single|keyword=taunt",))

    def test_camel_case_placeholder(self):
        template = skill_templates.Template(
            "t", "effect", "Aura", "ModifyStats", "friendly:all",
            "perHealth={perHealth}", "+0/+{perHealth}",
            (skill_templates.ArgSpec("perHealth", "int"),))
        self.renderer.templates["t"] = template
        call = skill_templates.TemplateCall("t", {"perHealth": "1"})
        self.assertEqual(self.renderer.expand(call),
                         ("Aura|ModifyStats|friendly:all|perHealth=1",))
        self.assertEqual(self.renderer.text(call), "+0/+1")

    def test_duration_suffix_spaces_after_number(self):
        template = skill_templates.Template(
            "t", "effect", "OnPlayEffect", "ModifyStats", "friendly:single",
            "", "获得 +{attack}/+{health}{duration.suffix}",
            (skill_templates.ArgSpec("attack", "int"),
             skill_templates.ArgSpec("health", "int"),
             skill_templates.ArgSpec("duration", "duration")))
        self.renderer.templates["t"] = template
        call = skill_templates.TemplateCall(
            "t", {"attack": "0", "health": "1", "duration": "combat"})
        self.assertEqual(self.renderer.text(call), "获得 +0/+1 直到回合结束")

    def test_text_override_wins(self):
        doc = {"template": "on-death-summon(token=baiyi,count=1)",
               "text": "亡语：召唤一个 1/2 的白蚁"}
        self.assertEqual(self.renderer.instance_text(doc),
                         "亡语：召唤一个 1/2 的白蚁")
