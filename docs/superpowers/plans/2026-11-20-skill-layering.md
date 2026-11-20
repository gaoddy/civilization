# 技能分层作者模型 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 247 份技能文档的作者层拆成「带参数的技能模板 + 技能实例」，卡面文案由模板渲染、由生成步骤回写，运行时与引擎保持不变。

**Architecture:** 新增 `scripts/skill_templates.py` 负责模板解析、参数校验、效果展开与文案渲染；`gen_card_data` 与 `gen_text` 在实例带 `template` 时走模板路径，否则保留旧 `effects` 路径以便分批迁移；新增 `render_docs.py` 重写技能实例与卡牌的派生小节；迁移结束后删除旧路径。

**Tech Stack:** Python 3（仅标准库）、Markdown front matter、现有 `make` 目标；Rust 引擎与协议不改动。

**Spec:** `docs/superpowers/specs/2026-11-20-skill-layering-design.md`

## Global Constraints

- 不修改 `server/crates/**`、`protocol/**`、`client/**` 的任何源码；`client/text/zh_CN.csv` 与 `client/generated/**` 由 `make data` 重新生成后必须无 diff。
- 迁移期间 `server/data/v0.1/*.json` 必须与已提交版本逐字节一致；任何 diff 都视为迁移错误。
- 不允许改写任何卡牌或技能文案：模板渲染结果必须与现状逐字相同，覆盖不了的用实例 `text`。
- Python 只用标准库，不引入 PyYAML；front matter 解析复用 `validate_docs.parse_front_matter`。
- 所有生成、检查与测试通过 `make` 目标执行；`python3 scripts/...` 只用于单测和迁移工具。
- 分支 `feature/skill-layering`，worktree `/Users/gaoddy/Projects/Civilization/.worktrees/feature-skill-layering`。
- 提交信息用中文，结尾附 `Co-Authored-By: Claude Code <noreply@anthropic.com>`。

## Review Focus

- 实例省略参数时，效果段里不得凭空出现 `k=`（默认值只影响文案）。由 Task 3 的 `test_expand_omits_missing_arg` 覆盖。
- 卡面名词缺失或写法与现状不同（动物/动物界、真菌/真菌界）。由 Task 1 的 `test_load_nouns_requires_all_tables` 与 Task 3 的 `test_text_override_wins` 覆盖。
- 关键词带层数的整段文案里 `N` 未替换或替换错位（减伤 2 的文案里两个 2）。由 Task 3 的 `test_keyword_text_substitutes_level` 覆盖。
- `render_docs` 误改卡牌手写小节（设计说明、出处）。由 Task 5 的 `test_render_preserves_handwritten_sections` 覆盖。
- 多模板调用时参数绑错模板。由 Task 3 的 `test_multiple_calls_bind_args_per_template` 覆盖。

---

## File Structure

| 文件 | 动作 | 职责 |
| --- | --- | --- |
| `docs/versions/v0.1/product/content/card-nouns.md` | 新建 | 分类/玩法/特殊筛选值的卡面名词，以及关键词整段文案 |
| `scripts/skill_templates.py` | 新建 | 模板与参数解析、名词表加载、效果展开、文案渲染 |
| `scripts/gen_card_data.py` | 修改 | `skill_entry` 支持模板实例；旧 `effects` 路径保留到收尾 |
| `scripts/gen_text.py` | 修改 | 技能文本取渲染结果 |
| `scripts/render_docs.py` | 新建 | 重写技能实例与卡牌的派生小节；`--check` 检测漂移 |
| `scripts/validate_docs.py` | 修改 | 技能字段集合、模板实例校验、模板属性行 |
| `scripts/skill_templates_propose.py` | 新建（迁移期临时） | 按触发时机拟合模板参数、改写实例文档；收尾删除 |
| `Makefile` | 修改 | 新增 `docs-render`，`data-check` 接入文档漂移检查 |
| `tests/test_skill_templates.py` | 新建 | 模板解析、展开、渲染、校验单测 |
| `tests/test_skill_generation.py` | 新建 | 生成链双路径一致性与文档渲染单测 |
| `docs/versions/v0.1/product/content/skill-templates/*.md` | 新建 | 40–60 个模板实体 |
| `docs/versions/v0.1/product/content/skills/*.md` | 修改 | 247 份实例改成 `template` + 参数 |

---

### Task 1: 卡面名词表与参数声明解析

**Files:**
- Create: `docs/versions/v0.1/product/content/card-nouns.md`
- Create: `scripts/skill_templates.py`
- Test: `tests/test_skill_templates.py`

