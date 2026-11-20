#!/usr/bin/env python3
"""重写 v0.1 内容文档中的派生小节。

技能实例与卡牌的「效果」正文和属性表派生行由模板与引用生成；本脚本按源文档
重写这些片段，手写小节（设计说明、出处等）原样保留。`--check` 只报告漂移，
不写文件，供 `make data-check` 拦截文档与数据不一致。
"""

import argparse
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import gen_card_data
import skill_templates
import validate_docs

ROOT = pathlib.Path(__file__).resolve().parent.parent
CONTENT = pathlib.Path("docs") / "versions" / "v0.1" / "product" / "content"

README = "README.md"
LINK_RE = re.compile(r"^\[([a-z0-9-]+)\]\(([a-z0-9-]+)\.md\)$")


def _set_section(text, heading, body):
    """把「## <heading>」小节正文替换为 body（行列表），保留其余小节。"""
    lines = text.split("\n")
    start = None
    for index, line in enumerate(lines):
        if line.strip() == f"## {heading}":
            start = index + 1
            break
    if start is None:
        raise ValueError(f"缺少「## {heading}」小节")
    end = len(lines)
    for index in range(start, len(lines)):
        if lines[index].startswith("## "):
            end = index
            break
    # 保留原标题与正文之间的空行数量，避免只改正文时制造无意义 diff。
    blanks = 0
    while start + blanks < end and not lines[start + blanks].strip():
        blanks += 1
    rest = lines[end:]
    rebuilt = lines[:start] + [""] * max(blanks, 1) + list(body)
    if rest:
        rebuilt += [""] + rest
    return "\n".join(rebuilt).rstrip("\n") + "\n"


def _table(rows):
    """属性表：表头、分隔行与数据行。"""
    return ["| 属性 | 值 |", "| --- | --- |"] + list(rows)


def _set_table_rows(text, heading, values):
    """只替换属性表中键出现在 values 里的行，其余行原样保留。"""
    lines = text.split("\n")
    start = None
    for index, line in enumerate(lines):
        if line.strip() == f"## {heading}":
            start = index + 1
            break
    if start is None:
        raise ValueError(f"缺少「## {heading}」小节")
    end = len(lines)
    for index in range(start, len(lines)):
        if lines[index].startswith("## "):
            end = index
            break
    for index in range(start, end):
        cells = lines[index].split("|")
        if len(cells) < 3:
            continue
        key = cells[1].strip()
        if key in values:
            cells[2] = f" {values[key]} "
            lines[index] = "|".join(cells)
    return "\n".join(lines)


def _keyword_label(keyword, keyword_labels):
    """`breed 1` 渲染为「繁衍 1」；无层数时只取名词。"""
    slug, _, level = str(keyword).partition(" ")
    label = keyword_labels[slug]
    return f"{label} {level}" if level else label


def _skill_table(doc, keyword_labels):
    keyword = doc.get("keyword")
    rows = [
        f"| id | {doc['id']} |",
        f"| 关键词 | {_keyword_label(keyword, keyword_labels) if keyword else '无'} |",
        f"| 确定性替换 | {doc.get('match_text') or '不适用'} |",
    ]
    if "template" in doc:
        ids = "；".join(call.template_id
                        for call in skill_templates.parse_template_field(doc["template"]))
        rows.append(f"| 模板 | {ids} |")
    if doc.get("cost") is not None:
        rows.append(f"| 费用 | {doc['cost']} |")
    if doc.get("uses_per_turn") is not None:
        rows.append(f"| 每回合次数 | {doc['uses_per_turn']} |")
    return rows


def _card_sections(card, skills, renderer, keyword_labels):
    """返回卡牌「技能」「关键词」两行与「效果」正文。"""
    ids = card.get("skills", [])
    labels = []
    for skill_id in ids:
        if skill_id not in skills:
            raise ValueError(f"{card['id']}: 引用的技能不存在 {skill_id!r}")
        keyword = skills[skill_id].get("keyword")
        if keyword:
            label = _keyword_label(keyword, keyword_labels)
            if label not in labels:
                labels.append(label)
    texts = [renderer.instance_text(skills[skill_id]) for skill_id in ids]
    return {
        "技能": "；".join(ids) if ids else "无",
        "关键词": "；".join(labels) if labels else "无",
    }, ["；".join(texts) if texts else "无"]


def _render_readme(text, skills, renderer):
    """重写技能索引表中每行的「文本」单元格。"""
    lines = []
    for line in text.split("\n"):
        cells = line.split("|")
        if len(cells) == 6:
            match = LINK_RE.fullmatch(cells[1].strip())
            if match and match.group(1) in skills:
                cells[3] = f" {renderer.instance_text(skills[match.group(1)])} "
                line = "|".join(cells)
        lines.append(line)
    return "\n".join(lines)


def _render(content, keyword_labels):
    """返回 {相对路径: 渲染后文本}。"""
    content = pathlib.Path(content)
    renderer = skill_templates.Renderer(content)
    skills = {doc["id"]: doc for doc in gen_card_data.load_docs(content, "skills")}
    rendered = {}
    for path in sorted((content / "skills").glob("*.md")):
        if path.name == README:
            continue
        doc = validate_docs.parse_front_matter(path)
        text = path.read_text(encoding="utf-8")
        text = _set_section(text, "属性", _table(_skill_table(doc, keyword_labels)))
        text = _set_section(text, "效果", [renderer.instance_text(doc)])
        rendered[path.relative_to(content).as_posix()] = text
    for path in sorted((content / "cards").glob("*.md")):
        if path.name == README:
            continue
        doc = validate_docs.parse_front_matter(path)
        text = path.read_text(encoding="utf-8")
        rows, body = _card_sections(doc, skills, renderer, keyword_labels)
        text = _set_table_rows(text, "属性", rows)
        text = _set_section(text, "效果", body)
        rendered[path.relative_to(content).as_posix()] = text
    readme_path = content / "skills" / README
    if readme_path.exists():
        rendered["skills/README.md"] = _render_readme(
            readme_path.read_text(encoding="utf-8"), skills, renderer)
    return rendered


def render(content=CONTENT, write=False):
    """重写派生小节；返回漂移文件列表（相对内容根）。"""
    content = pathlib.Path(content)
    keyword_labels = validate_docs.keyword_labels()
    drift = []
    for name, text in sorted(_render(content, keyword_labels).items()):
        path = content / name
        original = path.read_text(encoding="utf-8")
        if not original.endswith("\n"):
            text = text.rstrip("\n")
        if original == text:
            continue
        drift.append(name)
        if write:
            path.write_text(text, encoding="utf-8")
    return drift


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--content", default=str(ROOT / CONTENT), help="内容根目录")
    parser.add_argument("--check", action="store_true",
                        help="只检测漂移，不写文件；有差异退出 1")
    args = parser.parse_args(argv)
    drift = render(args.content, write=not args.check)
    if args.check:
        if drift:
            print(f"文档漂移 {len(drift)} 个文件：{', '.join(drift)}")
            return 1
        print("派生文档无漂移")
        return 0
    print(f"已重写 {len(drift)} 个文档")
    return 0


if __name__ == "__main__":
    sys.exit(main())
