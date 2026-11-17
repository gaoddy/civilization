# 战斗演示不播放反击 · 技术方案

> 状态：已实施
> 产品规则：[product.md](product.md)

## 1. 改动边界

| 端 | 范围 |
| --- | --- |
| 规则引擎 | `resolve_attack` 不再为反击生成第二条 `CombatAction::Attack`；反击伤害仍照常结算并写入主动攻击动作的快照 |
| 房间服务 | 无需改代码；测试中手工构造动作的 fixture 去掉反击条目，过滤层注释同步 |
| 协议 | 无字段与生成物变化；`CombatResult.actions` 的条数随引擎动作序列减少 |
| 客户端 | 无需改代码；`CombatPlayback` 继续按动作数推进，反击自然不再出现 |

不涉及：反击的结算规则、护盾与 `OnDealDamage` 的触发顺序、死亡判定、主动攻击的演出、横幅与 `CombatPlayed` 回报流程。

## 2. 规则引擎

`server/crates/rule-engine/src/combat.rs::resolve_attack` 保持 (a)–(e) 的结算顺序：

- (a) 主伤害与护盾、(b) 传染叠加、(c) 反击伤害计算与 `apply_damage` 全部保留。
- (d) 只保留主动攻击的 `push_action`。主动攻击的 `main_states` 本来就在反击伤害应用之后抓取，因此该条动作的 `source` / `target` 快照已经包含反击后的双方血量与存活状态。
- 删除 `counter_damage > 0` 时第二次 `push_action` 与 `counter_states`。
- `counter_damage` 变量继续保留，用于 (e) 的 `OnDealDamage` 触发；反击护盾消耗、死亡结算与行动次数推进不变。

`CombatAction::Attack` 的文档改为「记录一次主动攻击；反击伤害并入该条动作的快照」，类型与字段不变。

## 3. 协议与房间服务

- `protocol/schema/v1/messages.schema.json` 的 `CombatResult.actions` 结构不变，不重新生成协议。
- `room-service` 的 `filter.rs` 按现有 `report.actions` 与 `report.states` 同序映射；反击动作消失后无需特殊分支，只更新 `combat_action` 的注释。
- `SeatCombatReport.actions` / `states` 仍由引擎保证同序同长，索引对齐逻辑不变。

## 4. 客户端

无代码改动：`CombatPlayback` 只负责按动作应用服务端快照，`GameCombatView._play_events` 的 `attack` 分支自然只在主动攻击时触发。演示总时长按 `actions.size() × PACE_MS + TAIL_MS` 计算，动作数减少后自动缩短。

## 5. 测试调整

| 文件 | 调整 |
| --- | --- |
| `server/crates/rule-engine/tests/combat_damage.rs` | `counter_damage_updates_current` 改为断言主动攻击动作的快照已含反击后的攻击方血量，不再查找第二条攻击动作 |
| `server/crates/rule-engine/tests/combat.rs` | 删除「主伤害与反击各记录一条」的期望；反击致死的用例改为一条主动攻击加一条死亡动作；`action_count` 与快照断言同步 |
| `server/crates/room-service/tests/game.rs` | `real_report` fixture 去掉手工构造的反击动作与对应 states；侧别断言改用主动攻击的 `targetSide = opponent` |
| `server/crates/room-service/tests/filter.rs` | `real_report` fixture 与 `actions` 下标断言同步去掉反击条目，护盾挡伤用例改用主动攻击表达 |

验证命令：`cd server && cargo test --workspace`、`make lint`、`make client-check`、`make client-smoke`。

## 6. 文档回写

- `docs/current/technical/rule-engine-and-simulator.md` §9：`CombatLog` 动作与快照说明补「一次攻击交换只记录主动攻击，反击伤害并入该条快照」。
- `docs/current/technical/client.md`：战斗演示段落无需改语义；如叙述动作来源，补一句动作只含主动攻击。
- `server/crates/room-service/src/filter.rs` 与 `combat.rs` 的注释同步。
