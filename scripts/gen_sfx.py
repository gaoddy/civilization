#!/usr/bin/env python3
"""生成 v0.1 默认占位音效（合成波形，可随时替换为正式素材）。

输出：
- client/assets/audio/sfx/{reroll,freeze,unfreeze,shop_enter}.wav
- client/assets/audio/cards/default/{buy,attack,death}.wav

逐卡音效放在 client/assets/audio/cards/<card_id>/<cue>.wav，运行时优先使用；
本脚本只负责默认公共音效与占位素材，不做逐卡覆盖。
"""

import math
import pathlib
import random
import struct
import wave

RATE = 44100
PEAK = 0.45
ROOT = pathlib.Path(__file__).resolve().parents[1]
AUDIO = ROOT / "client" / "assets" / "audio"


def _samples(duration: float) -> int:
    return max(int(RATE * duration), 1)


def _fade(samples: list[float], attack: float = 0.006, release: float = 0.04) -> None:
    """首尾淡入淡出，避免波形突然起停产生爆音。"""
    attack_n = max(int(RATE * attack), 1)
    release_n = max(int(RATE * release), 1)
    for index in range(min(attack_n, len(samples))):
        samples[index] *= index / attack_n
    for index in range(min(release_n, len(samples))):
        samples[len(samples) - 1 - index] *= index / release_n


def _sweep(start: float, end: float, duration: float, gain: float = 1.0) -> list[float]:
    """线性扫频正弦；逐样本累加相位，避免频率跳变。"""
    total = _samples(duration)
    phase = 0.0
    result = []
    for index in range(total):
        ratio = index / total
        freq = start + (end - start) * ratio
        phase += 2.0 * math.pi * freq / RATE
        result.append(math.sin(phase) * gain)
    return result


def _bell(freq: float, duration: float, gain: float = 1.0, decay: float = 6.0) -> list[float]:
    """带指数衰减的钟声；叠加二次谐波让音色更亮。"""
    total = _samples(duration)
    phase = 0.0
    result = []
    for index in range(total):
        t = index / RATE
        phase += 2.0 * math.pi * freq / RATE
        envelope = math.exp(-decay * t)
        result.append((math.sin(phase) + 0.35 * math.sin(phase * 2.0)) * envelope * gain)
    return result


def _noise(duration: float, gain: float = 1.0, smooth: int = 12, seed: int = 1) -> list[float]:
    """低通噪声；一阶滑动平均压掉刺耳的高频。"""
    total = _samples(duration)
    rng = random.Random(seed)
    raw = [rng.uniform(-1.0, 1.0) for _ in range(total)]
    result = []
    for index in range(total):
        window = raw[max(0, index - smooth) : index + 1]
        result.append((sum(window) / len(window)) * gain)
    return result


def _thump(start: float, end: float, duration: float, gain: float = 1.0) -> list[float]:
    total = _samples(duration)
    phase = 0.0
    result = []
    for index in range(total):
        t = index / RATE
        ratio = index / total
        freq = start + (end - start) * ratio
        phase += 2.0 * math.pi * freq / RATE
        result.append(math.sin(phase) * math.exp(-18.0 * t) * gain)
    return result


def _mix(*tracks: list[float]) -> list[float]:
    length = max(len(track) for track in tracks)
    result = [0.0] * length
    for track in tracks:
        for index, value in enumerate(track):
            result[index] += value
    return result


def _pad(track: list[float], duration: float) -> list[float]:
    total = _samples(duration)
    return track + [0.0] * max(total - len(track), 0)


def _silence(duration: float) -> list[float]:
    return [0.0] * _samples(duration)


def _at(track: list[float], offset: float) -> list[float]:
    return _silence(offset) + track


def _write(name: str, samples: list[float]) -> None:
    peak = max(abs(value) for value in samples) or 1.0
    scale = PEAK / peak
    _fade(samples)
    data = b"".join(
        struct.pack("<h", int(max(-1.0, min(1.0, value * scale)) * 32767))
        for value in samples
    )
    path = AUDIO / name
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(RATE)
        handle.writeframes(data)
    print(f"写入 {path.relative_to(ROOT)}")


def build() -> None:
    # 刷新：两声快速上扬的短音。
    _write(
        "sfx/reroll.wav",
        _mix(
            _at(_sweep(420, 880, 0.075, 0.8), 0.0),
            _at(_sweep(620, 1240, 0.07, 0.6), 0.095),
            _noise(0.02, 0.15, smooth=6, seed=2),
        ),
    )

    # 冻结：高频冰晶下行，短促衰减。
    _write(
        "sfx/freeze.wav",
        _mix(
            _bell(2350, 0.34, 0.8, decay=9.0),
            _at(_bell(1760, 0.3, 0.55, decay=8.0), 0.02),
            _at(_bell(2940, 0.24, 0.35, decay=12.0), 0.045),
            _noise(0.16, 0.08, smooth=18, seed=7),
        ),
    )

    # 解冻：高频冰晶上行，尾音更长。
    _write(
        "sfx/unfreeze.wav",
        _mix(
            _bell(1320, 0.3, 0.5, decay=7.0),
            _at(_bell(1980, 0.3, 0.55, decay=7.0), 0.04),
            _at(_bell(2640, 0.34, 0.6, decay=6.0), 0.08),
        ),
    )

    # 回合开始商店入场：三音上行，留一点暖底。
    _write(
        "sfx/shop_enter.wav",
        _mix(
            _bell(523.25, 0.5, 0.55, decay=5.0),
            _at(_bell(659.25, 0.45, 0.5, decay=5.0), 0.09),
            _at(_bell(783.99, 0.55, 0.55, decay=4.5), 0.18),
            _pad(_sweep(196, 220, 0.32, 0.18), 0.55),
        ),
    )

    # 购入到手牌：两枚短促的上行音，接近硬币。
    _write(
        "cards/default/buy.wav",
        _mix(
            _bell(1046.5, 0.16, 0.6, decay=16.0),
            _at(_bell(1568.0, 0.14, 0.45, decay=18.0), 0.035),
        ),
    )

    # 攻击：低频冲击 + 低通噪声。
    _write(
        "cards/default/attack.wav",
        _mix(
            _thump(150, 70, 0.14, 0.9),
            _noise(0.11, 0.5, smooth=9, seed=3),
        ),
    )

    # 死亡：下行低音，带一点沙哑噪声。
    _write(
        "cards/default/death.wav",
        _mix(
            _sweep(520, 110, 0.42, 0.8),
            _at(_noise(0.32, 0.16, smooth=14, seed=5), 0.02),
            _thump(110, 55, 0.3, 0.5),
        ),
    )


if __name__ == "__main__":
    build()
