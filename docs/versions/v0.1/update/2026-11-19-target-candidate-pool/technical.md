# 目标候选下发 · 技术方案

> 状态：已实施
> 产品规则：[product.md](product.md)

## 1. 改动边界

| 端 | 范围 |
| --- | --- |
| 协议 | 新增 `TargetRef`；`PendingChoice.target`、`HandCard`、`heroPower` 下发候选；`PlayCard` / `UseHeroPower` / `ChooseTarget` 改为提交 `target` |
| 规则引擎 | `Selector::Target` 增加区域与筛选；目标池按区域计算；新增合法候选计算；`Copy` 支持按手牌目标复制；战吼待选按候选非空判定 |
| 房间服务 | 意图映射 `target`；视图映射候选；目标区域校验 |
| 客户端 | 候选只读视图；效果牌拖到候选上才提交；手牌卡与棋盘都可作为落点；待选与英雄技能浮层按候选渲染 |
| 机器人 | `sim` AI 与视图机器人改为按区域轮换候选，不再假设目标恒在棋盘 |
| 数据与脚本 | 三个技能选择器改为 `target:hand:type=minion`；生成器、校验器与审计同步 |

不涉及：敌方目标、以商店为目标的卡牌、战斗选目标算法、数值。

## 2. 协议

### 2.1 `TargetRef`

`view.schema.json` 新增：

```json
"TargetZone": { "type": "string", "enum": ["hand", "shop", "board"] },
"TargetRef": {
  "type": "object",
  "additionalProperties": false,
  "required": ["zone", "id"],
  "properties": {
    "zone": { "$ref": "view.schema.json#/$defs/TargetZone" },
    "id": { "type": "string", "description": "目标实例编号；三个区域共用同一编号空间" }
  }
}
```

`intents.schema.json` 通过跨文件 `$ref` 复用该定义。`id` 统一取实例编号：手牌 `instanceId`、商店槽 `instanceId`、棋盘 `minionId` 三者在引擎内由同一个 `next_instance_id` 分配，全局唯一。

### 2.2 下发字段

| 位置 | 新增字段 | 说明 |
| --- | --- | --- |
| `PendingChoice` 的 `target` 分支 | `candidates: TargetRef[]`（必填） | 战吼待选的合法目标 |
| `HandCard` | `targetCandidates: TargetRef[]`（必填） | 该效果牌的合法目标；随从牌为空数组 |
| `heroPower` | `requiresTarget: boolean`、`targetCandidates: TargetRef[]`（必填） | 主动技能是否需要目标与合法目标 |

- 候选顺序固定为「我方棋盘 → 手牌 → 商店」，各区域内按原有下标顺序；客户端按区域分组展示。
- `HandCard.requiresTarget` 继续表示「该牌存在需要目标的效果」，与 `targetCandidates` 是否为空无关；`playable` 在 `requiresTarget` 为真且候选为空时为 `false`，`disabledReason` 为 `NO_VALID_TARGET`。
- `heroPower.requiresTarget` 为真且候选为空时，`canUseHeroPower` 为 `false`，`heroPowerDisabledReason` 为 `NO_VALID_TARGET`。

### 2.3 意图

```json
{ "kind": "PlayCard", "handIndex": 0, "position": 2,
  "target": { "zone": "hand", "id": "41" } }
{ "kind": "ChooseTarget", "target": { "zone": "board", "id": "21" } }
{ "kind": "UseHeroPower", "skillId": "universal-manuscript",
  "target": { "zone": "hand", "id": "41" } }
```

- 删除 `PlayCard.targetMinionId`、`UseHeroPower.targetMinionId` 与 `ChooseTarget.minionId`。开发期两端同批更新，不做旧字段兼容。
- `target` 缺省表示不指定；区域必须是 `hand` / `shop` / `board` 之一，且 `id` 必须存在于该区域的**本人**实体中，否则返回 `NO_VALID_TARGET`。
- 协议样例 `IntentPlayCardPosition.json`、`IntentChooseTarget.json` 与新增 `IntentPlayCardTarget.json` 同步更新；`make protocol`、`make protocol-rs` 重新生成两端代码与文档。

## 3. 规则引擎

### 3.1 选择器

```rust
pub enum Selector {
    SelfCard,
    Target {
        #[serde(default)]
        zone: Zone,
        #[serde(default)]
        filters: Vec<Filter>,
    },
    Side { side: Side, pick: Pick, filters: Vec<Filter> },
}
```

- `Zone` 增加 `Default = Board`，旧数据 `{"kind":"target"}` 反序列化后仍指向棋盘。
- DSL 增加 `target:<zone>[:<筛选>...]`，如 `target:hand:type=minion`；裸 `target` 保持原义。
- `requires_target`、`check_targets`、`resolve_selector` 全部按新变体匹配。

### 3.2 目标池

新增只读方法：

