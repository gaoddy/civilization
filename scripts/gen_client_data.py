#!/usr/bin/env python3
"""组装客户端下发内容 `server/data/v0.1/client-content.json`。

与 `gen_card_data.py` 使用同一份生成结果，只额外附加展示费用 `display_cost`，
其余字段原样下发。文本仍由客户端本地文本表提供，客户端不做规则计算。

bundle 结构：`{"version", "cards", "heroes", "skills", "taxonomy"}`；
版本为四段确定性 JSON 内容的 SHA-256 前 16 位，供客户端缓存比对与 ETag。
"""

import argparse
import hashlib
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import gen_card_data  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = ROOT / "server" / "data" / "v0.1"
FILE = "client-content.json"
SECTIONS = ("cards", "heroes", "skills", "taxonomy")
VERSION_LENGTH = 16

# 与 rule-engine 的随从购买费用一致；客户端仅用于展示，不参与判定。
MINION_DISPLAY_COST = 3


def _display_card(card):
    """规则表原样保留，只补展示费用；随从固定 3，其余取 cost。"""
    return {
        **card,
        "display_cost": MINION_DISPLAY_COST if card["card_type"] == "minion" else card["cost"],
    }


def content_version(sections):
    """四段数据确定性序列化后的短哈希；任一内容变化都会改变版本。"""
    canonical = json.dumps(
        sections, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    )
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:VERSION_LENGTH]


def build(root):
    """返回文件名到确定性 JSON 文本的映射。"""
    tables = gen_card_data.build(root)
    sections = {name: json.loads(tables[f"{name}.json"]) for name in SECTIONS}
    sections["cards"] = [_display_card(card) for card in sections["cards"]]
    bundle = {"version": content_version(sections), **sections}
    return {FILE: gen_card_data.dumps(bundle)}


def write(root, output_dir):
    """将生成结果写入 output_dir（不存在时创建）。"""
    out = pathlib.Path(output_dir)
    out.mkdir(parents=True, exist_ok=True)
    (out / FILE).write_text(build(root)[FILE], encoding="utf-8")


def check(output_dir):
    """比较当前生成结果与已提交数据；返回差异文件名列表。"""
    path = pathlib.Path(output_dir) / FILE
    data = build(ROOT)[FILE]
    if not path.exists() or path.read_text(encoding="utf-8") != data:
        return [FILE]
    return []


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="检测生成数据是否漂移，有差异退出 1")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT), help="输出目录")
    args = parser.parse_args(argv)
    if args.check:
        drift = check(args.output)
        if drift:
            print(f"客户端数据漂移 {len(drift)} 个文件：{', '.join(drift)}")
            return 1
        print("客户端生成数据无漂移")
        return 0
    write(ROOT, args.output)
    print(f"已生成 {FILE} 到 {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