**Interfaces:**
- Produces: `Nouns(taxon: dict[str,str], playstyle: dict[str,str], special: dict[str,str], keyword_text: dict[str,str])`；`ArgSpec(name: str, type: str, minimum: int|None, maximum: int|None, default: str|None)`；`parse_arg_spec(name: str, raw: str) -> ArgSpec`；`load_nouns(path: pathlib.Path) -> Nouns`

- [ ] **Step 1: 写失败测试**

```python
import pathlib
import unittest

import skill_templates


class NounTableTest(unittest.TestCase):
    def test_load_nouns_requires_all_tables(self):
        nouns = skill_templates.load_nouns(
            pathlib.Path("docs/versions/v0.1/product/content/card-nouns.md"))
        self.assertEqual(nouns.taxon["animalia"], "动物")
        self.assertEqual(nouns.playstyle["machine"], "机械")
        self.assertEqual(nouns.special["scholar=true"], "学者")
        self.assertEqual(nouns.keyword_text["taunt"], "嘲讽")

    def test_parse_arg_spec_range_and_default(self):
        spec = skill_templates.parse_arg_spec("count", "count 1..3 default=1")
        self.assertEqual((spec.type, spec.minimum, spec.maximum, spec.default),
                         ("count", 1, 3, "1"))
        plain = skill_templates.parse_arg_spec("taxon", "taxon")
        self.assertEqual((plain.type, plain.minimum, plain.default), ("taxon", None, None))
```

- [ ] **Step 2: 运行测试确认失败**

Run: `python3 -m unittest tests.test_skill_templates.NounTableTest -v`
Expected: FAIL，`ModuleNotFoundError: No module named 'skill_templates'`

- [ ] **Step 3: 新建 `card-nouns.md` 并实现解析**

`card-nouns.md` 四张表，标题固定：`## 1. 分类节点`、`## 2. 玩法`、`## 3. 特殊筛选值`、`## 4. 关键词文案`。每张表两列（`键 | 卡面名词`，关键词表为 `id | 文案模板`）。迁移时按现有文案填全，不得改文案。

`skill_templates.py` 用与 `gen_card_data._section_rows` 相同的小节表格解析方式（可在本模块实现 `section_rows(text, heading)`），`load_nouns` 读四个小节并返回 `Nouns`；小节缺失或表头缺失抛 `ValueError`。

- [ ] **Step 4: 运行测试确认通过**

Run: `python3 -m unittest tests.test_skill_templates.NounTableTest -v`
Expected: PASS（2 tests）

- [ ] **Step 5: 提交**

```bash
git add docs/versions/v0.1/product/content/card-nouns.md scripts/skill_templates.py tests/test_skill_templates.py
git commit -m "feat(data): 增加卡面名词表与模板参数声明解析"
```

---

### Task 2: 模板文档与实例调用解析

**Files:**
- Modify: `scripts/skill_templates.py`
- Test: `tests/test_skill_templates.py`

**Interfaces:**
- Consumes: Task 1 的 `ArgSpec`
- Produces: `Template(id, kind, trigger, op, selector, params, text, args: tuple[ArgSpec,...])`；`TemplateCall(template_id: str, args: dict[str,str])`；`parse_template(doc: dict, file: str) -> Template`；`load_templates(content: pathlib.Path) -> dict[str, Template]`；`parse_template_field(raw: str) -> tuple[TemplateCall, ...]`

- [ ] **Step 1: 写失败测试**

```python
class TemplateParseTest(unittest.TestCase):
    def test_parse_template_effect(self):
        doc = {
            "id": "on-death-summon", "kind": "effect", "trigger": "OnDeath",
            "op": "Summon", "selector": "self",
            "arg.token": "token", "arg.count": "count 1..3 default=1",
            "params": "token={token},count={count}",
            "text": "亡语：召唤{count.cn}个{token.stats}的{token.name}",
        }
        template = skill_templates.parse_template(doc, "on-death-summon.md")
        self.assertEqual(template.trigger, "OnDeath")
        self.assertEqual([a.name for a in template.args], ["token", "count"])

    def test_parse_template_field(self):
        calls = skill_templates.parse_template_field(
            "on-death-summon(token=termite,count=1); keyword-self(keyword=taunt)")
        self.assertEqual([c.template_id for c in calls],
                         ["on-death-summon", "keyword-self"])
        self.assertEqual(calls[1].args, {"keyword": "taunt"})

    def test_parse_template_rejects_unknown_field(self):
        with self.assertRaises(ValueError):
            skill_templates.parse_template(
                {"id": "x", "trigger": "OnDeath", "op": "Summon",
                 "selector": "self", "text": "a", "unexpected": "1"}, "x.md")
```

- [ ] **Step 2: 运行测试确认失败**

Run: `python3 -m unittest tests.test_skill_templates.TemplateParseTest -v`
Expected: FAIL，`AttributeError: module 'skill_templates' has no attribute 'parse_template'`

