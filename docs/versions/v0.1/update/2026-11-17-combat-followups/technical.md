# 战斗棋盘容量与阵亡展示 · 技术方案

> 状态：已实施
> 产品规则：[product.md](product.md)

## 1. 改动边界

| 端 | 范围 |
| --- | --- |
| 协议 | `PlayerView` 新增 `boardSlots`；`BoardMinion.index` 与 `summon.position` 的口径改为「存活随从序列下标」，去掉固定上限 |
| 规则引擎 | 战斗棋盘按存活数限容；召唤/播种/复制/复生在满场时作废；召唤位置改发存活序列下标 |
| 房间服务 | 视图映射下发 `boardSlots`；过滤层沿用动作里的位置，补语义说明与断言用例 |
| 客户端 | 棋盘容量与提示取视图；战斗行只显示存活随从；召唤按存活序列下标插入；落点回调复查门控；战报按回合归属 |

不涉及：招募阶段容量与拖拽落点算法（沿用现有实现）、战斗结算算法与伤害公式、配对与阶段推进。

## 2. 协议变更

### 2.1 `PlayerView.boardSlots`

```json
"boardSlots": { "type": "integer", "minimum": 1, "description": "本方棋盘随从位上限，v0.1 为 7" }
```

- 加入 `required`；客户端在字段缺失时回退到 7，兼容旧服务端。
- 位置选在视图顶层而不是 `GameStarted`：重连只回 `Snapshot`，视图每次刷新都携带，客户端不需要额外的续传逻辑。

### 2.2 下标口径

- `BoardMinion.index`：该随从在**当前存活随从序列**中的下标。
- `summon.position`：召唤后该随从在存活序列中的下标；已阵亡的随从不计数。
- 两者去掉 `maximum: 6`（保留 `minimum: 0`），实际上界由 `boardSlots` 表达；v0.1 下仍为 0–6。
- `summon.minion.index` 沿用 `summon.position`，语义一致，客户端不读该字段。

### 2.3 生成物与样例

- `protocol/generated/protocol.rs`、`protocol/generated/protocol.gd`、`client/generated/protocol.gd` 与字段表由 `make protocol` / `make protocol-rs` 重新生成。
- 所有含 `PlayerView` 的样例（`Snapshot`、`StateUpdate`、`StateUpdatePendingChoices`、`StaleStateUpdate`、`GameStarted`）补 `boardSlots`。

## 3. 规则引擎

### 3.1 容量判定

`BattleField` 新增只读辅助（签名示意）：

```rust
fn alive_count(&self, side: usize) -> usize;
fn alive_rank(&self, side: usize, index: usize) -> usize; // index 之前存活数量
fn has_room(&self, catalog: &Catalog, side: usize) -> bool;
```

- `has_room`：`alive_count(side) < catalog.config.board_slots`。
- 判定放在**构造召唤实例之前**：容量不足时不扣池、不生成实例编号、不触发 `OnAllySummon`、不增加 `summon_count`。
- 受影响的进场路径：
  - `effects/exec.rs` 的召唤效果（含左邻插入与追加）；
  - `effects/exec.rs` 的 `Copy` 到棋盘；
  - `combat.rs` 死亡结算中的播种召唤；
  - `combat.rs` 的复生。

### 3.2 进场顺序

沿用现有结算顺序并显式化：死亡动作 → 亡语/播种召唤 → 复生。

- 死亡先把 `alive[index]` 置 false，存活数立即减一。
- 召唤占用空出的位置；满场时作废。
- 复生最后检查 `has_room`；无空位则保持阵亡，`reborn_used` 照常置 true（本次战斗不再尝试）。

### 3.3 召唤位置

- `append` / `insert_at` 内部仍按向量下标插入（已阵亡条目保留在向量里，用于维持顺序）。
- 写入 `CombatAction::Summon.position` 时改为 `alive_rank(side, 插入后的向量下标)`，值域 0–6。
- `append` / `insert_at` 返回 `Option<InstanceId>`：容量不足返回 `None`，调用方跳过后续的召唤计数与联动触发。

