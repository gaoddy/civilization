#!/usr/bin/env python3
"""从 protocol/schema/v1 生成 GDScript 协议常量。

运行：
    python3 scripts/gen_protocol_gdscript.py            # 生成到默认位置
    python3 scripts/gen_protocol_gdscript.py --check    # 有差异退出 1
"""

import argparse
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from validate_protocol import load_mapping  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = ROOT / "protocol" / "generated" / "protocol.gd"
CLIENT_OUTPUT = ROOT / "client" / "generated" / "protocol.gd"


def _const_name(value):
    parts = [p for p in re.split(r"[^A-Za-z0-9]+", value) if p]
    out = []
    for part in parts:
        out.append(part if part.isupper() else re.sub(r"(?<!^)(?=[A-Z])", "_", part).upper())
    return "_".join(out)


def _load(schema_dir, filename):
    return json.loads((pathlib.Path(schema_dir) / filename).read_text())


def render(schema_dir, mapping):
    schema_dir = pathlib.Path(schema_dir)
    lines = [
        "class_name Protocol",
        "# 本文件由 scripts/gen_protocol_gdscript.py 生成，不要手改。",
        "",
        f"const PROTOCOL_VERSION := {mapping['v']}",
        "",
    ]

    message_types = sorted({t for d in ("c2s", "s2c") for t in mapping["directions"][d]})
    lines.append("# 消息类型")
    for name in message_types:
        lines.append(f'const MSG_{_const_name(name)} := "{name}"')
    lines.append("")

    intents = _load(schema_dir, "intents.schema.json")["$defs"]["Intent"]["oneOf"]
    lines.append("# 意图")
    for kind in sorted(branch["properties"]["kind"]["const"] for branch in intents):
        lines.append(f'const INTENT_{_const_name(kind)} := "{kind}"')
    lines.append("")

    errors = _load(schema_dir, "errors.schema.json")["$defs"]["ErrorCode"]["enum"]
    lines.append("# 错误码")
    for code in sorted(errors):
        lines.append(f'const ERROR_{_const_name(code)} := "{code}"')
    lines.append("")

    phases = _load(schema_dir, "view.schema.json")["$defs"]["Phase"]["enum"]
    lines.append("# 阶段")
    for phase in phases:
        lines.append(f'const PHASE_{_const_name(phase)} := "{phase}"')
    lines.append("")

    events = _load(schema_dir, "view.schema.json")["$defs"]["GameEvent"]["properties"]["kind"]["enum"]
    lines.append("# 领域事件")
    for event in events:
        lines.append(f'const EVENT_{_const_name(event)} := "{event}"')
    lines.append("")

    view_fields = _load(schema_dir, "view.schema.json")["$defs"]["PlayerView"]["properties"]
    lines.append("# PlayerView 字段名")
    for field in sorted(view_fields):
        lines.append(f'const FIELD_{_const_name(field)} := "{field}"')
    lines.append("")
    return "\n".join(lines)


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--output", type=pathlib.Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--client-output", type=pathlib.Path, default=CLIENT_OUTPUT)
    args = parser.parse_args(argv)

    mapping = load_mapping()
    content = render(ROOT / "protocol" / "schema" / "v1", mapping)
    targets = [args.output, args.client_output]

    if args.check:
        for target in targets:
            existing = target.read_text(encoding="utf-8") if target.exists() else ""
            if existing != content:
                print(f"{target} 与 schema 不一致，请运行 make protocol", file=sys.stderr)
                return 1
        return 0

    for target in targets:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
        print(f"已生成 {target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
