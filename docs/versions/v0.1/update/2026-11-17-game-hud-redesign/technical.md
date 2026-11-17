# 对局页改版与幽灵战：技术方案

> 状态：已实施
> 设计依据：[对局页 HUD 改版 设计](../../../superpowers/specs/2026-11-17-game-hud-redesign-design.md)
> 产品依据：[product.md](product.md)

## 1. 协议增量

| 字段 | 位置 | 类型 | 来源 |
| --- | --- | --- | --- |
| `hpCap` | `PlayerView` 顶层，必填 | integer ≥ 1 | `catalog.config.initial_hp` |
| `combatLog` | `PlayerView` 顶层，必填 | array（最近 5 回合） | `GameTask.combat_log` |
| `rank` | `PlayerSummary`，必填、可空 | integer 1–8 或 null | `rule-engine` 的 `rank_of` |

`combatLog` 条目：`turn`、`participantIds`（1–2 个玩家 id）、`winnerId`（可空）、`loserId`（可空）、`damage`、`mirror`。

- 客户端回退：`hpCap` 缺失按 40；`rank` 缺失不显示名次角标；`combatLog` 缺失按空数组。
- 生成物 `protocol/generated/protocol.rs`、`client/generated/protocol.gd`、字段表由 `make protocol` / `make protocol-rs` 重建，样例同步补字段。

## 2. 引擎

### 2.1 名次

`settle.rs` 新增 `rank_of(state, seat) -> Option<u8>`：`GameOver` 阶段直接取 `placement`；已出局者取「存活数 + 榜内下标 + 1」；存活者取「1 + 比它强的存活者数」，比较器复用 `compare_alive`。

### 2.2 幽灵战

- `Pairing { a, b, ghost: Option<Seat> }` 取代 `Pairing.mirror`；落单配对由 `pair_round` 末尾补 `ghost`——从 `state.ranking` 里用 `Stream::Matchmaking` 抽一名，只抽一次，没有出局者时留 `None`（空阵容幽灵）。落单配对永远保留。
- `resolve_mirror` 改为 `resolve_ghost`：胜负按「幽灵侧清场即攻方胜、攻方清场则幽灵胜、双方同回合清空判平局」判定；幽灵胜时按 `damage_formula(败者时代, 幽灵存活星级和)` 写入 `pending_settle`；攻方胜时写 `last_won` / `last_winner`；平局两边都不写、不扣血。
- 幽灵阵容取 `ghost_board(engine, state, pairing.ghost)`：该出局座位的 `player.board`，按它自己的光环来源重算一次后 `fold_auras`，只在模拟克隆上生效；`None` 返回空阵容。为此把 `recompute_auras` 的循环体抽成 `player_auras(engine, state, &PlayerState)`。
- 删除 `mirror_ghost`、`mirror_ghost_with_auras`、`strip_ghost`、`GameState.previous_board` 及其写入与哈希行。
- `view.rs` 的 `opponent_seat` 对幽灵战返回幽灵归属座位；自战镜像回退分支一并删除。

### 2.3 平局

`settle.rs` 新增 `seat_winner(state, log, a, b) -> Option<Seat>`：`log.winner` 有值直接采用；`log.winner` 为 `None` 且双方 `survivors` 都为空（同回合清场）返回 `None` 判平局；其余 `None`（行动数到 200 次）仍走 `tie_break`。

- 平局：`damage = 0`、不写 `pending_settle`、不写 `last_won`、`last_winner` 保持不变；`previous_opponent` 照常记录；两侧报告 `winner` 为 `None`。
- 事件新增 `GameEvent::CombatDrawn { a, b }`，`filter.rs` 与 `CombatResolved` 一样不下发给客户端。
- 客户端：`CombatResult.winnerId` 为 `null` 时横幅已显示「平局」；`combatLog` 条目 `winnerId`/`loserId` 均为 `null`、`damage` 为 0。

### 2.4 战斗日志

`GameTask` 增加 `combat_log: Vec<CombatLogEntry>`，`enter_combat` 时从 `outcome.reports` 归一追加（真实战两条报告合成一条），保留最近 5 个回合。**不写入 `GameState`**，不影响 `state_hash`。`player_view` 组装完协议视图后填入。

## 3. 卡牌数据

- `scripts/validate_docs.py`：`era` 改为随从与效果牌都必填；`EFFECT_FIELDS` 加 `era`；效果牌属性表加「时代」行；新增效果牌时代分布断言。
- 36 张效果牌按「出处」年代补 `era`，分布：古典 4、封建 3、工业 10、现代 11、未来 8。
- 同步修订 `card-system.md` 的「仅随从」表述与 `cards/README.md` 的数值列。
- 生成物 `server/data/v0.1/cards.json`、`client/generated/cards.json` 由 `make data` 重建。

## 4. 客户端

- 删除顶栏节点与 `GameStore.event_log`，事件只走状态变化。
- 左栏改为座位行：`UI.avatar` + 名次角标 + 血量条（按 `hp / hpCap`）；点击展开战报，展开座位按 `playerId` 记住并在 `_refresh_players` 重建后恢复。
- 右栏重排为三段：身份块（售货员 / 对手英雄 / 幽灵）、回合与倒计时、技能与结束招募与英雄卡。
- 删除 `CombatOpponentStrip` / `CombatSelfStrip`。
- 商店卡左上角费用、右上角时代角标；冻结改为蒙层 + 冰晶纹理 + 右下角「冻」字。
- `ui_smoke.gd` 与 `fixture.gd` 同步改写与补字段。

## 5. 验证

| 命令 | 覆盖 |
| --- | --- |
| `make protocol-rs` / `make protocol`、`make protocol-rs-check` / `make protocol-check` | 生成同步、样例通过 |
| `python3 scripts/validate_docs.py` | 效果牌时代必填与分布 |
| `make data` / `make data-check` | 两处卡表生成且无漂移 |
| `make test` | 引擎名次、幽灵战、房间服务战斗日志 |
| `make lint` | fmt 与 clippy |
| `make client-check` / `make client-smoke` | 客户端边界与新结构 |
| `make m1b-e2e` | 真实客户端 + 机器人打完一局 |