### 3.4 测试

- 满 7 存活时亡语召唤不产生 `Summon` 动作、不扣池、`summon_count` 不变。
- 死亡空出位置后同一结算内的召唤成功；召唤占满后复生作废。
- 阵亡条目前有存活随从时，`position` 是存活序列下标而不是向量下标（构造「首张阵亡、后续召唤」场景断言 0–6）。
- 复制到棋盘在满场时作废。

## 4. 房间服务

- `view::to_protocol` 在 `PlayerView` 顶层写入 `board_slots = catalog.config.board_slots`。
- `filter::combat_result` 的 `Summon` 分支保持「`position` 原样、`minion.index = position`」，只更新注释说明新口径。
- 用例：视图携带 `boardSlots`；召唤动作的 `position` 与 `minion.index` 相等且在 0–6；满场作废的召唤不出现在 `CombatResult.actions`。

## 5. 客户端

### 5.1 容量来源

- `game.gd` 读取 `GameStore.view.boardSlots`，缺失时回退 7；`_refresh_board` 的 `board_full`、提示文案与空位数量全部使用该值。
- `BoardDrop` 仍由页面设置 `board_full`，控件本身不感知容量。

### 5.2 战斗行

- `CombatPlayback.own/opponent` 保留阵亡条目作为顺序锚点（不再显示）。
- `game.gd::_reconcile_row` 只渲染 `alive == true` 的条目，下标按存活序列计算。
- 召唤按下标插入：`_insert_at_alive(array, rank, minion)` 找到第 `rank` 个存活条目之前插入；`rank` 等于存活数时追加到存活序列末尾。
- 复生把 `alive` 置回 true，卡片在原相对位置重新出现。

### 5.3 落点门控

- `_on_board_dropped` 在 `epoch` 校验之外，再复查与拖拽起始同一组条件：阶段为招募、未结束招募、视图未过期、倒计时未归零、`Net.is_open()`、队首待选不是目标。任一不满足直接返回，不构造意图。

### 5.4 战报回合归属

- `GameStore` 新增 `last_combat_turn`，收到 `CombatResult` 时写入当前 `turn`；`reset_room` 清空。
- 战报行与战斗横幅只在 `last_combat_turn == GameStore.turn` 时展示；离开 `Combat` 时清空 `last_combat` 的现有逻辑保留。

### 5.5 测试

- `ui_smoke`：`boardSlots` 非 7 时提示、空位与满场判定跟随；阵亡卡从行内移除；召唤按存活下标插入；复生回原位。
- `ui_smoke`：阶段非招募或未连接时落点不产生意图。
- `ui_smoke`：回合号不匹配时不显示旧战报。

## 6. 兼容与影响

- `boardSlots` 为必填字段，旧客户端忽略新增字段；新客户端对旧服务端回退 7。
- 去掉下标上限是放宽约束，不产生反序列化问题；实际范围由 `boardSlots` 与引擎容量保证。
- 满场召唤/复生作废会改变部分战斗结果，`make m0-determinism` 仍需通过；历史 `output/m0-report.log` 数值不再与当前代码逐位一致。

## 7. 验证

| 命令 | 覆盖 |
| --- | --- |
| `make protocol-check`、`make protocol-rs-check` | 样例校验、两处生成物与字段表无漂移 |
| `make data-check` | 卡牌数据生成无漂移、效果审计通过 |
| `make lint` | `cargo fmt --check`、`clippy -D warnings` |
| `make test` | 引擎、房间服务、Python 测试全绿，含 e2e |
| `make client-check`、`make client-smoke` | 客户端边界断言、五场景无头冒烟 |
| `make m1b-e2e` | 真实客户端 + 7 机器人打完一局 |
| `make m0-determinism` | 同日志 100 次重放哈希一致 |

## 相关文档

- [产品规则](product.md)
- [规则引擎与模拟器](../../../../current/technical/rule-engine-and-simulator.md)
- [房间服务与联网](../../../../current/technical/room-service/README.md)
- [Godot 客户端](../../../../current/technical/client.md)
