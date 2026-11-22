import pathlib, sys, tempfile, unittest
ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import audit_effects
import skill_templates
import validate_docs
SKILLS = ROOT / "docs/current/product/content/skills"

def expected_counts():
    """按技能文档实际内容统计技能数与效果段数，避免写死数字。"""
    paths = [path for path in SKILLS.glob("*.md") if path.name != "README.md"]
    renderer = skill_templates.Renderer(SKILLS.parent)
    segments = 0
    for path in paths:
        doc = validate_docs.parse_front_matter(path)
        raw = ";".join(renderer.instance_segments(doc)) or "[]"
        effects, _ = validate_docs.parse_effects(raw)
        segments += len(effects)
    return len(paths), segments

class AuditEffectsTest(unittest.TestCase):
    def test_all_committed_skills_are_clean(self):
        report = audit_effects.audit(SKILLS)
        count, segments = expected_counts()
        self.assertEqual(report.violations, [])
        self.assertEqual(report.skill_count, count)
        self.assertEqual(sum(report.matrix.values()), segments)
    def test_times_if_without_times_is_reported(self):
        violations = audit_effects.check_effect(
            "OnPlay", "Growth", {"timesIf": "taxon=animalia:2"}, "friendly:single")
        self.assertTrue(any("timesIf" in v for v in violations))

    def test_when_on_play_is_reported(self):
        violations = audit_effects.check_effect(
            "OnPlay", "Growth", {"times": "1", "when": "taxon=animalia"}, "self")
        self.assertTrue(any("when" in v for v in violations))

    def test_missing_template_is_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = pathlib.Path(tmp) / "bad.md"
            p.write_text("---\nid: bad\ntext: 战吼：成长\n---\n", encoding="utf-8")
            violations = audit_effects.audit(pathlib.Path(tmp)).violations
            self.assertTrue(any("template" in v for v in violations))
