#!/usr/bin/env python3
"""生成客户端应用图标 client/icon.png。

纯标准库（无 Pillow），颜色取自 UITheme 原子层；2× 超采样后降采样得到抗锯齿边缘。
图标用于 macOS 与 iOS 导出：iOS 导出会从 application/config/icon 缩放生成各尺寸。
"""

from __future__ import annotations

import struct
import zlib
from pathlib import Path

SIZE = 1024
SUPERSAMPLE = 2
OUT = Path(__file__).resolve().parent.parent / "client" / "icon.png"

INK_950 = (7, 10, 15)
INK_800 = (18, 25, 37)
INK_700 = (26, 35, 49)
GOLD_400 = (240, 198, 107)
GOLD_600 = (185, 138, 47)


def _lerp(a: tuple[int, int, int], b: tuple[int, int, int], t: float) -> tuple[float, float, float]:
    t = max(0.0, min(1.0, t))
    return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t)


def _mix(dst: list[float], src: tuple[float, float, float], alpha: float) -> None:
    for i in range(3):
        dst[i] = dst[i] * (1.0 - alpha) + src[i] * alpha


def _in_polygon(x: float, y: float, points: list[tuple[float, float]]) -> bool:
    inside = False
    j = len(points) - 1
    for i, (xi, yi) in enumerate(points):
        xj, yj = points[j]
        if (yi > y) != (yj > y) and x < (xj - xi) * (y - yi) / (yj - yi) + xi:
            inside = not inside
        j = i
    return inside


def sample(u: float, v: float) -> tuple[float, float, float]:
    color = list(_lerp(INK_950, INK_800, v))
    # 四周压暗，中心略亮。
    vignette = ((u - 0.5) ** 2 + (v - 0.5) ** 2) ** 0.5
    _mix(color, INK_950, min(1.0, max(0.0, (vignette - 0.35) * 1.6)))

    # 旗杆。
    if 0.285 <= u <= 0.322 and 0.16 <= v <= 0.88:
        _mix(color, _lerp(GOLD_600, GOLD_400, (u - 0.285) / 0.037), 1.0)

    # 燕尾旗：左上到右下渐变，叠两条深色横带。
    flag = [(0.315, 0.20), (0.86, 0.20), (0.86, 0.76), (0.59, 0.625), (0.315, 0.76)]
    if _in_polygon(u, v, flag):
        base = _lerp(GOLD_400, GOLD_600, (v - 0.20) / 0.56)
        _mix(color, base, 1.0)
        if 0.30 <= v <= 0.318 or 0.545 <= v <= 0.563:
            _mix(color, INK_950, 0.22)
        # 旗面徽记：深色圆盘 + 金环 + 中心金点。
        ex, ey = u - 0.60, v - 0.425
        edist = (ex * ex + ey * ey) ** 0.5
        if edist < 0.105:
            _mix(color, INK_950, 0.82)
        if 0.092 <= edist < 0.105:
            _mix(color, GOLD_400, 1.0)
        if edist < 0.034:
            _mix(color, GOLD_400, 1.0)

    # 底部基线。
    if 0.24 <= u <= 0.90 and 0.885 <= v <= 0.905:
        _mix(color, GOLD_600, 0.9)
    return (color[0], color[1], color[2])


def render() -> bytes:
    hi = SIZE * SUPERSAMPLE
    rows: list[bytes] = []
    for py in range(SIZE):
        row = bytearray()
        for px in range(SIZE):
            acc = [0.0, 0.0, 0.0]
            for sy in range(SUPERSAMPLE):
                for sx in range(SUPERSAMPLE):
                    u = (px * SUPERSAMPLE + sx + 0.5) / hi
                    v = (py * SUPERSAMPLE + sy + 0.5) / hi
                    r, g, b = sample(u, v)
                    acc[0] += r
                    acc[1] += g
                    acc[2] += b
            n = SUPERSAMPLE * SUPERSAMPLE
            row += bytes((int(acc[0] / n), int(acc[1] / n), int(acc[2] / n), 255))
        rows.append(bytes(row))
    return b"".join(rows)


def write_png(path: Path, raw: bytes) -> None:
    def chunk(tag: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    stride = SIZE * 4
    scanlines = b"".join(b"\x00" + raw[i * stride : (i + 1) * stride] for i in range(SIZE))
    ihdr = struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0)
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(scanlines, 9))
        + chunk(b"IEND", b"")
    )


def main() -> None:
    write_png(OUT, render())
    print(f"icon written: {OUT} ({SIZE}x{SIZE})")


if __name__ == "__main__":
    main()