```rust
fn target_pool(&self, state, seat, zone, filters, ctx) -> Vec<InstanceId>
```

| 区域 | 池 |
| --- | --- |
| `Board` | 全部座位的棋盘（保留战斗内事件目标语义）；招募期由提交边界限制为本人 |
| `Hand` | 本人手牌的实例编号；`type=minion` 等筛选按卡面判定，效果牌匹配不到随从筛选 |
| `Shop` | 本人商店槽的实例编号；筛选只按卡面事实（星级、类型、玩法等） |

- 筛选复用 `CardFacts::matches`；关键词筛选只对棋盘与手牌中的随从实例生效，商店槽恒不匹配。
- `check_targets` 对 `Target` 改为：池为空时按 `reject_on_empty_pool` 决定拒绝或落空；池非空时要求 `ctx.target` 或 `ctx.event_target` 在池内。
- `Side::Single` 的池不变，仍为 `filtered_side`（棋盘）。

### 3.3 合法候选

新增公开方法：

```rust
pub fn target_candidates(
    &self,
    state: &GameState,
    effects: &[Effect],
    ctx: &EffectCtx,
    include_on_use: bool,
) -> Vec<TargetRef>
```

1. 先用 `ctx.target = None` 跑一次 `check_targets(..., reject_on_empty_pool = false)`；返回 `Ok` 表示当前状态不需要目标，直接返回空数组。
2. 否则枚举本人三个区域的实例编号（棋盘 → 手牌 → 商店），对每个候选以 `ctx.target = Some(id)` 重跑 `check_targets`；`include_on_use` 为真时再跑 `check_on_use_targets`（效果牌出牌路径）。
3. 只保留两次校验都通过的候选。

候选计算与拒绝校验共用同一套 `check_targets`，不另写筛选逻辑，避免视图与结算漂移。

### 3.4 目标提交与区域校验

- `Intent::PlayCard` / `UseHeroPower` 的 `target` 改为 `Option<TargetRef>`，`ChooseTarget` 改为 `TargetRef`。
- 新增 `fn target_in_own_zone(state, seat, target) -> bool`：`board` 只认本人棋盘，`hand` 只认本人手牌，`shop` 只认本人商店槽。三个意图入口先校验区域，再进入原有 `check_targets`；区域不符统一返回 `NoValidTarget`。
- 战吼待选的创建条件改为「`check_targets` 需要目标且 `target_candidates` 非空」；候选为空时战吼落空，不产生无法消费的待选。
- `choose_target` 去掉「必须在本人棋盘」的硬编码，改由区域校验与 `check_targets` 共同判定。

### 3.5 复制手牌随从

- 三个技能文档的选择器由 `self` 改为 `target:hand:type=minion`，参数保留 `from=hand,type=minion,to=hand`。
- `apply_recruit_copy` 改为：
  - 选择器为 `Target { zone: Hand, .. }` 时，按 `targets` 中的实例编号在本人手牌中找 `HandCard::Minion` 并复制；找不到则本次复制落空。
  - 选择器为 `Side` 时沿用棋盘目标复制。
  - 删除「`from=hand` 时静默取手牌第一张」的分支。
- `validate_effect` 增加约束：`Copy` 且 `from=hand` 时必须使用 `target:hand` 选择器，防止再出现静默取第一张的数据。

### 3.6 视图

- `HandCardView` 增加 `target_candidates: Vec<TargetCandidateView>`。
- `SelfView` 增加 `hero_power_requires_target: bool` 与 `hero_power_target_candidates`。
- `PendingChoiceView::Target` 增加 `candidates`。
- 候选结构直接复用引擎 `effects::TargetRef { zone, id }`，视图、协议与意图同源。
- `view()` 中先按手牌逐张计算候选，供手牌视图与 `HandAffordanceView.playable` 共用；英雄技能候选在 affordance 与视图之间共用一次计算结果。
- 效果牌 `playable`：`requires_target && candidates.is_empty()` 时置 `false`，原因 `NoValidTarget`。
- 英雄技能 `can_use_hero_power`：需要目标且候选为空时置 `false`，原因 `NoValidTarget`。

## 4. 房间服务

- `game.rs::map_intent` 把协议 `target` 映射为 `TargetRef`；`id` 解析失败返回 `NO_SUCH_MINION`，区域字符串由生成类型保证合法。
- `filter.rs` 不对目标区域做额外处理：候选只含本人区域，隐藏信息过滤不变。
- `view.rs` 映射三个新字段；`TargetRef.zone` 映射为协议 `TargetZone`。

## 5. 客户端

### 5.1 手牌拖拽

