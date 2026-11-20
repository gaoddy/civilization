#!/usr/bin/env python3
"""生成客户端文本表 client/text/zh_CN.csv。

内容 key（card/skill/token）由实体文档派生；人工 key（ui/error/枚举）保留在
CSV 中参与合并。生成部分每次整体重算，人工部分原样保留，未知前缀直接报错。
"""

import argparse
import csv
import io
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import gen_card_data
import i18n_terms
import skill_templates
import validate_docs

ROOT = pathlib.Path(__file__).resolve().parent.parent
CSV_PATH = ROOT / "client" / "text" / "zh_CN.csv"
CONTENT = pathlib.Path("docs") / "versions" / "v0.1" / "product" / "content"


def _unescape(value):
    """把 Godot CSV 的 `\\n`、`\\\\` 还原；从左到右扫描，避免二次解码。"""
    result = []
    index = 0
    while index < len(value):
        char = value[index]
        if char == "\\" and index + 1 < len(value):
            nxt = value[index + 1]
            if nxt == "n":
                result.append("\n")
                index += 2
                continue
            if nxt == "\\":
                result.append("\\")
                index += 2
                continue
        result.append(char)
        index += 1
    return "".join(result)


def _escape(value):
    """把真实反斜杠与换行转成 Godot CSV 的单行形式。"""
    return value.replace("\\", "\\\\").replace("\n", "\\n")


def load_existing(path):
    """读取已提交的文本表；文件不存在时返回空字典。"""
    path = pathlib.Path(path)
    if not path.exists():
        return {}
    entries = {}
    with path.open(encoding="utf-8", newline="") as handle:
        rows = csv.reader(handle)
        header = next(rows, None)
        if header != ["keys", "zh_CN"]:
            raise ValueError(f"{path}: 表头必须为 keys,zh_CN，实为 {header!r}")
        for row in rows:
            if not row:
                continue
            if len(row) != 2:
                raise ValueError(f"{path}: 行字段数不是 2：{row!r}")
            if row[0] in entries:
                raise ValueError(f"{path}: 重复的文本 key {row[0]!r}")
            entries[row[0]] = _unescape(row[1])
    return entries


