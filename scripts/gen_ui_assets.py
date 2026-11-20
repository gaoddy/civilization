#!/usr/bin/env python3
"""生成深色 UI 几何资产：卡框、稀有度徽章、UI 底材（bg/button/icon/panel）。

平面几何元素用代码按 client/ui/theme.gd 的色值令牌精确绘制，不走 AI 生图
（避免毛边、色偏与尺寸漂移）。视觉语言：深墨描金 + 科学仪器纹样，与英雄
立绘的风格一致。生成物随源提交，重跑本脚本可复现。

输出目录：
  client/assets/cards/frames/     frame_<rarity>.png      448x584 RGBA，中心透明
  client/assets/badges/rarity/    <rarity>.png            64x64   RGBA
                                  atlas.png + atlas.json  256x256 POT 图集
  client/assets/ui/bg/            bg_main.png             256x256 不透明，可平铺
                                  bg_panel.png            64x64   RGBA，九宫格
  client/assets/ui/button/        button_<variant>_<state>.png  96x40 RGBA，九宫格
  client/assets/ui/icon/          icon_slot*.png          64x64   RGBA
  client/assets/ui/panel/         panel_<variant>.png     64x64   RGBA，九宫格
  client/assets/ui/pill/          pill_<variant>.png      96x28   RGBA，九宫格（胶囊标签底）
  client/assets/ui/border/        tint_panel.png          64x64   RGBA，可染色描边（内近黑/边近白）
                                  tint_button.png         96x40   RGBA，可染色描边

用法：
  python3 scripts/gen_ui_assets.py
"""
from __future__ import annotations

import argparse
import json
import math
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "client" / "assets"
PACKER = (
    Path.home()
    / ".workbuddy/plugins/cache/experts/game-asset-director/1.0.0"
    / "skills/game-asset-forge/scripts/atlas_packer.py"
)

# 色板：与 client/ui/theme.gd 的 PRIMITIVE_COLOR 一致。
INK_950 = "070a0f"
INK_900 = "0c1118"
INK_800 = "121925"
INK_700 = "1a2331"
INK_600 = "243044"
INK_500 = "33435c"
SNOW_100 = "e9eef7"
SNOW_300 = "b7c3d6"
GOLD_400 = "f0c66b"
GOLD_600 = "b98a2f"
SLATE_400 = "93a6bd"
BLUE_400 = "62a8e8"
PURPLE_400 = "b28ce6"

RARITY_COLOR = {
    "common": SLATE_400,
    "rare": BLUE_400,
    "epic": PURPLE_400,
    "legendary": GOLD_400,
}

SS = 4  # 超采样倍率，缩小后用 LANCZOS 抗锯齿

Color = tuple[int, int, int, int]


def rgba(hex_or_tuple: str | Color, alpha: int = 255) -> Color:
    if isinstance(hex_or_tuple, tuple):
        r, g, b, a = hex_or_tuple
        return (r, g, b, alpha if alpha != 255 else a)
    h = hex_or_tuple
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), alpha)


def _to_hex(color: str | Color) -> str:
    if isinstance(color, str):
        return color
    return f"{color[0]:02x}{color[1]:02x}{color[2]:02x}"


def mix(
    fg: str | Color,
    bg: str | Color,
    t: float,
    alpha: int = 255,
) -> Color:
    """前景色按 t 混入背景色，返回 RGBA。"""
    f = rgba(_to_hex(fg))
    b = rgba(_to_hex(bg))
    return (
        round(f[0] * t + b[0] * (1 - t)),
        round(f[1] * t + b[1] * (1 - t)),
        round(f[2] * t + b[2] * (1 - t)),
        alpha,
    )


def _interp(stops: list[tuple[float, str | Color]], t: float) -> Color:
    for i in range(len(stops) - 1):
        p0, c0 = stops[i]
        p1, c1 = stops[i + 1]
        if p0 <= t <= p1:
            f = (t - p0) / max(1e-6, p1 - p0)
            a, b = rgba(_to_hex(c0)), rgba(_to_hex(c1))
            return tuple(round(a[k] * (1 - f) + b[k] * f) for k in range(4))  # type: ignore[return-value]
    return rgba(_to_hex(stops[-1][1]))


