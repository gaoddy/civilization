# 技能分层作者模型设计（模板 + 实例）

> 状态：待用户确认
> 任务分支：`feature/skill-layering`
> 关系：[Skill.effects 结构化设计](2026-11-17-structured-skill-effects-design.md) 把运行层固定为结构化数组；本文只改作者层，运行时产物与规则引擎不变。

## 1. 目标

现在 247 个技能是 247 份文档，每份把「触发时机 + 原语 + 选择器 + 数值」和一段卡面文案写死在一起。调一个数值要同时改 `effects` 与 `text`，再跑校验确认两者一致；同一种效果的不同数值（繁衍 1/2/3、亡语召唤 21 种参数）没有共同来源。

本次把作者层拆成两层：

- **技能模板**：一段带参数的原子技能，定义触发时机、原语、选择器形状、参数域与文案模板。
- **技能实例**：模板 + 参数；数值只出现在这里。

目标：

1. 调整技能与卡牌平衡时只改实例参数，卡面文案自动跟随。
2. 技能实例与卡牌的派生文案由生成步骤统一重写，不再手写与人工对齐。
3. 作者层与运行层解耦：模板只在编译期存在，`skills.json` 与引擎不变，为后续引擎优化留出边界。

## 2. 已确认的取舍

| 决策点 | 结论 |
| --- | --- |
| 分层落点 | 只改作者层与生成链；运行时 `skills.json` 结构、引擎加载与结算逻辑不变 |
| 实例形态 | 技能实例 = `template` + `args`；不再手写 `effects`，`text` 降为可选覆盖 |
| 文案来源 | 模板渲染；实例可写 `text` 覆盖特殊措辞 |
| 派生文档 | 技能实例与卡牌的「## 效果」「属性」派生行由 `render_docs.py` 重写，由 `--check` 拦截漂移 |
| 模板存放 | 模板实体文档放在 `content/skill-templates/`，不写进生成器代码 |

备选方案与取舍见讨论记录：卡牌内联技能（改动 535 处引用、失去按技能 id 调整的能力）与运行时组合（多一层间接、与引擎优化方向相反）均未采用。

## 3. 分层模型

| 层 | 实体 | 存放 | 现状 | 本次动作 |
| --- | --- | --- | --- | --- |
| L0 原语 | `Op` + `Params` | `rule-engine/src/effects/mod.rs` | 已有 21 种原语 | 不变 |
| L1 原子技能 | 技能模板 | `content/skill-templates/<id>.md` | 无 | 新建，约 40–60 个 |
| L2 技能 | 技能实例 | `content/skills/<id>.md` | 247 份，写死 effects 与 text | 改为模板 + 参数 |
| L3 卡牌 | 卡牌 | `content/cards/<id>.md` | `skills` 有序引用 | 引用不变，派生文案改为生成 |

「回合开始时获得 +0/+1」在 L0 是 `OnTurnStart + ModifyStats(health=1, duration=permanent)`，在 L1 是模板 `turn-start-stats`，在 L2 是实例 `args: health=1`，在 L3 被某张卡引用。文案由模板与参数渲染。

## 4. 作者格式

### 4.1 技能模板

路径：`docs/versions/v0.1/product/content/skill-templates/<id>.md`，文件名等于 `id`。模板是技术实体，`skill-templates/README.md` 说明它与实例的分工。

front matter 字段：

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `id` | 是 | `^[a-z][a-z0-9-]*$`，等于文件名，模板命名空间内唯一 |
| `kind` | 否 | `effect`（缺省）或 `keyword` |
| `trigger` | `kind=effect` 必填 | 触发时机，取值与引擎 `Trigger` 白名单一致 |
| `op` | `kind=effect` 必填 | 原语，取值与引擎 `Op` 白名单一致 |
| `selector` | `kind=effect` 必填 | 选择器原文，可含 `{arg}` 占位 |
| `params` | 否 | 参数原文，可含 `{arg}` 占位；无参数时省略 |
| `arg.<name>` | 否 | 参数声明，一行一个，见 §4.2 |
| `text` | 是 | 文案模板，可含占位符，见 §5 |

