# 成就运行时

> 状态：已实现（规则引擎、房间服务、账号级持久化与验证）
> 关联：[成就产品规则](../product/systems/achievements.md)、[成就索引](../product/content/achievements/README.md)、[发行渠道与账号](../../../current/product/accounts-and-distribution.md)、[Steam 接入](../../../current/technical/steam-integration.md)

本文记录 v0.2 六个局内成就的运行时落点：规则引擎内的触发与奖励、房间服务的下发、账号级解锁持久化与验证方式。产品条件与奖励以成就实体文档为准，本文不重复数值。

## 1. 已确认口径

| 项 | 口径 |
| --- | --- |
| 启用范围 | 所有对局都启用；不提供房主开关，`CreateRoom` 不增加字段 |
| 每局触发 | 每个成就按实体文档的 `count` 与「本局只触发一次」执行；不设「每局最多 3 个」的总上限 |
| 重复对局 | 同一账号每局都可重新触发并发放奖励 |
| Steam 解锁 | 账号级只同步一次；首次达成时下发 `AchievementUnlocked`，之后不重复下发 |
| 成长统计 | 统计所有成长来源：招募阶段的 `Growth` 结算、回合开始繁衍、战斗内成长；每命中一个随从计 1 次，`times` 不重复计数；战斗写回与回手不重复计 |
| 发现等级 | 成就奖励的发现使用玩家当前时代作为候选星级（6 星封顶，与现有三连奖励口径一致） |
| 奖励入账 | 奖励直接写入状态，不经过成就触发路径；手牌已满时「获得手牌」类奖励作废 |

## 2. 规则引擎

### 2.1 成就标识

新增 `server/crates/rule-engine/src/achievements.rs`：

```rust
pub enum Achievement {
    BiologicalEvolution,
    EducationForAll,
    Genetics,
    MachinesRoar,
    OneSmallStep,
    TuringMoment,
}
```

- `Achievement::id()` 返回实体文档的 `id`；`from_id()` 为反向解析。
- `Achievement::ALL` 按 id 升序排列，用作同一操作内多个成就同时满足时的固定结算顺序。
- 规则引擎不解析 `server/data/v0.2/achievements.json` 的条件文本；六个成就的行为在引擎内实现，测试断言引擎标识与生成数据 id 一一对应，防止数据漂移。

### 2.2 玩家状态

`PlayerState` 增加：

```rust
pub struct AchievementProgress {
    /// 本局已触发的成就。
    pub unlocked: BTreeSet<Achievement>,
    /// 累计成长结算次数（生物进化）；招募、回合开始与战斗内成长都计入。
    pub growth_events: u8,
    /// 获得过成长的随从 `subtype`（有教无类）。
    pub grown_subtypes: BTreeSet<String>,
    /// 下一次回合开始时额外发放的金币（人类一小步）。
    pub next_turn_gold_bonus: u8,
    /// 机械随从费用减免生效的回合号（机器轰鸣）。
    pub machine_discount_turn: Option<u8>,
}
```

- 每局由 `Engine::initial_state` 初始化为空；`start_turn` 不清空，成就状态属于整局。
- `hash.rs` 编码 `unlocked` 的 id、`growth_events`、`grown_subtypes`、`next_turn_gold_bonus` 与 `machine_discount_turn`；成就状态参与状态哈希与确定性重放。

### 2.3 触发点

| 成就 | 触发位置 | 判定 |
| --- | --- | --- |
| 人类一小步 | `recruit::upgrade` 成功升时代后 | 新时代码为 `Era::Future` |
| 图灵时刻 | `Engine::apply` 成功且完成光环重算后；`Engine::start_turn` 回合开始效果结算后 | 本座位棋盘上存在 `play:machine` 的随从，其关键词集合同时包含 `ignore_taunt` 与 `windfury`；关键词取卡面派生 + 已授予 + 光环派生 |
| 有教无类 | `Engine::evaluate_growth_achievements`（招募意图结束后、回合开始发金币后、战斗写回后） | 不同 `subtype` 计数达到 5 |
| 机器轰鸣 | `Engine::on_minion_entered_board`（手牌打出、效果召唤、效果复制）；战斗 `BattleField::append` / `insert_at` 记录入场，结算层按阵营触发 | 入场卡牌 id 为 `steam-engine` |
| 生物进化 | 同有教无类的三个阶段 | 成长结算次数达到 3 |
| 遗传学 | `Engine::check_triple` 每合成一次后 | 被合成的卡牌含 `play:plant` |

- 招募阶段 `Op::Growth` 的唯一执行点是 `effects/exec.rs::apply_growth`；只调用 `record_growth_event` 记账，成就判定在意图结束后统一执行，`times <= 0` 不计。
- 回合开始繁衍由 `Engine::apply_breed` 记账，成就判定在发金币之后执行，因此繁衍触发的 +1 金币当回合可用。
- 战斗内成长由 `BattleField::add_growth` 记入 `CombatLog::growth_events`；结算层在 `write_back_persistent` 之后按阵营记录并判定，写回不重复计。
- 图灵时刻在每次成功意图与回合开始时全量扫描本座位棋盘；同一成就本局只结算一次。
- 机器轰鸣的入场路径不含三连合成与站位调整；真实战按阵营归属，幽灵战只结算落单座位己方（side 0），幽灵侧不计。

### 2.4 奖励结算