- 页面构造手牌 payload 时附带 `targets`：`card.targetCandidates` 原样复制。
- `HandCard` 实现 `_can_drop_data` / `_drop_data`：仅当拖拽数据是「需要目标的效果牌」且本人 `{zone:"hand", id}` 在 `targets` 中时接受，发出 `target_dropped(payload, target)`；拖到自己身上拒绝。
- `BoardDrop` 的 `_resolve`：需要目标的效果牌只有落点随从在 `targets` 中才接受；不再接受任意随从。`dropped` 信号的目标参数由字符串改为 `Dictionary`（`{zone, id}`）。
- `BoardDrop` 的光标监听只在指针位于棋盘矩形内时设置禁止/允许；指针离开棋盘后交还给手牌卡的 `_can_drop_data`，避免手牌落点被错误标成禁止。

### 5.2 出牌意图

- `game.gd::_on_board_dropped` 与新增的 `_on_hand_target_dropped` 共用门控：epoch 一致、`_can_play_now()` 为真、目标非空。
- 需要目标的效果牌在棋盘或手牌候选上松手后发送 `PlayCard`，带 `target`；其它落点不发意图。
- 删除 `_target_requests` 与 `NO_VALID_TARGET` 自动补选路径；错误只提示并保留当前视图。

### 5.3 浮层

- 战吼待选：遍历 `pendingChoices[0].candidates` 渲染；棋盘候选显示名称与攻防，手牌与商店候选显示名称与费用/攻防；按区域排序；选中发送 `ChooseTarget { target }`。
- 主动技能：`heroPower.requiresTarget` 为假时直接发送 `UseHeroPower`；为真且候选非空时打开候选浮层，取消不发意图；为真且候选为空时不打开浮层、不发意图。
- 删除效果牌的二次确认浮层路径；`_target_intent` 只服务主动技能。
- 新增/复用文案键：区域标题「手牌 / 商店 / 我方棋盘」。候选为空时不显示空浮层。

## 6. 机器人

- `sim` 引擎 AI：
  - `PendingChoice::Target` 依次尝试本人棋盘、手牌、商店的实例编号，构造对应区域 `TargetRef`；被拒候选继续轮换。
  - 定向效果牌同样按三个区域轮换目标；`hero_power_use` 按 `Selector::Target { zone }` 选择对应区域的最强/首个随从。
  - `report.rs` 的意图 JSON 改为输出 `target: {zone,id}`。
- 视图机器人 `bot/brain.rs`：
  - `PendingChoice::Target { candidates }` 直接取第一个未被拒绝的候选。
  - 手牌目标直接取 `hand[].targetCandidates` 的第一个未被拒绝项，不再猜棋盘。
  - `BlockedTargets` 的键改为 `zone:id` 字符串。

## 7. 数据与脚本

- `scripts/validate_docs.py::check_selector` 接受 `target:<zone>[:筛选...]`，区域限定 `hand/shop/board`。
- `scripts/gen_card_data.py::_json_selector` 输出 `{"kind":"target","zone":...,"filters":[...]}`；裸 `target` 输出保持原样。
- `scripts/gen_card_data.py::check_structured` 校验目标区域合法；`scripts/audit_effects.py` 增加「`Copy` + `from=hand` 必须用 `target:hand`」。
- 三个技能文档与对应卡牌的「使用建议」同步改为「选择一张」；`make data` 重新生成 `skills.json`、客户端数据与文案。

## 8. 测试与验证

| 层 | 用例 |
| --- | --- |
| 引擎 | `Selector::Target` 区域池与筛选；手牌目标复制；候选为空不产生待选；区域不符返回 `NO_VALID_TARGET`；旧 `{"kind":"target"}` 默认棋盘 |
| 视图 | 手牌 `targetCandidates`、英雄技能 `requiresTarget/targetCandidates`、待选候选；候选为空时 `playable` 与 `can_use_hero_power` 为假 |
| 房间服务 | 三种意图的 `target` 映射；跨区域篡改返回 `NO_VALID_TARGET`；视图字段映射 |
| 协议 | `make protocol-rs-check`、`make protocol-check` 无差异；样例通过校验 |
| 客户端 | `make client-smoke`：待选候选渲染、手牌落点、棋盘落点、非候选取消、主动技能浮层 |
| 机器人 | `sim` 与 dev bot 能消费手牌目标待选；整局可结束 |
| 数据 | `make data-check`、`make audit`；三个技能生成为 `target:hand` |
| 全量 | `make test`、`make lint`、`make m0-determinism`；更新 `sim/tests/golden/seed-1-games-20.json` 基线 |

## 9. 兼容与风险

- 协议为开发期破坏性变更，两端同批更新；生成物随契约提交。
- `Target` 的棋盘池保留「全部座位」以维持战斗内的 `event_target` 语义；招募期提交边界单独限制为本人，避免把产品规则下沉到战斗。
- 三个复制手牌卡的模拟结果会变化，金标准哈希需重建；这是预期行为，不做旧值兼容。
- 商店区域本版没有卡牌使用，客户端暂不实现商店卡的落点高亮；协议与视图字段先就位，后续加卡时只补 UI 落点。
