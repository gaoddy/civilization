# 手牌拖拽出牌与目标选择 · 技术方案

> 状态：方案未实施
> 产品规则：[product.md](product.md)

## 1. 改动边界

| 端 | 范围 |
| --- | --- |
| 协议 | `PlayCard` 增加 `position`；新增 `ChooseTarget` 意图；`HandCard` 增加 `requiresTarget`；`PendingChoice` 增加 `target` 分支；移除 `SkipAnimation` |
| 规则引擎 | 战吼需要目标时改为入队待选；目标待选阻塞该座位的其它招募意图；`PlayCard` 支持指定站位 |
| 房间服务 | `map_intent` 透传新字段；视图映射新字段；意图门控不变（阶段与 `recruit_done` 由视图负责） |
| 客户端 | 手牌改为拖拽；棋盘提供插入落点与随从落点；目标待选浮层；移除点击出牌 |

不涉及：战斗结算算法、数值、卡牌数据。

## 2. 协议

### 2.1 `PlayCard`

```json
{
  "kind": "PlayCard",
  "handIndex": 0,
  "position": 2,
  "targetMinionId": "17"
}
```

- `position`：可选，`0..=6`，缺省表示追加到最右。指插入下标，语义与 `RearrangeBoard.toIndex` 一致：原下标及其右侧依次右移。
- 只有随从牌使用 `position`；效果牌忽略该字段（服务端不因此报错，便于客户端统一发送）。
- 需要目标的效果牌仍用 `targetMinionId`，服务端行为不变。

### 2.2 `ChooseTarget`

```json
{ "kind": "ChooseTarget", "minionId": "21" }
```

- 消费该座位队列中最早的目标待选，以 `minionId` 结算该次战吼。
- 目标不在合法池 → `NO_VALID_TARGET`。
- 该座位没有目标待选 → `INVALID_CHOICE`（与 `ChooseDiscover` 同类语义：待选不存在）。

### 2.3 `HandCard.requiresTarget`

- 新增必填布尔：该牌是否需要玩家指定单一目标。
- 计算口径见 §3.3，与引擎判定同源；客户端只读，不自行解析技能文本。

### 2.4 `PendingChoice` 新增分支

```json
{ "kind": "target", "sourceMinionId": "21" }
```

- `sourceMinionId`：触发该待选的随从，供客户端高亮。
- 候选列表不由服务端下发：候选恒为「本座位当前棋盘上的全部随从」，客户端直接读 `self.board`。目标待选期间该座位不能出售或移位，候选集合在待选存续期内不会变化。

### 2.5 移除 `SkipAnimation`

- 删除 `messages.schema.json` 的 `$defs.SkipAnimation` 与 `messages.json` 的 c2s 条目、`protocol/samples/v1/c2s/SkipAnimation.json`。
- 同步删除协议 crate 的 `ClientMessage::SkipAnimation` 分发与 `room-service` 的空实现分支。
- 删除后重新生成 `protocol/generated/protocol.rs` 与 `protocol/generated/protocol.gd`；`SkipAnimation` 已不存在，服务端收到该 `type` 返回 `UNKNOWN_TYPE`。
- 该消息未在客户端实现过，无客户端代码需要删除。

## 3. 规则引擎

### 3.1 新增待选与意图

```rust
pub enum PendingChoice {
    Discover { seat, candidates, source },
    Evolve { seat, source, remaining },
    /// 战吼已入场但尚未选定目标；`card_id`/`golden` 用于结算时重建效果上下文。
    Target { seat, source: InstanceId, card_id: String, golden: bool },
}

pub enum Intent {
    PlayCard { hand_index: u8, position: Option<u8>, target: Option<InstanceId> },
    ChooseTarget { minion: InstanceId },
}
```

### 3.2 `play_card` 的随从分支

1. `check_play_card` 增加位置校验：`position` 超过当前棋盘长度返回 `InvalidChoice`；棋盘已满返回 `NoSpace`（不变）。
2. 随从按 `position` 插入棋盘（`insert`，不是 `push`），登记进化、判定三连、触发 `OnAllyEvolve`，与现在一致。
3. 判定该牌是否有需要单一目标的 `OnPlay` 效果（§3.3）：
   - 没有 → 立即 `fire(Trigger::OnPlay)`，与现在一致。
   - 有，且入场后的合法池非空 → **不触发 `OnPlay`**，向队列推入 `PendingChoice::Target`；本次意图只完成入场。
   - 有，但入场后合法池为空 → 战吼落空，不推入待选，意图直接成功。
4. 合法池的判定时机改为**入场之后**，因此池内包含该随从自身。

- 需要目标的战吼整段延后到 `ChooseTarget` 时才 `fire(Trigger::OnPlay)`：同一张牌上并列的非目标效果也一起延后，口径统一为「战吼在选定目标后整段结算」。
- 效果牌分支不变：仍要求调用方直接带 `targetMinionId`，`check_targets` 保持「池非空而未指定即拒绝」。

### 3.3 「是否需要目标」的唯一判定

新增 `Engine::requires_target(&self, card_id: &str, trigger: Trigger) -> bool`：该卡的该触发器下，任一效果的选择器为 `Selector::Target`，或为 `Selector::Side { pick: Pick::Single, .. }` 且该效果没有条件分支豁免。

- 视图层用它填 `HandCard.requiresTarget`（`Trigger::OnPlayEffect`），`play_card` 用它决定入队（`Trigger::OnPlay`）。
- 与 `check_targets` 共用同一批选择器判定，避免两处口径漂移；`check_targets` 改为调用该判定后再做池内校验。

### 3.4 目标待选阻塞