def vgradient(size: tuple[int, int], stops: list[tuple[float, str | Color]]) -> Image.Image:
    """垂直线性渐变：先造 1px 宽渐变条再横向复制，比逐像素快。"""
    w, h = size
    strip = Image.new("RGBA", (1, h))
    px = strip.load()
    for y in range(h):
        px[0, y] = _interp(stops, y / max(1, h - 1))
    return strip.resize((w, h), Image.Resampling.NEAREST)


def sheet(w: int, h: int) -> tuple[Image.Image, ImageDraw.ImageDraw]:
    big = Image.new("RGBA", (w * SS, h * SS), (0, 0, 0, 0))
    return big, ImageDraw.Draw(big)


def finish(big: Image.Image, w: int, h: int) -> Image.Image:
    return big.resize((w, h), Image.Resampling.LANCZOS)


def rounded_gradient(
    w: int, h: int, radius: int, stops: list[tuple[float, str | Color]]
) -> Image.Image:
    """带回角遮罩的渐变层（超采样尺度）。"""
    s = SS
    grad = vgradient((w * s, h * s), stops)
    mask = Image.new("L", (w * s, h * s), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        [0, 0, w * s - 1, h * s - 1], radius=radius * s, fill=255
    )
    layer = Image.new("RGBA", (w * s, h * s), (0, 0, 0, 0))
    layer.paste(grad, (0, 0), mask)
    return layer


def diamond(draw: ImageDraw.ImageDraw, cx: int, cy: int, r: int, **kw) -> None:
    draw.polygon([(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)], **kw)


# ── 卡框：多层金属渐变描边 + 科学仪器纹样 ──────────────────────────────────
def draw_frame(rarity: str) -> Image.Image:
    w, h = 448, 584  # 与 card_<id>_face.png 同尺寸（1x 卡 112x146 的 4x）
    big, d = sheet(w, h)
    c = RARITY_COLOR[rarity]
    s = SS

    def rr(inset: int, color: Color, width: int, radius: int = 32) -> None:
        d.rounded_rectangle(
            [inset * s, inset * s, (w - inset) * s, (h - inset) * s],
            radius=max(1, radius - inset) * s,
            outline=color,
            width=width * s,
        )

    # 由外向内：暗底 → 高光 → 主色 → 暗部 → 反光 → 内暗线 → 内细线
    rr(2, rgba(INK_950), 3)
    rr(5, mix(SNOW_100, c, 0.15), 2)
    rr(7, rgba(c), 5)
    rr(12, mix(INK_950, c, 0.62), 2)
    rr(14, mix(SNOW_100, c, 0.35, 170), 1)
    rr(17, rgba(INK_950, 200), 2)
    rr(20, rgba(c, 120), 1)

    # 四角取景角 + 铆钉（科学仪器 / 罗盘意象）
    m = 34
    for x, y, sx, sy in ((m, m, 1, 1), (w - m, m, -1, 1), (m, h - m, 1, -1), (w - m, h - m, -1, -1)):
        d.line([x * s, y * s, (x + 18 * sx) * s, y * s], fill=rgba(c), width=2 * s)
        d.line([x * s, y * s, x * s, (y + 18 * sy) * s], fill=rgba(c), width=2 * s)
        diamond(d, x * s, y * s, 5 * s, fill=mix(SNOW_100, c, 0.3))
        diamond(d, x * s, y * s, 2 * s, fill=rgba(INK_950))

    # 上下中轴刻度
    cx = w // 2
    for cy, dy in ((26, 1), (h - 26, -1)):
        d.line([(cx - 30) * s, cy * s, (cx + 30) * s, cy * s], fill=rgba(c, 150), width=s)
        for k in (-18, -9, 0, 9, 18):
            ln = 6 if k != 0 else 10
            d.line([(cx + k) * s, cy * s, (cx + k) * s, (cy + ln * dy) * s], fill=rgba(c), width=s)
        diamond(d, cx * s, cy * s, 6 * s, fill=rgba(c))
        diamond(d, cx * s, cy * s, 3 * s, fill=rgba(INK_950))

    # 左右中点三角指示
    for x, sx in ((16, 1), (w - 16, -1)):
        y = h // 2
        d.polygon(
            [(x * s, (y - 8) * s), ((x + 8 * sx) * s, y * s), (x * s, (y + 8) * s)],
            fill=mix(SNOW_100, c, 0.2),
        )

    if rarity in ("epic", "legendary"):
        rr(23, rgba(GOLD_400, 140), 1)
        for x, y in ((m, m), (w - m, m), (m, h - m), (w - m, h - m)):
            for dx, dy in ((0, -1), (0, 1), (-1, 0), (1, 0)):
                d.line(
                    [x * s, y * s, (x + dx * 12) * s, (y + dy * 12) * s],
                    fill=mix(SNOW_100, c, 0.5, 170), width=s,
                )
    if rarity == "legendary":
        rr(1, rgba(GOLD_400, 180), 2)
        rr(26, rgba(GOLD_400, 160), 1)
    return finish(big, w, h)