- [ ] **Step 3: 实现模板与调用解析**

- `parse_template` 接受字段 `id`、`kind`（缺省 `effect`）、`trigger`、`op`、`selector`、`params`、`text`、`arg.<name>`；其他字段抛 `ValueError`。`kind=effect` 时 `trigger`/`op`/`selector`/`text` 必填；`kind=keyword` 时只要求 `text` 与 `arg.keyword`。
- `load_templates` 读 `content/skill-templates/*.md`，跳过 `README.md`，文件名必须等于 `id`，重复 id 抛错。
- `parse_template_field` 按 `;` 拆分，每段匹配 `^([a-z][a-z0-9-]*)\((.*)\)$` 或裸 `id`；参数按 `,`、`=` 拆分，重复参数抛错。

- [ ] **Step 4: 运行测试确认通过**

Run: `python3 -m unittest tests.test_skill_templates.TemplateParseTest -v`
Expected: PASS（3 tests）

- [ ] **Step 5: 提交**

```bash
git add scripts/skill_templates.py tests/test_skill_templates.py
git commit -m "feat(data): 解析技能模板文档与实例调用"
```

---

### Task 3: 效果展开与文案渲染

**Files:**
- Modify: `scripts/skill_templates.py`
- Test: `tests/test_skill_templates.py`

**Interfaces:**
- Consumes: Task 1 的 `Nouns`、Task 2 的 `Template`/`TemplateCall`
- Produces: `Renderer(content: pathlib.Path = CONTENT)`；`Renderer.expand(call: TemplateCall) -> tuple[str, ...]`；`Renderer.text(call: TemplateCall) -> str`；`Renderer.instance_segments(doc: dict) -> tuple[str, ...]`；`Renderer.instance_text(doc: dict) -> str`

- [ ] **Step 1: 写失败测试**

```python
class RenderTest(unittest.TestCase):
    def setUp(self):
        self.renderer = skill_templates.Renderer()
        self.summon = skill_templates.TemplateCall(
            "on-death-summon", {"token": "termite", "count": "1"})
        self.summon2 = skill_templates.TemplateCall(
            "on-death-summon", {"token": "spore", "count": "2"})

    def test_expand_effect(self):
        self.assertEqual(
            self.renderer.expand(self.summon),
            ("OnDeath|Summon|self|token=termite,count=1",))

    def test_expand_omits_missing_arg(self):
        call = skill_templates.TemplateCall("aura-mammal-stats", {"attack": "2", "health": "2"})
        self.assertEqual(self.renderer.expand(call),
                         ("Aura|ModifyStats|friendly:all:taxon=mammalia|attack=2,health=2",))

    def test_text_count_and_stats(self):
        self.assertEqual(self.renderer.text(self.summon2),
                         "亡语：召唤两个 2/2 的孢子")

    def test_keyword_text_substitutes_level(self):
        call = skill_templates.TemplateCall("keyword-self", {"keyword": "armor 2"})
        self.assertEqual(self.renderer.text(call), "减伤 2：受到伤害时，减免 2 点伤害")

    def test_duration_suffix(self):
        call = skill_templates.TemplateCall(
            "trigger-friendly-taunt", {"keyword": "taunt", "duration": "combat"})
        self.assertEqual(self.renderer.text(call), "使一个友方随从获得嘲讽直到回合结束")

    def test_multiple_calls_bind_args_per_template(self):
        doc = {"template": "on-play-refresh-shop(free=true); on-play-gain-minion(level=current,to=hand)"}
        self.assertEqual(self.renderer.instance_segments(doc), (
            "OnPlay|RefreshShop|self|free=true",
            "OnPlay|GainMinion|self|level=current,to=hand",
        ))

    def test_text_override_wins(self):
        doc = {"template": "on-death-summon(token=termite,count=1)",
               "text": "亡语：召唤一个 1/2 的白蚁"}
        self.assertEqual(self.renderer.instance_text(doc),
                         "亡语：召唤一个 1/2 的白蚁")
```

测试依赖以下模板，Step 3 一并创建：`on-death-summon`、`aura-mammal-stats`、`keyword-self`、`trigger-friendly-taunt`、`on-play-refresh-shop`、`on-play-gain-minion`。

- [ ] **Step 2: 运行测试确认失败**

Run: `python3 -m unittest tests.test_skill_templates.RenderTest -v`
Expected: FAIL，`AttributeError: ... 'Renderer'`

- [ ] **Step 3: 实现渲染器并创建上述 6 个模板**

渲染规则（与 spec §5 一致）：

