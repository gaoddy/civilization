# 招募结束后的战斗演示 · 技术方案

> 状态：已实施
> 产品规则：[product.md](product.md)

## 1. 改动边界

| 端 | 范围 |
| --- | --- |
| 协议 | `CombatResult.actions` 每条动作补齐随从 id 与动作后的攻防/存活；新增 `summon` 动作 |
| 规则引擎 | 战斗模拟逐条记录动作后的随从状态；`CombatLog` 保留动作序列不变 |
| 房间服务 | 把「进入战斗」与「战斗后结算」拆成两步；`Combat` 阶段按时长推进 |
| 客户端 | 战斗视图改为双方棋盘 + 手牌并排演示；按动作序列播放 |

不涉及：战斗规则、伤害公式、配对与结算语义（`resolve_settle` 内容不变，只是调用时机后移）。

## 2. 阶段推进改造

### 2.1 现状与问题

现在 `GameTask::advance_phase` 一次跑完 `finish_recruit → resolve_combat → 逐座位 CombatResult → resolve_settle → start_turn`，`Combat` 与 `Settle` 在同一毫秒内穿完，客户端几乎看不到战斗视图。演示需要 `Combat` 成为真实存在的时间段。

### 2.2 拆成两步

```text
enter_combat()      finish_recruit → resolve_combat → 逐座位 CombatResult
                    phase = Combat；deadline = now + T
finish_combat()     resolve_settle →（非 GameOver）start_turn
                    phase = Recruit；deadline = now + (150 − T)
```

- `enter_combat` 由「全员 `EndRecruit`」或招募超时触发；调用后不再推进阶段，等 `tick()`。
- `finish_combat` 由 `tick()` 在 `Combat` 且到达 `deadline` 时调用，是 `Combat → Settle → Recruit` 的唯一路径。
- 若 `resolve_settle` 把阶段置为 `GameOver`，不调用 `start_turn`，逐座位下发 `GameOver`，行为与现在一致。
- `Combat` 阶段同样下发 `View` 更新（阶段、对手、棋盘等公开信息），客户端据此切到战斗视图。

### 2.3 演示时长 T

T 由服务端按动作数算出，客户端只按同一节奏播放，两者不协商：

```text
T = action_count × PACE_MS + TAIL_MS
PACE_MS = 320      单条动作的播放时长
TAIL_MS = 2000     结果横幅停留
```

- `action_count` 取本场所有座位 `CombatResult.actions` 的最大长度（真实战两侧同一条战斗，镜像战只有一条报告），保证 8 个座位同时结束演示。
- 不设上限：动作多时 T 可能超过 60 秒，差额从下回合招募扣除；`recruit_ms = max(0, 150_000 − T)`。
- 三个常量（招募 90 秒、回合预算 150 秒、`PACE_MS`、`TAIL_MS`）集中定义在 `room-service`，客户端从同一份常量镜像（见 §5.3）。

### 2.4 重连

- `Combat` 阶段重连沿用现有 `Snapshot` 路径，不下发额外字段。
- 客户端用 `phaseDeadline` 反推进度：`elapsed = T − (deadline − now)`，跳过累计时长小于 `elapsed` 的动作，从当前动作继续；已播过的不重播。
- 服务端不为重连重发 `CombatResult`：客户端在 `Snapshot` 之后向服务端要一次，或由服务端在 `Combat` 阶段收到 `Reconnect` 时补发该座位的 `CombatResult`。采用后者（服务端保留本场逐座位报告到 `finish_combat` 为止），避免客户端再发请求。

## 3. 动作序列

### 3.1 现有形状

`CombatResult.actions[]` 目前只有 `kind`、`sourceIndex`、`targetIndex`、`damage`、`sourceSide`、`targetSide`。下标相对接收座位的两份战前快照，**战斗中召唤出来的随从不在快照里**，下标为 `null`；也没有动作后的攻防值，客户端只能自行累减。

### 3.2 新形状

每条动作补齐「谁、打谁、打完各剩多少」，客户端按值渲染，不做任何算术：

```json
{
  "kind": "attack",
  "sourceMinionId": "17",
  "targetMinionId": "41",
  "damage": 6,
  "source": { "minionId": "17", "attack": 6, "health": 3, "alive": true },
  "target": { "minionId": "41", "attack": 4, "health": 0, "alive": false }
}
```

| 字段 | 适用 kind | 说明 |
| --- | --- | --- |
| `sourceMinionId` / `targetMinionId` | `attack`、`contagion` | 随从 id；`contagion` 只有 `targetMinionId` |
| `damage` | `attack` | 护盾抵消时为 0 |
| `source` / `target` | `attack` | 动作执行后的攻防与存活，服务端直读战斗现场 |
| `target` | `death`、`reborn` | 死亡或复生后的状态；`reborn` 为 `health: 1`、`alive: true` |
| `summon` | `summon` | 见 §3.3 |

- 保留 `sourceIndex` / `targetIndex` / `sourceSide` / `targetSide`（战前快照内可定位时），用于兼容渲染与调试；新字段是渲染依据。
- 新增 `kind: "summon"`：战斗中召唤的随从。