# ── 稀有度徽章：多面切割宝石 + 金属外框 ────────────────────────────────────
def draw_badge(rarity: str) -> Image.Image:
    w = h = 64
    big, d = sheet(w, h)
    color = RARITY_COLOR[rarity]
    s = SS
    cx = cy = 32 * s
    r = 21 * s

    # 外框：暗底 → 金属高光 → 暗底
    diamond(d, cx, cy, r + 6 * s, fill=rgba(INK_950))
    diamond(d, cx, cy, r + 4 * s, fill=mix(SNOW_100, color, 0.15))
    diamond(d, cx, cy, r + 2 * s, fill=rgba(INK_950))

    # 宝石四面切割，明度各异
    top, right = (cx, cy - r), (cx + r, cy)
    bottom, left, center = (cx, cy + r), (cx - r, cy), (cx, cy)
    d.polygon([top, right, center], fill=mix(SNOW_100, color, 0.45))
    d.polygon([right, bottom, center], fill=mix(INK_950, color, 0.35))
    d.polygon([bottom, left, center], fill=mix(INK_950, color, 0.6))
    d.polygon([left, top, center], fill=mix(SNOW_100, color, 0.25))
    for pt in (top, right, bottom, left):
        d.line([center, pt], fill=rgba(INK_950, 120), width=s)

    # 中心高光与顶部反光点
    diamond(d, cx, cy, 6 * s, fill=mix(SNOW_100, color, 0.7))
    diamond(d, cx, cy, 3 * s, fill=rgba(SNOW_100, 230))
    d.ellipse([cx - 2 * s, cy - r + 5 * s, cx + 2 * s, cy - r + 9 * s], fill=rgba(SNOW_100, 200))

    if rarity in ("epic", "legendary"):
        for dx, dy in ((0, -1), (0, 1), (-1, 0), (1, 0)):
            x, y = cx + dx * 27 * s, cy + dy * 27 * s
            d.line([cx + dx * 23 * s, cy + dy * 23 * s, x, y], fill=mix(SNOW_100, color, 0.6), width=s)
            d.ellipse([x - s, y - s, x + s, y + s], fill=mix(SNOW_100, color, 0.8))
    return finish(big, w, h)


# ── 背景 ────────────────────────────────────────────────────────────────────
def draw_bg_main() -> Image.Image:
    w = h = 256
    big, d = sheet(w, h)
    s = SS
    d.rectangle([0, 0, w * s, h * s], fill=rgba(INK_950))
    for v in range(0, w + 1, 16):
        d.line([v * s, 0, v * s, h * s], fill=rgba(INK_800, 90), width=s)
        d.line([0, v * s, w * s, v * s], fill=rgba(INK_800, 90), width=s)
    for y in range(0, h, 16):
        for x in range(0, w, 16):
            d.ellipse([x * s - s, y * s - s, x * s + s, y * s + s], fill=rgba(INK_600, 140))
    d.ellipse([64 * s, 64 * s, 192 * s, 192 * s], outline=rgba(INK_700, 120), width=s)
    d.ellipse([96 * s, 96 * s, 160 * s, 160 * s], outline=rgba(INK_700, 100), width=s)
    return finish(big, w, h)


def draw_bg_panel() -> Image.Image:
    w = h = 64
    layer = rounded_gradient(w, h, 8, [(0, mix(SNOW_100, INK_900, 0.06)), (1, INK_950)])
    d = ImageDraw.Draw(layer)
    s = SS
    d.rounded_rectangle([s, s, (w - 1) * s, (h - 1) * s], radius=8 * s, outline=rgba(INK_600), width=s)
    d.line([6 * s, 3 * s, (w - 6) * s, 3 * s], fill=rgba(SNOW_100, 40), width=s)
    return finish(layer, w, h)


