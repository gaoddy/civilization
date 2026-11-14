# 移除 `hero_skill` 字段设计

> 状态：已确认，待实施
> 前置：[移除英雄独立实体](2026-11-14-remove-hero-entity-design.md)

## 1. 目标

上一轮已把 13 位英雄并入卡池，成为 `is_hero: true` 的普通随从卡；但英雄技能仍由卡上的 `hero_skill` 指针和 4 个宿主字段单独表达。本设计继续收敛：**英雄技能就是卡牌技能，不再有独立的英雄技能字段。**

## 2. 已确认的取舍

讨论过三种落法：

| 方案 | 做法 | 结论 |
| --- | --- | --- |
| A. 并入 `skills`，不再特殊 | 英雄技能写入卡的 `skills`；主动技能的费用与次数成为技能实体字段 | **采用** |
| B. 并入 `skills` 并标 `hero_only` | 保留「买卡不获得英雄技能」的差异，但多一个标记字段 | 未采用：仍是一套特例 |
| C. 英雄卡不再有技能 | 删除 13 个 `hero-*` 技能 | 未采用：砍掉整套玩法 |

方案 A 的直接后果：**买到同名英雄卡也会获得它的全部 `skills`，包括原来的英雄技能。** 这是有意的简化，不再保留「买卡不获得英雄技能」的特例。

## 3. 数据模型

### 3.1 从卡上删除

- `hero_skill`
- `hero_skill_cost`
- `hero_skill_type`
- `hero_skill_uses_per_turn`
- `hero_skill_trigger`

`is_hero` 与属性表的「英雄资格」行保留：它们只回答「这张卡能否被选为英雄」。

### 3.2 技能实体新增

主动技能新增两个可选字段，写在 front matter 的 `match_text` 之后：

| 字段 | 类型 | 取值域 | 说明 |
| --- | --- | --- | --- |
| `cost` | 整数 | 0–2 | 主动技能每次使用的金币费用 |
| `uses_per_turn` | 整数 | 1–2 | 主动技能每回合可使用次数 |

**判定规则**：同时出现 `cost` 与 `uses_per_turn` 的技能是主动技能；两者都不出现的技能是触发式或常驻技能。不允许只出现一个。

被动/触发技能的触发时机继续由文本前缀表达，沿用现有 `check_text_trigger` 口径：

- 「战斗开始时…」→ `OnCombatStart`
- 「每回合首次刷新时…」→ `OnFirstRefresh`

主动技能的 `effects` 继续以 `OnActivate` 开头，不需要新的触发词。

### 3.3 技能属性表

主动技能的 `## 属性` 表在「比赛模式替换」之后新增两行：

```markdown
| 费用 | 1 |
| 每回合次数 | 1 |
```

被动技能不加这两行。

### 3.4 卡牌 `skills` 变更

13 张英雄卡各引用恰好 1 个 `hero-*` 技能：

| 卡 | 原 `skills` | 新 `skills` | 原英雄技能 |
| --- | --- | --- | --- |
| adam-smith | 无 | `[hero-division-of-labor]` | hero-division-of-labor |
| darwin | `[aura-biology-plus-1-plus-1]` | `[aura-biology-plus-1-plus-1, hero-evolution-discovery]` | hero-evolution-discovery |
| davinci | 无 | `[hero-universal-manuscript]` | hero-universal-manuscript |
| edison | 无 | `[hero-invention-factory]` | hero-invention-factory |
| einstein | `[battlecry-all-friendly-growth-once]` | `[battlecry-all-friendly-growth-once, hero-light-speed-strike]` | hero-light-speed-strike |
| faraday | `[keyword-divine-shield]` | `[keyword-divine-shield, hero-faraday-cage]` | hero-faraday-cage |
| galileo | 无 | `[hero-observation]` | hero-observation |
| knuth | 无 | `[hero-efficient-algorithm]` | hero-efficient-algorithm |
| luxun | 无 | `[hero-shout]` | hero-shout |
| marie-curie | `[battlecry-friendly-growth-twice]` | `[battlecry-friendly-growth-twice, hero-radiation]` | hero-radiation |
| mendel | `[battlecry-friendly-plant-growth-once]` | `[battlecry-friendly-plant-growth-once, hero-heredity]` | hero-heredity |
| pasteur | 无 | `[hero-microbiology]` | hero-microbiology |
| tesla | 无 | `[hero-alternating-current]` | hero-alternating-current |

英雄技能统一追加在 `skills` 末尾：既有的随从技能保持在第一位，`## 效果` 的前半句不变。

### 3.5 主动技能清单（10 个）

| 技能 | 费用 | 每回合次数 |
| --- | --- | --- |
| hero-division-of-labor | 1 | 1 |
| hero-evolution-discovery | 2 | 1 |
| hero-universal-manuscript | 2 | 1 |
| hero-invention-factory | 2 | 1 |
| hero-light-speed-strike | 1 | 1 |
| hero-faraday-cage | 2 | 1 |
| hero-efficient-algorithm | 2 | 1 |
| hero-shout | 1 | 2 |
| hero-heredity | 1 | 1 |
| hero-alternating-current | 2 | 1 |