- 占位符正则 `\{([a-z][a-z0-9_]*)(?:\.([a-z]+))?\}`。
- `expand`：替换 selector/params 中的占位符；含未提供参数的 `k={k}` 段整段丢弃（按 `,` 拆分后过滤），无占位符的字面参数保留；缺必填参数、取值越界、未知占位符抛 `ValueError`。
- `text`：按 §4.2 类型表渲染；`count` 用 `一/两/三/四`，`int` 用数字；`token`/`card` 提供 `.stats`；`duration.suffix` 见 spec；`keyword` 拆 `id [层数]`，`.text` 用 `card-nouns.md` 的关键词文案表把 `{N}` 替换为层数。
- `instance_segments` / `instance_text`：读 `doc["template"]`；`instance_text` 在 `doc` 有 `text` 时直接返回覆盖值，否则各调用文案用「；」连接。
- 数据查找：token 用 `gen_card_data.parse_tokens` 与 `parse_token_names`，卡牌读 `content/cards/*.md` 的 `id`/`name`/`attack`/`health`，名词与关键词文案用 `load_nouns`。全部在 `Renderer.__init__` 加载一次。

- [ ] **Step 4: 运行测试确认通过**

Run: `python3 -m unittest tests.test_skill_templates.RenderTest -v`
Expected: PASS（7 tests）

- [ ] **Step 5: 提交**

```bash
git add scripts/skill_templates.py docs/versions/v0.1/product/content/skill-templates tests/test_skill_templates.py
git commit -m "feat(data): 实现技能模板效果展开与文案渲染"
```

---

### Task 4: 生成链接入模板实例

**Files:**
- Modify: `scripts/gen_card_data.py`（`skill_entry` 与 `build` 附近）
- Modify: `scripts/gen_text.py`（`generated_entries`）
- Create: `tests/test_skill_generation.py`

**Interfaces:**
- Consumes: Task 3 的 `Renderer.expand` / `Renderer.instance_segments` / `Renderer.instance_text`
- Produces: `gen_card_data.skill_entry(doc)` 对带 `template` 的文档返回与旧路径相同的结构；`gen_text` 的 `skill.<id>.text` 取 `instance_text`

- [ ] **Step 1: 写失败测试**

```python
class GeneratorTemplateTest(unittest.TestCase):
    def test_skill_entry_template_matches_legacy(self):
        renderer = skill_templates.Renderer()
        legacy = {"id": "x", "keyword": "deathrattle",
                  "effects": "OnDeath|Summon|self|token=termite,count=1"}
        templated = {"id": "x", "keyword": "deathrattle",
                     "template": "on-death-summon(token=termite,count=1)"}
        self.assertEqual(gen_card_data.skill_entry(legacy, renderer),
                         gen_card_data.skill_entry(templated, renderer))

    def test_skill_text_template_matches_legacy(self):
        rendered = skill_templates.Renderer().instance_text(
            {"id": "x", "template": "on-death-summon(token=termite,count=1)"})
        self.assertEqual(rendered, "亡语：召唤一个 1/2 的白蚁")
```

- [ ] **Step 2: 运行测试确认失败**

Run: `python3 -m unittest tests.test_skill_generation.GeneratorTemplateTest -v`
Expected: FAIL（`skill_entry` 不接受 `renderer`，或不处理 `template`）

- [ ] **Step 3: 实现双路径**

- `build(root)` 创建 `renderer = skill_templates.Renderer(content)`，传给 `skill_entry(doc, renderer)`。
- `skill_entry`：`template` 存在时 `effects = structured_effects(";".join(renderer.instance_segments(doc)))`（`kind=keyword` 返回空段时写 `[]`）；否则保持读 `doc["effects"]`。
- `gen_text.generated_entries`：`skill.<id>.text` 用 `renderer.instance_text(doc)`；`match_text` 照旧。
- 未改动的 247 份文档走旧路径，`make data-check` 必须无 diff。

- [ ] **Step 4: 运行测试确认通过**

Run: `python3 -m unittest tests.test_skill_generation.GeneratorTemplateTest -v`，再运行 `make data-check`
Expected: PASS；`生成数据无漂移`，`client/text/zh_CN.csv` 无 diff

- [ ] **Step 5: 提交**

```bash
git add scripts/gen_card_data.py scripts/gen_text.py tests/test_skill_generation.py
git commit -m "feat(data): 生成链支持模板实例并保持产物不变"
```

---

### Task 5: 派生文档渲染器与 `make docs-render`

**Files:**
- Create: `scripts/render_docs.py`
- Modify: `Makefile`
- Modify: `tests/test_skill_generation.py`

**Interfaces:**
- Consumes: Task 3 的 `Renderer`
- Produces: `render_docs.render(content: pathlib.Path, write: bool) -> list[str]`（返回漂移/已改文件列表）；CLI `--check`

- [ ] **Step 1: 写失败测试**

