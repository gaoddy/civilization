# 各玩法卡池扩充与专属机制 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 v0.1 的 13 个玩法补齐至少 12 张随从，并实现 15 个玩法专属机制，使每个玩法都有可辨识的核心循环。

**Architecture:** 先扩展规则引擎的 AST、关键词与战斗钩子，再按玩法分批写卡牌/技能/分类/Token 文档，最后统一生成数据并跑全量验证。15 个机制分 7 个引擎任务落地；106 张卡分 4 批落地，每批独立校验。

**Tech Stack:** Rust（rule-engine、room-service）、Python（文档校验与数据生成）、JSON 协议与生成物、Godot 客户端文本表。

**Spec:**
- 产品规则：[关键词与机制](../../current/product/rules/keywords-and-mechanics.md)
- 技术方案：[规则引擎与模拟器](../../current/technical/rule-engine-and-simulator.md)

## Global Constraints

- 既有 240 张卡的数值、效果、分类与技能不得修改。
- 每个玩法至少 12 张随从；机械 13 张；哺乳/病毒/植物各 +2；随从 309、卡牌 346。
- 新机制关键词与折算值必须同步 `keywords-and-mechanics.md` 与 `economy-and-numbers.md`。
- 新增生物分类叶子必须是「科」；机械继续落在 `automation`。
- 所有随机只用 `Stream::Combat` / `Stream::Effects`，禁止 `thread_rng`。
- 每个任务结束时 `cargo test -p rule-engine` 与相关 Python 测试必须通过。
- 提交信息按主题拆分，不把多个机制混在一个提交。

## Review Focus

- 潜行敌人是唯一可选目标时，战斗必须继续推进，不能死循环。
- 剧毒对护盾抵消的伤害不触发，对反击也生效。
- 减伤不能把传染的每回合掉血也减掉。
- 回手与复苏的死亡结算顺序不能重复触发亡语或复生。
- 吞噬/转化/引爆的目标在结算前离场时，效果必须静默落空，不报错。

---

### Task 1: 关键词与原语地基

**Files:**
- Modify: `server/crates/rule-engine/src/effects/mod.rs`
- Modify: `server/crates/rule-engine/src/effects/validate.rs`
- Modify: `server/crates/rule-engine/src/state.rs`
- Modify: `server/crates/rule-engine/src/hash.rs`
- Modify: `scripts/i18n_terms.py`
- Modify: `scripts/validate_docs.py`
- Modify: `scripts/gen_card_data.py`
- Modify: `scripts/audit_effects.py`
- Test: `server/crates/rule-engine/tests/effects_parse.rs`

**Interfaces:**
- Produces: `Op::{Mimic,Devour,Detonate,Convert,Revive,Recall,Salvage,Latent}`、`Pick::{LowestAttack,Adjacent}`、`Params::{per_attack,per_health,max_count,threshold}`、`MinionInstance::{stealth_broken,revive_used}`。
- Produces: 关键词 `poison`、`armor N`、`stealth` 可被 DSL 解析与校验。

- [ ] **Step 1: 写失败测试**

在 `effects_parse.rs` 增加：

```rust
#[test]
fn parses_new_ops_and_params() {
    let effects = effects_of("Aura|ModifyStats|friendly:all:playstyle=fish|attack=1,health=1,perAttack=1,perHealth=0,maxCount=6");
    assert_eq!(effects[0].params.per_attack, Some(1));
    assert_eq!(effects[0].params.max_count, Some(6));
    let effects = effects_of("OnCombatStart|Detonate|enemy:single|amount=2,threshold=3");
    assert_eq!(effects[0].op, Op::Detonate);
    assert_eq!(effects[0].params.threshold, Some(3));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd server && cargo test -p rule-engine --test effects_parse parses_new_ops_and_params -v`
Expected: FAIL，`Op::Detonate` 与 `per_attack` 不存在。

- [ ] **Step 3: 实现 AST 与 DSL 扩展**

在 `effects/mod.rs` 增加 §2.2 的 `Op`、`Pick`、`Params` 字段；在 `effects/validate.rs` 增加 `op_spec`、`PARAM_KEYS`、`KEYWORD_BASES`；在 `state.rs` 增加 `stealth_broken`、`revive_used` 并在 `MinionInstance::new` 初始化为 `false`；在 `hash.rs` 写入这两个布尔值。

