import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "process_assets.py"


def run_assets(root: Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            "--map",
            str(root / "asset_map.json"),
            "--portraits",
            str(root / "portraits"),
            "--avatars",
            str(root / "avatars"),
            "--faces",
            str(root / "faces"),
            "--manifest",
            str(root / "asset_manifest.json"),
        ],
        capture_output=True,
        text=True,
    )


class ManifestMergeTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        for name in ("portraits", "avatars", "faces"):
            (self.root / name).mkdir()
        Image.new("RGB", (1024, 1536), (10, 12, 20)).save(
            self.root / "portraits" / "curie_src.png"
        )

    def tearDown(self):
        self._tmp.cleanup()

    def test_keeps_entries_outside_asset_map(self):
        manifest = self.root / "asset_manifest.json"
        manifest.write_text(
            json.dumps(
                {
                    "version": 1,
                    "assets": {
                        "agi": {
                            "card_face": "res://assets/cards/faces/card_agi_face.png",
                            "display": {"card_face": "112x146"},
                        }
                    },
                }
            )
        )
        (self.root / "asset_map.json").write_text(
            json.dumps({"marie-curie": "curie_src.png"})
        )

        result = run_assets(self.root)

        self.assertEqual(result.returncode, 0, result.stderr)
        assets = json.loads(manifest.read_text())["assets"]
        self.assertIn("marie-curie", assets)
        self.assertEqual(
            assets["agi"]["card_face"], "res://assets/cards/faces/card_agi_face.png"
        )

    def test_writes_manifest_when_absent(self):
        (self.root / "asset_map.json").write_text(
            json.dumps({"marie-curie": "curie_src.png"})
        )

        result = run_assets(self.root)

        self.assertEqual(result.returncode, 0, result.stderr)
        assets = json.loads((self.root / "asset_manifest.json").read_text())["assets"]
        self.assertEqual(
            assets["marie-curie"]["card_face"],
            "res://assets/cards/faces/card_marie-curie_face.png",
        )


if __name__ == "__main__":
    unittest.main()
