#!/usr/bin/env python3
"""校验 protocol/samples/v1 下的协议样例。

只支持本项目 schema 使用的 JSON Schema 子集：$ref（文件内或同目录文件）、
type、required、properties、additionalProperties(bool)、items、enum、const、
oneOf、minimum、maximum、minLength、minItems、maxItems、format(date-time)。

运行：python3 scripts/validate_protocol.py
"""

import datetime
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SCHEMA_DIR = ROOT / "protocol" / "schema" / "v1"
SAMPLES_DIR = ROOT / "protocol" / "samples" / "v1"
ENVELOPE = {"c2s": "ClientEnvelope", "s2c": "ServerEnvelope"}

TYPES = {
    "object": dict,
    "array": list,
    "string": str,
    "integer": int,
    "boolean": bool,
    "number": (int, float),
    "null": type(None),
}


def load_mapping():
    return json.loads((SCHEMA_DIR / "messages.json").read_text())


def _load_schema(filename):
    return json.loads((SCHEMA_DIR / filename).read_text())


def _split_ref(ref):
    filename, _, pointer = ref.partition("#")
    return filename, pointer


def _resolve(ref, base_file="messages.schema.json"):
    filename, pointer = _split_ref(ref)
    filename = filename or base_file
    doc = _load_schema(filename)
    node = doc
    if pointer:
        for part in pointer.strip("/").split("/"):
            node = node[part]
    return node, filename


def _type_ok(value, expected):
    if expected == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if expected == "boolean":
        return isinstance(value, bool)
    return isinstance(value, TYPES[expected])


def _join(path, key):
    return f"{path}.{key}" if path else key


def _validate(value, schema, path, base_file, errors):
    if "$ref" in schema:
        target, target_file = _resolve(schema["$ref"], base_file)
        _validate(value, target, path, target_file, errors)
        return

    if "const" in schema and value != schema["const"]:
        errors.append(f"{path}: 必须等于 {schema['const']!r}")
    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{path}: 不在允许取值内")

    if "type" in schema:
        expected = schema["type"]
        expected = expected if isinstance(expected, list) else [expected]
        if not any(_type_ok(value, t) for t in expected):
            errors.append(f"{path}: 类型应为 {'/'.join(expected)}")
            return

    if isinstance(value, str):
        if "minLength" in schema and len(value) < schema["minLength"]:
            errors.append(f"{path}: 长度不能小于 {schema['minLength']}")
        if schema.get("format") == "date-time":
            try:
                datetime.datetime.fromisoformat(value.replace("Z", "+00:00"))
            except ValueError:
                errors.append(f"{path}: 不是 RFC 3339 时间")

    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in schema and value < schema["minimum"]:
            errors.append(f"{path}: 不能小于 {schema['minimum']}")
        if "maximum" in schema and value > schema["maximum"]:
            errors.append(f"{path}: 不能大于 {schema['maximum']}")

    if isinstance(value, dict):
        for key in schema.get("required", []):
            if key not in value:
                errors.append(f"{_join(path, key)}: 缺少必填字段")
        properties = schema.get("properties", {})
        if schema.get("additionalProperties") is False:
            for key in value:
                if key not in properties:
                    errors.append(f"{_join(path, key)}: 不允许的字段")
        for key, subschema in properties.items():
            if key in value:
                _validate(value[key], subschema, _join(path, key), base_file, errors)

    if isinstance(value, list):
        if "minItems" in schema and len(value) < schema["minItems"]:
            errors.append(f"{path}: 元素不能少于 {schema['minItems']}")
        if "maxItems" in schema and len(value) > schema["maxItems"]:
            errors.append(f"{path}: 元素不能多于 {schema['maxItems']}")
        items = schema.get("items")
        if items:
            for index, item in enumerate(value):
                _validate(item, items, f"{path}[{index}]", base_file, errors)

    if "oneOf" in schema:
        matches = 0
        for subschema in schema["oneOf"]:
            branch_errors = []
            _validate(value, subschema, path, base_file, branch_errors)
            if not branch_errors:
                matches += 1
        if matches != 1:
            errors.append(f"{path}: 必须匹配 oneOf 中的恰好一个分支")


def validate_message(direction, doc):
    """返回错误列表；空列表表示通过。"""
    mapping = load_mapping()
    errors = []
    if not isinstance(doc, dict):
        return ["消息: 必须是对象"]
    message_type = doc.get("type")
    if message_type not in mapping["directions"][direction]:
        return [f"type: 未知消息类型 {message_type}"]

    envelope, envelope_file = _resolve(f"envelope.schema.json#/$defs/{ENVELOPE[direction]}")
    _validate(doc, envelope, "", envelope_file, errors)

    payload_schema, payload_file = _resolve(mapping["directions"][direction][message_type])
    _validate(doc.get("payload"), payload_schema, "payload", payload_file, errors)
    return errors


def validate_all():
    mapping = load_mapping()
    failures = []
    for direction in ("c2s", "s2c"):
        expected = set(mapping["directions"][direction])
        found = set()
        for path in sorted((SAMPLES_DIR / direction).glob("*.json")):
            doc = json.loads(path.read_text())
            found.add(doc.get("type"))
            for error in validate_message(direction, doc):
                failures.append(f"{path.name}: {error}")
        for missing in sorted(expected - found):
            failures.append(f"{direction}: 缺少 {missing} 的样例")
    return failures


def main():
    failures = validate_all()
    for line in failures:
        print(line)
    if failures:
        print(f"校验失败：{len(failures)} 项", file=sys.stderr)
        return 1
    print("协议样例校验通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())