# ── 面板 ────────────────────────────────────────────────────────────────────
PANEL_COLOR = {
    "normal": (INK_900, INK_950, INK_600, False),
    "raised": (INK_700, INK_800, INK_500, False),
    "sunken": (INK_950, INK_700, INK_600, False),
    "accent": (GOLD_400, GOLD_400, GOLD_400, True),
    "success": ("5fce86", "5fce86", "5fce86", True),
    "danger": ("e2666b", "e2666b", "e2666b", True),
}


def draw_panel(variant: str) -> Image.Image:
    w = h = 64
    if variant == "raised":
        # 抬升面：明显更亮，顶部高光带更宽，读出「浮起」层次
        stops, border, tinted = [(0, mix(SNOW_100, INK_700, 0.22)), (0.35, INK_700), (1, INK_800)], INK_500, False
    elif variant == "sunken":
        # 凹陷面：更暗且顶部压暗，与 raised 拉开方向相反的渐变
        stops, border, tinted = [(0, INK_950), (0.4, mix(INK_950, INK_800, 0.5)), (1, INK_700)], INK_600, False
    elif variant in ("accent", "success", "danger"):
        top_c, bot_c, border, tinted = PANEL_COLOR[variant]
        stops = [(0, mix(top_c, INK_900, 0.12)), (1, mix(bot_c, INK_950, 0.06))]
    else:  # normal
        stops, border, tinted = [(0, mix(SNOW_100, INK_900, 0.08)), (0.5, INK_900), (1, INK_950)], INK_600, False
    layer = rounded_gradient(w, h, 12, stops)
    d = ImageDraw.Draw(layer)
    s = SS
    d.rounded_rectangle(
        [2 * s, 2 * s, (w - 2) * s, (h - 2) * s],
        radius=12 * s, outline=rgba(border), width=(2 if tinted else 1) * s,
    )
    d.line([12 * s, 4 * s, (w - 12) * s, 4 * s], fill=rgba(SNOW_100, 45), width=s)
    for x, y in ((8, 8), (w - 8, 8), (8, h - 8), (w - 8, h - 8)):
        d.ellipse([x * s - s, y * s - s, x * s + s, y * s + s], fill=mix(SNOW_100, border, 0.3))
    return finish(layer, w, h)


# ── 胶囊标签底 ──────────────────────────────────────────────────────────────
PILL_COLOR = {
    "neutral": INK_800,
    "accent": GOLD_400,
    "success": "5fce86",
    "danger": "e2666b",
    "info": BLUE_400,
    "plain": INK_950,
}


def draw_pill(variant: str) -> Image.Image:
    w, h = 96, 28          # 胶囊宽可拉伸，高对齐 caption 12px + 上下留白
    layer = rounded_gradient(w, h, 14, [(0, mix(SNOW_100, INK_700, 0.14)), (1, INK_800)])
    d = ImageDraw.Draw(layer)
    s = SS
    color = PILL_COLOR[variant]
    border = rgba(INK_600) if variant == "neutral" else mix(INK_950, color, 0.35)
    if variant == "plain":
        # plain 完全透明，只保留极淡描边；调用方通常不放文字底
        return Image.new("RGBA", (w, h), (0, 0, 0, 0))
    # 胶囊整体染一层极淡的语义色，保持深色可读
    tint = Image.new("RGBA", (w * s, h * s), mix(color, INK_900, 0.86))
    mask = Image.new("L", (w * s, h * s), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, w * s - 1, h * s - 1], radius=14 * s, fill=255)
    layer = Image.alpha_composite(layer, Image.composite(tint, Image.new("RGBA", layer.size, (0, 0, 0, 0)), mask))
    d = ImageDraw.Draw(layer)
    d.rounded_rectangle(
        [1 * s, 1 * s, (w - 1) * s, (h - 1) * s],
        radius=14 * s, outline=border, width=s,
    )
    d.line([16 * s, 3 * s, (w - 16) * s, 3 * s], fill=rgba(SNOW_100, 30), width=s)
    return finish(layer, w, h)


