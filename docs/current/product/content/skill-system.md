# 技能规范与数据

> 依赖：[卡牌规范与数据](card-system.md) §2.1「英雄卡」、[关键词与机制](../rules/keywords-and-mechanics.md)
> 状态：已实现
> 数据：`docs/current/product/content/skills/`（262 份，文件名 = id）
> 模板：`docs/current/product/content/skill-templates/`（文件名 = id）

本文描述技能实体的定义、字段规范、引用与渲染规则、共享规则、数据总览与校验方式；不描述具体实现与后续版本技能。归属已实现产品状态的内容。

## 1. 定义与关系

- **技能**：一段可独立结算的效果。技能有独立 `id` 与规范文本，可被任意卡牌（含英雄卡）引用，也可被多张同时引用。
- **技能模板**：一段带参数的原子技能，定义触发时机、原语、选择器形状、参数域与文案模板；模板是作者层的编译单元，运行时不感知。
- **技能实例**：技能实体文档只写模板调用与参数，不写结构化效果与文案；数值只出现在参数里。
- **效果牌**：本质上是一定携带技能的卡牌，必须携带至少 1 个技能，技能数量不设上限。
- **随从牌**：可携带 0–N 个技能；省略 `skills` 等于不带技能。
- **英雄卡**：`content/heroes/` 下的独立卡牌，只有身份与技能字段；技能同样全部通过 `skills` 引用，不含 `card_type`、数值、时代与科学分类（见 [卡牌规范](card-system.md) §2.1）。
- **关键词**：不再是卡牌字段，而是技能上的属性 `keyword`；卡牌的关键词由所引用技能派生。

技能 `id` 与卡牌 `id` 分属两个命名空间，各自查重，允许同名。

**共享引用**：同一技能被多张卡牌（含英雄卡）引用时，它们看到同一份规范文本，技能改动同步生效；但每处引用各自结算，互不影响。

## 2. 字段规范

每个技能一个文档：`docs/current/product/content/skills/<id>.md`，文件名等于 `id`。文档由 YAML front matter 与正文组成。

| 字段 | 必填 | 取值域 | 说明 |
| --- | --- | --- | --- |
| `id` | 是 | `^[a-z][a-z0-9-]*$` | 技能命名空间内唯一；必须等于文件名 |
| `name` | 条件必填 | 自由 | 显示名；被效果牌引用或属于主动技能的技能必填，普通随从技能可省略；运行时由文本表 `skill.<id>.name` 承载 |
| `keyword` | 否 | 16 个常驻关键词的英文 id（含层数：`breed N`、`assemble N`、`armor N`） | 该技能使携带者登记的关键词；赋予他人关键词的效果不填。取值见[关键词与机制](../rules/keywords-and-mechanics.md) §1 |
| `text` | 否 | 自由 | 文案覆盖；不填时由模板渲染，填写后优先于模板渲染。运行时由文本表 `skill.<id>.text` 承载 |
| `text_golden` | 否 | 自由 | 金色文案覆盖；不填时由模板的 `text_golden` 或按模板文案自动翻倍数字参数。运行时由文本表 `skill.<id>.text.golden` 承载，与普通文案相同时不生成该键 |
| `template` | 是 | `模板 id(参数=值, ...)`，多个模板以 `；` 分隔 | 模板调用；参数只绑定到同一条模板。语法见 §2.1 |
| `cost` | 条件必填 | 整数 0–3 | **仅主动技能**；每次使用的金币费用；与 `uses_per_turn` 必须成对出现 |
| `uses_per_turn` | 条件必填 | 整数 1–2 | **仅主动技能**；每回合可使用次数；与 `cost` 必须成对出现 |

**运行时文本来源**：front matter 的 `name`、`text` 与 `text_golden` 是文本的编辑来源（`text` 缺省时取模板渲染结果，`text_golden` 缺省时取模板的金色渲染结果），构建期由 `scripts/gen_text.py` 汇总到 `client/text/zh_CN.csv`（key 分别为 `skill.<id>.name`、`skill.<id>.text`、`skill.<id>.text.golden`），客户端按 id 取文本；金色键只在金色文案与普通文案不同时生成，缺键回落普通文案；服务端生成数据不含这些展示字段。新增语言需要扩展文本生成工具并补充各语言文本、注册对应翻译，业务代码与技能实体结构不改动。

