"""页面背景生成器。

只生成一个产物：

  client/assets/ui/bg/bg_main.png    512x512 RGB，整屏拉伸，不平铺

近纯深底 + 中心极淡的椭圆径向提亮 + 纸张颗粒。纯标准库（zlib + struct），
不依赖 Pillow；固定算法，重跑字节级一致。
"""

import math
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BG_SIZE = 512
OUT = ROOT / "client" / "assets" / "ui" / "bg" / "bg_main.png"

# 色值取自 client/ui/theme.gd：ink-950 #070a0f 为底。
BASE = (0x07, 0x0A, 0x0F)
GLOW_INNER = (0x2B, 0x3A, 0x52)  # 中心提亮
GLOW_MID = (0x18, 0x21, 0x2F)  # 半程过渡
GLOW_INNER_ALPHA = 0.88
GLOW_MID_ALPHA = 0.44
NOISE_LEVEL = 2


def _lerp(start: float, end: float, k: float) -> float:
    return start + (end - start) * k


def _glow_at(t: float) -> tuple[float, float, float, float]:
    """椭圆径向分量：t=0 内环，t=0.5 过渡环，t=1 完全透明。"""
    if t <= 0.5:
        k = t / 0.5
        return (
            _lerp(GLOW_INNER[0], GLOW_MID[0], k),
            _lerp(GLOW_INNER[1], GLOW_MID[1], k),
            _lerp(GLOW_INNER[2], GLOW_MID[2], k),
            _lerp(GLOW_INNER_ALPHA, GLOW_MID_ALPHA, k),
        )
    k = (t - 0.5) / 0.5
    return (
        GLOW_MID[0],
        GLOW_MID[1],
        GLOW_MID[2],
        _lerp(GLOW_MID_ALPHA, 0.0, k),
    )


def _noise(x: int, y: int) -> int:
    """确定性纸张颗粒：同一坐标恒得同一偏移。"""
    h = (x * 374761393 + y * 668265263) & 0xFFFFFFFF
    h = ((h ^ (h >> 13)) * 1274126177) & 0xFFFFFFFF
    h ^= h >> 16
    return (h % (NOISE_LEVEL * 2 + 1)) - NOISE_LEVEL


def _chunk(kind: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + kind
        + data
        + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    )


def _png(width: int, height: int, raw: bytes) -> bytes:
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + _chunk(b"IHDR", ihdr)
        + _chunk(b"IDAT", zlib.compress(raw, 9))
        + _chunk(b"IEND", b"")
    )


def render_background(size: int = BG_SIZE) -> bytes:
    raw = bytearray()
    for y in range(size):
        raw.append(0)  # 每行 filter byte：None
        ny = (y / size - 0.4) / 0.8
        for x in range(size):
            nx = (x / size - 0.5) / 0.8
            t = min(1.0, math.hypot(nx, ny))
            gr, gg, gb, ga = _glow_at(t)
            grain = _noise(x, y)
            raw.append(
                max(0, min(255, round(BASE[0] * (1.0 - ga) + gr * ga + grain)))
            )
            raw.append(
                max(0, min(255, round(BASE[1] * (1.0 - ga) + gg * ga + grain)))
            )
            raw.append(
                max(0, min(255, round(BASE[2] * (1.0 - ga) + gb * ga + grain)))
            )
    return _png(size, size, bytes(raw))


def main() -> int:
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(render_background(BG_SIZE))
    print(OUT)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