在 `scripts/i18n_terms.py` 的 `KEYWORD_BASES` 增加 `poison`、`armor`、`stealth`；在 `scripts/validate_docs.py` 的 `KEYWORD_RE` / `SKILL_KEYWORD_RE` 增加 `armor \d+`（玩法归属与折算值归 Task 10，本任务不加 `KEYWORD_PLAYSTYLE`）；在 `scripts/gen_card_data.py` 的 `OP_JSON`、`PICK_JSON` 增加映射；在 `scripts/audit_effects.py` 的白名单增加新原语。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd server && cargo test -p rule-engine --test effects_parse -v`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add server/crates/rule-engine/src/effects server/crates/rule-engine/src/state.rs server/crates/rule-engine/src/hash.rs scripts/i18n_terms.py scripts/validate_docs.py scripts/gen_card_data.py scripts/audit_effects.py server/crates/rule-engine/tests/effects_parse.rs
git commit -m "feat(rule-engine): 扩展玩法机制的 AST 与 DSL 地基"
```

### Task 2: 剧毒与减伤

**Files:**
- Modify: `server/crates/rule-engine/src/combat.rs`
- Test: `server/crates/rule-engine/tests/combat_damage.rs`

**Interfaces:**
- Consumes: `poison`、`armor N` 关键词；`combat::keywords` 返回带层数字符串。
- Produces: `combat::has_poison(catalog, minion) -> bool`、`combat::armor_value(catalog, minion) -> i32`。

- [ ] **Step 1: 写失败测试**

在 `combat_damage.rs` 增加：

```rust
#[test]
fn poison_kills_surviving_target_but_not_shielded() {
    // 3/3 剧毒攻击 0/10 白板：目标受 3 点后仍存活，应被剧毒杀死。
    // 带护盾的目标被剧毒随从攻击：护盾抵消伤害，不触发剧毒。
}

#[test]
fn armor_reduces_damage_and_does_not_block_contagion_tick() {
    // 2 点伤害对减伤 1 目标只掉 1。
    // 传染层数的回合开始掉血不受减伤影响。
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd server && cargo test -p rule-engine --test combat_damage -v`
Expected: FAIL，剧毒目标未死亡、减伤未生效。

- [ ] **Step 3: 实现战斗钩子**

在 `combat.rs` 增加 `POISON`、`ARMOR` 常量与两个 helper。`resolve_attack` 在主伤害与反击后各判断一次剧毒：仅当 `damage > 0`、目标仍存活且未被护盾抵消时，施加致死伤害并记录 `CombatAction::Poisoned`。`apply_damage` 在扣除前减去目标 `armor` 之和；传染的回合开始掉血路径直接扣 `stats.health.current`，不经过 `apply_damage` 的减伤。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd server && cargo test -p rule-engine --test combat_damage -v`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add server/crates/rule-engine/src/combat.rs server/crates/rule-engine/tests/combat_damage.rs
git commit -m "feat(rule-engine): 剧毒与减伤战斗机制"
```

### Task 3: 潜行与俯冲

**Files:**
- Modify: `server/crates/rule-engine/src/combat.rs`
- Modify: `server/crates/rule-engine/src/effects/exec.rs`
- Modify: `server/crates/rule-engine/src/effects/aura.rs`
- Modify: `server/crates/rule-engine/src/view.rs`
- Modify: `server/crates/room-service/src/view.rs`
- Modify: `protocol/schema/v1/view.schema.json`
- Test: `server/crates/rule-engine/tests/combat.rs`
- Test: `server/crates/rule-engine/tests/view.rs`

**Interfaces:**
- Consumes: `Pick::LowestAttack`、`MinionInstance::stealth_broken`。
- Produces: `combat::has_stealth(catalog, minion) -> bool`；`MinionView.stealth: bool`。

- [ ] **Step 1: 写失败测试**

在 `combat.rs` 增加：

```rust
#[test]
fn stealth_cannot_be_targeted_by_single_effects_but_aoe_hits() {
    // 敌方单体 3 点伤害不会选中潜行随从；全体 2 点伤害会命中。
}

#[test]
fn all_enemies_stealthed_still_attack_leftmost_and_break_stealth() {
    // 防守方只有潜行随从时，攻击落在最左侧并打破潜行，战斗必须推进。
}

#[test]
fn lowest_attack_pick_breaks_ties_leftmost() {
    // 敌方 2 攻与 2 攻并列，俯冲命中棋盘最左侧。
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd server && cargo test -p rule-engine --test combat -v`
Expected: FAIL。

