# 技能模板

本目录存放 v0.1 的技能模板（作者层 L1）。模板是一段带参数的原子技能：触发时机、原语、选择器形状、参数域与文案模板都写在这里；技能实体（[技能索引](../skills/README.md)）只写模板 id 与参数，数值只出现在实例里。模板只在构建期存在，展开成 `effects` 后进入 `skills.json`，运行时与规则引擎不感知模板。

字段规范、参数类型与渲染规则的权威说明见[技能规范与数据](../skill-system.md) §2.1；本文给写法与日常使用口径。

## 1. 文件与字段

每个模板一个文档：`<id>.md`，文件名等于 `id`，模板命名空间内唯一。

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `id` | 是 | `^[a-z][a-z0-9-]*$`，等于文件名 |
| `kind` | 否 | `effect`（缺省）或 `keyword`；`keyword` 模板只提供文案，不产生效果段 |
| `trigger` | `kind=effect` 必填 | 触发时机，取值与引擎 `Trigger` 白名单一致 |
| `op` | `kind=effect` 必填 | 原语，取值与引擎 `Op` 白名单一致 |
| `selector` | `kind=effect` 必填 | 选择器原文，可含 `{参数}` 占位 |
| `params` | 否 | 参数原文，可含 `{参数}` 占位；无参数时省略 |
| `arg.<name>` | 否 | 参数声明，一行一个 |
| `text` | 是 | 文案模板，可含占位符 |

示例：

```
---
id: on-death-summon
trigger: OnDeath
op: Summon
selector: self
arg.token: token
arg.count: count 1..4 default=1
params: token={token},count={count}
text: 亡语：召唤{count.cn}个 {token.stats} 的{token.name}
---
```

对应实例：

```
---
id: deathrattle-summon-seedling
keyword: deathrattle
template: on-death-summon(token=seedling,count=1)
---
```

同一个模板按参数产生多个技能：「繁衍 1/2/3」「亡语召唤不同非池卡与数量」都是同模板不同参数。调平衡时只改实例参数，卡面文案随模板渲染，不再手改 `## 效果`。

## 2. 参数声明

语法：`arg.<name>: <type> [<min>..<max>] [default=<value>]`。

| 类型 | 取值 | 默认渲染 | 修饰符 |
| --- | --- | --- | --- |
| `token` | 非池卡（`in_shop_pool: false`）id | 展示名 | `.stats` → 攻/血 |
| `card` | 进池卡（`in_shop_pool: true`）id | 卡牌名 | `.stats` → 攻/血 |
| `taxon` | 分类节点 id | 卡面名词 | — |
| `playstyle` | 玩法 id | 卡面名词 | — |
| `keyword` | 关键词 id，可带层数（`breed 2`） | 名称 + 层数 | `.name`、`.level`、`.text` |
| `int` | 整数 | 数字 | `.cn` → 中文数词 |
| `count` | 整数 1–4 | 中文数词（一/两/三/四） | `.num` → 数字 |
| `duration` | `permanent` / `combat` | 空串 | `.suffix` → 「直到回合结束」/ 空 |

- 未声明默认值的参数必须由实例提供；声明默认值的参数可省略。
- **默认值只参与文案，不参与效果**：实例未提供的参数，其 `k={k}` 段在效果里整段省略。需要固定数值时直接在 `params` 写死（如 `duration=permanent`），不要靠默认值补齐。
- 参数名在模板内唯一；实例出现未声明参数、缺必填参数、取值越界都是校验错误。

## 3. 渲染规则

**效果**：把 `selector` / `params` 中的 `{参数}` 替换为实例值，与 `trigger`、`op` 拼成 `Trigger|Op|Selector|Params`，再走[关键词与机制](../../rules/keywords-and-mechanics.md) §9.4 的同一套校验与编译。关键词参数可拆成 `.name` / `.level` 两段写进效果段，缺失的分量整段省略。

**文案**：占位符写作 `{name}` 或 `{name.modifier}`；未知占位符、未声明参数、缺失必填参数都是错误。`{duration.suffix}` 接在数字或拉丁字母后时，渲染器补一个空格（如「获得 +0/+1 直到回合结束」）。

**覆盖**：实例写了 `text` 时直接使用覆盖文案，不再渲染。当前共 5 个实例保留覆盖，逐条原因见[技能规范与数据](../skill-system.md) §2.1「例外清单」。

**组合**：实例的 `template` 可写多个调用，以 `；` 分隔；效果按顺序拼接，文案用「；」连接。

## 4. 卡面名词表

模板里的 `taxon` / `playstyle` / `keyword` 文案来自[卡面名词表](../card-nouns.md)的四张表：分类节点、玩法、特殊筛选值、关键词文案。渲染器只从这张表取名词，不回退到 `taxonomy.md` 的显示名（卡面「动物」、分类树「动物界」）。模板用到的筛选值必须在表中有条目，缺失即校验错误。

## 5. 命名约定

模板按「触发时机 + 动作 + 目标形状」命名，便于按句式查找：

| 前缀 | 含义 | 例 |
| --- | --- | --- |
| `on-play-` | 战吼 / 出场效果 | `on-play-stats-single-both`、`on-play-refresh-shop`、`on-play-grant-minion` |
| `on-death-` | 亡语与死亡触发 | `on-death-summon`、`on-death-summon-split` |
| `on-activate-` | 主动技能 | `on-activate-refresh-shop`、`on-activate-gain-minion` |
| `on-combat-start-` / `combatstart-` | 战斗开始时 | `combatstart-damage-all` |
| `turn-start-` | 回合开始时 | `turn-start-self-health` |
| `aura-` | 光环 | `aura-all-health` |
| `keyword-` | 关键词文案与自持关键词 | `keyword-self` |

模板按句式划分，不按数值划分：数值变化用参数，措辞变化才新建模板。

## 6. 校验

模板随数据一起校验，修改后运行：

```bash
make data-check
```

包含模板字段与参数声明校验、实例展开后的 DSL 校验（§9.4）、卡面名词表完整性，以及 `render_docs.py --check` 对派生文档的漂移检查。改了实例参数后若 `## 效果` 或卡牌派生行有变，运行 `make docs-render` 重写文档，再提交。