示例（与现有数据逐字对应）：

```
# on-death-summon.md
---
id: on-death-summon
kind: effect
trigger: OnDeath
op: Summon
selector: self
arg.token: token
arg.count: count 1..3 default=1
params: token={token},count={count}
text: 亡语：召唤{count.cn}个{token.stats}的{token.name}
---
```

```
# on-play-modify-stats-single.md（无筛选）
---
id: on-play-modify-stats-single
kind: effect
trigger: OnPlayEffect
op: ModifyStats
selector: friendly:single
arg.attack: int
arg.health: int
params: attack={attack},health={health},duration=permanent
text: 战吼：使一个友方随从获得 +{attack}/+{health}
---
```

```
# on-play-modify-stats-taxon.md（按分类筛选）
---
id: on-play-modify-stats-taxon
kind: effect
trigger: OnPlayEffect
op: ModifyStats
selector: friendly:single:taxon={taxon}
arg.taxon: taxon
arg.attack: int
arg.health: int
params: attack={attack},health={health},duration=permanent
text: 战吼：使一个友方{taxon}随从获得 +{attack}/+{health}
---
```

```
# keyword-self.md
---
id: keyword-self
kind: keyword
arg.keyword: keyword
text: {keyword.text}
---
```

筛选用独立模板表达：无筛选的写入 `selector` 本身，有筛选的写成 `selector: friendly:single:taxon={taxon}` 并由参数域限制取值。模板不提供「可选筛选段」这套语法。

### 4.2 参数声明

语法：`arg.<name>: <type> [<min>..<max>] [default=<value>]`，同一行内以空格分隔。

| 类型 | 取值 | 默认渲染 `{name}` | 修饰符 |
| --- | --- | --- | --- |
| `token` | Token 注册表 id；查不到时回退为卡牌 id | 展示名 | `.stats` → `攻/血` |
| `card` | 卡牌 id | 卡牌名 | `.stats` → `攻/血` |
| `taxon` | `taxonomy.md` 分类节点 id | 卡面名词（见 §5.4，如「鱼纲」「动物」） | — |
| `playstyle` | 玩法 id | 卡面名词（如「机械」「真菌」） | — |
| `scholar` | 固定 `true` | 「学者」 | — |
| `keyword` | 关键词 id，可带层数（`breed 2`） | 名称 + 层数（如「繁衍 2」） | `.name` 仅名称、`.level` 仅层数、`.text` 关键词整段卡面文案（§5.4） |
| `int` | 整数 | 数字 | `.cn` → 中文数词 |
| `count` | 整数 1–4 | 中文数词（一/两/三/四） | `.num` → 数字 |
| `duration` | `permanent` / `combat` | 空串 | `.suffix` → `combat` 为「直到回合结束」，`permanent` 为空 |

- 未声明默认值的参数必须由实例提供；声明默认值的参数可省略。
- **默认值只参与文案渲染，不参与效果渲染**：实例未提供的参数，其 `k={k}` 段在效果中整段省略。模板需要固定数值时直接在 `params` 里写字面量（如 `duration=permanent`），不要靠参数默认值补齐——这样迁移前后 `skills.json` 才能逐字段一致。
- `token` 与 `card` 都提供 `.stats`：`deathrattle-summon-termite` 的 `card=baiyi` 需要「1/2 的白蚁」，与 `token=` 的用法共用同一套渲染。
- 参数名在模板内唯一；实例出现未声明的参数、缺少必填参数、取值越界都是校验错误。

### 4.3 技能实例