- [ ] **Step 3: 实现潜行与俯冲**

在 `combat.rs` 增加 `STEALTH` 常量与 `has_stealth`；`pick_target` 的候选池过滤仍有潜行的目标，过滤后为空时退回完整候选池取最左侧；`filter_matches` 对敌方单目标跳过潜行；随从造成伤害后置 `stealth_broken = true`。在 `effects/exec.rs::apply_pick` 与 `aura.rs::pick_targets` 增加 `Pick::LowestAttack`：按 `effective_attack` 取最低，并列取最左。

在 `view.rs` 的 `MinionView` 增加 `stealth` 字段；`room-service/src/view.rs` 映射；`view.schema.json` 与样例同步。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd server && cargo test -p rule-engine --test combat --test view -v`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add server/crates/rule-engine/src/combat.rs server/crates/rule-engine/src/effects server/crates/rule-engine/src/view.rs server/crates/room-service/src/view.rs protocol/schema/v1/view.schema.json server/crates/rule-engine/tests/combat.rs server/crates/rule-engine/tests/view.rs
git commit -m "feat(rule-engine): 潜行与俯冲机制"
```

### Task 4: 群聚与相邻

**Files:**
- Modify: `server/crates/rule-engine/src/effects/aura.rs`
- Modify: `server/crates/rule-engine/src/effects/validate.rs`
- Test: `server/crates/rule-engine/tests/auras.rs`

**Interfaces:**
- Consumes: `Params::{per_attack,per_health,max_count}`、`Pick::Adjacent`。
- Produces: 光环按友方同玩法数量递增；`Adjacent` 只作用于来源左右相邻随从。

- [ ] **Step 1: 写失败测试**

在 `auras.rs` 增加：

```rust
#[test]
fn school_aura_scales_with_other_same_playstyle_count() {
    // 1 个鱼群首领 + 2 个其他鱼纲：每个其他鱼获得基础 +1/+1 与 2 点额外攻击。
}

#[test]
fn adjacency_aura_only_buffs_left_and_right_neighbors() {
    // 3 个哺乳随从，中间为携带者：只有左右两个获得 +2/+2，携带者不加。
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd server && cargo test -p rule-engine --test auras -v`
Expected: FAIL。

- [ ] **Step 3: 实现光环扩展**

在 `aura.rs` 计算来源玩家的匹配随从数量（排除来源自身，`max_count` 封顶），把 `per_attack`/`per_health` 乘入 `AuraEffect`；为 `Pick::Adjacent` 在 `pick_targets` 增加来源下标参数（`source_index: Option<usize>`），取左右相邻的存活随从，并同步更新 Task 3 引入的 `LowestAttack` 分支及其它调用点。在 `validate.rs` 限制 `perAttack`/`perHealth`/`maxCount` 只用于光环 `ModifyStats`，`Adjacent` 只用于光环。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd server && cargo test -p rule-engine --test auras -v`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add server/crates/rule-engine/src/effects/aura.rs server/crates/rule-engine/src/effects/validate.rs server/crates/rule-engine/tests/auras.rs
git commit -m "feat(rule-engine): 群聚与相邻光环"
```

### Task 5: 拟态与吞噬

**Files:**
- Modify: `server/crates/rule-engine/src/effects/exec.rs`
- Modify: `server/crates/rule-engine/src/effects/validate.rs`
- Test: `server/crates/rule-engine/tests/effects_exec.rs`

**Interfaces:**
- Consumes: `Op::Mimic`、`Op::Devour`、`Params::{duration,value}`。
- Produces: `Engine::apply_mimic(...)`、`Engine::apply_devour(...)`；战斗内记录 `CombatAction::Mimicked` / `Devoured`。

- [ ] **Step 1: 写失败测试**

在 `effects_exec.rs` 增加：

```rust
#[test]
fn mimic_copies_current_stats_and_keywords_not_growth() {
    // 目标 5/5 + 成长 + 嘲讽；拟态后来源为 5/5 + 嘲讽，不含成长。
}

#[test]
fn devour_only_accepts_target_at_or_below_limit() {
    // 攻击力 3 的目标可被 value=3 吞噬；攻击力 4 的目标落空。
}

#[test]
fn devour_and_mimic_are_silent_when_target_is_missing() {
    // 选择器没有合法目标时，结算不报错、来源属性不变。
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd server && cargo test -p rule-engine --test effects_exec -v`
Expected: FAIL。

