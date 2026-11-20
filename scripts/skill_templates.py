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
    "token", "card", "taxon", "playstyle",
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



_CN_NUMBERS = {1: "一", 2: "两", 3: "三", 4: "四", 5: "五", 6: "六",
               7: "七", 8: "八", 9: "九", 10: "十"}
_PLACEHOLDER = re.compile(r"\{([a-zA-Z][a-zA-Z0-9_]*)(?:\.([a-z]+))?\}")
_KEYWORD_RE = re.compile(r"^([a-z_]+)(?: (\d+))?$")
_DURATION_VALUES = ("permanent", "combat")


class Renderer:
    """把模板调用展开为效果 DSL 段与卡面文案。"""

    def __init__(self, content=CONTENT):
        import validate_docs

        content = pathlib.Path(content)
        self.templates = load_templates(content)
        self.nouns = load_nouns(content / NOUN_FILE)

        self.tokens = {}
        self.cards = {}
        self.pool_cards = {}
        for path in sorted((content / "cards").glob("*.md")):
            if path.name == "README.md":
                continue
            doc = validate_docs.parse_front_matter(path)
            entry = {
                "name": doc.get("name", doc["id"]),
                "attack": doc.get("attack"),
                "health": doc.get("health"),
            }
            self.cards[doc["id"]] = entry
            if doc.get("in_shop_pool", True):
                self.pool_cards[doc["id"]] = entry
            else:
                self.tokens[doc["id"]] = entry

    # ---- 参数 ----

    def _template(self, call):
        try:
            return self.templates[call.template_id]
        except KeyError:
            raise ValueError(f"模板不存在 {call.template_id!r}")

    def _arg_values(self, template, call):
        """返回 (实例提供的值, 渲染文案的值)；默认值只进后者。"""
        specs = {spec.name: spec for spec in template.args}
        unknown = sorted(set(call.args) - set(specs))
        if unknown:
            raise ValueError(f"模板 {template.id} 不认识参数 {unknown}")
        provided = {}
        resolved = {}
        for spec in template.args:
            if spec.name in call.args:
                value = self._check_value(spec, call.args[spec.name])
                provided[spec.name] = value
                resolved[spec.name] = value
            elif spec.default is not None:
                provided[spec.name] = None
                resolved[spec.name] = self._check_value(spec, spec.default)
            else:
                raise ValueError(f"模板 {template.id} 缺少参数 {spec.name}")
        return provided, resolved

    def _check_value(self, spec, raw):
        value = str(raw).strip()
        if spec.type == "token":
            if value not in self.tokens:
                raise ValueError(f"参数 {spec.name} 的 token 不存在或不是衍生物 {value!r}")
        elif spec.type == "card":
            if value not in self.pool_cards:
                raise ValueError(f"参数 {spec.name} 的卡牌不存在或不在商店卡池 {value!r}")
        elif spec.type == "taxon":
            if value not in self.nouns.taxon:
                raise ValueError(f"参数 {spec.name} 的分类名词缺失 {value!r}")
        elif spec.type == "playstyle":
            if value not in self.nouns.playstyle:
                raise ValueError(f"参数 {spec.name} 的玩法名词缺失 {value!r}")
        elif spec.type == "keyword":
            match = _KEYWORD_RE.fullmatch(value)
            if not match or match.group(1) not in self.nouns.keyword_text:
                raise ValueError(f"参数 {spec.name} 的关键词非法 {value!r}")
        elif spec.type == "int":
            if not re.fullmatch(r"-?\d+", value):
                raise ValueError(f"参数 {spec.name} 必须为整数，实为 {value!r}")
        elif spec.type == "count":
            if not re.fullmatch(r"\d+", value) or not 1 <= int(value) <= 4:
                raise ValueError(f"参数 {spec.name} 必须为 1–4，实为 {value!r}")
        elif spec.type == "duration":
            if value not in _DURATION_VALUES:
                raise ValueError(f"参数 {spec.name} 的时长非法 {value!r}")
        if spec.minimum is not None and int(value) < spec.minimum:
            raise ValueError(f"参数 {spec.name} 小于下限 {spec.minimum}")
        if spec.maximum is not None and int(value) > spec.maximum:
            raise ValueError(f"参数 {spec.name} 大于上限 {spec.maximum}")
        return value

    # ---- 效果 ----

    def expand(self, call):
        template = self._template(call)
        if template.kind == "keyword":
            return ()
        provided, _ = self._arg_values(template, call)
        selector = self._substitute_effect(template.selector, provided, template)
        params = self._effect_params(template, provided)
        segment = f"{template.trigger}|{template.op}|{selector}"
        return (f"{segment}|{params}",) if params else (segment,)

    def _effect_value(self, template, specs, name, modifier, value):
        """效果段只接受原值；关键词可拆分 `.name` / `.level` 两段。"""
        if modifier is None:
            return value
        spec = specs.get(name)
        if spec is not None and spec.type == "keyword" \
                and modifier in ("name", "level"):
            base, level = _KEYWORD_RE.fullmatch(value).groups()
            return base if modifier == "name" else (level or "")
        raise ValueError(f"模板 {template.id} 的效果段不支持修饰符 {{{name}.{modifier}}}")

    def _substitute_effect(self, raw, values, template):
        specs = {spec.name: spec for spec in template.args}

        def replace(match):
            name, modifier = match.groups()
            if values.get(name) is None:
                raise ValueError(f"模板 {template.id} 缺少参数 {name}")
            return self._effect_value(template, specs, name, modifier, values[name])

        return _PLACEHOLDER.sub(replace, raw)

    def _effect_params(self, template, values):
        if not template.params:
            return ""
        specs = {spec.name: spec for spec in template.args}
        pairs = []
        for pair in template.params.split(","):
            pair = pair.strip()
            if not pair:
                continue
            names = {name for name, _ in _PLACEHOLDER.findall(pair)}
            if any(values.get(name) is None for name in names):
                continue
            substituted = _PLACEHOLDER.sub(
                lambda m: self._effect_value(
                    template, specs, m.group(1), m.group(2), values[m.group(1)]),
                pair)
            # 拆分后为空的分量（如无层数关键词的 value）整段省略。
            if substituted.split("=", 1)[-1] == "":
                continue
            pairs.append(substituted)
        return ",".join(pairs)

    # ---- 文案 ----

    def text(self, call):
        template = self._template(call)
        _, values = self._arg_values(template, call)
        specs = {spec.name: spec for spec in template.args}

        output = []
        position = 0
        for match in _PLACEHOLDER.finditer(template.text):
            name, modifier = match.groups()
            if name not in specs:
                raise ValueError(f"模板 {template.id} 文案引用未声明参数 {name!r}")
            value = values.get(name)
            if value is None:
                raise ValueError(f"模板 {template.id} 缺少参数 {name}")
            output.append(template.text[position:match.start()])
            rendered = self._render_value(specs[name], modifier, value)
            # 时长后缀接在数字或拉丁字母后时补一个空格，与现有排版一致。
            prefix = "".join(output)
            if modifier == "suffix" and rendered and prefix \
                    and prefix[-1].isascii() and prefix[-1].isalnum():
                output.append(" ")
            output.append(rendered)
            position = match.end()
        output.append(template.text[position:])
        return "".join(output)

    def _render_value(self, spec, modifier, value):
        if spec.type in ("token", "card"):
            entry = self.tokens.get(value) or self.cards[value]
            if modifier is None:
                return entry["name"]
            if modifier == "name":
                return entry["name"]
            if modifier == "stats":
                return f"{entry['attack']}/{entry['health']}"
        elif spec.type == "taxon":
            return self.nouns.taxon[value]
        elif spec.type == "playstyle":
            return self.nouns.playstyle[value]
        elif spec.type == "keyword":
            return self._keyword_text(value, modifier)
        elif spec.type == "int":
            if modifier is None:
                return value
            if modifier == "cn":
                return _CN_NUMBERS.get(int(value), value)
        elif spec.type == "count":
            if modifier in (None, "cn"):
                return _CN_NUMBERS[int(value)]
            if modifier == "num":
                return str(int(value))
        elif spec.type == "duration":
            if modifier is None:
                return ""
            if modifier == "suffix":
                return "直到回合结束" if value == "combat" else ""
        raise ValueError(f"参数 {spec.name} 不支持修饰符 {modifier!r}")

    def _keyword_text(self, value, modifier):
        base, level = _KEYWORD_RE.fullmatch(value).groups()
        text = self.nouns.keyword_text[base]
        head = text.split("：", 1)[0]
        name = re.sub(r"\s*\{N\}", "", head).strip()
        label = f"{name} {level}" if level else name
        if modifier is None:
            return label
        if modifier == "name":
            return name
        if modifier == "level":
            return level or ""
        if modifier == "text":
            if "{N}" in text and not level:
                raise ValueError(f"关键词 {base} 的文案需要层数")
            return text.replace("{N}", level or "")
        raise ValueError(f"关键词参数不支持修饰符 {modifier!r}")

    # ---- 实例 ----

    def instance_segments(self, doc):
        if "template" not in doc:
            raise ValueError(f"{doc.get('id', '?')}: 缺少 template")
        segments = []
        for call in parse_template_field(doc["template"]):
            segments.extend(self.expand(call))
        return tuple(segments)

    def instance_text(self, doc):
        if doc.get("text"):
            return str(doc["text"])
        parts = [self.text(call) for call in parse_template_field(doc["template"])]
        return "；".join(parts)