```python
class RenderDocsTest(unittest.TestCase):
    def test_render_skill_attrs_and_body(self):
        with tempfile.TemporaryDirectory() as tmp:
            content = build_fixture(tmp)  # 返回 tmp/content，含 skills/x.md 与 cards/c.md
            render_docs.render(content, write=True)
            text = (content / "skills/x.md").read_text()
            self.assertIn("| 模板 | on-death-summon |", text)
            self.assertIn("## 效果\n\n亡语：召唤一个 1/2 的白蚁", text)

    def test_render_preserves_handwritten_sections(self):
        with tempfile.TemporaryDirectory() as tmp:
            content = build_fixture(tmp)
            render_docs.render(content, write=True)
            text = (content / "cards/c.md").read_text()
            self.assertIn("## 设计说明\n\n定位：测试。", text)
            self.assertIn("## 出处\n\n测试出处。", text)

    def test_render_check_reports_drift_without_writing(self):
        with tempfile.TemporaryDirectory() as tmp:
            content = build_fixture(tmp)
            (content / "skills/x.md").write_text("...旧内容...")
            drift = render_docs.render(content, write=False)
            self.assertTrue(drift)
            self.assertIn("...旧内容...", (content / "skills/x.md").read_text())
```

`build_fixture` 是测试内辅助函数：复制最小 front matter + 属性表 + 效果节，卡牌含 `## 效果`、`## 设计说明`、`## 出处`。

- [ ] **Step 2: 运行测试确认失败**

Run: `python3 -m unittest tests.test_skill_generation.RenderDocsTest -v`
Expected: FAIL，`ModuleNotFoundError: No module named 'render_docs'`

- [ ] **Step 3: 实现渲染器与 Make 入口**

- 技能实例：重写 `## 属性` 表中的「关键词」「确定性替换」「模板」「费用」「每回合次数」行与 `## 效果` 正文；正文取 `instance_text`。
- 卡牌：重写 `## 效果`（按 `skills` 拼接，「；」连接，无技能写「无」）与属性表「技能」「关键词」行；其他小节原样保留。
- `skills/README.md`：重写每行的「文本」单元格为渲染文案。
- 只替换目标小节的正文行，不重排其他内容；写入使用 UTF-8 与原文换行风格。
- `Makefile` 新增：

```make
docs-render:
	$(PYTHON) scripts/render_docs.py
```

并在 `data-check` 增加 `$(PYTHON) scripts/render_docs.py --check`。

- [ ] **Step 4: 运行测试与整体检查**

Run: `python3 -m unittest tests.test_skill_generation.RenderDocsTest -v`，再运行 `make data-check`
Expected: PASS；旧文档全部走旧路径时 `render_docs --check` 报告 0 处漂移

- [ ] **Step 5: 提交**

```bash
git add scripts/render_docs.py Makefile tests/test_skill_generation.py
git commit -m "feat(data): 增加派生文档渲染与漂移检查"
```

---

### Task 6: 校验接入模板实例

**Files:**
- Modify: `scripts/validate_docs.py`
- Test: `tests/test_skill_generation.py`

**Interfaces:**
- Consumes: Task 2/3 的解析与渲染
- Produces: `expected_skill_attrs` 对 `template` 实例增加「模板」行；`check_skills` 用渲染文案替代 `skill["text"]`；`validate_skill_template(skill: dict, templates: dict[str, Template], renderer: Renderer) -> list[str]`

- [ ] **Step 1: 写失败测试**

```python
class ValidateTemplateTest(unittest.TestCase):
    def test_expected_attrs_include_templates(self):
        skill = {"id": "x", "template": "on-death-summon(token=termite,count=1)"}
        attrs = validate_docs.expected_skill_attrs(skill)
        self.assertEqual(attrs["模板"], "on-death-summon")

    def test_missing_template_reported(self):
        errors = validate_docs.validate_skill_template(
            {"id": "x", "template": "not-a-template", "_file": "x.md"}, {}, renderer)
        self.assertTrue(any("模板不存在" in e for e in errors))
```

- [ ] **Step 2: 运行测试确认失败**

Run: `python3 -m unittest tests.test_skill_generation.ValidateTemplateTest -v`
Expected: FAIL

- [ ] **Step 3: 实现校验双路径**

- `SKILL_FIELDS` 改为必填 `["id"]`；新增 `SKILL_OPTIONAL_FIELDS` 集合，出现集合外字段报错。
- 新增 `validate_skill_template(skill, templates, renderer) -> list[str]`：模板存在、参数声明与取值、展开后的 DSL 经 `parse_effects`/`check_params`、渲染文案非空。
- `check_skills`：`template` 存在时 `raw_effects = ";".join(instance_segments)`、`text = instance_text`；否则沿用 `effects`/`text`。
- `expected_skill_attrs` 增加「模板」行（多个模板用「；」连接）。
- 迁移期间两种形态都必须通过；未转换的技能行为不变。