**主动与触发的判定**：同时具备 `cost` 与 `uses_per_turn` 的技能是**主动技能**，由玩家支付费用后触发，模板展开后的效果以 `OnActivate` 开头；两者都不具备的是**触发式或常驻技能**，按文本前缀在对应时机自动结算。不允许只出现其中一个字段。

**编码约定**：front matter 一行一个字段；模板调用的参数用 `,`、多个模板用 `；`；模板展开后的 `effects` 段用 `|`、效果用 `;`、参数用 `,`。

**正文约定**：一级标题在 `name` 存在时为显示名，否则为 `id`；`## 属性` 用表格复述 `id`、关键词与模板，主动技能再加「费用」「每回合次数」两行；`## 效果` 写渲染后的文案。属性表与 front matter 必须一致，由 §6 的校验脚本交叉核对，派生行由 `render_docs.py` 重写。

**`text` 的形态**：沿用卡面现有写法。战吼、亡语、繁衍、装配、播种、进化、传染等带触发关键词的效果保留前缀（如「战吼：使一个友方随从获得一次成长」）；主动技能不带前缀，由 `cost` / `uses_per_turn` 判定为主动；其余被动技能保留触发前缀（如「战斗开始时…」「每回合首次刷新时…」）。

**技能边界**：一段技能是一个可独立结算的效果单元，不等于按「；」机械切分。「若其为<筛选对象>，改为<效果>」是前一段效果的条件变体，必须与前一段留在同一个技能内（如炼金术士、巫医），因此技能 `text` 内部可以出现「；」。其余并列效果各成一个技能。

**`keyword` 的口径**：只登记「携带者自身拥有的常驻关键词」。例如白蚁的「繁衍 1」登记在技能 `keyword` 上，再由白蚁这张卡派生；而效果牌「光合作用」的文本是「使一个友方植物随从获得繁衍 2」，关键词是赋予目标的，技能的 `keyword` 留空，卡牌也不因此获得关键词。每段技能最多登记 1 个关键词；一张卡有多个关键词时，拆成多个技能。

**参数化**：同一种效果的不同数值共用一个模板，数值只写在实例参数里。例如「繁衍 1」「繁衍 2」「繁衍 3」是同一个关键词模板的三个实例参数；修改数值只改实例，卡面文案随模板渲染同步。

**`template` 与 `text` 的关系**：`template` 是效果的唯一作者来源，构建期展开为 `effects` 并渲染 `text`；实例 `text` 只用于模板无法逐字覆盖的特殊措辞。`effects` 是编译产物，不写在实例上，也不进入 `## 属性` 表——引用方通过 `skills` 继承。

### 2.1 技能模板与渲染

模板是作者层的原子技能，每个模板一个文档：`content/skill-templates/<id>.md`，文件名等于 `id`，模板命名空间内唯一。模板与实例的分工、写法与示例见[技能模板说明](skill-templates/README.md)。

| 字段 | 必填 | 取值域 | 说明 |
| --- | --- | --- | --- |
| `id` | 是 | `^[a-z][a-z0-9-]*$` | 模板命名空间内唯一；必须等于文件名 |
| `kind` | 否 | `effect`（缺省）/ `keyword` | `keyword` 模板只提供文案，不产生效果段 |
| `trigger` | `kind=effect` 必填 | §9.4 触发白名单 | 触发时机 |
| `op` | `kind=effect` 必填 | §9.4 原语白名单 | 结构化动作 |
| `selector` | `kind=effect` 必填 | §9.4 选择器语法 | 可含 `{参数}` 占位 |
| `params` | 否 | `key={参数}, ...` | 可含占位；无参数时省略 |
| `arg.<name>` | 否 | `<类型> [min..max] [default=值]` | 参数声明，一行一个 |
| `text` | 是 | 文案模板 | 可含占位符，见渲染规则 |
| `text_golden` | 否 | 文案模板 | 金色文案；不填时按 `text` 自动翻倍数字参数。用于条件与阈值不翻倍、只有产出数值翻倍的模板 |

**参数类型**：

