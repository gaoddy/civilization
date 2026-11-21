#!/usr/bin/env python3
"""生成 Godot 页面线框图。

调用无头探针（`client/tests/wireframe_probe.tscn`）装载页面，取出引擎解算后的
控件矩形，再画成 SVG；PNG 交给本机无头浏览器截图，没有浏览器时退回引擎栅格化。

用法：
    python3 scripts/ui_wireframe.py --godot <godot> --page gallery [--device phone]
    python3 scripts/ui_wireframe.py --godot <godot> --all

产物写到 `output/wireframes/<页面>[-<设备>].{json,svg,png}`（该目录被 git 忽略）。
"""
from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path
from xml.sax.saxutils import escape

ROOT = Path(__file__).resolve().parent.parent
PAGES_DIR = ROOT / "client" / "pages"
OUT_DIR = ROOT / "output" / "wireframes"
PROBE_SCENE = "res://tests/wireframe_probe.tscn"
TIMEOUT_SECONDS = 120

LEAF_CLASSES = {"Label", "Button", "RichTextLabel", "LineEdit", "CheckBox", "OptionButton"}
# 背景图与纯装饰节点不进线框。
SKIP_CLASSES = {"TextureRect"}
CONTAINER_LABEL_DEPTH = 4
PNG_WIDTH = 1600
MAX_TEXT = 28

COLOR = {
    "background": "#fbfbfa",
    "leaf_fill": "#f1f1ef",
    "button_fill": "#e4e4e1",
    "leaf_stroke": "#9a9a98",
    "container_stroke": "#c6c6c3",
    "text": "#3a3a38",
    "label": "#9a9a96",
}


def discover_pages() -> list[str]:
    return sorted(p.parent.name for p in PAGES_DIR.glob("*/*.tscn"))


def page_scene(page: str) -> str:
    return f"res://pages/{page}/{page}.tscn"


def run_probe(
    godot: str,
    scene: str,
    out_json: Path,
    device: str,
) -> None:
    command = [
        godot,
        "--headless",
        "--path",
        str(ROOT / "client"),
        PROBE_SCENE,
        "--",
        f"--wireframe-scene={scene}",
        f"--wireframe-out={out_json}",
    ]
    if device:
        command.append(f"--wireframe-device={device}")
    _run(command, "探针")


def _run(command: list[str], label: str) -> None:
    result = subprocess.run(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=TIMEOUT_SECONDS,
    )
    if result.returncode != 0:
        tail = "\n".join(result.stdout.strip().splitlines()[-8:])
        raise RuntimeError(f"{label}退出码 {result.returncode}\n{tail}")


def render_svg(layout: dict) -> str:
    width, height = layout["size"]
    nodes = [n for n in layout["nodes"] if n["class"] not in SKIP_CLASSES]
    body: list[str] = []
    labels: list[tuple[int, float, float, str, int, str]] = []  # 优先级, x, y, 文本, 字号, 颜色
    for node in nodes:
        x, y, node_w, node_h = node["rect"]
        # 滚动区里的内容会排到画布外，这里统一裁到画布内；兜底栅格化不处理 clip-path。
        left, top = max(x, 0.0), max(y, 0.0)
        right, bottom = min(x + node_w, width), min(y + node_h, height)
        if right - left <= 0.5 or bottom - top <= 0.5:
            continue
        x, y, node_w, node_h = left, top, right - left, bottom - top
        name = node["name"]
        depth = node.get("depth", 99)
        is_leaf = node["class"] in LEAF_CLASSES
        if is_leaf:
            fill = COLOR["button_fill"] if node["class"] == "Button" else COLOR["leaf_fill"]
            stroke, dash = COLOR["leaf_stroke"], ""
        else:
            if depth > 6 and not (node_w > 120 and node_h > 60):
                continue
            fill, stroke, dash = "none", COLOR["container_stroke"], ' stroke-dasharray="3 3"'
        body.append(
            f'<rect x="{x:.1f}" y="{y:.1f}" width="{max(node_w, 1):.1f}" '
            f'height="{max(node_h, 1):.1f}" fill="{fill}" stroke="{stroke}" '
            f'stroke-width="0.8"{dash}/>'
        )
        # Godot 自动命名的节点（@Label@4）没有可读名字，只保留容器标注以外的文字。
        if name.startswith("@") and not is_leaf:
            continue
        if not (is_leaf or depth <= CONTAINER_LABEL_DEPTH):
            continue
        if node_w < 18 or node_h < 10:
            continue
        text = str(node.get("text") or ("" if name.startswith("@") else name)).strip()
        if not text:
            continue
        labels.append(
            (
                0 if is_leaf else 1,
                x + 2,
                y,
                text[:MAX_TEXT],
                9 if is_leaf else 8,
                COLOR["text"] if is_leaf else COLOR["label"],
            )
        )

    # 叶子文字优先；同一落点已被更重要的标签占用时跳过，避免容器名压住页面标题。
    occupied: list[tuple[float, float]] = []
    rendered: list[str] = []
    for _priority, x, y, text, size, color in sorted(labels, key=lambda item: item[0]):
        slot = (round(x / 6), round(y / 6))
        if any(abs(slot[0] - other[0]) <= 1 and abs(slot[1] - other[1]) <= 1 for other in occupied):
            continue
        occupied.append(slot)
        rendered.append(
            f'<text x="{x:.1f}" y="{y + size + 1:.1f}" font-size="{size}" '
            f'fill="{color}">{escape(text)}</text>'
        )

    return "\n".join(
        [
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{width:g}" height="{height:g}" '
            f'viewBox="0 0 {width:g} {height:g}">',
            f'<defs><clipPath id="canvas"><rect width="{width:g}" height="{height:g}"/></clipPath></defs>',
            '<style>text{font-family:"PingFang SC","Noto Sans CJK SC","Noto Sans SC",sans-serif;'
            'paint-order:stroke;stroke:' + COLOR["background"] + ';stroke-width:2.5px;'
            "stroke-linejoin:round;}</style>",
            '<g clip-path="url(#canvas)">',
            f'<rect width="{width:g}" height="{height:g}" fill="{COLOR["background"]}"/>',
            *body,
            *rendered,
            "</g>",
            "</svg>",
        ]
    )