- [ ] **Step 4: 运行测试确认通过**

Run: `python3 -m unittest tests.test_skill_generation.ValidateTemplateTest -v`，再运行 `python3 scripts/validate_docs.py`
Expected: PASS；既有输出不变

- [ ] **Step 5: 提交**

```bash
git add scripts/validate_docs.py tests/test_skill_generation.py
git commit -m "feat(data): 校验支持模板实例与模板属性行"
```

---

### Task 7: 迁移装配器与逐字校验

**Files:**
- Create: `scripts/skill_templates_propose.py`
- Test: `tests/test_skill_generation.py`

**Interfaces:**
- Produces: `fit_template(template: Template, skill: dict, renderer: Renderer) -> TemplateCall | None`；CLI `--trigger <Trigger>`、`--write`

- [ ] **Step 1: 写失败测试**

```python
class ProposeTest(unittest.TestCase):
    def test_fit_template_extracts_args(self):
        templates = skill_templates.load_templates(CONTENT)
        skill = {"id": "deathrattle-summon-termite",
                 "effects": "OnDeath|Summon|self|token=termite,count=1",
                 "text": "亡语：召唤一个 1/2 的白蚁"}
        call = propose.fit_template(templates["on-death-summon"], skill, renderer)
        self.assertEqual(call.args, {"token": "termite", "count": "1"})

    def test_fit_rejects_text_mismatch(self):
        skill = {"id": "x", "effects": "OnDeath|Summon|self|token=termite,count=1",
                 "text": "亡语：召唤一个白蚁"}
        self.assertIsNone(propose.fit_template(templates["on-death-summon"], skill, renderer))
```

- [ ] **Step 2: 运行测试确认失败**

Run: `python3 -m unittest tests.test_skill_generation.ProposeTest -v`
Expected: FAIL

- [ ] **Step 3: 实现装配器**

- `fit_template`：把模板 selector/params 的 `{name}` 转成正则捕获组，匹配技能的每个效果段；同一模板的所有段匹配成功、参数一致、`renderer.text(call)` 逐字等于 `skill["text"]` 才算拟合。
- `main`：遍历指定触发的技能，按 `(trigger, op)` 找候选模板，输出拟合表与未拟合清单；`--write` 时把 front matter 的 `effects`/`text` 换成 `template`（保留 `keyword`/`cost`/`name`/`match_text`），并把属性行改为 `模板`。
- 脚本不修改渲染结果不确定的技能；只有文本与效果都逐字一致的技能才会被写。

- [ ] **Step 4: 运行测试确认通过**

Run: `python3 -m unittest tests.test_skill_generation.ProposeTest -v`，再运行 `python3 scripts/skill_templates_propose.py --trigger OnDeath`
Expected: PASS；输出拟合表与未拟合清单，未加 `--write` 时文档无改动

- [ ] **Step 5: 提交**

```bash
git add scripts/skill_templates_propose.py tests/test_skill_generation.py
git commit -m "feat(data): 增加模板迁移装配器与逐字校验"
```

---

### Task 8: 迁移战斗效果牌技能（OnPlayEffect，63）

**Files:**
- Create: `docs/versions/v0.1/product/content/skill-templates/*.md`（本批所需模板）
- Modify: `docs/versions/v0.1/product/content/skills/*.md`（该批技能）

- [ ] **Step 1: 写模板**

按句式拆模板（不对每个数值建模板），覆盖 `ModifyStats`、`GrantKeyword`、`Growth`、`Summon` 等原语；文案模板逐字对齐现有文本。

- [ ] **Step 2: 拟合与改写**

Run: `python3 scripts/skill_templates_propose.py --trigger OnPlayEffect`
Expected: 输出拟合清单；未拟合项逐条处理（补模板，或效果可拟合、文案特殊时写 `text` 覆盖）

Run: `python3 scripts/skill_templates_propose.py --trigger OnPlayEffect --write`

- [ ] **Step 3: 验证本批**

Run: `python3 scripts/validate_docs.py`，`make data-check`，`git diff --stat`
Expected: 校验通过；`生成数据无漂移`；diff 只包含 `content/skill-templates/` 与本批 `content/skills/`

- [ ] **Step 4: 提交**

```bash
git add docs/versions/v0.1/product/content/skill-templates docs/versions/v0.1/product/content/skills
git commit -m "refactor(data): 战斗效果牌技能迁移到模板实例"
```

---

### Task 9: 迁移亡语技能（OnDeath，39）

**Files:**
- Create: `docs/versions/v0.1/product/content/skill-templates/*.md`（本批模板）
- Modify: `docs/versions/v0.1/product/content/skills/*.md`（OnDeath 技能）

