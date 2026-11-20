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
