# 各玩法卡池扩充与专属机制 · 技术方案

> 状态：已确认
> 产品规则：[product.md](product.md)

## 1. 改动边界

| 端 | 范围 |
| --- | --- |
| 规则引擎 | 新增 3 个关键词、7 个效果原语、1 个选择器与若干参数；战斗伤害、死亡结算、光环与目标选择接入新机制 |
| 数据生成 | `gen_card_data.py`、`gen_client_data.py`、`gen_text.py` 识别新关键词、原语与参数 |
| 文档校验 | `validate_docs.py` 增加关键词白名单、玩法归属、折算值与数量期望；`audit_effects.py` 覆盖新原语 |
| 协议与视图 | 仅新增潜行状态字段；其余机制复用既有技能/关键词展示 |
| 房间服务 | 战斗日志映射新动作；回手牌写入持久手牌 |
| 客户端 | 新关键词图标与卡面文本；潜行状态显示；不需要新页面 |
| 数据内容 | 106 张随从、约 60 个分类节点、十余个 Token、配套技能文档 |

不涉及：既有 240 张卡的数值与效果、牌池份数、商店概率、金币曲线、英雄卡数量、效果牌数量。

## 2. 数据结构

### 2.1 关键词

`server/crates/rule-engine/src/effects/validate.rs` 与 `scripts/i18n_terms.py` 的 `KEYWORD_BASES` 增加：

| 关键词 | 数据 id | 携带层数 | 玩法归属 |
| --- | --- | --- | --- |
| 剧毒 | `poison` | 否 | `reptile`、`cephalopod` |
| 减伤 | `armor N` | 是 | `reptile`、`bacteria`、`cnidarian` |
| 潜行 | `stealth` | 否 | `cephalopod` |

`armor N` 复用 `breed N` / `assemble N` 的「id + 空格 + 层数」写法；`Skill.keyword` 与 `KeywordSpec` 都按字符串保存，`combat::keywords` 返回带层数的字符串。新增常量：

```rust
const POISON: &str = "poison";
const ARMOR: &str = "armor";
const STEALTH: &str = "stealth";
```

`scripts/validate_docs.py` 的 `KEYWORD_RE` / `SKILL_KEYWORD_RE` 增加 `armor \d+`；`keyword_cost` 增加：`poison` 3、`armor N` 为 `2 + 2*(N-1)`、`stealth` 3。

### 2.2 效果原语

`server/crates/rule-engine/src/effects/mod.rs` 的 `Op` 增加：

```rust
Mimic,
Devour,
Detonate,
Convert,
Revive,
Recall,
Salvage,
Latent,
```

`Pick` 增加 `LowestAttack`（仅用于 `DealDamage` 的敌方选择器）与 `Adjacent`（仅用于光环，DSL 写作 `adjacent`）；既有 `LeftmostAdjacent` 保持不变。

`Params` 增加：

| 字段 | DSL 键 | 用途 |
| --- | --- | --- |
| `per_attack: Option<i32>` | `perAttack` | 群聚：每个其他同玩法随从额外攻击力 |
| `per_health: Option<i32>` | `perHealth` | 群聚：每个其他同玩法随从额外生命值 |
| `max_count: Option<i32>` | `maxCount` | 群聚：最多计算的随从数 |
| `threshold: Option<i32>` | `threshold` | 引爆/转化/潜伏的层数阈值 |

`op_spec` 增加：

```rust
Op::Mimic => (&["duration"], &["statsOnly"], &[]),
Op::Devour => (&["value"], &["duration", "when"], &[]),
Op::Detonate => (&["amount"], &["threshold", "when"], &[]),
Op::Convert => (&["threshold"], &["token", "card", "count", "when"], &[&["token", "card"]]),
Op::Revive => (&["attack", "health"], &[], &[]),
Op::Recall => (&["to"], &[], &[]),
Op::Salvage => (&["pool", "count"], &[], &[]),
Op::Latent => (&["amount", "count"], &["threshold", "when"], &[]),
```

`PARAM_KEYS` 增加 `perAttack`、`perHealth`、`maxCount`、`threshold`。

### 2.3 随从状态

`MinionInstance` 增加：

```rust
/// 潜行是否已被打破；没有潜行关键词时无意义。
pub stealth_broken: bool,
/// 复苏是否已使用；与 reborn_used 独立。
pub revive_used: bool,
/// 潜伏爆发是否已用过；每局最多一次。
pub latent_used: bool,
```