路径不变：`content/skills/<id>.md`。

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `id` | 是 | 等于文件名，技能命名空间内唯一 |
| `template` | 是 | `模板 id(参数=值, ...)`；参数只绑定到同一条模板，多个模板以 `；` 分隔 |
| `keyword` | 否 | 携带者自身登记的关键词，语义不变 |
| `cost` / `uses_per_turn` | 否 | 主动技能字段，语义不变 |
| `name` / `match_text` | 条件必填 | 语义与现有规则不变 |
| `text` | 否 | 文案覆盖；填写后优先于模板渲染 |

示例：

```
# deathrattle-summon-termite.md
---
id: deathrattle-summon-termite
template: on-death-summon(token=termite,count=1)
keyword: deathrattle
---
```

```
# battlecry-friendly-plus-1-health.md
---
id: battlecry-friendly-plus-1-health
template: on-play-modify-stats-single(attack=0,health=1)
keyword: battlecry
---
```

参数内联在模板上，不设全局 `args` 字段：一个实例的每个参数只属于一条模板，避免「多条模板时参数归谁」的歧义。

组合：实例的 `template` 可写多个原子技能，文案按顺序用「；」连接。现有只有 `efficient-algorithm` 一个技能有两个效果段，本次按下列写法迁移；它的现有文案没有「；」，因此用实例 `text` 覆盖保持逐字一致。

```
template: on-play-refresh-shop(free=true); on-play-gain-minion(level=current,to=hand)
```

正文与属性表：

- `## 属性` 增加「模板」行，写模板 id（多个用「；」分隔）。
- `## 效果` 写渲染文案；由 `render_docs.py` 重写。

### 4.4 卡牌派生小节

卡牌 front matter 与 `skills` 不动。由 `render_docs.py` 重写：

- `## 效果`：按 `skills` 顺序拼接实例渲染文案，用「；」连接；无技能写「无」。
- `## 属性` 的「技能」「关键词」两行：由引用派生，口径不变。

卡牌的定位、实现口径、出处等手写小节不重写。

## 5. 渲染规则

### 5.1 效果渲染

1. 取模板的 `selector` / `params` 原文，把 `{arg}` 替换为实例值，修饰符按 §4.2 展开；实例未提供的参数，其 `k={k}` 段整段省略。
2. 与 `trigger`、`op` 拼成现有 DSL 段：`Trigger|Op|Selector|Params`。
3. 走现有 `validate_docs.parse_effects` 与 `audit_effects.check_effect`，再编译为 `skills.json` 的结构化数组。

模板不引入第二套效果语法；解析、白名单与语义校验全部复用现状。

### 5.2 文案渲染

- 占位符写作 `{name}` 或 `{name.modifier}`；未知占位符、未声明参数、缺失必填参数都是校验错误。
- 数字：`int` 默认输出阿拉伯数字；`count` 默认输出中文数词（一/两/三/四），现有「一个」「两个」「三个」「四个」逐字一致。
- 名词来源：`taxon` / `playstyle` 取 §5.4 卡面名词表；`token` 取 `tokens/README.md` 的展示名与攻/血，查不到时回退为卡牌；`card` 取卡牌 `name`、`attack`、`health`。
- 关键词：`{kw}` 输出「名称 + 层数」（繁衍 2、减伤 2），`{kw.name}` 只输出名称，`{kw.level}` 只输出层数，`{kw.text}` 输出 §5.4 关键词文案表里的整段卡面文案。现有「获得 1 层传染」这类写法单独建模板或使用 `text` 覆盖。
- 时长：`{duration.suffix}` 在 `combat` 时输出「直到回合结束」、在 `permanent` 时输出空串；「本回合获得连击」这类句式另建模板或使用 `text` 覆盖。
- 标点与空格由模板原文控制，渲染器不自动加空格或标点。
- 实例 `text` 覆盖时直接使用，不再渲染；但仍受现有「文本前缀与触发时机一致」校验。

### 5.3 组合

多个模板的效果按顺序拼接，文案用「；」连接后写入实例的 `## 效果` 与引用它的卡牌。

