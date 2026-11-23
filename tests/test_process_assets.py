import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

try:
    from PIL import Image

    HAS_PILLOW = True
except ImportError:  # scripts/process_assets.py 的运行时依赖，缺失时跳过
    HAS_PILLOW = False

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "process_assets.py"


def run_assets(root: Path, jobs: int | None = None) -> subprocess.CompletedProcess:
    args = [
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
    ]
    if jobs is not None:
        args.extend(["--jobs", str(jobs)])
    return subprocess.run(args, capture_output=True, text=True)


def run_covers(root: Path, jobs: int | None = None) -> subprocess.CompletedProcess:
    args = [
        sys.executable,
        str(SCRIPT),
        "--covers",
        str(root / "covers"),
        "--faces",
        str(root / "faces"),
        "--manifest",
        str(root / "asset_manifest.json"),
    ]
    if jobs is not None:
        args.extend(["--jobs", str(jobs)])
    return subprocess.run(args, capture_output=True, text=True)


def output_without_manifest_path(stdout: str) -> str:
    """去掉 manifest 绝对路径行，便于比较两次运行的其余日志。"""
    return "\n".join(
        line for line in stdout.splitlines() if not line.startswith("wrote ")
    )


def make_cover(
    root: Path,
    card_id: str,
    *,
    color=(10, 12, 20),
    name="Single_main.png",
    size=(448, 576),
    watermark=True,
    mtime=None,
) -> Path:
    card_dir = root / "covers" / card_id
    card_dir.mkdir(parents=True, exist_ok=True)
    im = Image.new("RGB", size, color)
    if watermark:
        for x in range(396, min(size[0], 444)):
            for y in range(550, min(size[1], 572)):
                im.putpixel((x, y), (240, 240, 240))
    path = card_dir / name
    im.save(path)
    if mtime is not None:
        os.utime(path, (mtime, mtime))
    return path


@unittest.skipUnless(HAS_PILLOW, "需要 Pillow（scripts/process_assets.py 的运行时依赖）")
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


@unittest.skipUnless(HAS_PILLOW, "需要 Pillow（scripts/process_assets.py 的运行时依赖）")
class CoverPipelineTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        for name in ("covers", "faces"):
            (self.root / name).mkdir()

    def tearDown(self):
        self._tmp.cleanup()

    def test_clean_covers_resize_and_register_card_face(self):
        make_cover(self.root, "aolinpike-jidian")

        result = run_covers(self.root)

        self.assertEqual(result.returncode, 0, result.stderr)
        face_path = self.root / "faces" / "card_aolinpike-jidian_face.png"
        face = Image.open(face_path)
        self.assertEqual(face.size, (448, 584))
        self.assertLess(max(face.getpixel((420, 568))[:3]), 80)
        assets = json.loads(
            (self.root / "asset_manifest.json").read_text()
        )["assets"]
        self.assertEqual(
            assets["aolinpike-jidian"],
            {
                "card_face": (
                    "res://assets/cards/faces/card_aolinpike-jidian_face.png"
                ),
                "display": {"card_face": "112x146"},
            },
        )

    def test_cover_merge_keeps_existing_portrait_fields(self):
        manifest = self.root / "asset_manifest.json"
        manifest.write_text(
            json.dumps(
                {
                    "version": 1,
                    "assets": {
                        "aolinpike-jidian": {
                            "portrait": "res://p.png",
                            "avatar": "res://a.png",
                            "card_face": "res://old.png",
                            "display": {
                                "portrait": "200x280",
                                "avatar": "22/30/38",
                                "card_face": "old",
                            },
                        }
                    },
                }
            )
        )
        make_cover(self.root, "aolinpike-jidian")

        result = run_covers(self.root)

        self.assertEqual(result.returncode, 0, result.stderr)
        entry = json.loads(manifest.read_text())["assets"]["aolinpike-jidian"]
        self.assertEqual(entry["portrait"], "res://p.png")
        self.assertEqual(entry["avatar"], "res://a.png")
        self.assertEqual(
            entry["card_face"],
            "res://assets/cards/faces/card_aolinpike-jidian_face.png",
        )
        self.assertEqual(entry["display"]["portrait"], "200x280")
        self.assertEqual(entry["display"]["avatar"], "22/30/38")
        self.assertEqual(entry["display"]["card_face"], "112x146")

    def test_cover_rejects_unexpected_size(self):
        make_cover(self.root, "wrong-size", size=(400, 500))

        result = run_covers(self.root)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("448x576", result.stderr)
        self.assertFalse(
            (self.root / "faces" / "card_wrong-size_face.png").exists()
        )
        assets = json.loads(
            (self.root / "asset_manifest.json").read_text()
        )["assets"]
        self.assertNotIn("wrong-size", assets)

    def test_cover_uses_newest_image_in_card_dir(self):
        make_cover(
            self.root,
            "aolinpike-jidian",
            color=(200, 0, 0),
            name="Single_old.png",
            watermark=False,
            mtime=1000,
        )
        make_cover(self.root, "aolinpike-jidian", name="Single_new.png")

        result = run_covers(self.root)

        self.assertEqual(result.returncode, 0, result.stderr)
        face = Image.open(
            self.root / "faces" / "card_aolinpike-jidian_face.png"
        )
        self.assertLess(max(face.getpixel((224, 292))[:3]), 60)