def generated_entries(root):
    """从实体文档派生内容 key 与枚举 key。"""
    content = pathlib.Path(root) / CONTENT
    renderer = skill_templates.Renderer(content)
    entries = {}
    for doc in gen_card_data.load_docs(content, "cards"):
        entries[f"card.{doc['id']}.name"] = doc["name"]
        match_text = doc.get("match_text")
        if match_text:
            entries[f"card.{doc['id']}.match_text"] = match_text
    for doc in gen_card_data.load_docs(content, "skills"):
        if doc.get("name"):
            entries[f"skill.{doc['id']}.name"] = doc["name"]
        entries[f"skill.{doc['id']}.text"] = renderer.instance_text(doc)
        match_text = doc.get("match_text")
        if match_text:
            entries[f"skill.{doc['id']}.match_text"] = match_text
    token_names = gen_card_data.parse_token_names(content / "tokens" / "README.md")
    for token_id, name in token_names.items():
        entries[f"token.{token_id}.name"] = name

    raw = validate_docs.load(
        "cards", ["id", "card_type", "categories", "taxon", "era"], root=content)
    enum_labels = validate_docs.card_enum_labels(raw, root=content)
    hero_raw = validate_docs.load(
        "heroes", ["id", "name", "card_type", "categories"], root=content)
    for doc in hero_raw:
        entries[f"card.{doc['id']}.name"] = doc["name"]
    hero_labels = validate_docs.card_enum_labels(hero_raw, root=content, kind="heroes")
    for value, label in hero_labels["card_type"].items():
        entries[f"enum.card_type.{value}"] = label
    for field, prefix in (("card_type", "enum.card_type."),
                          ("era", "enum.era.")):
        for value, label in enum_labels[field].items():
            entries[prefix + value] = label
    # 玩法与科学分类展示名以 taxonomy.md 为唯一来源；属性表中文名用于冲突校验。
    taxonomy_nodes, playstyles = validate_docs.parse_taxonomy(content / "taxonomy.md")
    for playstyle_id, label in playstyles or []:
        entries[f"enum.playstyle.{playstyle_id}"] = label
    # 中立不是玩法表的一行，但 Token 与筛选需要同样的取值入口。
    entries["enum.playstyle.neutral"] = "中立"
    playstyle_ids = {playstyle_id for playstyle_id, _ in playstyles or []}
    for doc in raw + hero_raw:
        for kind, target in validate_docs.parse_categories(doc):
            if kind == "play" and target not in playstyle_ids:
                raise ValueError(
                    f"{doc['id']}: play {target!r} 不在 taxonomy.md 的玩法表中")
    for node_id, node in (taxonomy_nodes or {}).items():
        entries[f"enum.taxon.{node_id}"] = node["name"]
    for value, label in enum_labels["playstyle"].items():
        expected = entries.get(f"enum.playstyle.{value}")
        if expected is not None and expected != label:
            raise ValueError(
                f"卡牌属性表 playstyle {value} 的中文名 {label!r} 与 taxonomy.md "
                f"的 {expected!r} 冲突")
    for value, label in enum_labels["taxon"].items():
        # 属性表「科学分类」列是完整路径，取末级节点名参与比对。
        leaf_label = label.rsplit("›", 1)[-1].strip()
        expected = entries.get(f"enum.taxon.{value}")
        if expected is not None and expected != leaf_label:
            raise ValueError(
                f"卡牌属性表 taxon {value} 的中文名 {leaf_label!r} 与 taxonomy.md "
                f"的 {expected!r} 冲突")
    for slug, label in validate_docs.keyword_labels().items():
        entries[f"enum.keyword.{slug}"] = label
    for value, label in enum_labels["subtype"].items():
        entries[f"enum.subtype.{value}"] = label
    return entries


def merge(existing, generated):
    """丢弃 existing 中生成前缀的 key，保留人工前缀，其余前缀报错。"""
    merged = {}
    for key, value in existing.items():
        if key.startswith(i18n_terms.GENERATED_PREFIXES):
            continue
        if key.startswith(i18n_terms.LEGACY_PREFIXES):
            continue
        if not key.startswith(i18n_terms.MANUAL_PREFIXES):
            raise ValueError(f"未知前缀的文本 key: {key}")
        merged[key] = value
    for key in generated:
        if not key.startswith(i18n_terms.GENERATED_PREFIXES):
            raise ValueError(f"生成 key 前缀非法: {key}")
    merged.update(generated)
    return merged


def build(root, existing_path=CSV_PATH):
    """返回完整 CSV 文本：表头、按 key 排序、值转义。"""
    entries = merge(load_existing(existing_path), generated_entries(root))
    buffer = io.StringIO()
    writer = csv.writer(buffer, lineterminator="\n")
    writer.writerow(["keys", "zh_CN"])
    for key in sorted(entries):
        writer.writerow([key, _escape(entries[key])])
    return buffer.getvalue()


def write(root, path=CSV_PATH):
    """按 build 结果写入文本表。"""
    path = pathlib.Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(build(root, path), encoding="utf-8")


def check(path=CSV_PATH):
    """比较已提交文本表与生成结果；一致返回空列表。"""
    path = pathlib.Path(path)
    if path.exists() and path.read_text(encoding="utf-8") == build(ROOT, path):
        return []
    try:
        return [str(path.relative_to(ROOT))]
    except ValueError:
        return [str(path)]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="检测文本表是否漂移，有差异退出 1")
    parser.add_argument("--output", default=str(CSV_PATH), help="输出文件")
    args = parser.parse_args(argv)
    if args.check:
        drift = check(args.output)
        if drift:
            print(f"文本漂移 {len(drift)} 个文件：{', '.join(drift)}")
            return 1
        print("文本表无漂移")
        return 0
    write(ROOT, args.output)
    print(f"已生成文本表 {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
