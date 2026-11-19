#!/usr/bin/env python3
"""Post-process generated portraits into the in-game asset set.

For each card portrait:
  - remove the generator's 'AI生成 / WORKBUDDY' watermark (column-sampled
    gradient fill, feathered at edges) -> ch_<id>_full.png
  - derive a square avatar (face crop) -> ch_<id>_ava.png
  - derive a card-face cover (portrait scaled to 448x584) -> card_<id>_face.png

Also writes client/assets/asset_manifest.json mapping id -> asset paths/sizes.

Usage:
  python3 scripts/process_assets.py --map <json> \
      --portraits client/assets/characters/portraits \
      --avatars client/assets/characters/avatars \
      --faces client/assets/cards/faces \
      --manifest client/assets/asset_manifest.json

The --map json is a dict {card_id: source_portrait_filename}.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw

# Watermark sits in the bottom-right corner of generated 1024x1536 images.
# Region is sampled per-column from a clean row just above it so the local
# horizontal gradient is preserved; edges are feathered to avoid seams.
WM_BOX = (886, 1452, 1016, 1535)  # x0, y0, x1, y1
SAMPLE_Y = 1446
FEATHER = 6

FACE_SIDE = 256          # source avatar resolution
FACE_CROP = (0.18, 0.08, 0.82, 0.60)  # fractional crop of portrait (face area)
CARD_SIZE = (448, 584)   # 4x of the 112x146 in-game card


def dewatermark(im: Image.Image) -> Image.Image:
    im = im.convert("RGBA")
    px = im.load()
    x0, y0, x1, y1 = WM_BOX
    for x in range(x0, x1):
        tr, tg, tb = px[x, SAMPLE_Y][:3]
        for y in range(y0, y1):
            fy = max(0.0, min(1.0, (y - y0) / FEATHER, (y1 - 1 - y) / FEATHER))
            fx = max(0.0, min(1.0, (x - x0) / FEATHER, (x1 - 1 - x) / FEATHER))
            f = fy * fx
            r, g, b, a = px[x, y]
            px[x, y] = (
                int(r * (1 - f) + tr * f),
                int(g * (1 - f) + tg * f),
                int(b * (1 - f) + tb * f),
                a,
            )
    return im


def derive_avatar(im: Image.Image) -> Image.Image:
    w, h = im.size
    l, t, r, b = FACE_CROP
    box = (int(w * l), int(h * t), int(w * r), int(h * b))
    avatar = im.crop(box).resize((FACE_SIDE, FACE_SIDE), Image.LANCZOS)
    # circular mask with transparent corners
    mask = Image.new("L", (FACE_SIDE, FACE_SIDE), 0)
    ImageDraw.Draw(mask).ellipse((0, 0, FACE_SIDE - 1, FACE_SIDE - 1), fill=255)
    avatar.putalpha(mask)
    return avatar


def derive_face_cover(im: Image.Image) -> Image.Image:
    # center-crop portrait to card aspect then scale, keeping the subject
    w, h = im.size
    target = CARD_SIZE[0] / CARD_SIZE[1]
    if w / h > target:
        nw = int(h * target)
        left = (w - nw) // 2
        im = im.crop((left, 0, left + nw, h))
    else:
        nh = int(w / target)
        top = int((h - nh) * 0.35)  # bias upward to keep the face
        im = im.crop((0, top, w, top + nh))
    return im.resize(CARD_SIZE, Image.LANCZOS)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--map", required=True)
    ap.add_argument("--portraits", required=True)
    ap.add_argument("--avatars", required=True)
    ap.add_argument("--faces", required=True)
    ap.add_argument("--manifest", required=True)
    args = ap.parse_args()

    mapping = json.loads(Path(args.map).read_text())
    portraits = Path(args.portraits)
    avatars = Path(args.avatars)
    faces = Path(args.faces)
    for d in (avatars, faces):
        d.mkdir(parents=True, exist_ok=True)

    manifest: dict = {"version": 1, "assets": {}}
    for card_id, src in mapping.items():
        src_path = portraits / src
        if not src_path.exists():
            print(f"MISSING {src_path}")
            continue
        im = Image.open(src_path)
        clean = dewatermark(im)
        clean.save(portraits / f"ch_{card_id}_full.png")
        derive_avatar(clean).save(avatars / f"ch_{card_id}_ava.png")
        derive_face_cover(clean).save(faces / f"card_{card_id}_face.png")
        manifest["assets"][card_id] = {
            "portrait": f"res://assets/characters/portraits/ch_{card_id}_full.png",
            "avatar": f"res://assets/characters/avatars/ch_{card_id}_ava.png",
            "card_face": f"res://assets/cards/faces/card_{card_id}_face.png",
            "display": {
                "portrait": "200x280",
                "avatar": "22/30/38",
                "card_face": "112x146",
            },
        }
        print(f"processed {card_id}")

    Path(args.manifest).write_text(json.dumps(manifest, ensure_ascii=False, indent=2))
    print(f"wrote {args.manifest} ({len(manifest['assets'])} cards)")


if __name__ == "__main__":
    main()
