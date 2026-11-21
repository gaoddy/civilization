import importlib.util
import struct
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
_spec = importlib.util.spec_from_file_location(
    "gen_ui_assets", ROOT / "scripts" / "gen_ui_assets.py"
)
gen = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(gen)


class BackgroundTextureTest(unittest.TestCase):
    def test_png_signature_and_size(self):
        data = gen.render_background(64)
        self.assertEqual(data[:8], b"\x89PNG\r\n\x1a\n")
        width, height = struct.unpack(">II", data[16:24])
        self.assertEqual((width, height), (64, 64))

    def test_deterministic(self):
        self.assertEqual(gen.render_background(64), gen.render_background(64))

    def test_no_pillow_dependency(self):
        source = (ROOT / "scripts" / "gen_ui_assets.py").read_text(encoding="utf-8")
        self.assertNotIn("PIL", source)
