import pathlib, sys, tempfile, unittest
ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import audit_effects
SKILLS = ROOT / "docs/versions/v0.1/product/content/skills"

class AuditEffectsTest(unittest.TestCase):
    def test_all_committed_skills_are_clean(self):
        report = audit_effects.audit(SKILLS)
        self.assertEqual(report.violations, [])
        self.assertEqual(report.skill_count, 180)
        self.assertEqual(sum(report.matrix.values()), 168)  # 180 个技能中按实际有 effects 的技能汇总，共 168 段效果
    def test_times_if_without_times_is_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = pathlib.Path(tmp) / "bad.md"
            p.write_text("---\nid: bad\ntext: 战吼：成长\neffects: OnPlay|Growth|friendly:single|timesIf=human:2\n---\n")
            self.assertTrue(any("timesIf" in v for v in audit_effects.audit(pathlib.Path(tmp)).violations))
    def test_when_on_play_is_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = pathlib.Path(tmp) / "bad.md"
            p.write_text("---\nid: bad\ntext: 战吼：成长\neffects: OnPlay|Growth|self|times=1,when=tribe=human\n---\n")
            self.assertTrue(any("when" in v for v in audit_effects.audit(pathlib.Path(tmp)).violations))