BROWSERS = [
    "google-chrome",
    "chromium",
    "chromium-browser",
    "microsoft-edge",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
]


def find_browser() -> str | None:
    for candidate in BROWSERS:
        found = candidate if "/" in candidate and Path(candidate).exists() else shutil.which(candidate)
        if found:
            return found
    return None


def render_png(godot: str, svg: Path, canvas: tuple[float, float]) -> Path | None:
    """SVG 转 PNG。浏览器截图保留文字；引擎自带的 ThorVG 不渲染文本，只作兜底。"""
    width, height = canvas
    if width <= 0:
        return None
    png_path = svg.with_suffix(".png")
    png_width = PNG_WIDTH
    png_height = max(1, round(PNG_WIDTH * height / width))
    browser = find_browser()
    if browser is not None:
        wrapper = svg.with_name(svg.stem + ".preview.html")
        wrapper.write_text(
            '<!doctype html><html><head><meta charset="utf-8"><style>'
            f"html,body{{margin:0;padding:0;background:{COLOR['background']}}}"
            "img{display:block;width:100vw;height:100vh;object-fit:contain}</style></head>"
            f'<body><img src="{svg.name}"></body></html>\n'
        )
        try:
            result = subprocess.run(
                [
                    browser,
                    "--headless=new",
                    "--disable-gpu",
                    "--hide-scrollbars",
                    "--force-device-scale-factor=1",
                    f"--window-size={png_width},{png_height}",
                    f"--screenshot={png_path}",
                    wrapper.as_uri(),
                ],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=TIMEOUT_SECONDS,
            )
            if result.returncode == 0 and png_path.exists():
                return png_path
            print(f"WARN 浏览器截图失败，改用引擎栅格化：{browser}", file=sys.stderr)
        finally:
            wrapper.unlink(missing_ok=True)
    _run(
        [
            godot,
            "--headless",
            "--path",
            str(ROOT / "client"),
            PROBE_SCENE,
            "--",
            f"--wireframe-svg={svg}",
            f"--wireframe-png={png_path}",
            f"--wireframe-scale={png_width / width:.4f}",
        ],
        "栅格化",
    )
    print("WARN PNG 缺少文字：未找到可用浏览器", file=sys.stderr)
    return png_path if png_path.exists() else None


def generate(godot: str, page: str, device: str) -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)  # 目录不存在时探针写文件失败会挂住进程
    suffix = f"-{device}" if device else ""
    stem = OUT_DIR / f"{page}{suffix}"
    json_path = stem.with_suffix(".json")
    run_probe(godot, page_scene(page), json_path, device)
    layout = json.loads(json_path.read_text())
    if not layout.get("nodes"):
        raise RuntimeError("探针没有导出任何可见控件")
    svg_path = stem.with_suffix(".svg")
    svg_path.write_text(render_svg(layout))
    png_path = render_png(godot, svg_path, tuple(layout["size"]))
    size = layout["size"]
    print(
        f"OK {page}{suffix}: {len(layout['nodes'])} 个控件, "
        f"画布 {size[0]:g}x{size[1]:g}, {layout.get('device', 'unknown')} -> "
        f"{svg_path.relative_to(ROOT)}{' + png' if png_path else ''}"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="生成 Godot 页面线框图")
    parser.add_argument("--godot", required=True, help="Godot 可执行文件路径")
    parser.add_argument("--page", help="client/pages 下的页面名，如 gallery")
    parser.add_argument("--all", action="store_true", help="生成全部页面")
    parser.add_argument(
        "--device", choices=["desktop", "tablet", "phone"], default="", help="设备档位"
    )
    args = parser.parse_args()
    pages = discover_pages() if args.all else [args.page] if args.page else []
    if not pages:
        parser.error("需要 --page <页面> 或 --all")
    if args.page and args.page not in discover_pages():
        parser.error(f"没有页面 {args.page}；可用：{', '.join(discover_pages())}")

    failures = 0
    for page in pages:
        try:
            generate(args.godot, page, args.device)
        except (RuntimeError, subprocess.TimeoutExpired, OSError) as error:
            failures += 1
            print(f"FAIL {page}: {error}", file=sys.stderr)
    if failures:
        print(f"{failures}/{len(pages)} 个页面失败", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
