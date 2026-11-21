#!/usr/bin/env python3
"""从 v0.2 成就实体文档生成成就映射数据。

只读 docs/versions/v0.2/product/content/achievements/ 下的实体文档，输出确定性 JSON
到 server/data/v0.2/achievements.json。front matter 解析复用 scripts/validate_docs.py，
不重复实现。输出使用 ensure_ascii=False、indent=2、sort_keys=True，并以换行结尾。

服务端按 `id` 建立 `steam_api_name` 映射；其余字段原样保留，供后续成就运行时读取。
"""

import argparse
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import validate_docs

ROOT = pathlib.Path(__file__).resolve().parent.parent
CONTENT = ROOT / "docs" / "versions" / "v0.2" / "product" / "content"
DEFAULT_OUTPUT = ROOT / "server" / "data" / "v0.2" / "achievements.json"
FIELDS = validate_docs.ACHIEVEMENT_FIELDS


def dumps(value):
    """确定性 JSON 文本：UTF-8 原文、两空格缩进、键排序、末尾换行。"""
    return json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n"


def load_docs(content=CONTENT):
    """读取成就实体文档；按 id 排序，字段顺序不参与输出（写入时排序键）。"""
    docs = []
    for path in sorted((content / "achievements").glob("*.md")):
        if path.name == "README.md":
            continue
        doc = validate_docs.parse_front_matter(path)
        if path.stem != doc.get("id"):
            raise ValueError(f"{path.name}: 文件名与 id {doc.get('id')!r} 不一致")
        docs.append(doc)
    return sorted(docs, key=lambda doc: doc["id"])


def build(content=CONTENT):
    """返回 {输出文件名: JSON 文本}。"""
    records = [{key: doc[key] for key in FIELDS} for doc in load_docs(content)]
    return {DEFAULT_OUTPUT.name: dumps(records)}


def write(content, output_path):
    """把生成结果写入 output_path（父目录不存在时创建）。"""
    out = pathlib.Path(output_path)
    out.parent.mkdir(parents=True, exist_ok=True)
    data = build(content)
    out.write_text(data[out.name], encoding="utf-8")


def check(output_path):
    """比较当前生成结果与已提交数据；返回是否漂移。"""
    out = pathlib.Path(output_path)
    data = build(CONTENT)
    if not out.exists():
        return True
    return out.read_text(encoding="utf-8") != data[out.name]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="检测生成数据是否漂移，有差异退出 1")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT), help="输出文件")
    args = parser.parse_args(argv)
    if args.check:
        if check(args.output):
            print(f"数据漂移: {args.output}")
            return 1
        print("生成数据无漂移")
        return 0
    write(CONTENT, args.output)
    print(f"已生成 {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