### 3.3 `summon` 动作

```json
{
  "kind": "summon",
  "side": "opponent",
  "position": 3,
  "minion": {
    "minionId": "88", "cardId": "seedling", "golden": false,
    "attack": 1, "health": 1, "keywords": []
  }
}
```

- `side`：相对接收座位的「own / opponent」。
- `position`：在该侧棋盘中的插入下标，客户端按此插入即可，不需要推导顺序。
- `minion`：完整展示数据，字段与 `BoardMinion` 一致（`index` 与 `stats` 可缺省）。

### 3.4 引擎侧记录

`combat.rs` 的 `BattleField` 在每条动作入队时同时记录受影响随从的当前状态：攻击的攻方与受方、死亡与复生的目标、召唤的新随从与其插入位置。

- `CombatAction` 保持现有的四种动作与字段，新增 `Summon { minion, side, position }`。
- 新增 `CombatLog.states: Vec<Vec<MinionSnapshot>>`，与 `actions` 同序同长，下标对应：每条动作写入时把受影响随从的 `{instance_id, attack, health, alive}` 追加进去。`MinionSnapshot` 只读派生自战斗现场，不参与 `state_hash`。
- `CombatLog` 的其它字段（幸存者、回写、扣池、手牌复制）不变，战斗结果与随机流不受影响。

## 4. 房间服务映射

- `filter::combat_action` 按动作与同下标的状态快照产出协议动作：攻击填两侧 id 与状态，死亡/复生填 `target`，召唤填完整 `minion`。
- 召唤的 `BoardMinion` 由 `catalog` 补 `cardId`/`keywords`，与现有 `board_minion_from` 共用。
- `CombatResult` 其它字段（`ownBoard`、`opponentBoard`、`damage`、`winnerId`、`mirror`）不变。

## 5. 客户端

### 5.1 战斗视图结构

```text
战斗面板
├─ 对手英雄条
├─ 对手棋盘行        ← 上方
├─ 结果横幅（演示结束后出现）
├─ 我方棋盘行        ← 下方
├─ 我方英雄条
└─ 手牌行（只读）     ← 与招募同一组件，禁用拖拽
```

- 招募面板、商店、刷新/升时代/冻结、英雄技能与结束招募在 `Combat` 阶段隐藏（现有 `_set_combat_mode` 扩展：手牌行移出招募面板，成为中区常驻行）。
- 双方棋盘行按动作序列原地更新，不重建节点：维护 `minionId → 卡节点` 的映射，攻击改数值，死亡移除，召唤按 `position` 插入。
- 不提供跳过或加速入口。

### 5.2 播放器

- 收到 `CombatResult` 后建立播放队列：动作按 `PACE_MS` 节奏依次应用，应用即更新节点与数值。
- 每条动作的数值直接取 `source` / `target` / `minion`，客户端不做加减。
- 队列播完 + `TAIL_MS` 后显示结果横幅（胜负、伤害）；随后等服务端推进到 `Recruit` 的视图。
- 播放器状态只存在于场景内，视图重建（`view_changed`）不清空播放进度；`Combat` 阶段结束时清理。

### 5.3 进度对齐与重连

- 客户端持有 `PACE_MS`、`TAIL_MS` 常量，与服务端一致；不用它们计算结算，只用于播放节奏与重连续播。
- 重连进入 `Combat` 时，按 §2.4 用 `phaseDeadline` 反推已播放时长，快进到对应动作继续。

## 6. 兼容与迁移

- `CombatResult.actions` 新增字段与 `summon` 类型：旧客户端忽略未知字段、遇到 `summon` 会当作未知动作跳过，因此不会崩，但演示不完整。协议 `v` 保持 1，两端同批发布。
- `remove SkipAnimation` 与协议改动同批，见[手牌拖拽方案](../2026-11-16-hand-drag-and-target/technical.md) §2.5。
- 现有「一次推进到底」的房间测试需要改为两步：先断言 `Combat` 与 `CombatResult`，再推进时钟到演示结束断言 `Recruit`。

## 7. 验证

| 项 | 方式 |
| --- | --- |
| 动作状态 | 引擎测试：攻击、护盾、死亡、复生、召唤四类动作的 `states` 与战斗现场一致 |
| 阶段拆分 | 房间测试：`EndRecruit` 后停在 `Combat` 且 `deadline = now + T`；推进 T 后进入 `Recruit` |
| 时长记账 | 房间测试：T 为 30 秒时下回合招募截止为 `now + 120` 秒；T 超过 150 秒时为 `now` |
| 对局结束 | 房间测试：`Combat` 后直接 `GameOver`，不下发新一轮 `Recruit` |
| 重连 | 房间测试：`Combat` 中重连收到 `Snapshot` 与该座位 `CombatResult` |
| 协议漂移 | `make protocol-check`、`make protocol-rs-check` |
| 端到端 | `make m1b-e2e`：机器人也经过 `Combat` 等待，总时长增加但错误白名单不变 |
| 客户端 | `make client-check`、`make client-smoke`：战斗视图含上下两块棋盘、手牌保留、无跳过入口 |
