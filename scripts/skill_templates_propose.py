#!/usr/bin/env python3
"""把现有技能文档按触发时机拟合到技能模板，供批量迁移使用。

拟合要求效果段逐字可展开；文案与模板渲染逐字一致的技能直接改写为
`template` 实例。迁移完成后本脚本删除。
"""

import argparse
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import render_docs
import skill_templates
import validate_docs

ROOT = pathlib.Path(__file__).resolve().parent.parent
CONTENT = pathlib.Path("docs") / "versions" / "v0.1" / "product" / "content"

_PLACEHOLDER = re.compile(r"\{([a-z][a-z0-9_]*)(?:\.[a-z]+)?\}")


def _segments(skill):
    raw = str(skill.get("effects", "")).strip()
    if raw in ("", "[]"):
        return []
    return [part.strip() for part in raw.split(";") if part.strip()]


def _split(segment):
    parts = segment.split("|")
    if len(parts) not in (3, 4):
        return None
    params = parts[3] if len(parts) == 4 else ""
    return parts[0].strip(), parts[1].strip(), parts[2].strip(), params


def _pairs(raw):
    """`k=v,k=v` → [(k, v)]；空串返回 []。"""
    if not raw:
        return []
    result = []
    for pair in raw.split(","):
        pair = pair.strip()
        if not pair:
            continue
        key, _, value = pair.partition("=")
        result.append((key.strip(), value.strip()))
    return result


def _match_value(pattern, actual, values):
    """模板值匹配实际值；占位符捕获后要求同参数前后一致。"""
    match = _PLACEHOLDER.fullmatch(pattern)
    if match is None:
        if "{" not in pattern:
            return pattern == actual
        # 参数值内嵌占位（如 timesIf=playstyle:{playstyle}:2）。
        regex, names = [], []
        position = 0
        for index, placeholder in enumerate(_PLACEHOLDER.finditer(pattern)):
            regex.append(re.escape(pattern[position:placeholder.start()]))
            regex.append(f"(?P<g{index}>.+?)")
            names.append(placeholder.group(1))
            position = placeholder.end()
        regex.append(re.escape(pattern[position:]))
        embedded = re.fullmatch("".join(regex), actual)
        if embedded is None:
            return False
        for name, captured in zip(names, embedded.groups()):
            if values.get(name, captured) != captured:
                return False
            values[name] = captured
        return True
    name = match.group(1)
    if values.get(name, actual) != actual:
        return False
    values[name] = actual
    return True


def _match_segment(template, segment):
    parts = _split(segment)
    if parts is None:
        return None
    trigger, op, selector, params = parts
    if trigger != template.trigger or op != template.op:
        return None
    values = {}
    selector_parts = selector.split(":")
    pattern_parts = template.selector.split(":")
    if len(selector_parts) != len(pattern_parts):
        return None
    for actual, pattern in zip(selector_parts, pattern_parts):
        key, sep, pattern_value = pattern.partition("=")
        if sep:
            actual_key, actual_sep, actual_value = actual.partition("=")
            if actual_key != key or not actual_sep:
                return None
            if not _match_value(pattern_value, actual_value, values):
                return None
        elif actual != pattern:
            return None
    actual_params = _pairs(params)
    actual_map = dict(actual_params)
    if len(actual_map) != len(actual_params):
        return None
    # 引擎把关键词层数拆在 value 里；模板按 `keyword` 参数携带层数。
    if "keyword" in actual_map and "value" in actual_map:
        actual_map["keyword"] = f"{actual_map.pop('keyword')} {actual_map.pop('value')}"
    pattern_params = _pairs(template.params or "")
    allowed = {key for key, _ in pattern_params}
    if set(actual_map) - allowed:
        return None
    for key, pattern_value in pattern_params:
        if key in actual_map:
            if not _match_value(pattern_value, actual_map[key], values):
                return None
        elif _PLACEHOLDER.fullmatch(pattern_value) is None:
            return None
    return values