### 3.6 被动技能清单（3 个）

| 技能 | 触发时机（文本前缀） |
| --- | --- |
| hero-observation | 每回合首次刷新时 |
| hero-microbiology | 战斗开始时 |
| hero-radiation | 战斗开始时 |

> 修正：批准时口头说的「8 个主动、2 个被动」是笔误。按现有卡表实际为 **10 个主动、3 个被动**；方案本身不变。

## 4. 行为变化

- 开局选中英雄卡：获得该卡的全部 `skills`。
- 买到同名英雄卡：同样获得全部 `skills`，包括 `hero-*` 技能；不再有「买卡不获得英雄技能」的规则。
- `UseHeroPower`：改为「使用所选英雄卡上的一个主动技能」；费用取技能的 `cost`，每回合次数取 `uses_per_turn`。
- 被动技能：以玩家为宿主，按文本前缀在对应时机触发，行为不变。
- 平衡矩阵不变：主动技能不是关键词，不进入 `keyword_cost`；卡牌数值未改。

## 5. 文档与校验变更

- `cards/`：13 张英雄卡的 front matter 删除 5 个字段、改 `skills`；属性表删除 5 行，保留「英雄资格」；删除 `## 英雄技能` 小节；`## 效果` 由技能渲染自动包含英雄技能文本。
- `skills/`：10 个主动技能 front matter 增加 `cost`、`uses_per_turn`；属性表增加两行。
- `card-system.md`：删除英雄技能字段行；「英雄卡」小节改为「英雄卡 = 带 `is_hero` 的普通随从卡，技能全部走 `skills`」；买卡规则改写。
- `skill-system.md`：删除 `hero_skill` 引用口径；新增主动技能字段；数据总览改为「118 个技能全部经 `skills` 引用」。
- `core-rules.md`：`UseHeroPower` 与「战斗开始时」表述改为主动/被动技能口径；删除「买同名卡不获得英雄技能」。
- `keywords-and-mechanics.md`、`modes-and-achievements.md`、`network-architecture.md`、各 README 与总览：删除 `hero_skill` 与「英雄技能」特有表述，统一为「英雄卡上的技能」。
- `scripts/validate_docs.py`：
  - 删除 `HERO_CARD_FIELDS`、`check_hero_skill_rendering`、`check_skill_context` 的 hero 分支。
  - `expected_card_attrs` 不再输出 5 个英雄技能行。
  - `check_hero_cards` 改为：英雄卡恰好 13 张；每张在 `skills` 中恰好引用 1 个 `hero-*` 技能；`hero-*` 技能恰好被 1 张英雄卡引用；非英雄卡不得有 `is_hero: true`。
  - `check_skills` 增加主动技能字段校验：`cost`/`uses_per_turn` 必须成对出现、取值合法、`name` 与 `match_text` 必填；`hero-*` 技能必须有 `name`。
  - `expected_skill_attrs` 为主动技能输出「费用」「每回合次数」两行。
  - 无技能随从期望 15 → 7。
- `tests/`：更新英雄卡属性表测试；新增主动技能字段与「英雄卡各引用 1 个 hero-* 技能」的断言。

## 6. 数据结果

| 维度 | 变更后 |
| --- | --- |
| 卡牌 | 184 随从（含 13 张英雄卡）+ 36 效果牌 = 220 |
| 技能 | 118，全部经 `skills` 引用 |
| 无技能随从 | 7 |
| 卡牌技能引用 | 247 段（随从 211 + 效果 36） |
| 被引用的唯一技能 | 118 |

校验末行保持：`随从 184 张、效果牌 36 张、技能 118 个、英雄卡 13 张、成就 6 个，校验通过`。

## 7. 验收标准

1. 全仓无 `hero_skill`、`hero_skill_cost`、`hero_skill_type`、`hero_skill_uses_per_turn`、`hero_skill_trigger`，无 `## 英雄技能` 小节（历史设计文档除外）。
2. 13 张英雄卡各引用恰好 1 个 `hero-*` 技能；每个 `hero-*` 技能恰好被 1 张英雄卡引用。
3. 10 个主动技能带合法的 `cost` 与 `uses_per_turn`，属性表含对应两行；3 个被动技能不带。
4. `python3 scripts/validate_docs.py` 退出码 0，末行为第 6 节的输出。
5. `python3 -m unittest discover -s tests -v` 全部通过。
6. 相对链接校验通过，文档计数与校验脚本一致。

## 8. 非目标

- 不重命名 `hero-*` 技能 ID；前缀只作为「由英雄卡引用」的命名线索。
- 不调整 13 张英雄卡的数值、星级、时代、流派与出处。
- 不改动 36 张效果牌。
- 不为主动技能新增触发关键词或 DSL 原语。
