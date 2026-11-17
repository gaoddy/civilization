#!/usr/bin/env python3
"""从 v0.1 实体文档生成客户端只读展示表。

只读 docs/versions/v0.1/product 下的实体文档，输出确定性 JSON 到
client/generated/。只输出展示费用与规则字段，文本由文本表提供；客户端不做规则计算。
费用只用于展示：随从购买费用固定 3，效果牌取文档 cost。
"""

import argparse
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import gen_card_data  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = ROOT / "client" / "generated"
FILES = ("cards.json", "skills.json", "tokens.json")

# 与 rule-engine 的随从购买费用一致；客户端仅用于展示，不参与判定。
MINION_DISPLAY_COST = 3


def _display_card(doc):
    card = gen_card_data.card_entry(doc)
    card["display_cost"] = MINION_DISPLAY_COST if card["card_type"] == "minion" else card["cost"]
    return card


def _display_token(token):
    token = dict(token)
    token["display_cost"] = MINION_DISPLAY_COST
    return token


def build(root):
    """返回文件名到确定性 JSON 文本的映射。"""
    content = pathlib.Path(root) / "docs" / "versions" / "v0.1" / "product" / "content"
    cards = [_display_card(doc) for doc in gen_card_data.load_docs(content, "cards")]
    skills = [gen_card_data.skill_entry(doc) for doc in gen_card_data.load_docs(content, "skills")]
    tokens = [_display_token(token) for token in gen_card_data.parse_tokens(content / "tokens" / "README.md")]
    return {
        "cards.json": gen_card_data.dumps(cards),
        "skills.json": gen_card_data.dumps(skills),
        "tokens.json": gen_card_data.dumps(tokens),
    }


def write(root, output_dir):
    """将生成结果写入 output_dir（不存在时创建）。"""
    out = pathlib.Path(output_dir)
    out.mkdir(parents=True, exist_ok=True)
    data = build(root)
    for name in FILES:
        (out / name).write_text(data[name], encoding="utf-8")


def check(output_dir):
    """比较当前生成结果与已提交数据；返回差异文件名列表。"""
    out = pathlib.Path(output_dir)
    data = build(ROOT)
    drift = []
    for name in FILES:
        path = out / name
        if not path.exists() or path.read_text(encoding="utf-8") != data[name]:
            drift.append(name)
    return drift


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
    print(f"已生成 {len(FILES)} 个文件到 {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