| 类型 | 取值 | 默认渲染 `{name}` | 修饰符 |
| --- | --- | --- | --- |
| `token` | 非池卡 id（`in_shop_pool: false`） | 展示名 | `.stats` → 攻/血 |
| `card` | 进池卡 id（`in_shop_pool: true`） | 卡牌名 | `.stats` → 攻/血 |
| `taxon` | 分类节点 id | 卡面名词 | — |
| `playstyle` | 玩法 id | 卡面名词 | — |
| `keyword` | 关键词 id，可带层数（`breed 2`） | 名称 + 层数 | `.name` 仅名称、`.level` 仅层数、`.text` 关键词整段文案 |
| `int` | 整数 | 数字 | `.cn` → 中文数词、`.x2` → 金色时 ×2 |
| `count` | 整数 1–4 | 中文数词（一/两/三/四） | `.num` → 数字、`.x2` → 金色时 ×2 |
| `duration` | `permanent` / `combat` | 空串 | `.suffix` → `combat` 为「直到回合结束」，`permanent` 为空 |

**效果渲染**：把 `selector` / `params` 里的 `{参数}` 替换为实例值，与 `trigger`、`op` 拼成 `Trigger|Op|Selector|Params`，再走 §9.4 的同一套解析与校验。默认值只参与文案渲染，不参与效果渲染：实例未提供的参数，其 `k={k}` 段整段省略；模板需要固定数值时直接在 `params` 里写字面量（如 `duration=permanent`）。

**文案渲染**：占位符写作 `{name}` 或 `{name.modifier}`；未知占位符、未声明参数、缺失必填参数都是校验错误。实例写了 `text` 时直接使用覆盖文案，不再渲染。多个模板的效果按顺序拼接，文案用「；」连接。

**金色文案**：金色状态默认把 `int`、`count` 与关键词层数 ×2；模板或实例提供 `text_golden` 时改用该文案，并用 `.x2` 只标记需要翻倍的产出数值，条件与阈值占位符保持原值。

**卡面名词表**：`content/card-nouns.md`，四张表——分类节点（`taxon` → 卡面名词，如 `animalia` → 动物）、玩法（`playstyle` → 机械、真菌界）、特殊筛选值（`other=true` → 其他、`type=minion` → 随从、`id=<卡牌 id>` → 卡名）、关键词文案（关键词 id → 卡面整段文案，层数位写 `{N}`）。渲染器只从这张表取名词，不回退到 `taxonomy.md` 的显示名：卡面用「动物」，分类树写「动物界」，两者不能混用。

**例外清单**：实例保留 `text` 覆盖的共 5 个，原因如下：

| 实例 | 原因 |
| --- | --- |
| `battlecry-friendly-plant-kingdom-growth-once` | 名词表 `plant` 渲染为「植物」，卡面写「植物界」 |
| `trigger-fungus-single-plus-0-plus-3` | 名词表 `fungus` 渲染为「真菌界」，卡面写「真菌」 |
| `deathrattle-summon-termite` | 卡面沿用手写「1/2 的白蚁」，与 `baiyi` 当前 2/2 不一致 |
| `deathrattle-summon-3-termites` | 非池卡展示名已含数值，模板的 `{token.stats}` 会重复 |
| `efficient-algorithm` | 两个模板组合，卡面文案没有「；」 |

### 2.2 按科学分类与玩法筛选

技能的目标选择器按以下键筛选，任一键写在选择器或 `condition` / `when` / `timesIf` 中时都按对应口径匹配：

| 键 | 匹配口径 |
| --- | --- |
| `taxon` | 卡牌 `taxon` 等于该节点或位于其下；写上级节点即包含全部下级，如 `taxon=animalia` 命中全部动物卡 |
| `playstyle` | 卡牌任一条 `play:` 精确相等；不命中 `neutral` |
| `play` | 卡牌 `categories` 中存在 `play:<值>`；任一条命中即可，不区分工具卡与效果牌 |
| `prey` / `host` | 卡牌捕食 / 寄生分类中存在与筛选节点相同或位于其下的节点 |
| `subtype` / `id` / `type` / `keyword` / `other` | 沿用既有语义 |