- [ ] **Step 3: 实现两个原语**

在 `exec.rs::apply_op` 增加 `Op::Mimic` 与 `Op::Devour` 分支；实现 `apply_mimic` 读取目标当前 `attack()`/`health()` 与 `keywords`，写入来源 `ModifyStats` 与 `granted`；实现 `apply_devour` 校验攻击力上限、施加致死伤害、把目标卡面基础值加到来源 `permanent`。战斗内分支记录对应 `CombatAction`。招募阶段敌方目标沿用既有延迟结算路径。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd server && cargo test -p rule-engine --test effects_exec -v`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add server/crates/rule-engine/src/effects/exec.rs server/crates/rule-engine/src/effects/validate.rs server/crates/rule-engine/tests/effects_exec.rs
git commit -m "feat(rule-engine): 拟态与吞噬原语"
```

### Task 6: 复苏与回手

**Files:**
- Modify: `server/crates/rule-engine/src/combat.rs`
- Modify: `server/crates/rule-engine/src/effects/exec.rs`
- Modify: `server/crates/rule-engine/src/settle.rs`
- Modify: `server/crates/rule-engine/src/state.rs`
- Test: `server/crates/rule-engine/tests/death_effects.rs`

**Interfaces:**
- Consumes: `Op::Revive`、`Op::Recall`、`MinionInstance::revive_used`、`CombatLog::hand_minions`。
- Produces: `CombatHandMinion`；战斗结束回写手牌。

- [ ] **Step 1: 写失败测试**

在 `death_effects.rs` 增加：

```rust
#[test]
fn revive_restores_with_bonus_and_only_once() {
    // 复苏 +2/+2 的 3/2 死亡后以 5/4 复活；再次死亡不再复活。
}

#[test]
fn recall_returns_minion_to_hand_with_growth() {
    // 带 2 次成长的随从死亡后进入 hand_minions，状态保留成长。
}

#[test]
fn recall_is_dropped_when_hand_is_full() {
    // 手牌已满时不记录回手。
}

#[test]
fn revive_does_not_replay_deathrattle_within_one_death_event() {
    // 带亡语与复苏的随从死亡一次：亡语只触发一次，复苏也只结算一次。
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd server && cargo test -p rule-engine --test death_effects -v`
Expected: FAIL。

- [ ] **Step 3: 实现死亡结算**

在战斗死亡结算中，复生之后检查 `Op::Revive` 与 `Op::Recall`：复苏按 `attack`/`health` 增加永久值并 `revive_to`，置 `revive_used`/`reborn_used`；回手克隆到 `hand_minions` 并检查手牌容量。`settle.rs` 把 `hand_minions` 写入持久手牌。`CombatAction` 记录 `Revived` / `Recalled`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd server && cargo test -p rule-engine --test death_effects -v`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add server/crates/rule-engine/src/combat.rs server/crates/rule-engine/src/effects/exec.rs server/crates/rule-engine/src/settle.rs server/crates/rule-engine/src/state.rs server/crates/rule-engine/tests/death_effects.rs
git commit -m "feat(rule-engine): 复苏与回手机制"
```

### Task 7: 引爆与转化

**Files:**
- Modify: `server/crates/rule-engine/src/effects/exec.rs`
- Modify: `server/crates/rule-engine/src/combat.rs`
- Modify: `server/crates/rule-engine/src/effects/validate.rs`
- Test: `server/crates/rule-engine/tests/kills.rs`

**Interfaces:**
- Consumes: `Op::Detonate`、`Op::Convert`、`Params::{amount,threshold,token,count}`。
- Produces: 传染层消耗、阈值消灭、召唤归属。

- [ ] **Step 1: 写失败测试**

在 `kills.rs` 增加：

```rust
#[test]
fn detonate_consumes_layers_and_deals_per_layer_damage() {
    // 3 层传染 + 每层 2 点 = 6 点伤害，层数清零。
}

#[test]
fn detonate_threshold_destroys_even_through_shield() {
    // 3 层、阈值 3：目标有护盾也直接被消灭。
}

#[test]
fn convert_requires_threshold_and_summons_token_for_source() {
    // 2 层不转化；3 层转化并召唤 2/2 细菌到来源方。
}

#[test]
fn detonate_and_convert_are_silent_when_target_is_missing() {
    // 转身或目标已离场时，不消耗层数、不召唤、不报错。
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd server && cargo test -p rule-engine --test kills -v`
Expected: FAIL。