`MinionInstance::new` 把三个字段初始化为 `false`；`hash.rs` 把三个字段写入状态哈希；复制、三连、复生、回手克隆时原样保留（`latent_used` 随实例保留，重打出或回手后不重置）。

### 2.4 战斗日志

`CombatAction` 增加：

```rust
Poisoned { target: InstanceId },
Detonated { target: InstanceId, layers: i32, damage: i32 },
Converted { target: InstanceId, token: String },
Revived { minion: InstanceId },
Recalled { minion: InstanceId },
Salvaged { seat: Seat, card_id: String },
Mimicked { source: InstanceId, target: InstanceId },
Devoured { source: InstanceId, target: InstanceId },
```

俯冲不新增独立动作：伤害与击杀复用通用伤害动作（`Damage` / `Death`），日志中只通过来源随从区分。

`CombatLog` 增加：

```rust
pub hand_minions: Vec<CombatHandMinion>,
```

`CombatHandMinion` 记录手牌牌面归属的战斗方：

```rust
pub struct CombatHandMinion {
    pub side: Seat,
    pub minion: MinionInstance,
}
```

回手牌在战斗结束后由结算层写入持久手牌；手牌已满时不记录。

### 2.5 视图

`server/crates/rule-engine/src/view.rs` 的 `MinionView` 增加 `stealth: bool`，含义为「当前仍处于潜行」：有关键词且 `stealth_broken == false`。`protocol/schema/v1/view.schema.json` 与样例同步；房间服务 `view.rs` 映射该字段。其余机制只通过技能文本与关键词展示，不新增视图字段。

## 3. 机制实现

### 3.1 群聚

- 数据：`Aura | ModifyStats | friendly:all:playstyle=<玩法>`，参数 `attack`/`health` 为基础加成，`perAttack`/`perHealth` 为每个其他同玩法随从的额外加成，`maxCount` 为计数上限（缺省 6）。
- 引擎：`effects/aura.rs::aura_targets` 已能取到目标集合；在 `accumulate` 前按来源玩家的棋盘统计「满足筛选且不是来源自身」的数量，把 `count.min(max_count)` 乘到 `perAttack`/`perHealth` 后并入。
- 口径：只统计友方棋盘，不统计英雄卡；金色不翻倍；光环全量重算，不消耗随机流。
- 校验：`perAttack`/`perHealth`/`maxCount` 只允许出现在 `Aura` / `ConditionalAura` 的 `ModifyStats` 上。

### 3.2 相邻

- 数据：`Aura | ModifyStats | friendly:all:...`，`Pick::Adjacent` 只作为光环选择器；DSL 写 `friendly:adjacent`。
- 引擎：`Pick::Adjacent` 加入 `SELECTOR_PICKS`；`aura.rs::pick_targets` 需要来源实例的棋盘下标，取 `index-1` 与 `index+1` 的存活随从；没有来源实例（英雄卡）时返回空。
- 校验：`Adjacent` 只能用于光环；非光环效果使用时报错。
- 口径：最多 2 个目标；携带者自身不获得；金色不翻倍。

### 3.3 剧毒

- 引擎：`combat.rs::resolve_attack` 在主伤害与反击伤害结算后各判断一次：
  - 攻击者带 `poison`、目标未被护盾抵消、目标在伤害后仍存活 → 对目标施加一次致死伤害（`apply_damage(..., i32::MAX, true, DamageSource::Minion(attacker))`），并记录 `CombatAction::Poisoned`。
  - 防守方带 `poison`、反击伤害后攻击者仍存活 → 对攻击者同理。
- 口径：护盾抵消的伤害为 0，不触发；剧毒不额外触发传染；召唤物同样适用；学者攻击力为 0，不能携带。
- 测试：护盾抵挡、主伤害剧毒、反击剧毒、双方剧毒、剧毒击杀触发亡语。

### 3.4 减伤

- 引擎：`combat.rs::apply_damage` 在写入伤害前计算目标当前 `armor N` 之和，`amount = (amount - armor).max(0)`；`can_kill=false` 的夹取在减伤之后执行。
- 口径：护盾由调用方先处理；减伤不作用于传染的每回合掉血（该路径走 `deal_damage`，需要显式跳过减伤，或在传染结算处直接扣血）；多个 `armor N` 叠加。
- 测试：护盾先抵消、减伤后仍可致死、减伤把伤害降到 0、传染掉血不受减伤。

### 3.5 潜行