- 科学分类与玩法是两条独立维度：`taxon` 按科学分类筛选；`playstyle` / `play` 按玩法归属筛选，任一条 `play:` 命中即可。非池卡同样按自身 `categories` 参与匹配。
- 技能文本必须与筛选键一致：写「你的犬科随从」用 `taxon=canidae`，写「你的机械随从」用 `playstyle=machine`，「你的动物随从」用 `taxon=animalia`。
- 召唤物按自身玩法与科学分类参与匹配；它们与普通卡同为 `cards.json` 中的卡牌实体，不再有独立注册表或回退路径。
- 当前数据只有 `taxon`、`playstyle`、`play`、`id`、`type`、`keyword`、`other` 被实际使用；`play`、`prey`、`host`、`subtype` 已定义，暂无技能使用。

### 2.3 次数与条件参数

- **`timesIf`**：格式为 `<键>:<值>:<次数>`，例如 `timesIf=playstyle:plant:2`、`timesIf=taxon:hominidae:2`；目标匹配该键值时，次数改为给定值。`scholar` 没有取值，写 `timesIf=scholar:2`。旧格式 `timesIf=human:2` 不再合法。
- **`condition`**：格式为 `count:<阵营>:<键>=<值>:<比较符><数值>`，例如 `count:friendly:taxon=hominidae:>=3`、`count:friendly:playstyle=machine:>=2`；用于「若你控制至少 N 个…」类文本，条件不满足时该效果不执行。
- **`when`**：格式为 `<键>=<值>`，限定触发事件的对象，例如 `when=taxon=animalia`、`when=playstyle=machine`；`condition` 与 `when` 与选择器复用同一套筛选键与取值域。

### 2.4 分解关键词

`分解` 是专属关键词，登记在技能 `keyword` 为 `decompose`，结构化效果使用 `Decompose` 原语，技能 `text` 以「分解：」开头。

- **触发**：由技能数据选择三种之一——本随从击杀随从（含击杀友方，`OnKill`）、友方随从死亡（不含携带者自身，`OnFriendlyDeath`）、友方随从击杀敌方随从（`OnAllyKill`）。携带者与被击杀者同时死亡时不触发。
- **生命值口径**（`basis`）：`original` 原始生命值（卡面基础生命值，金色 ×2）、`combat_start` 战斗开始时生命值、`pre_kill` 被击杀前生命值。
- **时长**（`duration`，必填）：`permanent` 永久，生命值上限与当前生命值同时增加；`combat` 本回合，只增加当前生命值，回合结束时回落。省略或写 `instant` 视为数据错误。
- **结算**：被分解随从的亡语先结算，再结算分解；金色随从获得量 ×2、单次结算；多个分解各自独立触发；获得量为 0、目标已离场或快照缺失时不获得生命值，不报错。
- **归属约束**：任一条 `play:` 命中真菌界或细菌域的卡可以登记 `decompose`；本版本尚无卡牌携带分解，规则与校验先行。

数据写法示例：

```
OnKill|Decompose|self|basis=original,duration=permanent
OnFriendlyDeath|Decompose|self|basis=combat_start,duration=combat
OnAllyKill|Decompose|self|basis=pre_kill,duration=permanent
```

对应卡面文本分别为「分解：本随从击杀随从后，获得其原始生命值」「分解：友方随从死亡后，本回合获得其战斗开始时的生命值」「分解：友方随从击杀敌方随从后，获得其被击杀前的生命值」。

## 3. 引用与渲染

### 3.1 卡牌

卡牌用 `skills: [<技能 id>, ...]` 引用技能，有序列表，顺序即结算顺序，等价于原「卡面文本从上到下」。效果牌的 `skills` 必填且非空；随从牌可省略，省略等于不带技能。

- **`## 效果`**：按 `skills` 顺序取技能渲染文案（实例 `text` 覆盖或模板渲染结果），用「；」连接；不带技能时写「无」。
- **属性表**：新增「技能」行，列出技能 `id`，多个用「；」分隔；无技能写「无」。技能实体自身的属性表另有「模板」行，列出模板调用。
- **关键词**：随从牌的「关键词」行保留，值由所引用技能的 `keyword` 按 `skills` 顺序派生；没有关键词写「无」。卡牌不再有 `keywords` 字段。

### 3.2 英雄卡