# ── 按钮 ────────────────────────────────────────────────────────────────────
def draw_button(variant: str, state: str) -> Image.Image:
    w, h = 96, 40
    s = SS
    # 变体归一：default/micro/slot 复用深色底，focus 视觉同 hover。
    color_variant = variant if variant in ("primary", "danger", "ghost") else "secondary"
    visual_state = "hover" if state == "focus" else state

    if color_variant == "primary":
        accent = GOLD_400
        if visual_state == "disabled":
            stops, border = [(0, INK_800), (1, INK_700)], INK_600
        elif visual_state == "pressed":
            stops = [(0, mix(INK_950, GOLD_400, 0.8)), (1, mix(INK_950, GOLD_600, 0.75))]
            border = GOLD_600
        else:
            top = mix(SNOW_100, GOLD_400, 0.5)
            mid = mix(SNOW_100, GOLD_400, 0.2) if visual_state == "hover" else GOLD_400
            stops = [(0, top), (0.45, mid), (1, GOLD_600)]
            border = mix(SNOW_100, GOLD_400, 0.3)
    elif color_variant == "danger":
        accent = "e2666b"
        if visual_state == "disabled":
            stops, border = [(0, INK_800), (1, INK_900)], INK_700
        elif visual_state == "pressed":
            stops = [(0, mix(INK_950, accent, 0.7)), (1, mix(INK_950, accent, 0.55))]
            border = accent
        else:
            top = mix(SNOW_100, accent, 0.35) if visual_state == "hover" else mix(SNOW_100, accent, 0.18)
            stops = [(0, top), (0.45, accent), (1, mix(INK_900, accent, 0.6))]
            border = mix(SNOW_100, accent, 0.25)
    elif color_variant == "secondary":
        accent = SLATE_400
        if visual_state == "disabled":
            stops, border = [(0, INK_800), (1, INK_900)], INK_700
        elif visual_state == "pressed":
            stops, border = [(0, INK_800), (1, INK_900)], INK_600
        else:
            top = mix(SNOW_100, INK_700, 0.12 if visual_state == "hover" else 0.05)
            stops, border = [(0, top), (1, INK_800)], (INK_500 if visual_state == "hover" else INK_600)
    else:  # ghost
        accent = INK_700
        if visual_state in ("normal", "disabled"):
            stops, border = [(0, (0, 0, 0, 0)), (1, (0, 0, 0, 0))], INK_700
        else:
            stops, border = [(0, rgba(INK_800, 220)), (1, rgba(INK_900, 220))], INK_600

    layer = rounded_gradient(w, h, 10, stops)
    d = ImageDraw.Draw(layer)
    d.rounded_rectangle(
        [2 * s, 2 * s, (w - 2) * s, (h - 2) * s], radius=10 * s, outline=rgba(border), width=s,
    )
    if color_variant != "ghost" or visual_state in ("hover", "pressed"):
        d.line([10 * s, 4 * s, (w - 10) * s, 4 * s], fill=rgba(SNOW_100, 50), width=s)
        d.line([10 * s, (h - 4) * s, (w - 10) * s, (h - 4) * s], fill=rgba(INK_950, 90), width=s)
    return finish(layer, w, h)


# ── 可染色描边底 ────────────────────────────────────────────────────────────
# 内部近黑、描边近白：StyleBoxTexture.modulate_color 染色后，内部仍近黑、
# 描边取目标色，从而在纯图片资产上保留「动态着色」能力（选中金边/玩法色边框）。
def draw_tint(kind: str) -> Image.Image:
    w, h, radius = (64, 64, 12) if kind == "panel" else (96, 40, 10)
    big, d = sheet(w, h)
    s = SS
    box = [2 * s, 2 * s, (w - 2) * s, (h - 2) * s]
    d.rounded_rectangle(
        box, radius=radius * s,
        fill=mix(INK_950, INK_900, 0.5),
        outline=rgba(SNOW_100), width=2 * s,
    )
    d.line([12 * s, 4 * s, (w - 12) * s, 4 * s], fill=rgba(SNOW_100, 40), width=s)
    return finish(big, w, h)