- [ ] **Step 1: 出拟合报告**

Run: `python3 scripts/skill_templates_propose.py --trigger OnDeath`
Expected: 打印可拟合技能与未拟合清单（无文档改动）

- [ ] **Step 2: 补模板**

为未拟合清单按句式建模板；`card=` 与 `token=` 两种召唤写法都要覆盖；「分裂：」前缀的文案单独建模板，不改写文案。

- [ ] **Step 3: 改写**

Run: `python3 scripts/skill_templates_propose.py --trigger OnDeath --write`
Expected: 该批技能换成 `template`，属性表出现「模板」行

- [ ] **Step 4: 处理例外**

对效果可拟合、文案特殊的技能加 `text` 覆盖；对其余情况补模板后重跑 Step 3。

- [ ] **Step 5: 验证**

Run: `python3 scripts/validate_docs.py`，`make data-check`，`git diff --stat`
Expected: 校验通过；数据无漂移；diff 只含模板目录与 OnDeath 技能文档

- [ ] **Step 6: 提交**

```bash
git add docs/versions/v0.1/product/content/skill-templates docs/versions/v0.1/product/content/skills
git commit -m "refactor(data): 亡语技能迁移到模板实例"
```

---

### Task 10: 迁移战吼随从技能（OnPlay，37）

**Files:**
- Create: 本批模板
- Modify: `OnPlay` 技能文档

- [ ] **Step 1: 出拟合报告**

Run: `python3 scripts/skill_templates_propose.py --trigger OnPlay`

- [ ] **Step 2: 补模板并改写**

按句式建模板后运行 `python3 scripts/skill_templates_propose.py --trigger OnPlay --write`；例外加 `text` 覆盖。

- [ ] **Step 3: 验证**

Run: `python3 scripts/validate_docs.py`，`make data-check`，`git diff --stat`
Expected: 校验通过；数据无漂移；diff 只含模板目录与 OnPlay 技能文档

- [ ] **Step 4: 提交**

```bash
git commit -m "refactor(data): 战吼随从技能迁移到模板实例"
```

---

### Task 11: 迁移战斗开始技能（OnCombatStart，37）

**Files:**
- Create: 本批模板
- Modify: `OnCombatStart` 技能文档

- [ ] **Step 1: 出拟合报告**

Run: `python3 scripts/skill_templates_propose.py --trigger OnCombatStart`

- [ ] **Step 2: 补模板并改写**

按句式建模板后运行 `python3 scripts/skill_templates_propose.py --trigger OnCombatStart --write`；例外加 `text` 覆盖。

- [ ] **Step 3: 验证**

Run: `python3 scripts/validate_docs.py`，`make data-check`，`git diff --stat`
Expected: 校验通过；数据无漂移；diff 只含模板目录与本批技能文档

- [ ] **Step 4: 提交**

```bash
git commit -m "refactor(data): 战斗开始技能迁移到模板实例"
```

---

### Task 12: 迁移光环技能（Aura 与 ConditionalAura，25）

**Files:**
- Create: 本批模板
- Modify: 光环技能文档

- [ ] **Step 1: 出拟合报告**

Run: `python3 scripts/skill_templates_propose.py --trigger Aura`，再运行 `--trigger ConditionalAura`

- [ ] **Step 2: 补模板并改写**

群聚参数要覆盖 `perAttack`/`perHealth`/`maxCount`；按句式建模板后分别运行 `--trigger Aura --write` 与 `--trigger ConditionalAura --write`；例外加 `text` 覆盖。

- [ ] **Step 3: 验证**

Run: `python3 scripts/validate_docs.py`，`make data-check`，`git diff --stat`
Expected: 校验通过；数据无漂移；diff 只含模板目录与光环技能文档

- [ ] **Step 4: 提交**

```bash
git commit -m "refactor(data): 光环技能迁移到模板实例"
```

---

### Task 13: 迁移主动技能（OnActivate，18）

**Files:**
- Create: 本批模板
- Modify: `OnActivate` 技能文档

- [ ] **Step 1: 出拟合报告**

Run: `python3 scripts/skill_templates_propose.py --trigger OnActivate`

- [ ] **Step 2: 补模板并改写**

运行 `--trigger OnActivate --write`；`cost`/`uses_per_turn`/`name`/`match_text` 原样保留（装配器不得删除这些字段）；例外加 `text` 覆盖。

- [ ] **Step 3: 验证**

Run: `python3 scripts/validate_docs.py`，`make data-check`，`git diff --stat`
Expected: 校验通过；数据无漂移；主动技能字段完整

- [ ] **Step 4: 提交**

```bash
git commit -m "refactor(data): 主动技能迁移到模板实例"
```

---

