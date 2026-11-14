#!/usr/bin/env python3
"""从 v0.1 实体文档生成卡牌、技能、Token 与经济配置数据。

只读 docs/versions/v0.1/product 下的实体文档与数值系统文档，输出确定性 JSON
到 server/data/v0.1/。front matter 解析复用 scripts/validate_docs.py，不重复实现。
输出使用 ensure_ascii=False、indent=2、sort_keys=True，并以换行结尾。
"""

import argparse
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import validate_docs

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = ROOT / "server" / "data" / "v0.1"
FILES = ("cards.json", "skills.json", "tokens.json", "config.json")

# 产品基线常量；来源 docs/versions/v0.1/product/systems/economy-and-numbers.md
# 与 rules/ 下的核心规则（生命、槽位、回合上限、伤害上下限）。
CONFIG = {
    "initial_hp": 40,
    "board_slots": 7,
    "hand_slots": 10,
    "shop_slots": 6,
    "turn_limit": 20,
    "damage_cap": 15,
    "damage_floor": 1,
    "era_costs": [5, 7, 9, 10, 11, 12],
    "pool_counts": [16, 15, 13, 11, 9, 7, 5],
    "shop_weights": [
        [100, 0, 0, 0, 0, 0, 0],
        [70, 30, 0, 0, 0, 0, 0],
        [50, 35, 15, 0, 0, 0, 0],
        [30, 35, 25, 10, 0, 0, 0],
        [20, 25, 25, 20, 10, 0, 0],
        [10, 15, 25, 25, 15, 10, 0],
        [5, 10, 15, 20, 20, 18, 12],
    ],
    "gold_curve": [3, 3, 4, 5, 6, 7, 8, 9, 10, 10,
                   10, 10, 10, 10, 10, 10, 10, 10, 10, 10],
}


def dumps(value):
    """确定性 JSON 文本：UTF-8 原文、两空格缩进、键排序、末尾换行。"""
    return json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def load_docs(content, kind):
    """按给定内容根读取实体文档并校验文件名与 id 一致；按 id 排序。"""
    docs = []
    for path in sorted((content / kind).glob("*.md")):
        if path.name == "README.md":
            continue
        doc = validate_docs.parse_front_matter(path)
        if path.stem != doc["id"]:
            raise ValueError(f"{path.name}: 文件名与 id {doc['id']!r} 不一致")
        docs.append(doc)
    return sorted(docs, key=lambda doc: doc["id"])


def raw_effects(value):
    """effects 字段保留原始 DSL 字符串；front matter 的 `[]` 还原为 "[]"。"""
    if isinstance(value, list):
        if value:
            raise ValueError(f"effects 不能是非空列表: {value!r}")
        return "[]"
    return str(value)


def card_entry(doc):
    return {
        "id": doc["id"],
        "card_type": doc["card_type"],
        "name": doc["name"],
        "tribe": doc["tribe"],
        "subtype": doc.get("subtype"),
        "level": doc["level"],
        "era": doc.get("era"),
        "attack": doc.get("attack"),
        "health": doc.get("health"),
        "skills": doc.get("skills", []),
        "is_scholar": doc.get("is_scholar", False),
        "is_hero": doc.get("is_hero", False),
        "cost": doc.get("cost"),
        "match_text": doc.get("match_text"),
    }


def skill_entry(doc):
    return {
        "id": doc["id"],
        "name": doc.get("name"),
        "keyword": doc.get("keyword"),
        "text": doc["text"],
        "effects": raw_effects(doc["effects"]),
        "cost": doc.get("cost"),
        "uses_per_turn": doc.get("uses_per_turn"),
        "match_text": doc.get("match_text"),
    }


def parse_tokens(path):
    """解析 tokens/README.md 的「## 2. 注册表」表格。"""
    text = path.read_text(encoding="utf-8")
    marker = "## 2. 注册表"
    if marker not in text:
        raise ValueError(f"{path.name}: 缺少「{marker}」小节")
    rest = text.split(marker, 1)[1]
    section = rest.split("\n## ", 1)[0]
    tokens = []
    for line in section.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) != 6:
            continue
        if set(cells[0]) <= {"-"} or cells[0] == "id":
            continue
        name, attack, health, keywords = cells[1], cells[2], cells[3], cells[4]
        tokens.append({
            "id": cells[0].strip("`"),
            "name": name,
            "attack": int(attack),
            "health": int(health),
            "keywords": [] if keywords == "无" else keywords.split("、"),
        })
    return sorted(tokens, key=lambda token: token["id"])


def build(root):
    """返回文件名到确定性 JSON 文本的映射。"""
    content = pathlib.Path(root) / "docs" / "versions" / "v0.1" / "product" / "content"
    cards = [card_entry(doc) for doc in load_docs(content, "cards")]
    skills = [skill_entry(doc) for doc in load_docs(content, "skills")]
    tokens = parse_tokens(content / "tokens" / "README.md")
    return {
        "cards.json": dumps(cards),
        "skills.json": dumps(skills),
        "tokens.json": dumps(tokens),
        "config.json": dumps(CONFIG),
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
            print(f"数据漂移 {len(drift)} 个文件：{', '.join(drift)}")
            return 1
        print("生成数据无漂移")
        return 0
    write(ROOT, args.output)
    print(f"已生成 {len(FILES)} 个文件到 {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