# ── 图标底槽 ────────────────────────────────────────────────────────────────
def draw_icon_slot(variant: str) -> Image.Image:
    w = h = 64
    big, d = sheet(w, h)
    s = SS
    cx = cy = 32 * s
    if variant == "slot":
        outer, inner = INK_700, INK_800
    elif variant == "active":
        outer, inner = GOLD_400, INK_800
    else:
        outer, inner = INK_700, INK_900

    d.ellipse([4 * s, 4 * s, 60 * s, 60 * s], fill=rgba(INK_950))
    d.ellipse([5 * s, 5 * s, 59 * s, 59 * s], fill=rgba(outer))
    d.ellipse([7 * s, 7 * s, 57 * s, 57 * s], fill=rgba(INK_950))
    d.ellipse([9 * s, 9 * s, 55 * s, 55 * s], fill=rgba(inner))
    d.ellipse([13 * s, 13 * s, 51 * s, 51 * s], outline=mix(INK_950, outer, 0.3, 150), width=s)

    # 顶部高光弧
    for i in range(20):
        a = math.radians(200 + i * 7)
        x, y = cx + 22 * s * math.cos(a), cy + 22 * s * math.sin(a)
        d.ellipse([x - s, y - s, x + s, y + s], fill=rgba(SNOW_100, 60))
    if variant == "active":
        for k in range(4):
            a = math.radians(k * 90)
            x, y = cx + 24 * s * math.cos(a), cy + 24 * s * math.sin(a)
            diamond(d, round(x), round(y), 3 * s, fill=rgba(GOLD_400))
    return finish(big, w, h)


def save(im: Image.Image, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    im.save(path)


def build_atlas(src_dir: Path, out_png: Path, out_json: Path) -> None:
    """用专家工具链把徽章打成 POT 图集；staging 目录避免图集打包自身。"""
    with tempfile.TemporaryDirectory() as td:
        staging = Path(td)
        for png in sorted(src_dir.glob("*.png")):
            if png.stem in RARITY_COLOR:
                shutil.copy(png, staging / png.name)
        subprocess.run(
            [
                sys.executable, str(PACKER), str(staging),
                str(out_png), str(out_json), "--padding", "4", "--max-pot", "256",
            ],
            check=True,
        )
        # 打包器默认 pivot 是角色脚底基线 (0.5,0.1)；徽章是居中 UI 图标，改为 (0.5,0.5)。
        data = json.loads(out_json.read_text(encoding="utf-8"))
        for frame in data.get("frames", {}).values():
            frame["pivot"] = {"x": 0.5, "y": 0.5}
        out_json.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="仅校验脚本可读性")
    args = ap.parse_args()
    if args.check:
        print("gen_ui_assets.py: OK")
        return 0

    frames = ASSETS / "cards" / "frames"
    badges = ASSETS / "badges" / "rarity"
    bg = ASSETS / "ui" / "bg"
    button = ASSETS / "ui" / "button"
    icon = ASSETS / "ui" / "icon"
    panel = ASSETS / "ui" / "panel"
    pill = ASSETS / "ui" / "pill"
    border = ASSETS / "ui" / "border"

    for rarity in RARITY_COLOR:
        save(draw_frame(rarity), frames / f"frame_{rarity}.png")
        save(draw_badge(rarity), badges / f"{rarity}.png")
    build_atlas(badges, badges / "atlas.png", badges / "atlas.json")

    save(draw_bg_main(), bg / "bg_main.png")
    save(draw_bg_panel(), bg / "bg_panel.png")

    # theme.gd 的 button_style 变体：default/primary/danger/ghost/slot/micro，
    # 状态 normal/hover/pressed/focus/disabled。default/micro/slot 归一到 secondary 视觉。
    button_variants = ("primary", "secondary", "danger", "ghost")
    button_states = ("normal", "hover", "pressed", "focus", "disabled")
    for variant in button_variants:
        for state in button_states:
            save(draw_button(variant, state), button / f"button_{variant}_{state}.png")

    for variant in ("slot", "active", "disabled"):
        save(draw_icon_slot(variant), icon / f"icon_slot_{variant}.png")

    for variant in PANEL_COLOR:
        save(draw_panel(variant), panel / f"panel_{variant}.png")

    for variant in PILL_COLOR:
        save(draw_pill(variant), pill / f"pill_{variant}.png")

    for kind in ("panel", "button"):
        save(draw_tint(kind), border / f"tint_{kind}.png")

    print("UI 几何资产已生成到 client/assets/")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