英雄卡与普通随从卡使用同一套 `skills`，技能 id 不带任何英雄专属前缀，卡上也没有独立的英雄技能字段。32 张英雄卡各自在自己的 `skills` 中引用自己的技能，也可以同时引用普通随从技能；部分英雄卡复用同一技能。英雄卡与同名随从卡的 `skills` 各自维护、不要求一致，只要求 `name`、`categories` 相同；英雄技能查询只走英雄池，随从与效果牌的技能查询只走随从池。英雄技能只能为主动或被动技能，不登记战吼、护盾、亡语等随从战场关键词，因为英雄不进入战场；一个英雄可以有多个主动技能，按英雄卡 `skills` 顺序展示与结算，各自计次。主动技能的费用与每回合次数属于技能实体（`cost` / `uses_per_turn`），不写在卡上；被动技能按自身文本前缀触发。同一个技能可以被多张卡共用，触发与结算各自独立。

### 3.3 关键词派生

卡牌派生出的关键词只来自技能 `keyword`，不来自卡面文本的任意词。效果文本中「使目标获得某关键词」不使携带者登记该关键词（见 §2「`keyword` 的口径」）。数值折算使用派生后的关键词。

## 4. 共享规则

- 只有 `text` 完全相同、且 `keyword` 上下文一致（同一段落在各卡上登记的关键词相同）时才共享同一个技能；否则各自建立独立技能。
- 措辞不同但语义相近的保持独立，不修改既有卡面文本。
- 唯一需要拆分的共享文本是「战吼：使你的所有机械随从获得一次成长」：克劳德·香农把它登记为 `战吼`，无线电报在迁移前只登记 `装配 2`，两者不能共用同一个 `keyword` 值，因此拆成两个技能。
- 技能文本变化时，所有引用它的卡牌（含英雄卡）同步变化；这是共享引用的预期行为。
- 同一技能被多张卡引用时，各自结算，互不影响。

## 5. 数据总览

| 维度 | 数量 |
| --- | --- |
| 技能总数 | 262 |
| 经卡牌 `skills` 引用的技能 | 251 |
| 仅经英雄卡 `skills` 引用的技能 | 7 |
| 既未被卡牌也未被英雄引用的技能 | 4 |
| 引用技能的效果牌 | 100（每张 1 个技能） |
| 引用技能的随从牌 | 308 |
| 不带技能的随从牌 | 24 |
| 卡牌技能引用 | 538 段效果单元 |

262 个技能中，251 个经卡牌 `skills` 引用，7 个仅由英雄卡引用（`binomial-nomenclature`、`cold-damage-treatise`、`mass-energy-equivalence`、`materia-medica`、`mechanism-art`、`radium`、`seven-voyages`，均为英雄专属主动技能），其余 4 个既未被卡牌也未被英雄引用。24 张不带技能的随从牌中，23 张是非池衍生随从，另 1 张是池内的 `einstein-fridge`（爱因斯坦冰箱，3 星 3/5，效果为「无」）。卡牌技能共 538 段引用（随从 438 段 + 效果牌 100 段），其中 2 处「若其为<筛选对象>」条件变体已并回前一段（见 §2「技能边界」）。

### 5.1 筛选键使用分布

按实例模板展开后的效果中实际出现的筛选键统计，含 `selector.filters` 与 `timesIf` / `condition` / `when` 里的筛选条件（同一技能内同一键只计一次）：

| 筛选键 | 取值示例 | 使用技能数 |
| --- | --- | --- |
| `taxon` | `taxon=animalia`、`taxon=canidae` | 22 |
| `playstyle` | `playstyle=machine`、`playstyle=plant`、`playstyle=virus` | 74 |
| `keyword` | `keyword=breed`、`keyword=taunt`、`keyword=poison` | 0 |
| `type` | `type=minion` | 3 |
| `id` | `id=gongjianshou` | 1 |
| `other` | `other=true` | 20 |
| `play` / `prey` / `host` / `subtype` | — | 0 |

`timesIf` 共 2 处，分别为 `timesIf=taxon:hominidae:2` 与 `timesIf=playstyle:plant:2`；`condition` 共 6 处，均按 `taxon` 统计阵营内数量（`taxon=animalia` 1 处、`taxon=hominidae` 5 处）；`when` 共 3 处，其中 1 处按 `taxon=animalia` 限定触发对象、2 处按 `taxon=hominidae` 限定。数值以脚本统计为准，改技能后同步更新。

### 5.2 共享度

