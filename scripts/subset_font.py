#!/usr/bin/env python3
"""生成客户端内置的中文字体子集。

工程把这一份子集设成 Godot 默认主题字体，iOS 不再依赖系统字体回退。
字符集 = `client/text/zh_CN.csv` 的全部字符 + 可打印 ASCII + GB2312 常用字，
兼顾界面文案与玩家自填显示名；文案新增生僻字时重跑本脚本。

依赖 fontTools：

    python3 -m venv output/fonttools-venv
    output/fonttools-venv/bin/pip install fonttools
    make font-subset PYTHON=output/fonttools-venv/bin/python

源字体为 Google Fonts 的 Noto Sans SC（OFL），变量字体按 wght=400 取静态实例。
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

DEFAULT_WEIGHT = 400
PRINTABLE_ASCII = {chr(code) for code in range(0x20, 0x7F)}
FAMILY_NAME = "Noto Sans SC"
STYLE_NAME = "Regular"
NAME_IDS = {
    1: FAMILY_NAME,
    2: STYLE_NAME,
    4: f"{FAMILY_NAME} {STYLE_NAME}",
    6: f"NotoSansSC-{STYLE_NAME}",
    16: FAMILY_NAME,
    17: STYLE_NAME,
}


def normalize_names(font: TTFont) -> None:
    """变量字体实例化的默认命名是 Thin，按实际字重改写名称与 OS/2 字重。"""
    font["OS/2"].usWeightClass = DEFAULT_WEIGHT
    name_table = font["name"]
    for record in name_table.names:
        if record.nameID in NAME_IDS:
            record.string = NAME_IDS[record.nameID].encode(record.getEncoding())


def normalize_metrics(font: TTFont) -> None:
    """垂直度量对齐 PingFang SC：桌面原本用系统回退的繁黑渲染，行高按它定版。"""
    upem = font["head"].unitsPerEm
    scale = upem / 1000.0
    hhea = font["hhea"]
    hhea.ascent = round(1060 * scale)
    hhea.descent = round(-340 * scale)
    hhea.lineGap = 0
    os2 = font["OS/2"]
    os2.sTypoAscender = round(860 * scale)
    os2.sTypoDescender = round(-140 * scale)
    os2.sTypoLineGap = round(400 * scale)
    os2.usWinAscent = round(1060 * scale)
    os2.usWinDescent = round(340 * scale)


def csv_chars(path: Path) -> set[str]:
    """文本表的全部非空白字符。"""
    text = path.read_text(encoding="utf-8")
    return {ch for ch in text if not ch.isspace()}


def gb2312_chars() -> set[str]:
    """GB2312 可编码字符：覆盖常用字与全角标点，供玩家自填显示名。"""
    chars: set[str] = set()
    for high in range(0xA1, 0xF8):
        for low in range(0xA1, 0xFF):
            try:
                chars.add(bytes([high, low]).decode("gb2312"))
            except UnicodeDecodeError:
                continue
    return chars


def build_subset(source: Path, output: Path, charset: set[str]) -> None:
    font = TTFont(source)
    if "fvar" in font:
        font = instantiateVariableFont(font, {"wght": DEFAULT_WEIGHT})
        normalize_names(font)
    else:
        normalize_names(font)
    normalize_metrics(font)
    options = subset.Options()
    # 保留完整 name 表：OFL 要求随字体分发版权与许可信息。
    options.name_IDs = ["*"]
    options.name_languages = ["*"]
    options.hinting = True
    subsetter = subset.Subsetter(options=options)
    subsetter.populate(unicodes={ord(ch) for ch in charset})
    subsetter.subset(font)
    output.parent.mkdir(parents=True, exist_ok=True)
    font.save(output)
    font.close()


def missing_chars(output: Path, charset: set[str]) -> list[str]:
    font = TTFont(output)
    cmap = font.getBestCmap()
    font.close()
    return sorted(ch for ch in charset if ord(ch) not in cmap)


def main() -> int:
    parser = argparse.ArgumentParser(description="生成内置中文字体子集")
    parser.add_argument("--csv", default="client/text/zh_CN.csv", help="文本表路径")
    parser.add_argument("--source", required=True, help="源字体路径（Noto Sans SC）")
    parser.add_argument("--output", required=True, help="输出字体路径")
    parser.add_argument("--no-gb2312", action="store_true", help="不并入 GB2312 常用字")
    args = parser.parse_args()

    csv_path = Path(args.csv)
    if not csv_path.is_file():
        print(f"缺少文本表：{csv_path}", file=sys.stderr)
        return 1
    source = Path(args.source)
    if not source.is_file():
        print(f"缺少源字体：{source}", file=sys.stderr)
        return 1

    text_chars = csv_chars(csv_path)
    charset = set(text_chars) | PRINTABLE_ASCII | {" "}
    if not args.no_gb2312:
        charset |= gb2312_chars()

    output = Path(args.output)
    build_subset(source, output, charset)

    missing = missing_chars(output, text_chars)
    if missing:
        print("子集缺少文本表字符：" + "".join(missing), file=sys.stderr)
        return 1
    print(f"字体子集：{output}（{len(charset)} 字符，{output.stat().st_size} 字节）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
