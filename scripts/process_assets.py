#!/usr/bin/env python3
"""Post-process generated art into the in-game asset set.

Portrait mode (--map):
  - remove the generator's 'AI生成 / WORKBUDDY' watermark (column-sampled
    gradient fill, feathered at edges) -> ch_<id>_full.png
  - derive a square avatar (face crop) -> ch_<id>_ava.png
  - derive a card-face cover (portrait scaled to 448x584) -> card_<id>_face.png

Cover mode (--covers):
  - each subdirectory name is a card_id; use its newest PNG (448x576)
  - remove the same watermark with the cover geometry
  - scale to 448x584 -> card_<id>_face.png; register card_face only

Also writes client/assets/asset_manifest.json mapping id -> asset paths/sizes.

Usage:
  python3 scripts/process_assets.py --map <json> \
      --portraits client/assets/characters/portraits \
      --avatars client/assets/characters/avatars \
      --faces client/assets/cards/faces \
      --manifest client/assets/asset_manifest.json
  python3 scripts/process_assets.py --covers output/effect-covers/generated \
      --faces client/assets/cards/faces \
      --manifest client/assets/asset_manifest.json

The --map json is a dict {card_id: source_portrait_filename}.

Parallelism: --jobs N (default: CPU count, capped at 8) processes images in
subprocesses. Manifest entries, stdout order and output bytes are identical
to a serial run; --jobs 1 forces serial processing.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path
from typing import Callable, Iterator, TypeVar

try:
    from PIL import Image
except ImportError as exc:  # pragma: no cover - depends on the local env
    raise SystemExit(
        "process_assets.py 需要 Pillow：python3 -m pip install pillow"
    ) from exc

# Watermark sits in the bottom-right corner of generated 1024x1536 images.
# Region is sampled per-column from a clean row just above it so the local
# horizontal gradient is preserved; edges are feathered to avoid seams.
WM_BOX = (886, 1452, 1016, 1535)  # x0, y0, x1, y1
SAMPLE_Y = 1446
FEATHER = 6

# Effect-card covers are generated at 448x576. Their watermark sits lower and
# further right than the portrait one, and can reach the bottom-right edge
# (x=447, y=575). The box extends past the image so those edge pixels still get
# a full fill; dewatermark clamps the loop to the image bounds.
COVER_SIZE = (448, 576)
COVER_WM_BOX = (392, 544, 452, 580)
COVER_SAMPLE_Y = 540
COVER_FEATHER = 4

FACE_SIDE = 256          # source avatar resolution
# Fractional crop of the portrait covering the face area. The crop is cut to a
# square (top-aligned, horizontally centered) before resizing, so avatars are
# never stretched and stay square with opaque corners.
FACE_CROP = (0.18, 0.08, 0.82, 0.60)
CARD_SIZE = (448, 584)   # 4x of the 112x146 in-game card


def dewatermark(
    im: Image.Image,
    box: tuple[int, int, int, int] = WM_BOX,
    sample_y: int = SAMPLE_Y,
    feather: int = FEATHER,
) -> Image.Image:
    im = im.convert("RGBA")
    px = im.load()
    w, h = im.size
    x0, y0, x1, y1 = box
    sample_y = min(h - 1, sample_y)
    for x in range(max(0, x0), min(w, x1)):
        tr, tg, tb = px[x, sample_y][:3]
        for y in range(max(0, y0), min(h, y1)):
            fy = max(0.0, min(1.0, (y - y0) / feather, (y1 - 1 - y) / feather))
            fx = max(0.0, min(1.0, (x - x0) / feather, (x1 - 1 - x) / feather))
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
    crop_w = int(w * (r - l))
    crop_h = int(h * (b - t))
    side = min(crop_w, crop_h)
    left = int(w * l) + (crop_w - side) // 2
    top = int(h * t)  # keep the head framed by the top edge
    box = (left, top, left + side, top + side)
    return im.crop(box).resize((FACE_SIDE, FACE_SIDE), Image.LANCZOS)


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


T = TypeVar("T")
R = TypeVar("R")


def default_jobs() -> int:
    return max(1, min(os.cpu_count() or 1, 8))


def iter_results(func: Callable[[T], R], tasks: list[T], jobs: int) -> Iterator[R]:
    """按任务顺序产出结果；jobs > 1 时用子进程并行处理。

    子进程只接收路径并把清单条目返回父进程，图片对象不跨进程传递；
    产出顺序固定，清单与日志因此与串行执行一致。
    """
    if jobs <= 1 or len(tasks) <= 1:
        for task in tasks:
            yield func(task)
        return
    with ProcessPoolExecutor(max_workers=min(jobs, len(tasks))) as pool:
        yield from pool.map(func, tasks)


def process_portrait(
    task: tuple[str, str, str, str, str]
) -> tuple[str, dict | None, str]:
    """去水印并派生一个立绘的头像与卡面，返回 (card_id, 清单条目, 日志行)。"""
    card_id, src, portraits, avatars, faces = task
    src_path = Path(src)
    if not src_path.exists():
        return card_id, None, f"MISSING {src_path}"
    im = Image.open(src_path)
    try:
        clean = dewatermark(im)
    finally:
        im.close()
    clean.save(Path(portraits) / f"ch_{card_id}_full.png")
    derive_avatar(clean).save(Path(avatars) / f"ch_{card_id}_ava.png")
    derive_face_cover(clean).save(Path(faces) / f"card_{card_id}_face.png")
    return (
        card_id,
        {
            "portrait": f"res://assets/characters/portraits/ch_{card_id}_full.png",
            "avatar": f"res://assets/characters/avatars/ch_{card_id}_ava.png",
            "card_face": f"res://assets/cards/faces/card_{card_id}_face.png",
            "display": {
                "portrait": "200x280",
                "avatar": "22/30/38",
                "card_face": "112x146",
            },
        },
        f"processed {card_id}",
    )


def process_cover(task: tuple[str, str, str]) -> tuple[str, str, str, str]:
    """处理一张效果牌封面，返回 (card_id, 卡面路径, 日志行, 错误)。"""
    card_id, src, faces = task
    src_path = Path(src)
    im = Image.open(src_path)
    try:
        if im.size != COVER_SIZE:
            return (
                card_id,
                "",
                "",
                f"{card_id}: 封面尺寸应为 {COVER_SIZE[0]}x{COVER_SIZE[1]}，"
                f"实际 {im.size[0]}x{im.size[1]}（{src_path.name}）",
            )
        clean = dewatermark(im, COVER_WM_BOX, COVER_SAMPLE_Y, COVER_FEATHER)
    finally:
        im.close()
    out = Path(faces) / f"card_{card_id}_face.png"
    clean.resize(CARD_SIZE, Image.LANCZOS).save(out)
    return (
        card_id,
        f"res://assets/cards/faces/{out.name}",
        f"cover {card_id} <- {src_path.name}",
        "",
    )


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--map")
    ap.add_argument("--portraits")
    ap.add_argument("--avatars")
    ap.add_argument("--covers")
    ap.add_argument("--faces", required=True)
    ap.add_argument("--manifest", required=True)
    ap.add_argument(
        "--jobs",
        type=int,
        default=default_jobs(),
        help=f"并行处理的子进程数，默认 {default_jobs()}（CPU 数，上限 8）",
    )
    args = ap.parse_args()

    if not args.map and not args.covers:
        ap.error("需要 --map 或 --covers")
    if args.map and not (args.portraits and args.avatars):
        ap.error("--map 需要 --portraits 与 --avatars")
    if args.jobs < 1:
        ap.error("--jobs 必须是正整数")

    faces = Path(args.faces)
    faces.mkdir(parents=True, exist_ok=True)

    # Keep entries written outside this pipeline (e.g. covers generated
    # directly at card-face size) instead of rebuilding the manifest from the
    # mapping alone.
    manifest_path = Path(args.manifest)
    assets: dict = {}
    if manifest_path.exists():
        assets = json.loads(manifest_path.read_text()).get("assets", {})

    if args.map:
        mapping = json.loads(Path(args.map).read_text())
        portraits = Path(args.portraits)
        avatars = Path(args.avatars)
        avatars.mkdir(parents=True, exist_ok=True)
        tasks = [
            (card_id, str(portraits / src), str(portraits), str(avatars), str(faces))
            for card_id, src in mapping.items()
        ]
        for card_id, entry, line in iter_results(process_portrait, tasks, args.jobs):
            print(line)
            if entry is not None:
                assets[card_id] = entry

    errors: list[str] = []
    if args.covers:
        covers = Path(args.covers)
        tasks = []
        for card_dir in sorted(p for p in covers.iterdir() if p.is_dir()):
            images = sorted(card_dir.glob("*.png"), key=lambda p: p.stat().st_mtime)
            if not images:
                continue
            tasks.append((card_dir.name, str(images[-1]), str(faces)))
        for card_id, face, line, error in iter_results(
            process_cover, tasks, args.jobs
        ):
            if line:
                print(line)
            if error:
                errors.append(error)
            if face:
                entry = assets.setdefault(card_id, {})
                entry["card_face"] = face
                entry.setdefault("display", {})["card_face"] = "112x146"

    manifest = {"version": 1, "assets": assets}
    manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2))
    print(f"wrote {args.manifest} ({len(assets)} cards)")

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        raise SystemExit(1)


if __name__ == "__main__":
    main()
