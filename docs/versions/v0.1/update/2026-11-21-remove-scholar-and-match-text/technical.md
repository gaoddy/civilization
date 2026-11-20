# 移除学者机制与确定性替换文本技术方案

> 状态：方案未实施
> 需求归属：v0.1
> 产品规则：[product.md](product.md)（已确认）

## 1. 改动边界

- 包含：卡牌/技能源文档与索引、数据生成与校验、技能模板、rule-engine 数据模型与效果结算、客户端展示数据与文本表、测试、当前技术文档、v0.2 比赛模式文档与引用。
- 不包含：协议 schema 字段、数据库结构、room-service 对外接口、商店概率与卡牌基础数值。

## 2. 内容文档

### 2.1 移除字段

- 332 份随从文档删除 `is_scholar` 行；49 份 `is_scholar: true` 的卡保留 `attack: 0` 与生命值。
- 100 份效果牌文档与 109 份技能文档删除 `match_text` 行。
- 卡牌与技能属性表由 `scripts/render_docs.py` 重写，删除「学者」「确定性替换」派生行；手写小节不动。
- `cards/README.md`、`skills/README.md` 由渲染脚本重算；删除学者与确定性替换相关说明。

### 2.2 学者定向技能与模板

删除：

- 技能 `aura-scholar-plus-0-plus-2`、`aura-scholar-plus-0-plus-3`、`scholar-growth-twice`、`battlecry-friendly-growth-scholar-twice`；
- 模板 `aura-scholar-health`、`activate-growth-single-scholar`、`on-play-battlecry-growth-single-timesif-scholar`。

新增通用技能：

- `aura-other-plus-0-plus-2`：模板 `aura-all-other-health(health=2)`；
- `aura-other-plus-0-plus-3`：模板 `aura-all-other-health(health=3)`；
- `activate-friendly-growth-twice`：模板 `activate-growth-single(times=2)`，`name: 推演`，`cost: 1`，`uses_per_turn: 1`。

引用改配：

- `kongzi` → `aura-other-plus-0-plus-2`；
- `sagan` → `aura-other-plus-0-plus-3`；
- `gauss`（随从与英雄）→ `activate-friendly-growth-twice`；
- `lianjinshushi` → `battlecry-friendly-growth-once`。

### 2.3 删除 v0.2 比赛模式

- 删除 `docs/versions/v0.2/product/systems/competitive-mode.md`。
- 清理引用：`docs/versions/v0.2/README.md`、`product/README.md`、`open-questions.md`、`product/systems/achievements.md`、`product/systems/ranked-and-matchmaking.md`、`docs/version-roadmap.md`、`docs/versions/v0.1/product/overview/design-overview.md`。
- 删除 v0.1 卡牌与英雄设计说明中的「比赛模式下改为……」分句（`marie-curie`、`galileo` 各两份）。

## 3. 生成与校验

- `scripts/gen_card_data.py`：`card_entry` 不再输出 `is_scholar`；`TIMES_IF_KEYS` 删除 `scholar`，`_json_times_if` 只接受三段的「键:值:次数」。
- `scripts/skill_templates.py`：删除 `scholar` 选择器类型与「学者」展示名；模板参数校验不再接受 `scholar`。
- `scripts/validate_docs.py`：
  - 字段白名单删除 `is_scholar` 与 `match_text`；
  - 删除学者攻击力/生命值分支，改为「`attack == 0` 的随从按同星级生命上限 ×1.3 校验生命值」；
  - 删除 `scholar` 筛选键与 `timesIf=scholar` 语法；
  - 属性表期望行删除「学者」「确定性替换」；
  - 删除「主动技能/效果牌必须填写 match_text」「有 name 必须填写 match_text」「被效果牌引用必须填写 match_text」；
  - `进化` 校验改为「生物卡且不带 `play:machine`」；
  - 无技能随从约束改为「造物工具卡」。
- `scripts/render_docs.py`：卡牌属性表不再输出「学者」行；技能属性表不再输出「确定性替换」行。
- `scripts/gen_text.py`：不再生成 `card.<id>.match_text` 与 `skill.<id>.match_text`。
- `scripts/balance_report.py`：攻血比统计从「非学者随从」改为全部随从；删除学者相关说明与错误分支。
- 重新生成 `server/data/v0.1/cards.json`、`client/generated/cards.json`、`client/generated/skills.json`、`client/text/zh_CN.csv`。

## 4. 规则引擎

- `data.rs`：`Card` 删除 `is_scholar`；加载期不再接受该字段。
- `state.rs`：`MinionInstance` 删除 `is_scholar`；`attack()` 直接返回非负当前值。
- `combat.rs`：`effective_attack_with` 删除学者强制归零。
- `effects/exec.rs`：成长结算删除 `minion.is_scholar` 分支；`timesIf` 删除 `scholar`；筛选事实与 `scholar` 键删除。
- `effects/parse.rs` / `effects/validate.rs` / `effects/mod.rs`：删除 `scholar` 筛选与无值 `timesIf` 解析；`timesIf` 只接受三段格式。
- `effects/evolve.rs`：可进化条件改为「不带 `play:machine`」。
- `hash.rs` / `view.rs`：删除 `is_scholar` 哈希与攻击力投影分支。
- 加载器 `deny_unknown_fields` 会让旧的 `is_scholar` 数据直接报错，因此必须同步重生成数据。

## 5. 客户端

- `client/generated/cards.json` 不再含 `is_scholar`；`CardDB` 与图鉴不读取该字段。
- 删除相关文本 key 与 UI 冒烟断言中的「学者」口径；卡牌详情「玩法」「关联分类」等展示不变。

## 6. 测试与基线

- 所有 `Card` / `MinionInstance` 测试构造删除 `is_scholar`。
- 删除或改写学者专项测试：
  - 攻击力强制 0、成长只加生命、繁衍只加生命、学者筛选、学者入池等断言；
  - 改为断言攻击力 0 的普通随从成长 +1/+1、可被加攻。
- `scripts` 单测更新字段白名单、属性表期望与 `timesIf` 语法。
- `server/crates/sim/tests/golden/seed-1-games-20.json` 按 `sim_determinism` 既有流程重定，并在测试注释记录「移除学者攻击锁定导致成长后攻击力变化」。
- 验证命令：`make data-check`、`make test`、`make lint`、`make client-check`、`make client-smoke`。

## 7. 文档回写

- `docs/versions/v0.1/product/content/card-system.md`、`skill-system.md`、`card-nouns.md`、`balance-analysis.md`、`balance-questions.md`、`taxonomy.md` 删除学者与 `match_text` 规则。
- `docs/current/technical/rule-engine-and-simulator.md`、`docs/current/technical/client.md` 删除 `is_scholar` / `match_text` 字段说明。
- 历史 `docs/versions/v0.1/update/*` 与 `docs/superpowers/*` 作为历史记录不改。
