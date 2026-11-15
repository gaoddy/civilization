#!/usr/bin/env python3
"""从 protocol/schema/v1 生成协议字段表。

运行：
    python3 scripts/gen_protocol_docs.py            # 生成到默认位置
    python3 scripts/gen_protocol_docs.py --check    # 有差异退出 1
"""

import argparse
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from validate_protocol import _resolve, load_mapping  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = ROOT / "docs" / "versions" / "v0.1" / "technical" / "architecture" / "protocol-messages.md"


def _load(schema_dir, filename):
    return json.loads((pathlib.Path(schema_dir) / filename).read_text())


def _type_label(schema):
    if "$ref" in schema:
        return schema["$ref"].rsplit("/", 1)[-1]
    if "enum" in schema:
        return "enum"
    if "oneOf" in schema:
        return "oneOf"
    value = schema.get("type", "object")
    if isinstance(value, list):
        return " | ".join(value)
    if value == "array":
        return f"array<{_type_label(schema.get('items', {}))}>"
    return value


def _rows(schema, prefix, required, out):
    if "$ref" in schema:
        return
    if "oneOf" in schema:
        out.append((prefix.rstrip(".") or "(根)", "oneOf", "是", "下列分支任选其一"))
        for index, branch in enumerate(schema["oneOf"], start=1):
            _rows(branch, f"{prefix}分支{index}.", set(branch.get("required", [])), out)
        return
    for name, subschema in schema.get("properties", {}).items():
        path = f"{prefix}{name}"
        is_required = "是" if name in required else "否"
        out.append((path, _type_label(subschema), is_required, subschema.get("description", "")))
        if subschema.get("type") == "object":
            _rows(subschema, f"{path}.", set(subschema.get("required", [])), out)
        elif subschema.get("type") == "array" and isinstance(subschema.get("items"), dict):
            items = subschema["items"]
            _rows(items, f"{path}[].", set(items.get("required", [])), out)


def _table(title, schema, base_file="messages.schema.json"):
    while "$ref" in schema:
        schema, base_file = _resolve(schema["$ref"], base_file)
    lines = [f"### {title}", "", "| 字段 | 类型 | 必填 | 说明 |", "| --- | --- | --- | --- |"]
    rows = []
    _rows(schema, "", set(schema.get("required", [])), rows)
    for path, type_label, required, description in rows:
        lines.append(f"| `{path}` | {type_label} | {required} | {description} |")
    lines.append("")
    return lines


def render(schema_dir, mapping):
    schema_dir = pathlib.Path(schema_dir)
    messages = _load(schema_dir, "messages.schema.json")["$defs"]
    lines = [
        "# 协议字段表 v1",
        "",
        "> 本文件由 `scripts/gen_protocol_docs.py` 从 `protocol/schema/v1/` 生成，不要手改。",
        "> 规则与消息语义见 [protocol.md](protocol.md)。",
        "",
    ]

    for direction, heading in (("c2s", "上行消息（C→S）"), ("s2c", "下行消息（S→C）")):
        lines.extend([f"## {heading}", ""])
        for name in sorted(mapping["directions"][direction]):
            def_name = mapping["directions"][direction][name].rsplit("/", 1)[-1]
            lines.extend(_table(name, messages[def_name]))

    lines.extend(["## 共享类型", ""])
    for filename in ("view.schema.json", "intents.schema.json", "errors.schema.json"):
        for name, schema in _load(schema_dir, filename)["$defs"].items():
            lines.extend(_table(name, schema))
    return "\n".join(lines).rstrip() + "\n"


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--output", type=pathlib.Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args(argv)

    content = render(ROOT / "protocol" / "schema" / "v1", load_mapping())

    if args.check:
        existing = args.output.read_text() if args.output.exists() else ""
        if existing != content:
            print(f"{args.output} 与 schema 不一致，请运行 make protocol", file=sys.stderr)
            return 1
        return 0

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(content)
    print(f"已生成 {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