| 技能 | 引用数 |
| --- | --- |
| `shout` 战吼 | 22 |
| `keyword-taunt` 嘲讽 | 21 |
| `keyword-contagion` 传染 | 16 |
| `keyword-sow` 播种 | 16 |
| `keyword-ignore-taunt` 无视嘲讽 | 12 |

引用数由校验脚本按卡牌（不含英雄卡）统计，不写入技能实体文档；另有并列 11 次引用（`keyword-windfury`、`keyword-breed-1`、`keyword-assemble-1`），表中从略。

## 6. 校验

每次修改技能或卡牌文档后必须运行：

```bash
python3 scripts/validate_docs.py
```

技能相关校验覆盖：

1. 技能文档 262 份，`id` 等于文件名且在技能命名空间内唯一；
2. 技能属性表与 front matter 一致；
3. 主动技能必须有 `name`，`cost` 与 `uses_per_turn` 成对出现且取值在 0–3 / 1–2 内；
4. 每张效果牌至少引用 1 个技能；全池恰好 1 张无技能随从（`einstein-fridge`）的 `skills` 为空且 `## 效果` 为「无」；
5. 卡牌 `## 效果` 与所引用技能 `text` 按顺序拼接的结果一致；
6. 卡牌属性表的「技能」「关键词」行与 front matter、技能 `keyword` 一致；主动技能的属性表含「费用」「每回合次数」两行；
7. 英雄卡恰好 32 张，每张英雄卡都有同 id 的同名随从卡且 `name` / `categories` 一致，`skills` 各自维护、不比较；英雄卡引用的技能不得登记 `keyword`，且必须符合主动/被动语义：有 `cost` 的技能只能有 `OnActivate` 触发，无 `cost` 的技能只能有光环、首次刷新、战斗开始或回合开始等被动触发；随从与效果牌不得再有 `is_hero`；卡牌不得残留旧版独立的技能字段或小节；
8. 每个技能都有 `template` 字段；模板文档存在、参数名已声明、取值在域内、必填参数齐全；模板展开后的 DSL 按[关键词与机制](../rules/keywords-and-mechanics.md) §9.4 校验：段数 3–4 段、触发与动作原语在白名单内、选择器合法（筛选值为英文 id、分类节点合法、玩法 id 合法）、`duration` 取值合法、参数写作 `key=value`；`timesIf` 使用「键:值:次数」；`Decompose` 必须带 `basis` 与 `duration`；`OnKill` / `OnAllyKill` 在触发白名单内；
9. 技能文本的固定前缀与触发时机一致：「战吼」对应 `OnPlay`、「亡语」对应 `OnDeath`、「战斗开始时」对应 `OnCombatStart`、「回合开始时」对应 `OnTurnStart`、「每回合首次刷新时」对应 `OnFirstRefresh`、「每当」对应事件类触发、「分解：」对应 `OnKill` / `OnFriendlyDeath` / `OnAllyKill`、`你的…随从获得` 句式对应 `Aura`、「若你控制」对应 `ConditionalAura`；
10. 卡牌分类字段按[卡牌规范与数据](card-system.md) §1.1 校验：`categories` 至少 1 条 `play:`、玩法取值不含 `neutral`、条目与顺序合法、`prey:` / `host:` 只允许生物卡；`taxon` 为分类树叶子；英雄与同 id 随从的 `categories` 一致；
11. 专属关键词按玩法与分类校验：`decompose` 仅 play 命中真菌界/细菌域、`assemble` 仅 play 命中机械（含造物六类工具卡）、`sow` 仅 play 命中植物界、`evolve` 仅生物卡；
12. 派生小节由 `python3 scripts/render_docs.py --check` 校验：技能 `## 属性` 的派生行、技能与卡牌的 `## 效果` 无漂移；`make data-check` 已包含这一步。

期望：输出包含 `随从 332 张、效果牌 100 张、技能 262 个、英雄卡 32 张，校验通过`，退出码 0；否则逐条打印违规并以退出码 1 结束。

模板渲染、`effects` 解析与校验函数有独立单元测试，改动校验逻辑后运行：

```bash
python3 -m unittest discover -s tests -v
```

## 相关文档

- [技能索引](skills/README.md)
- [卡牌规范与数据](card-system.md)
- [关键词与机制](../rules/keywords-and-mechanics.md)