- [ ] **Step 3: 实现引爆与转化**

在 `exec.rs` 增加两个分支：`Detonate` 读取并清零 `contagion`，按层数造成伤害，达到 `threshold` 阈值直接致死；`Convert` 在层数达到 `threshold` 时致死目标并调用召唤路径生成 Token。护盾只影响伤害，不影响阈值消灭。记录 `CombatAction::Detonated` / `Converted`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd server && cargo test -p rule-engine --test kills -v`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add server/crates/rule-engine/src/effects/exec.rs server/crates/rule-engine/src/combat.rs server/crates/rule-engine/src/effects/validate.rs server/crates/rule-engine/tests/kills.rs
git commit -m "feat(rule-engine): 引爆与转化机制"
```

### Task 8: 潜伏与零件回收

**Files:**
- Modify: `server/crates/rule-engine/src/effects/exec.rs`
- Modify: `server/crates/rule-engine/src/combat.rs`
- Modify: `server/crates/rule-engine/src/effects/validate.rs`
- Test: `server/crates/rule-engine/tests/turn_effects.rs`

**Interfaces:**
- Consumes: `Op::Latent`、`Op::Salvage`、`Params::{amount,threshold,count,pool}`。
- Produces: `latent_used` 标记；`Salvage` 随机机械牌写入 `hand_copies`。

- [ ] **Step 1: 写失败测试**

在 `turn_effects.rs` 增加：

```rust
#[test]
fn latent_adds_attack_per_contagion_layer_and_bursts_at_threshold() {
    // 2 层获得 +2 攻击；3 层爆发对敌方全体造成 3 点并清零。
}

#[test]
fn salvage_adds_only_machine_minions_to_hand() {
    // 随机结果全部是 machine 玩法随从；手牌满时不加入。
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd server && cargo test -p rule-engine --test turn_effects -v`
Expected: FAIL。

- [ ] **Step 3: 实现潜伏与零件回收**

`Latent`：按 `contagion` 层数给来源 `add_permanent` 攻击力；层数达到 `threshold` 时清零层数、对敌方全体造成 `count` 点伤害，并置 `latent_used`。`Salvage`：从目录筛选机械随从（排除 Token/英雄/奖励牌），用 `Stream::Combat` 随机取 `count` 张，写入 `hand_copies`；手牌满时不写。记录对应 `CombatAction`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd server && cargo test -p rule-engine --test turn_effects -v`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add server/crates/rule-engine/src/effects/exec.rs server/crates/rule-engine/src/combat.rs server/crates/rule-engine/src/effects/validate.rs server/crates/rule-engine/tests/turn_effects.rs
git commit -m "feat(rule-engine): 潜伏与零件回收机制"
```

### Task 9: 分类树与 Token

**Files:**
- Modify: `docs/current/product/content/taxonomy.md`
- Modify: `docs/current/product/content/tokens/README.md`
- Modify: `server/data/v0.1/taxonomy.json`（生成物）
- Modify: `client/generated/taxonomy.json`（生成物）
- Test: `python3 scripts/validate_docs.py`

**Interfaces:**
- Consumes: 产品方案 §5 的分类与 Token 清单。
- Produces: 所有新增卡牌的 `taxon` 叶子与召唤 Token id。

- [ ] **Step 1: 补充分类节点**

按产品方案 §5.1 在 `taxonomy.md` 增加鱼纲、真菌界、原生生物界、鸟纲、爬行纲、昆虫纲、头足纲、水螅纲、细菌域的真实科级节点与必要的目/纲。每个生物叶子必须是「科」；机械不新增节点。

- [ ] **Step 2: 补充 Token**

在 `tokens/README.md` 注册产品方案 §5.2 的 Token，写清攻击力、生命值、关键词、玩法、科学分类与生成来源。

- [ ] **Step 3: 运行校验**

Run: `python3 scripts/validate_docs.py`
Expected: 分类树与 Token 校验通过；新卡尚未加入，数量期望暂时不变。

- [ ] **Step 4: 提交**

```bash
git add docs/current/product/content/taxonomy.md docs/current/product/content/tokens/README.md server/data/v0.1/taxonomy.json client/generated/taxonomy.json
git commit -m "data(content): 扩充玩法卡池的科学分类与 Token"
```