### 5.4 卡面名词表

路径：`docs/versions/v0.1/product/content/card-nouns.md`，三张表：

| 表 | 键 | 值 |
| --- | --- | --- |
| 分类节点 | `taxonomy.md` 的节点 id | 卡面名词，如 `animalia` → 动物、`fish` → 鱼纲 |
| 玩法 | 玩法 id | 卡面名词，如 `machine` → 机械、`fungus` → 真菌 |
| 特殊筛选值 | `scholar=true`、`other=true`、`type=minion`、`id=<卡牌 id>` | 卡面名词，如 学者、其他、随从、弓箭手 |
| 关键词文案 | 关键词 id（`breed`、`armor`、`taunt`…） | 卡面整段文案模板，层数位写 `{N}`，如 `breed` → `繁衍 {N}：回合开始时获得 +{N} 生命`、`taunt` → `嘲讽` |

- 渲染器只从这张表取名词，不回退到 `taxonomy.md` 的显示名：卡面用的是「动物」，分类树写的是「动物界」，两者不能混用。
- 关键词文案表以卡面为准，不直接复用 `keywords-and-mechanics.md` 的规则表：两处措辞有差异（如潜行的卡面文案比规则表少了「攻击或」），迁移必须逐字一致。
- 模板用到的每个筛选值必须在表中有条目，缺失即校验错误。
- 同一取值在现有文案里出现两种写法（如「真菌」与「真菌界」）时，以名词表为准，少数写法用实例 `text` 覆盖，并进入例外清单。

## 6. 生成链改造

| 文件 | 动作 |
| --- | --- |
| `scripts/skill_templates.py` | 新增。加载与校验模板；加载 §5.4 卡面名词表；参数域校验；展开效果 DSL 与文案；供生成、文本表与文档渲染共用 |
| `scripts/gen_card_data.py` | `skill_entry` 改为从模板展开 `effects`；输出结构与现在一致 |
| `scripts/gen_text.py` | `skill.<id>.text` 改为渲染结果；`match_text` 仍取实例字段 |
| `scripts/render_docs.py` | 新增。重写技能实例与卡牌的派生小节；`--check` 只报告漂移，退出码 1 |
| `scripts/validate_docs.py` | 技能字段集合改为 `id` + `template` + 可选字段；增加模板文档校验；派生一致性交给 `render_docs.py --check` |
| `Makefile` | 新增 `docs-render`；`data-check` 增加模板校验与 `render_docs.py --check` |
| `tests/` | 新增模板渲染单测；迁移逐字一致性测试 |

`make data` 只生成运行数据（`server/data`、客户端数据、文本表），不改写源文档；改写源文档由显式的 `make docs-render` 执行。这是为了让自动改文档的动作可见、可审。

## 7. 引擎边界

- `skills.json` 的字段与取值不变，引擎 crate 无代码改动。
- 模板只存在于作者层与编译期；运行时不感知模板。
- 后续引擎优化（紧凑 AST、字符串驻留、预编译原语表）只改编译产物与 runtime，不回头影响模板与实例。
- 模板的 `trigger` / `op` / 筛选键白名单直接引用现有校验常量，与 Rust 枚举的对应关系由现有测试继续守护。

## 8. 迁移方案

1. **基础设施**：模板 schema、渲染器、卡面名词表骨架、单测；此时不改任何实体文档。
2. **名词表**：统计技能里用到的全部筛选值，填齐 §5.4 三张表，并逐条对照现有文案确认写法。
3. **模板提取**：脚本逐技能尝试「模板 × 参数」，要求展开后的结构化效果与文案逐字等于现状。产出报告：可覆盖的技能按模板分组；产不出的进入例外清单（保留 `text` 覆盖，或新增模板）。
4. **分批改写实例**：按触发动机分批（`OnPlayEffect` 63、`OnDeath` 39、`OnPlay` 37、`OnCombatStart` 37、`Aura` 21、`OnActivate` 18、其余触发 18、纯关键词 14）。每批执行 `make data-check`、相关单测。
5. **全量文档渲染**：先 `render_docs.py --check` 出报告，再写；核对卡牌 diff 只出现在「## 效果」与属性派生行。
6. **文档回写**：`skill-system.md`、`skills/README.md`、`card-system.md`（技能行口径）、`keywords-and-mechanics.md` §9.4（作者 DSL 说明改为模板 DSL）、新增 `skill-templates/README.md` 与 `card-nouns.md`。
7. **收尾**：删除实例上的 `effects` 字段解析分支与对应校验，`validate_docs` 只保留模板路径。

