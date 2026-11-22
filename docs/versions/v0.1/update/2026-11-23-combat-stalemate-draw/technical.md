# 战斗无伤害僵局直接判平（技术）

> 状态：已实施（2026-11-23）
> 适用版本：v0.1
> 产品规则：[product.md](product.md)

## 1. 结论

规则引擎在战斗行动循环中增加「双方都没有能造成伤害的随从」判定：满足时立即停止战斗，`CombatLog.winner` 为空并新增 `stalemate: true`；结算层据此判平，不再走行动上限后的生命 → 累计伤害 → 座位分胜负。协议、客户端、数据库不改。

## 2. 规则引擎

### 2.1 判定口径

「能造成伤害的随从」按战斗现场判定：存活、整场能出手（`SideBoard::starting_tokens > 0`）、当前攻击力 `effective_attack > 0`。

- `starting_tokens` 是 `SideBoard` 在随从入场时按 `cannot_attack` 算出的每轮行动次数；整场无法出手的随从为 0。
- 不使用 `MinionInstance::cannot_attack` 字段：战斗开始时施加的「本回合无法攻击」不改变入场时算出的行动次数，后续轮次会按该次数恢复出手；用字段判断会把这类随从误判为永久不能造成伤害。
- 不遍历卡牌效果：所有战斗效果都依赖受伤、死亡、召唤、主动攻击或入场才会触发；双方都没有能造成伤害的攻击者时这些事件不会发生，未触发效果无法改变现场。

### 2.2 代码落点

`server/crates/rule-engine/src/combat.rs`：

- `SideBoard` 新增 `fn can_deal_damage(&self, catalog: &Catalog) -> bool`：任一存活下标满足 `starting_tokens[index] > 0 && effective_attack(catalog, minion) > 0`。
- `BattleField` 新增 `fn no_damage_stalemate(&self, catalog: &Catalog) -> bool`：两侧 `can_deal_damage` 均为假。
- `simulate_inner` 的 `while action_count < MAX_ACTIONS` 循环在存活判定之后、选择出手随从之前检查 `no_damage_stalemate`；为真时置 `stalemate = true` 并 `break`，不再产生动作。
- `CombatLog` 新增 `pub stalemate: bool`；正常结束与行动上限为 `false`。
- 胜负 `match` 增加 `_ if stalemate => None` 分支。

### 2.3 结算层

`server/crates/rule-engine/src/settle.rs`：

- `seat_winner`：`log.winner` 为空且 `log.stalemate` 时返回 `None`，优先于 `tie_break`。
- `resolve_real`：`winner` 为 `None` 时沿用既有分支，伤害 0、不写 `last_winner` / `last_won`、发 `CombatDrawn`；不新增分支。
- `resolve_ghost`：`log.stalemate` 时 `winner = None`、`damage = 0`，不写 `last_won` / `last_winner`，发 `CombatDrawn { a: seat, b: seat }`；`ghost_won` 计算需排除僵局。
- 七下西洋：`voyage_result(None, seat)` 已是 `Draw`，取 `max(占位数, 空位数)`，无需改动。
- 测试辅助 `empty_log()` 补 `stalemate: false`。

## 3. 协议与客户端

- `CombatResult` 无字段变化：判平时 `winnerId` 为空、`damage` 为 0，`actions` 为空或截断到僵局点。
- 客户端已处理空动作与平局：`CombatPlayback` 总时长按动作数计算，空动作只剩尾段；`hero_clash_requested` 在 `winnerId` 为空时传空 `winner_side`。不改客户端代码。
- 断线重连沿用同一 `CombatResult`，不新增补偿逻辑。

## 4. 测试

`server/crates/rule-engine/tests/combat.rs` 与 `settle.rs` 单测：

1. 双方开战攻击力全为 0：`stalemate` 为真、`actions` 为空、`winner` 为空。
2. 双方交战后只剩 0 攻随从：在剩余随从全为 0 攻的那一拍停止，动作截断，判平。
3. 一方 0 攻、另一方攻击力大于 0：不判平，按既有规则推进到清场。
4. 双方正攻但整场无法出手（入场前 `cannot_attack`）：判平、无动作。
5. 战斗开始使最高攻随从本回合无法攻击：不判平，该随从下一轮出手。
6. 幽灵战双方都无伤害来源：判平、伤害 0、落单座位不计胜者。
7. `seat_winner`：`stalemate` 优先于生命/伤害/座位判据。

验证命令：`cargo test -p rule-engine`、`cargo test -p room-service`、`make lint`；涉及对局结果，另跑 `make m0-determinism` 确认同种子复现不变。

规则改变会改变部分对局结果：按 `sim_determinism` 既有流程重定
`server/crates/sim/tests/golden/seed-1-games-20.json`，第 7、8 局（下标 6、7）
哈希改变，其余不变；重定原因写入 `sim_determinism.rs` 注释。

## 5. 文档回写

- `docs/versions/v0.1/product/rules/core-rules.md` §5.3、§5.6、§9。
- `docs/versions/v0.1/product/overview/design-overview.md` 幽灵战平局口径。
- `docs/current/technical/rule-engine-and-simulator.md` 战斗结算与幽灵战说明。