### Task 10: 校验、生成与计数

**Files:**
- Modify: `scripts/validate_docs.py`
- Modify: `scripts/gen_card_data.py`
- Modify: `scripts/gen_client_data.py`
- Modify: `scripts/gen_text.py`
- Modify: `scripts/audit_effects.py`
- Test: `tests/test_validate_effects.py`

**Interfaces:**
- Consumes: 新关键词、原语、参数与玩法归属。
- Produces: `validate_docs.py` 能校验新卡；生成器能输出新字段；测试期望更新为 346/309。

- [ ] **Step 1: 写失败测试**

在 `tests/test_validate_effects.py` 增加一个只依赖合成 YAML 的用例：含 `armor 2` 与 `Detonate` 的技能通过校验，`poison` 出现在 `mammal` 时被拒绝。该用例不依赖真实卡池数量，因此不会因新卡尚未加入而失败。

- [ ] **Step 2: 运行测试确认失败**

Run: `python3 -m unittest tests.test_validate_effects -v`
Expected: FAIL，`armor` 与 `Detonate` 尚未进入白名单。

- [ ] **Step 3: 实现校验与生成**

在 `validate_docs.py` 增加新关键词白名单、玩法归属与折算值；在 `gen_card_data.py` 增加 `OP_JSON`/`PICK_JSON`/参数映射；在 `gen_client_data.py` 与 `gen_text.py` 确认新字段与文本自动汇总；在 `audit_effects.py` 增加新原语覆盖。技能数与效果段数改为从生成结果计算，不在测试里写死。`EXPECT_ERA`/`EXPECT_LEVEL` 与总数期望**不在本任务改动**，留给 Task 15 与实际数据一起更新。

- [ ] **Step 4: 运行测试确认通过**

Run: `python3 -m unittest tests.test_validate_effects -v && python3 scripts/validate_docs.py`
Expected: PASS；现有 240 张卡仍全部通过。

- [ ] **Step 5: 提交**

```bash
git add scripts tests
git commit -m "build(data): 接入玩法机制的校验与生成规则"
```

### Task 11: 鱼纲 / 真菌界 / 原生生物界卡牌

**Files:**
- Create: `docs/current/product/content/cards/<id>.md`（36 份）
- Create: `docs/current/product/content/skills/<id>.md`（机制共享技能）
- Modify: `docs/current/product/content/cards/README.md`
- Modify: `docs/current/product/content/skills/README.md`
- Test: `python3 scripts/validate_docs.py`

**Interfaces:**
- Consumes: 产品方案 §4.1–§4.3 的卡表；Task 1–10 的机制与分类。
- Produces: 三个玩法各 12 张卡；`群聚`、`吞噬`、`分裂`、`分解` 的首批使用者。

- [ ] **Step 1: 创建共享技能**

为 `群聚`、`吞噬`、`分裂`、`分解`、`孢子召唤` 建立技能文档，DSL 与文本按技术方案 §3；技能 id 按语义命名（如 `aura-school-fish-1`、`devour-1`、`deathrattle-summon-fish-fry-1`）。

- [ ] **Step 2: 创建 36 张卡牌文档**

按产品方案 §4.1–§4.3 逐张创建卡牌文档：front matter、属性表、效果、出处、设计说明齐全；`playstyle`、`taxon`、`subtype`、`level`、`era`、`attack`、`health`、`skills` 与卡表一致。

- [ ] **Step 3: 更新索引**

在 `cards/README.md` 增加 36 行；在 `skills/README.md` 增加新技能行。

- [ ] **Step 4: 运行校验**

Run: `python3 scripts/validate_docs.py`
Expected: 36 张新卡通过字段、分类、关键词、DSL、出处与属性表校验。

- [ ] **Step 5: 提交**

```bash
git add docs/current/product/content/cards docs/current/product/content/skills
git commit -m "content(cards): 补充鱼纲、真菌界与原生生物界卡牌"
```

### Task 12: 头足纲 / 水螅纲 / 细菌域卡牌

**Files:**
- Create: `docs/current/product/content/cards/<id>.md`（33 份）
- Create: `docs/current/product/content/skills/<id>.md`（机制共享技能）
- Modify: `docs/current/product/content/cards/README.md`
- Modify: `docs/current/product/content/skills/README.md`
- Test: `python3 scripts/validate_docs.py`

