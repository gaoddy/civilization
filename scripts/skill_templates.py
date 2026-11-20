#!/usr/bin/env python3
"""技能模板：参数声明、效果展开与文案渲染。

模板是作者层的原子技能，实例只保存模板 id 与参数。本模块供数据生成、
文本表生成、文档渲染与校验共用；不引入第三方依赖。
"""

import dataclasses
import pathlib
import re

CONTENT = pathlib.Path("docs") / "versions" / "v0.1" / "product" / "content"
TEMPLATE_DIR = "skill-templates"
NOUN_FILE = "card-nouns.md"

ARG_TYPES = (
    "token", "card", "taxon", "playstyle", "scholar",
    "keyword", "int", "count", "duration",
)


@dataclasses.dataclass(frozen=True)
class ArgSpec:
    name: str
    type: str
    minimum: int | None = None
    maximum: int | None = None
    default: str | None = None


@dataclasses.dataclass(frozen=True)
class Nouns:
    taxon: dict[str, str]
    playstyle: dict[str, str]
    special: dict[str, str]
    keyword_text: dict[str, str]


def section_rows(text, heading):
    """返回「## <heading>」小节的表格数据行；跳过表头与分隔行。"""
    match = re.search(rf"(?m)^(##+) {re.escape(heading)}\s*$", text)
    if not match:
        raise ValueError(f"缺少「## {heading}」小节")
    marks = match.group(1)
    rest = text[match.end():]
    next_heading = re.search(rf"(?m)^{marks} ", rest)
    if next_heading:
        rest = rest[:next_heading.start()]
    rows = []
    for line in rest.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if not cells or cells[0] == "键" or set(cells[0]) <= {"-"}:
            continue
        rows.append(cells)
    return rows


def _two_column_table(text, heading):
    rows = section_rows(text, heading)
    result = {}
    for cells in rows:
        if len(cells) != 2:
            raise ValueError(f"「{heading}」行列数非法：{cells!r}")
        key, value = cells
        if key in result:
            raise ValueError(f"「{heading}」重复键 {key!r}")
        result[key] = value
    if not result:
        raise ValueError(f"「{heading}」为空")
    return result


def load_nouns(path):
    """读取 card-nouns.md 的四张表。"""
    text = pathlib.Path(path).read_text(encoding="utf-8")
    return Nouns(
        taxon=_two_column_table(text, "1. 分类节点"),
        playstyle=_two_column_table(text, "2. 玩法"),
        special=_two_column_table(text, "3. 特殊筛选值"),
        keyword_text=_two_column_table(text, "4. 关键词文案"),
    )


_ARG_TYPES = "|".join(ARG_TYPES)
_ARG_SPEC = re.compile(rf"^({_ARG_TYPES})(?:\s+(-?\d+)\.\.(-?\d+))?(?:\s+default=(\S+))?$")


@dataclasses.dataclass(frozen=True)
class Template:
    id: str
    kind: str
    trigger: str | None
    op: str | None
    selector: str | None
    params: str | None
    text: str
    args: tuple[ArgSpec, ...]


@dataclasses.dataclass(frozen=True)
class TemplateCall:
    template_id: str
    args: dict[str, str]


TEMPLATE_FIELDS = {"id", "kind", "trigger", "op", "selector", "params", "text"}
TEMPLATE_KINDS = ("effect", "keyword")


def parse_template(doc, file):
    """把模板文档的 front matter 解析为 Template。"""
    unknown = sorted(set(doc) - TEMPLATE_FIELDS
                     - {key for key in doc if key.startswith("arg.")})
    if unknown:
        raise ValueError(f"{file}: 未知模板字段 {unknown}")
    template_id = str(doc.get("id", ""))
    kind = str(doc.get("kind", "effect"))
    if kind not in TEMPLATE_KINDS:
        raise ValueError(f"{file}: kind 非法 {kind!r}")
    if not template_id:
        raise ValueError(f"{file}: 缺少 id")
    if "text" not in doc:
        raise ValueError(f"{file}: 缺少 text")
    args = tuple(parse_arg_spec(key[len("arg."):], value)
                 for key, value in doc.items() if key.startswith("arg."))
    names = [spec.name for spec in args]
    if len(names) != len(set(names)):
        raise ValueError(f"{file}: 参数声明重复")
    if kind == "effect":
        for field in ("trigger", "op", "selector"):
            if field not in doc:
                raise ValueError(f"{file}: 效果模板缺少 {field}")
        return Template(template_id, kind, str(doc["trigger"]), str(doc["op"]),
                        str(doc["selector"]), doc.get("params"),
                        str(doc["text"]), args)
    return Template(template_id, kind, None, None, None, None,
                    str(doc["text"]), args)


def load_templates(content):
    """读取 <content>/skill-templates/*.md；文件名必须等于 id。"""
    import validate_docs

    directory = pathlib.Path(content) / TEMPLATE_DIR
    templates = {}
    for path in sorted(directory.glob("*.md")):
        if path.name == "README.md":
            continue
        doc = validate_docs.parse_front_matter(path)
        template = parse_template(doc, path.name)
        if path.stem != template.id:
            raise ValueError(f"{path.name}: 文件名与 id {template.id!r} 不一致")
        if template.id in templates:
            raise ValueError(f"{path.name}: 模板 id 重复 {template.id!r}")
        templates[template.id] = template
    return templates


_CALL_RE = re.compile(r"^([a-z][a-z0-9-]*)(?:\((.*)\))?$")


def parse_template_field(raw):
    """解析实例的 `template` 字段：`id(k=v, ...)`，多个以 `；` 分隔。"""
    calls = []
    for chunk in re.split(r"[;；]", str(raw)):
        chunk = chunk.strip()
        if not chunk:
            continue
        match = _CALL_RE.fullmatch(chunk)
        if not match:
            raise ValueError(f"template 段非法 {chunk!r}")
        template_id, body = match.groups()
        args = {}
        if body is not None:
            for pair in body.split(","):
                pair = pair.strip()
                if not pair:
                    continue
                if "=" not in pair:
                    raise ValueError(f"template 参数缺少 =: {pair!r}（{chunk}）")
                key, value = (part.strip() for part in pair.split("=", 1))
                if key in args:
                    raise ValueError(f"template 参数重复 {key!r}（{chunk}）")
                args[key] = value
        calls.append(TemplateCall(template_id, args))
    if not calls:
        raise ValueError("template 字段为空")
    return tuple(calls)


def parse_arg_spec(name, raw):
    """解析 `arg.<name>: <type> [min..max] [default=value]`。"""
    match = _ARG_SPEC.fullmatch(str(raw).strip())
    if not match:
        raise ValueError(f"参数声明非法 arg.{name}: {raw!r}")
    kind, minimum, maximum, default = match.groups()
    return ArgSpec(
        name=name,
        type=kind,
        minimum=int(minimum) if minimum is not None else None,
        maximum=int(maximum) if maximum is not None else None,
        default=default,
    )