def _call_value(template, values):
    ordered = [f"{spec.name}={values[spec.name]}"
               for spec in template.args if spec.name in values]
    return f"{template.id}({','.join(ordered)})" if ordered else template.id


def _fit(template, skill, renderer, require_text):
    """效果拟合；require_text 时文案也必须逐字一致。返回 (TemplateCall, text_ok)。"""
    segments = _segments(skill)
    if not segments:
        return None
    merged = {}
    for segment in segments:
        values = _match_segment(template, segment)
        if values is None:
            return None
        for key, value in values.items():
            if merged.get(key, value) != value:
                return None
            merged[key] = value
    call = skill_templates.TemplateCall(template.id, merged)
    try:
        expanded = ";".join(renderer.expand(call))
        text = "；".join(renderer.text(call) for _ in segments)
    except ValueError:
        return None
    if expanded != str(skill.get("effects", "")).strip():
        return None
    text_ok = text == str(skill.get("text", "")).strip()
    if require_text and not text_ok:
        return None
    return call, text_ok


def fit_template(template, skill, renderer):
    """模板与技能的效果、文案都逐字一致时返回 TemplateCall，否则 None。"""
    fitted = _fit(template, skill, renderer, require_text=True)
    return fitted[0] if fitted else None


def _rewrite_front_matter(text, template_value, keep_text):
    head, sep, body = text.partition("\n---\n")
    lines = head.split("\n")
    result = []
    placed = False
    for line in lines:
        key = line.split(":", 1)[0].strip()
        if key == "effects":
            result.append(f"template: {template_value}")
            placed = True
        elif key == "text" and not keep_text:
            continue
        else:
            result.append(line)
    if not placed:
        result.append(f"template: {template_value}")
    return "\n".join(result) + sep + body


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trigger", required=True, help="触发时机，如 OnDeath")
    parser.add_argument("--write", action="store_true", help="改写拟合成功的文档")
    parser.add_argument("--text-override", action="store_true",
                        help="效果拟合但文案不同时，保留 text 覆盖后改写")
    args = parser.parse_args(argv)

    content = ROOT / CONTENT
    renderer = skill_templates.Renderer(content)
    templates = [t for t in skill_templates.load_templates(content).values()
                 if t.kind == "effect" and t.trigger == args.trigger]
    if not templates:
        print(f"没有 trigger={args.trigger} 的模板")
        return 1

    full, override, missed = [], [], []
    for path in sorted((content / "skills").glob("*.md")):
        if path.name == "README.md":
            continue
        skill = validate_docs.parse_front_matter(path)
        if "template" in skill or not _segments(skill):
            continue
        if not any((_split(s) or ("",))[0] == args.trigger for s in _segments(skill)):
            continue
        matched = None
        for template in templates:
            fitted = _fit(template, skill, renderer, require_text=False)
            if fitted is not None:
                matched = (template, fitted[0], fitted[1])
                if fitted[1]:
                    break
        if matched is None:
            missed.append(skill["id"])
            continue
        template, call, text_ok = matched
        entry = (path, skill, _call_value(template, call.args), text_ok)
        (full if text_ok else override).append(entry)

    print(f"trigger={args.trigger}：逐字拟合 {len(full)}，"
          f"需 text 覆盖 {len(override)}，未拟合 {len(missed)}")
    for path, skill, value, _ in full:
        print(f"  fit  {skill['id']}: {value}")
    for path, skill, value, _ in override:
        print(f"  text {skill['id']}: {value}")
    for skill_id in missed:
        print(f"  miss {skill_id}")

    if not args.write:
        return 0
    written = full + (override if args.text_override else [])
    for path, skill, value, text_ok in written:
        text = path.read_text(encoding="utf-8")
        path.write_text(_rewrite_front_matter(text, value, keep_text=not text_ok),
                        encoding="utf-8")
    render_docs.render(content, write=True)
    print(f"已改写 {len(written)} 个文档")
    return 0


if __name__ == "__main__":
    sys.exit(main())