**Interfaces:**
- Consumes: 产品方案 §4.7–§4.9；Task 1–10。
- Produces: 三个玩法各 12 张卡；`潜行`、`拟态`、`复苏`、`回手`、`引爆`、`转化` 的首批使用者。

- [ ] **Step 1: 创建共享技能**

为 `潜行`、`拟态`、`复苏`、`回手`、`引爆`、`转化` 建立技能文档，DSL 与文本按技术方案 §3。

- [ ] **Step 2: 创建 33 张卡牌文档**

按产品方案 §4.7–§4.9 逐张创建，确保 `poison`/`armor`/`stealth` 只出现在允许的玩法上。

- [ ] **Step 3: 更新索引**

在 `cards/README.md` 与 `skills/README.md` 增加对应行。

- [ ] **Step 4: 运行校验**

Run: `python3 scripts/validate_docs.py`
Expected: 新卡与既有卡全部通过。

- [ ] **Step 5: 提交**

```bash
git add docs/current/product/content/cards docs/current/product/content/skills
git commit -m "content(cards): 补充头足纲、水螅纲与细菌域卡牌"
```

### Task 13: 鸟纲 / 爬行纲 / 昆虫纲 / 机械卡牌

**Files:**
- Create: `docs/current/product/content/cards/<id>.md`（31 份）
- Create: `docs/current/product/content/skills/<id>.md`（机制共享技能）
- Modify: `docs/current/product/content/cards/README.md`
- Modify: `docs/current/product/content/skills/README.md`
- Test: `python3 scripts/validate_docs.py`

**Interfaces:**
- Consumes: 产品方案 §4.4–§4.6、§4.10；Task 1–10。
- Produces: 鸟/爬行/昆虫各 12 张、机械 13 张；`剧毒`、`减伤`、`俯冲`、`零件回收` 的首批使用者。

- [ ] **Step 1: 创建共享技能**

为 `剧毒`、`减伤 1/2`、`俯冲`、`零件回收`、`分裂` 的后续变体建立技能文档。

- [ ] **Step 2: 创建 31 张卡牌文档**

按产品方案 §4.4–§4.6 与 §4.10 逐张创建；机械卡的 `taxon` 必须为 `automation`。

- [ ] **Step 3: 更新索引**

在 `cards/README.md` 与 `skills/README.md` 增加对应行。

- [ ] **Step 4: 运行校验**

Run: `python3 scripts/validate_docs.py`
Expected: 通过。

- [ ] **Step 5: 提交**

```bash
git add docs/current/product/content/cards docs/current/product/content/skills
git commit -m "content(cards): 补充鸟纲、爬行纲、昆虫纲与机械卡牌"
```

### Task 14: 哺乳纲 / 病毒 / 植物界补强

**Files:**
- Create: `docs/current/product/content/cards/<id>.md`（6 份）
- Create: `docs/current/product/content/skills/<id>.md`（机制共享技能）
- Modify: `docs/current/product/content/cards/README.md`
- Modify: `docs/current/product/content/skills/README.md`
- Test: `python3 scripts/validate_docs.py`

**Interfaces:**
- Consumes: 产品方案 §4.11–§4.13。
- Produces: 哺乳 63、病毒 24、植物 24；`群聚`、`潜伏` 的成熟玩法使用者。

- [ ] **Step 1: 创建 6 张卡牌文档**

按产品方案 §4.11–§4.13 创建尼安德特人、猛犸象、诺如病毒、登革病毒、兰花、银杏。

- [ ] **Step 2: 创建共享技能**

为 `潜伏 3`、`群聚` 的哺乳变体建立技能文档；复用既有战吼/播种/繁衍技能。

- [ ] **Step 3: 更新索引**

在 `cards/README.md` 与 `skills/README.md` 增加对应行。

- [ ] **Step 4: 运行校验**

Run: `python3 scripts/validate_docs.py`
Expected: 13 个玩法全部达标，总数 309/346。

- [ ] **Step 5: 提交**

```bash
git add docs/current/product/content/cards docs/current/product/content/skills
git commit -m "content(cards): 补强哺乳纲、病毒与植物界卡池"
```

### Task 15: 数据生成与文档回写