- 引擎：新增 `combat::has_stealth(catalog, minion) -> bool`，等价于 `keywords` 含 `stealth` 且 `!stealth_broken`。
- 目标选择：`pick_target` 的候选池过滤掉仍有潜行的防守方随从；若过滤后为空，退回完整候选池并取最左侧（保证战斗推进），该次攻击会打破潜行。
- 效果选择：`filter_matches` 的敌方单目标筛选跳过仍有潜行的随从；全体伤害不跳过。
- 破隐：随从造成伤害（攻击、反击、俯冲、效果伤害且来源是该随从）后 `stealth_broken = true`。
- 视图：`stealth` 字段按 `has_stealth` 输出。
- 测试：不能被单体效果选中、全体伤害可命中、攻击后破隐、所有敌人潜行时攻击最左侧并破隐。

### 3.6 俯冲

- 数据：`OnCombatStart | DealDamage | enemy:lowestAttack | amount=<攻击力>`；`amount` 用卡牌攻击力写死，不使用动态值。
- 引擎：`Pick::LowestAttack` 在 `apply_pick` / `pick_targets` 中取攻击力最低者，并列取棋盘最左侧；参与筛选。
- 口径：伤害来源为携带者，可触发剧毒与击杀；护盾可抵消；`amount` 按卡面基础攻击力写死，攻击被增益后俯冲伤害不变。
- 测试：最低攻击选择、并列取最左、护盾抵消、击杀触发亡语。

### 3.7 分裂

- 数据：沿用 `OnDeath | Summon | self | token=<token>,count=N`。
- 引擎：不需要改动；Token 注册表新增条目。
- 测试：死亡召唤数量、复生/复苏后再次死亡可再次触发、Token 不参与三连。

### 3.8 拟态

- 数据：`OnPlay | Mimic | enemy:single | duration=combat`，或 `OnCombatStart | Mimic | enemy:single | duration=combat`。
- 引擎：`exec.rs::apply_op` 增加 `Op::Mimic`：
  1. 取 `effect.target`（由待选或选择器确定）；目标不存在或已离场时落空；敌方与友方目标都允许，由卡牌选择器决定阵营。
  2. 读取目标当前 `attack()` / `health()`（不含光环）与 `keywords`。战斗克隆的当前值与 `granted` 已含光环折叠结果，因此要减去 `stats.attack.aura` / `stats.health.aura`，并剔除仅由光环授予的关键词（光环关键词以 `Duration::Aura` 打标，`combat::keywords_without_aura` 负责过滤）。
  3. 给来源写入 `ModifyStats` 等价的临时/永久增量；关键词写入 `granted`，时长取 `params.duration`；`statsOnly=true` 时跳过关键词（卡面只写攻防值的卡使用，如乌贼、吸血鬼乌贼、大王酸浆鱿）。
  4. 不复制 `growth`、`breed`、`contagion`、`shield_used`、`reborn_used`、`revive_used`、`stealth_broken`。
  5. 记录 `CombatAction::Mimicked`（战斗内）。
- 口径：金色触发两次时以第二次为准；目标不存在时不阻塞出牌。
- 测试：复制当前攻防、复制关键词、不复制成长、目标不存在时落空、持续到本回合结束后回滚。

### 3.9 复苏

- 数据：`OnDeath | Revive | self | attack=X,health=Y`。
- 引擎：`combat.rs` 的死亡结算在亡语、播种、复生之后检查 `revive_used`；未使用时：
  1. `stats.attack.add_permanent(X)`、`stats.health.add_permanent(Y)`；
  2. `stats.health.revive_to(max(1, stats.health.current + Y))`（或按卡面基础值 + Y，取实现口径）；
  3. `revive_used = true`、`reborn_used = true`，避免同一次死亡再触发复生；
  4. 记录 `CombatAction::Revived`。
- 口径：复苏与复生互斥，触发复苏后本次死亡不再触发复生（置 `reborn_used`），复苏本身每局只生效一次（`revive_used`）；金色只复活一次；复活后保留成长与已授予关键词。
- 测试：以声明值复活、只复活一次、与复生互斥、复活后再次死亡不再触发。

### 3.10 回手

- 数据：`OnDeath | Recall | self | to=hand`。
- 引擎：死亡结算时若携带 `Recall` 且手牌有空位：
  1. 克隆当前 `MinionInstance`（保留成长、繁衍、传染、已授予关键词、金色）；
  2. 写入 `CombatLog.hand_minions`；
  3. 记录 `CombatAction::Recalled`；
  4. 该随从正常离场。