## 9. 验收

- 迁移前后 `server/data/v0.1/skills.json` 逐字段一致；`client/text/zh_CN.csv` 中 `skill.*.text` 逐字一致；全部卡牌 `## 效果` 逐字一致。
- `make data-check`、`make test`、`make lint`、`make m1-check` 全绿。
- `render_docs.py --check` 幂等，退出码 0。
- `python3 scripts/validate_docs.py` 仍输出「随从 309 张、效果牌 100 张、技能 247 个、英雄卡 32 张，校验通过」。
- 引擎 crate 无 diff。
- 例外清单写入设计文档附录或 `skill-templates/README.md`，数量与原因可查。

## 10. 非目标

- 不改任何游戏规则、数值与卡面文案。
- 不改运行时结构、引擎代码与协议。
- 不合并「同模板同参数」的重复技能 id（后续可做）。
- 不为尚未使用的未来 `Op` 预建模板。
- 不改卡牌分类、经济、素材与客户端表现。

## 11. 风险

| 风险 | 应对 |
| --- | --- |
| 文案逐字还原创不出（措辞变体、特殊后缀如「（复制体）」） | 先跑迁移报告；例外用实例 `text` 覆盖，逐字比对通过才算完成 |
| front matter 解析器不支持嵌套 | 参数声明用扁平 `arg.<name>` 键，实例参数用 `args: k=v,k=v` |
| 模板与引擎白名单漂移 | 模板校验直接复用 Trigger/Op/筛选键常量；与 Rust 枚举的对应关系沿用现有测试 |
| 自动重写 410 份卡牌文档 diff 大 | `render_docs` 只改派生小节；先 `--check` 出报告再分批写入 |
| 模板数量膨胀 | 模板按句式划分，不按数值划分；评审参数域与文案模板 |

## 12. 我做的判断项

- 模板放在 `content/skill-templates/` 文档中，不写进 Python 代码：文案与参数域可评审、可校验，符合「数据唯一源在 docs」。
- 模板复用现有 DSL 语法加 `{arg}` 占位，不引入第二套效果语法。
- 卡面名词单独建 `content/card-nouns.md`，不从 `taxonomy.md` 的显示名派生：卡面写「动物」，分类树写「动物界」，两者必须分开维护。
- 参数内联在 `template` 上，不设全局 `args`：避免多条模板时参数归属歧义。
- 默认值只参与文案渲染；效果里不出现「未提供的参数」，需要固定值就在 `params` 写字面量。这是迁移前后 `skills.json` 逐字段一致的前提。
- 关键词用 `.name` / `.level` / `.text`，时长用 `.suffix`；覆盖不到的句式用实例 `text` 覆盖并计入例外清单，不为个别措辞扩张参数语法。
- `token` 类型查不到时回退为卡牌，`card` 类型同样提供 `.stats`：`card=` 与 `token=` 两种召唤写法共用模板。
- `make data` 不写文档；新增 `make docs-render`；`data-check` 拦截文档漂移。
- 纯关键词技能使用 `kind: keyword` 模板，文案取自关键词判定表，不再在 14 份技能文档里重复。
- 只有 `efficient-algorithm` 一个双效果技能使用实例级组合语法，文案用 `text` 覆盖保持逐字一致。