@unittest.skipUnless(HAS_PILLOW, "需要 Pillow（scripts/process_assets.py 的运行时依赖）")
class ParallelProcessingTest(unittest.TestCase):
    def _cover_tree(self, base: Path, name: str) -> Path:
        root = base / name
        for part in ("covers", "faces"):
            (root / part).mkdir(parents=True)
        for index, color in enumerate(((10, 12, 20), (30, 0, 0), (0, 30, 0))):
            make_cover(root, f"card-{index}", color=color)
        return root

    def _portrait_tree(self, base: Path, name: str) -> Path:
        root = base / name
        for part in ("portraits", "avatars", "faces"):
            (root / part).mkdir(parents=True)
        mapping = {}
        for index, size in enumerate(((300, 450), (320, 480), (280, 420))):
            source = f"src_{index}.png"
            Image.new("RGB", size, (10, 12, 20)).save(root / "portraits" / source)
            mapping[f"card-{index}"] = source
        (root / "asset_map.json").write_text(json.dumps(mapping))
        return root

    def test_parallel_covers_match_serial(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            serial_root = self._cover_tree(base, "serial")
            parallel_root = self._cover_tree(base, "parallel")

            serial = run_covers(serial_root, jobs=1)
            parallel = run_covers(parallel_root, jobs=4)

            self.assertEqual(serial.returncode, 0, serial.stderr)
            self.assertEqual(parallel.returncode, 0, parallel.stderr)
            self.assertEqual(
                (serial_root / "asset_manifest.json").read_text(),
                (parallel_root / "asset_manifest.json").read_text(),
            )
            for face in sorted((serial_root / "faces").glob("*.png")):
                self.assertEqual(
                    face.read_bytes(),
                    (parallel_root / "faces" / face.name).read_bytes(),
                    face.name,
                )
            self.assertEqual(
                output_without_manifest_path(serial.stdout),
                output_without_manifest_path(parallel.stdout),
            )

    def test_parallel_portraits_match_serial(self):
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            serial_root = self._portrait_tree(base, "serial")
            parallel_root = self._portrait_tree(base, "parallel")

            serial = run_assets(serial_root, jobs=1)
            parallel = run_assets(parallel_root, jobs=3)

            self.assertEqual(serial.returncode, 0, serial.stderr)
            self.assertEqual(parallel.returncode, 0, parallel.stderr)
            self.assertEqual(
                (serial_root / "asset_manifest.json").read_text(),
                (parallel_root / "asset_manifest.json").read_text(),
            )
            for pattern in (
                "portraits/ch_*_full.png",
                "avatars/ch_*_ava.png",
                "faces/card_*_face.png",
            ):
                for output in sorted(serial_root.glob(pattern)):
                    self.assertEqual(
                        output.read_bytes(),
                        (parallel_root / output.relative_to(serial_root)).read_bytes(),
                        str(output),
                    )
            self.assertEqual(
                output_without_manifest_path(serial.stdout),
                output_without_manifest_path(parallel.stdout),
            )


class AssetMapTest(unittest.TestCase):
    def test_map_covers_every_committed_portrait(self):
        portraits = ROOT / "client" / "assets" / "characters" / "portraits"
        mapping = json.loads((portraits / "asset_map.json").read_text())
        ids = {p.stem[3:-5] for p in portraits.glob("ch_*_full.png")}
        self.assertEqual(set(mapping), ids)
        for card_id, source in mapping.items():
            self.assertTrue((portraits / source).is_file(), f"{card_id}: {source}")


if __name__ == "__main__":
    unittest.main()