- 结算层：`settle.rs` 把 `hand_minions` 写入对应座位的持久手牌；手牌已满时丢弃。
- 测试：回手保留状态、手牌已满不回手、回手后再次打出按当前状态入场。

### 3.11 引爆

- 数据：`OnCombatStart | Detonate | enemy:single | amount=<每层伤害>,threshold=<阈值>`。
- 引擎：`exec.rs::apply_op` 增加 `Op::Detonate`：
  1. 读取目标 `contagion` 层数；为 0 时落空。
  2. 移除全部层数。
  3. 造成 `layers * amount` 点伤害，来源为携带者。
  4. 若 `layers >= threshold`，直接施加致死伤害。
  5. 记录 `CombatAction::Detonated`。
- 口径：护盾可抵消伤害但不能阻止阈值消灭；引爆移除的层数不再产生每回合掉血。
- 测试：层数伤害、阈值消灭、护盾抵挡伤害但阈值仍消灭、0 层落空。

### 3.12 吞噬

- 数据：`OnPlay/OnCombatStart | Devour | enemy:single | value=<攻击力上限>,duration=permanent`。
- 引擎：`exec.rs::apply_op` 增加 `Op::Devour`：
  1. 目标不存在、不在敌方或攻击力大于 `value` 时落空。
  2. 对目标施加致死伤害（护盾不能阻止）。
  3. 来源在本场战斗内永久获得目标卡面基础攻击力与生命值（`add_permanent`，战斗结束不写回）。
  4. 记录 `CombatAction::Devoured`（战斗内）。
- 口径：招募阶段的敌方战吼按既有延迟机制在战斗开始时结算；获得值不含目标成长与光环；获得只在当前战斗内生效。
- 测试：攻击力上限、超限落空、护盾不能阻止、永久获得属性。

### 3.13 转化

- 数据：`OnCombatStart | Convert | enemy:single | threshold=<层数阈值>,token=<token>,count=1`。
- 引擎：`exec.rs::apply_op` 增加 `Op::Convert`：
  1. 目标 `contagion < threshold` 时落空。
  2. 对目标施加致死伤害。
  3. 在来源方棋盘召唤 Token（沿用 `summon` 路径，受战场槽位限制）。
  4. 记录 `CombatAction::Converted`。
- 口径：护盾不能阻止转化；召唤物属于来源玩家；层数不消耗（转化是门槛，不是引爆）。
- 测试：阈值达成、未达阈值落空、召唤归属、护盾不能阻止。

### 3.14 潜伏

- 数据：`OnTurnStart | Latent | self | amount=<每层攻击力>,threshold=<阈值>,count=<爆发伤害>`。
- 引擎：`exec.rs::apply_op` 增加 `Op::Latent`：
  1. 读取来源 `contagion` 层数。
  2. 每层使来源永久获得 `amount` 点攻击力（`add_permanent`）。
  3. 若层数达到 `threshold` 且 `count > 0`：移除全部层数、置 `latent_used`，对敌方全体随从按招募阶段通用伤害规则结算（先消耗圣盾、再扣减伤、伤害写入永久层），击杀的随从立即离场并触发亡语；`count = 0` 时不清层也不置标记。
- 口径：爆发每局最多一次（用 `latent_used` 标记）；金色按通用规则触发两遍，加攻双倍，爆发仍只一次；伤害来源为携带者。
- 测试：按层加攻、阈值爆发、爆发后层数清零、只触发一次、击杀离场并触发亡语、减伤、圣盾、`count = 0`。

### 3.15 零件回收

- 数据：`OnDeath | Salvage | self | pool=playstyle,count=1`。
- 引擎：`exec.rs::apply_op` 增加 `Op::Salvage`：
  1. 从卡池中筛选本局启用玩法、非 Token、非英雄专属、非奖励牌的随从牌。
  2. 按 `Stream::Combat` 随机取 `count` 张，去重（不足时按实际数量）。
  3. 写入 `CombatLog.hand_copies`；手牌已满时不获得。
  4. 记录 `CombatAction::Salvaged`。
- 口径：获得卡面基础牌，不继承来源状态；金色触发两次时获得两张。
- 测试：随机池只含机械、手牌已满不获得、金色获得两张、随机结果确定。

## 4. 数据生成与校验

### 4.1 `scripts/gen_card_data.py`

- `TRIGGER_JSON` / `OP_JSON` 增加新原语。
- `PICK_JSON` 增加 `LowestAttack`。
- 参数解析增加 `perAttack`、`perHealth`、`maxCount`、`threshold`。
- Token 注册表解析新增条目。