| 成就 | 奖励实现 |
| --- | --- |
| 人类一小步 | `next_turn_gold_bonus += 1`；下一次 `start_turn` 在正常金币后叠加并清零 |
| 图灵时刻 | 按棋盘顺序找到第一个满足条件的机械随从，永久 +2/+2；记录 `Permanent` 层来源 |
| 有教无类 | 从本局启用玩法内、`in_shop_pool: true`、且不含 `Op::Growth` 的效果牌中按 `Stream::Effects` 随机一张进入手牌；手牌已满时作废 |
| 机器轰鸣 | 招募触发：`machine_discount_turn = Some(state.turn)`；战斗触发：`Some(state.turn + 1)`，留到下一次招募；购买 `play:machine` 随从的费用为 `max(1, 3 - 1) = 2`，其它随从与效果牌不变 |
| 生物进化 | 招募与回合开始触发：金币 +1（饱和加）；战斗触发：`next_turn_gold_bonus += 1` |
| 遗传学 | 以玩家当前时代为星级（6 星封顶），从植物玩法池内抽最多 3 张候选，登记 `PendingChoice::Discover`；无候选时不登记 |

- 购买费用在 `check_buy` 与 `buy` 共用同一计算函数，视图通过 `check_buy` 自动反映减免。
- 有教无类排除含成长的效果牌，避免奖励直接加速成长类成就；其余奖励不经过 `Growth` 或成就触发路径。
- 图灵时刻的永久增益通过 `StatComponent::add_permanent` 写入，并记录 `Permanent` 层来源；来源类型暂映射到「未知来源」展示，不新增协议枚举。

## 3. 房间服务

### 3.1 局内解锁下发

- 引擎不为成就产生 `GameEvent`；`GameTask` 在 `handle` / `tick` 返回前比较每个座位 `AchievementProgress::unlocked` 与上次基线，新增项生成 `UpdateKind::AchievementUnlocked`。
- `GameTask` 持有 `Arc<AchievementCatalog>`，按成就 id 填入 `steamApiName`。
- `RoomTask::deliver_excluding` 收到成就更新时先调用账号存储 `unlock_achievement`；账号级已解锁则不再下发。持久化失败只记日志，不阻断对局。

### 3.2 账号级持久化

- `server/migrations/0001_baseline.sql` 新增 `account_achievement(account_id, achievement_id, unlocked_at)`，主键 `(account_id, achievement_id)`；开发阶段直接改基线。
- 基线改动会改变 `sqlx` 迁移校验和：已按旧基线部署过的 dev 集群库在下次启动时会报 `VersionMismatch`。部署前需重建 dev 集群库后重新执行迁移；`make db-reset` 只重建本地 docker 库，不覆盖集群。
- `db.rs` 增加载入与 `INSERT ... ON CONFLICT DO NOTHING` 写入。
- `AccountStore` 增加 `achievements: BTreeMap<String, BTreeSet<String>>`、`unlocked_achievements(player_id)`、`unlock_achievement(player_id, achievement_id, now_ms)`；启动时随账号一并载入，内存模式同样生效。
- 认证成功后的 `AchievementState` 改为账号已解锁列表，按 id 升序；未知 id 跳过。非 Steam 客户端仍按类型忽略。

## 4. 文件落点

| 文件 | 改动 |
| --- | --- |
| `server/crates/rule-engine/src/achievements.rs` | 新增：成就标识、成长记录、触发与奖励 |
| `server/crates/rule-engine/src/state.rs` | `AchievementProgress` 与 `PlayerState` 字段 |
| `server/crates/rule-engine/src/engine.rs` | 初始化、`apply`/`start_turn` 的图灵判定、繁衍成长记账、成长成就统一判定、下回合金币 |
| `server/crates/rule-engine/src/recruit.rs` | 升时代、任意入场触发、购买费用 |
| `server/crates/rule-engine/src/triple.rs` | 遗传学触发与植物发现候选 |
| `server/crates/rule-engine/src/effects/exec.rs` | 招募成长记录、召唤与复制入场触发 |
| `server/crates/rule-engine/src/combat.rs` | 战斗成长与蒸汽机入场记录 |
| `server/crates/rule-engine/src/settle.rs` | 战斗写回后的成长与入场成就结算 |
| `server/crates/rule-engine/src/hash.rs` | 成就状态哈希 |
| `server/crates/rule-engine/tests/achievements.rs` | 新增：六个成就与边界用例 |
| `server/crates/room-service/src/game.rs` | 成就基线 diff、`UpdateKind` 与编码 |
| `server/crates/room-service/src/net.rs` | 认证下发账号成就状态、投递时落库 |
| `server/crates/room-service/src/account.rs` | 账号成就缓存与查询 |
| `server/crates/room-service/src/db.rs` | 成就表读写 |
| `server/migrations/0001_baseline.sql` | `account_achievement` 表 |

## 5. 验证

| 项 | 命令 |
| --- | --- |
| 规则引擎与房间服务测试（含新增成就用例） | `cargo test -p rule-engine -p room-service` |
| 静态检查 | `make lint` |
| 数据与文档校验 | `make data-check` |
| 确定性重放 | `make sim-determinism` |
| 数据库迁移与账号成就落库 | `make db-test` |

客户端不改代码：`AchievementState` / `AchievementUnlocked` 的幂等同步逻辑已存在，非 Steam 渠道只记录状态。

## 6. 未做与边界

- 不做局内成就弹窗、成就列表或结算页展示；本版只保证 Steam 同步与局内奖励。
- 不做房主开关；所有对局启用。
- 图灵时刻的永久增益来源在详情页显示为「未知来源」，不新增协议来源枚举。
- 手牌已满时，有教无类与遗传学的奖励作废或留待选（按现有发现规则处理），不额外补发。