**Files:**
- Modify: `server/data/v0.1/cards.json`、`skills.json`、`tokens.json`、`taxonomy.json`（生成物）
- Modify: `client/generated/cards.json`、`skills.json`、`tokens.json`、`taxonomy.json`（生成物）
- Modify: `client/text/zh_CN.csv`（生成物）
- Modify: `docs/current/product/content/card-system.md`
- Modify: `docs/current/product/content/skill-system.md`
- Modify: `docs/current/product/rules/keywords-and-mechanics.md`
- Modify: `docs/current/product/systems/economy-and-numbers.md`
- Modify: `docs/current/product/content/cards/README.md`
- Modify: `docs/current/product/content/skills/README.md`
- Modify: `tests/test_gen_data.py`（卡牌 346、随从 309、技能数与效果段数按实际）
- Modify: `tests/test_audit_effects.py`（不再写死 131/122）
- Test: `make data-check`

**Interfaces:**
- Consumes: Task 11–14 的卡牌与技能文档。
- Produces: 服务端/客户端数据、文本表与产品基线文档一致。

- [ ] **Step 1: 生成数据**

Run: `make data`
Expected: 生成 `server/data/v0.1/*`、`client/generated/*`、`client/text/zh_CN.csv`，无异常。

- [ ] **Step 2: 更新校验期望与测试断言**

按实际生成结果更新 `scripts/validate_docs.py` 的 `EXPECT_ERA`、`EXPECT_LEVEL` 与总数期望；把 `tests/test_gen_data.py` 的卡牌/随从断言改为 `346`/`309`，技能数按实际；把 `tests/test_audit_effects.py` 的技能数与效果段数改为从生成结果计算。`tests/test_validate_hero_cards.py` 的英雄数保持 32。

- [ ] **Step 3: 更新产品基线**

把 §2 的机制、关键词、折算值、数量与新卡说明回写到 `keywords-and-mechanics.md`、`economy-and-numbers.md`、`card-system.md`、`skill-system.md`；更新两份 README 索引与统计。

- [ ] **Step 4: 校验生成物与统计**

Run: `make data-check && python3 -m unittest tests.test_gen_data tests.test_audit_effects tests.test_validate_hero_cards -v`
Expected: 退出码 0，无漂移；卡牌 346、随从 309、英雄 32。

- [ ] **Step 5: 提交**

```bash
git add server/data client/generated client/text docs/current/product scripts/validate_docs.py tests
git commit -m "data(content): 生成玩法扩充后的服务端与客户端数据"
```

### Task 16: 全量验证与状态哈希

**Files:**
- Modify: `server/crates/rule-engine/tests/combat.rs`（金标准哈希）
- Modify: `server/crates/sim/tests/golden/seed-1-games-20.json`（如模拟结果变化）
- Test: 全量命令

**Interfaces:**
- Consumes: 全部机制与卡牌。
- Produces: 全量验证通过，状态哈希与金标准一致。

- [ ] **Step 1: 运行全量验证**

Run: `make m0-check`
Expected: `data-check`、`cargo test --workspace`、`cargo fmt --check`、`cargo clippy`、`m0-determinism` 全部退出 0。

- [ ] **Step 2: 更新金标准**

若 `combat_state_hash_matches_golden_baseline` 或 `sim` 金标准因新卡池变化失败，按新数据重算并更新期望值；不得跳过或放宽断言。更新后在测试旁写明变更原因。

- [ ] **Step 3: 客户端与协议验证**

Run: `make protocol-check && make protocol-rs-check && make client-check && make client-smoke`
Expected: 全部退出 0。

- [ ] **Step 4: 生成端到端验证**

Run: `make m1b-e2e`
Expected: 退出 0；8 人对局能使用新卡完成。

- [ ] **Step 5: 提交**

```bash
git add server/crates/rule-engine/tests server/crates/sim/tests tests docs
git commit -m "test(rule-engine): 玩法扩充后的全量验证与金标准"
```

---

## Self-Review

- **Spec coverage:** 产品方案 §3 的 15 个机制分别落在 Task 2–8；§4 的 106 张卡落在 Task 11–14；§5 的分类与 Token 落在 Task 9；§7 的验收标准落在 Task 15–16 的验证矩阵。
- **Step scan:** 每个任务都有失败测试、实现、通过测试与提交；卡牌任务以 `validate_docs.py` 作为可执行验收。
- **Type consistency:** `Params` 字段、`Op` 名称、`Pick::LowestAttack`、`MinionInstance` 字段与 `CombatAction` 在各任务中保持一致。
- **Review Focus:** 潜行死循环、剧毒护盾、减伤与传染、回手/复苏顺序、目标离场落空均已写入对应任务的测试步骤。