### 4.2 `scripts/validate_docs.py`

- `KEYWORD_BASES` / `ALL_KEYWORD_BASES` 增加 `poison`、`armor`、`stealth`。
- `KEYWORD_RE` / `SKILL_KEYWORD_RE` 支持 `armor \d+`。
- `KW_UNLOCK` 增加 `poison`、`armor`、`stealth`：新机制不按时代解锁，最小星级按实际卡池登记（当前 poison 3、armor 2、stealth 3），`check_keyword_unlock` 校验实际最小星级与登记值精确相等。
- `KEYWORD_PLAYSTYLE` 增加：
  - `poison: {"reptile", "cephalopod", "cnidarian"}`
  - `armor: {"reptile", "bacteria", "cnidarian"}`
  - `stealth: {"cephalopod"}`
- `keyword_cost` 增加三种关键词的折算。
- 新增原语专属校验：`Mimic` 仅头足纲、`Devour` 仅原生生物界、头足纲（大王乌贼）、`Detonate` 仅细菌域/病毒、`Convert` 仅细菌域、`Revive`/`Recall` 仅水螅纲、`Salvage` 仅机械、`Pick::LowestAttack` 仅鸟纲。
- `EXPECT_ERA`、`EXPECT_LEVEL` 与期望输出改为新总数。

### 4.3 `scripts/audit_effects.py`

`audit_effects.py` 没有独立的 `OP_WHITELIST` / `TRIGGER_WHITELIST`：它直接复用 `validate_docs.parse_effects`，以 `validate_docs` 的 `EFFECT_TRIGGERS` / `EFFECT_OPS` / `SELECTOR_PICKS` 为唯一白名单，新原语与选择器随 `validate_docs.py` 自动生效。审计层只在解析之后补充跨字段检查：

- `GrantKeyword` 的层数规则（`VALUE_KEYWORDS`，已含 `armor`）。
- 既有检查：`timesIf`、`when`、`Summon`、`Discover`、`GainMinion`、`Decompose`。
- 新机制原语的目标侧约定：`Mimic` 允许敌方或友方选择器，`Devour` / `Detonate` / `Convert` 使用敌方选择器，`Revive` / `Recall` / `Salvage` / `Latent` 使用 `self`；`Recall` 的 `to` 必须为 `hand`。
- 统计输出按「触发 × 动作」自动汇总新原语；关键词覆盖由 `validate_docs` 的关键词统计输出承担。

### 4.4 测试期望

- `tests/test_gen_data.py`：卡牌 346、随从 309、技能数按实际、英雄 32。
- `tests/test_audit_effects.py`：技能数与效果段数按实际。
- `tests/test_validate_hero_cards.py`：英雄 32 不变。
- 金标准与状态哈希：新增字段后更新 `combat_state_hash_matches_golden_baseline`。

## 5. 协议与客户端

- `view.schema.json` 的 `Minion` 增加 `stealth: boolean`；`protocol/samples` 同步。
- `client` 卡面按 `keywords` 显示新关键词；潜行随从在 `stealth=true` 时显示潜行标识。
- 文本表由 `gen_text.py` 自动生成新卡名、技能名与效果文本；不需要手写翻译。
- 战斗日志按新 `CombatAction` 映射为现有展示事件；没有对应 UI 的动作只写入日志，不影响客户端流程。

## 6. 验证矩阵

| 项 | 方式 |
| --- | --- |
| 关键词与原语 | `cargo test -p rule-engine`，每个机制至少 3 个用例 |
| 数据与文档 | `python3 scripts/validate_docs.py`、`make data-check` |
| 生成物 | `make data` 后 `git diff` 只含预期文件 |
| 协议 | `make protocol-check`、`make protocol-rs-check` |
| 确定性 | `make m0-determinism`、`make m0-batch` |
| 全量 | `make m0-check`、`make client-check` |

## 7. 风险与约束

- 潜行与自动战斗的目标选择需要保证战斗必然推进；所有敌人潜行时退回最左侧目标。
- 回手与复苏改变死亡结算顺序，必须放在复生之后、离场之前，避免重复触发。
- 新机制关键词折算值写入产品文档后再改校验，避免先实现后补规则。
- 106 张卡分 4 批落地，每批完成后跑一次 `make data-check` 与相关引擎测试，避免一次性引入大量数据错误。
- 新增分类节点必须保持「生物叶子为科」与父子层级合法。