### Task 14: 迁移其余触发与纯关键词技能（32）

**Files:**
- Create: 本批模板
- Modify: 其余技能文档

- [ ] **Step 1: 逐个触发迁移**

对 `OnTurnStart`、`OnFirstRefresh`、`OnKill`、`OnAllyKill`、`OnFriendlyDeath`、`OnEnemyDeath`、`OnAllySummon`、`OnAllyGrowth`、`OnAllyEvolve`、`OnUse`、`OnDealDamage`、`OnCombatEnd` 依次运行 `python3 scripts/skill_templates_propose.py --trigger <Trigger>`，补模板后加 `--write`。

- [ ] **Step 2: 纯关键词技能**

14 个 `effects: []` 的技能改为 `template: keyword-self(keyword=<id> [层数])`；`text` 由 `card-nouns.md` 关键词文案表渲染，不写覆盖。

- [ ] **Step 3: 组合技能**

`efficient-algorithm` 改为两个模板调用，保留 `text` 覆盖以维持现有无「；」的文案。

- [ ] **Step 4: 验证**

Run: `python3 scripts/validate_docs.py`，`make data-check`，`python3 -m unittest discover -s tests -v`
Expected: 全绿，数据无漂移

- [ ] **Step 5: 提交**

```bash
git add docs/versions/v0.1/product/content/skill-templates docs/versions/v0.1/product/content/skills
git commit -m "refactor(data): 其余触发与关键词技能迁移到模板实例"
```

---

### Task 15: 全量派生文档与验收

**Files:**
- Modify: 派生小节（应无实际 diff）

- [ ] **Step 1: 全量渲染**

Run: `make docs-render`
Expected: 完成；随后 `git status` 应无改动（迁移批次已逐字保持文案）

- [ ] **Step 2: 检查卡牌 diff 范围**

Run: `git diff --stat`，`git diff -- docs/versions/v0.1/product/content/cards`
Expected: 无卡牌 diff；若出现，只允许「## 效果」与属性派生行，逐条核对后修正

- [ ] **Step 3: 全量验证**

Run: `make data-check`，`python3 -m unittest discover -s tests -v`，`python3 scripts/validate_docs.py`
Expected: 数据无漂移；全部单测通过；输出仍为「随从 309 张、效果牌 100 张、技能 247 个、英雄卡 32 张，校验通过」

- [ ] **Step 4: 提交（如有派生改动）**

```bash
git add docs/versions/v0.1/product/content
git commit -m "docs(data): 重算技能与卡牌派生文案"
```

---

### Task 16: 文档回写、删除旧路径与最终验收

**Files:**
- Modify: `docs/versions/v0.1/product/content/skill-system.md`
- Modify: `docs/versions/v0.1/product/content/skills/README.md`
- Modify: `docs/versions/v0.1/product/content/card-system.md`（技能行口径）
- Modify: `docs/versions/v0.1/product/rules/keywords-and-mechanics.md` §9.4
- Create: `docs/versions/v0.1/product/content/skill-templates/README.md`
- Modify: `scripts/gen_card_data.py`、`scripts/validate_docs.py`（删除旧 `effects` 分支）
- Delete: `scripts/skill_templates_propose.py`

- [ ] **Step 1: 回写规范文档**

按 spec 的字段与分层更新上述文档：技能实例字段表、模板字段表、参数类型、名词表、渲染规则、异常清单数量与原因。

- [ ] **Step 2: 删除旧路径**

所有 247 个技能都带 `template` 后，删除 `gen_card_data.skill_entry` 的 `effects` 分支与 `validate_docs` 的旧字段/旧分支；未带 `template` 的技能视为数据错误。

- [ ] **Step 3: 最终验证**

Run: `make m1-check`，`make data-check`，`python3 -m unittest discover -s tests -v`
Expected: 全绿；`git diff server/crates protocol client` 为空

- [ ] **Step 4: 提交**

```bash
git add docs/versions/v0.1/product scripts Makefile tests
git commit -m "refactor(data): 收尾技能分层迁移并回写文档"
```

---

## Self-Review

- **Spec coverage:** §4 作者格式 → Task 1–3；§5 渲染规则 → Task 3；§6 生成链 → Task 4–6；§7 引擎边界 → Global Constraints 与 Task 16；§8 迁移 → Task 7–15；§9 验收 → Task 15–16；§10 非目标 → Global Constraints；§12 判断项 → 各任务实现口径。
- **Type consistency:** `Template`/`TemplateCall`/`Nouns`/`Renderer` 在 Task 1–3 定义，Task 4–7 只使用不重定义；`instance_segments` / `instance_text` 名称在 Task 3、4、6 一致。
- **Review Focus:** 五条失败模式分别落在 Task 3、1、3、5、3 的测试断言中。