- `recruit::apply` 在分派前检查该座位队列头部是否为 `Target`：是则除 `ChooseTarget` 外一律返回 `InvalidStage`（含 `EndRecruit`、`BuyMinion`、`PlayCard` 等）。
- 队列头部是 `Discover` / `Evolve` 时行为不变（不阻塞招募意图）。
- `ChooseDiscover` 遇到头部是 `Target` 时返回 `InvalidChoice`，不越序消费。

### 3.5 超时作废

待选队列现在会跨回合保留（发现、进化待选允许玩家下回合再选）。`Target` 待选不能这样留：它引用的是本回合入场的随从，跨回合后棋盘已变。

- `finish_recruit` 进入 `Combat` 前清空所有 `Target` 待选（战吼落空），`Discover` / `Evolve` 保持现状。
- 招募超时同样走 `finish_recruit`，因此超时与全员 `EndRecruit` 两种路径口径一致。
- `state_hash` 需要覆盖新的 `PendingChoice::Target` 变体（`hash.rs` 按变体打 tag，追加 `u8(2)` 分支）。

### 3.6 `choose_target`

```rust
fn choose_target(engine, state, seat, minion) -> Result<Outcome, Reject>
```

1. 取队列头部；非 `Target` → `InvalidChoice`。
2. 目标必须在本座位棋盘上 → 否则 `NoValidTarget`。
3. 弹出待选，按记录的 `card_id`/`golden` 重建 `EffectCtx`（`source` 为待选里的实例编号），执行 `fire(Trigger::OnPlay)`，事件与立即结算时一致。
4. 战吼可能再次产生待选（例如战吼带发现），照常推入队列。

### 3.7 视图

- `PendingChoiceView` 增加 `Target { source_minion_id }`，`view()` 逐条映射。
- `HandCardView` 增加 `requires_target`。

### 3.8 机器人

- `sim` 的 AI 在 `pendingChoices` 头部为 `target` 时选第一个己方随从发 `ChooseTarget`；`brain` 同样处理。
- 现有 `PlayCard` 不带 `position`，行为为追加，与改动前一致。

## 4. 房间服务

- `map_intent` 透传 `position` 与 `ChooseTarget`；`position` 越界由引擎返回 `InvalidChoice`，房间层不额外校验。
- 视图映射补两个新字段，无其它改动。
- `error_code_of` 无需新增映射。

## 5. 客户端

### 5.1 手牌改为拖拽

- 手牌卡从 `Button` 改为 `PanelContainer`，实现 `_get_drag_data`：返回 `{hand_index, card_id, requires_target}`，并用一张缩小卡作为拖拽预览。
- 删除 `pressed` 回调与 `_on_play` 的按钮路径；点击不再产生任何意图。
- 手牌不可拖动的情况（视图过期、倒计时归零、已结束招募、离线、目标待选阻塞）由 `_get_drag_data` 返回 `null` 实现，拖拽根本不开始。

### 5.2 棋盘落点

- 我方棋盘行在每张随从卡与空位之间插入一个**落点控件**（固定宽度的 `Control`，`MOUSE_FILTER_STOP`），下标 0..=当前随从数。
- 落点 `_can_drop_data`：数据为手牌随从 → 仅当棋盘未满时接受；数据为效果牌 → 仅当 `requires_target` 为假时接受。
- 落点 `_drop_data`：发送 `PlayCard{handIndex, position}`，效果牌不带 `targetMinionId`。
- 随从卡自身作为落点：只接受 `requires_target` 为真的效果牌，接受后发送 `PlayCard{handIndex, targetMinionId}`。
- 棋盘已满时不再渲染落点控件，拖拽到棋盘区域自然落空即取消。

### 5.3 目标待选

- `self.pendingChoices` 头部为 `target` 时弹出选择浮层：列出 `self.board` 全部随从，高亮 `sourceMinionId`。
- 浮层不提供取消；选中后发送 `ChooseTarget`。
- 浮层打开期间手牌与棋盘不响应拖拽，与 §3.4 的门控一致。

### 5.4 文档与测试

- 客户端边界测试新增断言：手牌卡不再有 `pressed` 连接；`PlayCard` 只从拖拽路径发出。
- 冒烟测试新增：拖拽随从牌到指定落点后发出带 `position` 的意图；拖拽需要目标的效果牌到空地不发意图；目标待选浮层出现且列出全部己方随从。

## 6. 兼容与迁移

- `PlayCard.position` 与 `ChooseTarget` 为新增可选字段与新意图，`requiresTarget` 为新增必填字段。协议 `v` 保持 1：本版本尚未发版，客户端与服务端同批发布，不做兼容层。
- `SkipAnimation` 移除后，旧客户端发该类型会收到 `UNKNOWN_TYPE`；旧客户端不存在于线上，接受该后果。

## 7. 验证

| 项 | 方式 |
| --- | --- |
| 站位插入 | 引擎测试：插入下标 0、中间、末尾与越界 |
| 战吼入队 | 引擎测试：需要目标的战吼入场后产生待选、池含自身、选定后效果生效 |
| 阻塞 | 引擎测试：待选期间 `BuyMinion`/`EndRecruit` 被拒、`ChooseDiscover` 不越序 |
| 空池 | 引擎测试：入场后无合法目标时不产生待选、战吼落空 |
| 超时作废 | 引擎测试：`Target` 待选在 `finish_recruit` 后被清空，`Discover`/`Evolve` 保留 |
| 视图字段 | 引擎视图测试 + 协议样例 |
| 协议漂移 | `make protocol-check`、`make protocol-rs-check` |
| 端到端 | `make m1b-e2e`：机器人全程使用新意图路径，错误白名单不含新增错误码 |
| 客户端 | `make client-check`、`make client-smoke` |
