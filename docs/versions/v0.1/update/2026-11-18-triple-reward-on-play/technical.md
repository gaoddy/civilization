# 三连奖励改为金色上场时发放 · 技术方案

> 状态：待确认
> 产品规则：[product.md](product.md)

## 1. 改动边界

| 端 | 范围 |
| --- | --- |
| 协议 | 视图新增实例编号：商店槽 `instanceId`、手牌 `instanceId`；手牌新增 `golden`、`attack`、`health`、`rewardLevel`。意图不变 |
| 规则引擎 | 商店槽与手牌携带实例编号；随从牌在手牌里携带完整实例状态；三连改为金色入手牌、按最早实例继承状态、记录三张来源编号；金色打出时发放奖励牌；发现等级改为奖励牌锁定值 |
| 房间服务 | 视图映射新字段；隐藏信息过滤不变 |
| 数据 | 新增「三连奖励」效果牌，复用既有发现技能；从商店与发现候选中排除 |
| 客户端 | 手牌金色标识与真实身材；奖励牌卡面与星级展示 |

不涉及：战斗结算算法、数值与牌池份数、三连的判定范围与判定时机、成就系统。

分两个阶段实施：**阶段 1 实例身份**（商店/手牌/棋盘同一编号，随从状态随卡走），**阶段 2 三连改动**（金色入入手牌、继承状态、奖励牌）。阶段 2 依赖阶段 1。

## 2. 数据模型

### 2.1 实例编号

- 游戏内出现的每一张卡——商店槽位、手牌、棋盘随从、复制品、召唤物、三连奖励牌——都有全局唯一 `InstanceId`，来自 `state.next_instance_id` 单调递增。
- 冻结槽位保留原编号；商店重抽作废旧编号；牌池只记录份数，不记录编号。
- 三连奖励牌同样占一个编号，来源星级记录见 §2.5。

### 2.2 商店槽位

```rust
pub struct ShopSlot {
    pub instance_id: InstanceId,
    pub card_id: String,
    pub frozen: bool,
}
```

- `draw_shop_card` 仍只决定卡牌；`roll_player` 在写入槽位时分配编号，冻结槽位不重新分配。
- 效果牌与随从牌一视同仁。

### 2.3 手牌

```rust
pub enum HandCard {
    /// 随从牌：编号、卡牌、金色标记与全部状态都在随从实例上。
    Minion(MinionInstance),
    /// 效果牌：只有身份与「三连奖励」锁定的发现星级。
    Effect {
        instance_id: InstanceId,
        card_id: String,
        reward_level: Option<u8>,
    },
}
```

- 访问器：`card_id()`、`instance_id()`、`golden()`（效果牌恒 `false`）、`reward_level()`。
- 随从牌从商店买入时按卡面数值构造 `MinionInstance`（`golden = false`），编号取商店槽位的编号。
- 效果牌买入时只带编号与卡牌，`reward_level = None`。

### 2.4 随从实例

`MinionInstance` 增加 `sources: Vec<InstanceId>`，只对三连合成的金色非空，记录被合成的三张实例编号（按原编号升序）。该字段不参与状态哈希——不影响任何后续决策。

### 2.5 三连合成

`triple.rs::form_golden` 改写：

1. `find_triple` 收集棋盘与手牌中的非金色随从，按 `instance_id` 升序取前三；效果牌、金色、Token 不参与。
2. 基底 = 编号最小的副本；金色作为**新实例**（编号 = `state.next_instance_id++`）进入手牌，三张副本的编号全部作废。
3. 继承基底的全部状态：`stats` 分层（成长、永久、回合层、光环）、`breed`、`breed_applied`、`contagion`、`granted`、`shield_used`、`reborn_used`；`initial` 按金色卡面重设（现有 `make_golden` 口径），`attacks_left` 归零、`cannot_attack` 清除。
4. `sources` 记录三张原编号。
5. 三张副本从棋盘/手牌移除。
6. 落点：手牌有空位 → `HandCard::Minion(金色)` 追加到末尾；手牌已满 → 金色留在基底原位置（基底在棋盘则为金色随从，基底在手牌则为金色手牌），其余两张移除（产品兜底）。
7. 只推入 `TripleFormed { seat, card_id, golden_instance_id, sources }`，不再登记 `PendingChoice::Discover`。

### 2.6 打出随从

`play_card` 的随从分支不再调用 `new_minion`，而是把手牌里的 `MinionInstance` 整体移入棋盘指定位置：

- 编号与状态原样保留；`attacks_left = 0`、`cannot_attack = false`（与现状一致）。
- `state.next_instance_id` 不变（编号在商店或复制时已分配）。
- 之后照旧：登记进化 → 战吼（含目标待选）→ 三连判定 → `OnAllyEvolve`。

### 2.7 奖励牌

- 常量 `REWARD_CARD_ID = "triple-reward"`，卡牌目录中注册为效果牌。
- 打出金色随从、入场成功时（`play_card` 随从分支的三连判定之前）向手牌推入：

```rust
HandCard::Effect {
    instance_id: state.next_instance_id++,
    card_id: REWARD_CARD_ID.to_string(),
    reward_level: Some(card.level.saturating_add(1).min(6)),
}
```

- 手牌必然有空位：本次打出的随从已离开手牌。
- 手牌已满兜底留下的金色不经过本路径，因此不发奖励牌。

### 2.8 其它入手牌入口

| 入口 | 编号 | 状态 |
| --- | --- | --- |
| `choose_discover` 选中候选 | 新编号 | 按卡面数值 |
| 效果牌复制，`state=base`（缺省） | 新编号 | 按卡面数值，不继承来源状态 |
| 效果牌复制，`state=current` | 新编号 | 完整继承来源实例状态 |
| 战后复制回写 | 新编号 | 按卡面数值 |

## 3. 规则引擎

### 3.1 发现等级

- `EffectCtx` 增加 `discover_level: Option<u8>`。
- `apply_discover`：`discover_level` 有值时用它，否则用玩家当前时代（`player.era`）。
- `play_card` 的效果牌分支从 `HandCard::Effect` 取 `reward_level` 填入；随后 `Trigger::OnUse` 的上下文用 `discover_level: None`，避免响应打出效果牌的随从误用该等级。
- 发现候选口径不变：星级、本局启用流派或通用、共享池有剩余、最多三张。没有合格候选时 `apply_discover` 不登记待选，奖励牌已从手牌移除，与产品规则一致。

### 3.2 复制口径

- `Params` 增加 `state: Option<CopyState>`，取值 `base` / `current`，缺省 `base`；DSL 写 `state=current`，校验白名单与 `to`、`from`、`type` 同级。
- `apply_recruit_copy`：
  - `state=base`：按 `card_id` / `golden` 构造新实例（现有行为），不继承任何状态。
  - `state=current`：克隆来源实例，只换新编号；来源在手牌时取手牌实例，在棋盘时取棋盘实例。
  - 两种口径都清空 `turn` / `aura` 层与 `sources`（临时层不随副本离场或跨区），并把 `attacks_left`、`cannot_attack` 归零。
  - `to=hand` 入 `HandCard::Minion`，`to=board` 入棋盘；两者的编号都是新的，状态按口径决定。
- `apply_combat_copy`：`to=board` 已是完整克隆，补齐 `sources` 清空；`to=hand` 仍只记录卡牌与金色，战后回写按基础牌处理（战斗内读不到持久手牌）。
- 现有数据：`trigger-copy-friendly-minion`（有丝分裂）效果加 `state=current`；`trigger-copy-hand-minion`（印刷术）与 `universal-manuscript`（万能手稿）保持缺省。

### 3.3 排除奖励牌

- 商店：`is_eligible` 增加 `card.id != REWARD_CARD_ID`。
- 发现候选：`is_discover_candidate` 增加同一排除。
- 两处共用同一个常量，不新增卡牌字段。

### 3.4 事件

`GameEvent::TripleFormed` 增加 `golden_instance_id` 与 `sources`；`filter.rs` 仍只发给操作者，不广播。

## 4. 协议

### 4.1 视图字段

`protocol/schema/v1/view.schema.json`：

| 类型 | 新增字段 | 说明 |
| --- | --- | --- |
| `ShopSlot` | `instanceId`（string 或 null，必填） | 空槽位为 null，与 `cardId` 同口径 |
| `HandCard` | `instanceId`（string，必填） | 卡牌实例编号 |
| `HandCard` | `golden`（boolean，必填） | 效果牌恒 false |
| `HandCard` | `attack` / `health`（integer 或 null） | 仅随从牌非空，取当前有效值（学者攻击为 0，已夹非负） |
| `HandCard` | `rewardLevel`（integer 1–6 或 null） | 「三连奖励」锁定的星级 |

- 意图不变：`PlayCard` 仍按 `handIndex`，`BuyMinion` 仍按 `slot`。
- 样例 `protocol/samples/v1/...` 同步，`make protocol`、`make protocol-rs` 重新生成，`*-check` 无差异。
- 协议版本 `v` 保持 1：本版本未发版，两端同批发布。

### 4.2 引擎视图

`ShopSlotView`、`HandCardView` 增加对应字段；手牌随从的攻防取 `MinionInstance::attack()` / `health()`（攻击学者恒 0、生命下限 0，已夹非负），与棋盘口径一致。手牌随从牌恒不需要目标，`requiresTarget` 只对效果牌按 `OnPlayEffect` 判定。

## 5. 房间服务

- `view.rs` 映射新字段；`shop_slot_to_protocol` 与手牌映射补字段。
- `filter.rs` 不变：商店与手牌本来就只发给本座位；`TripleFormed` 仍只发操作者。
- 意图映射不变。

## 6. 数据与内容

- 新增实体文档 `docs/versions/v0.1/product/content/cards/triple-reward.md`：
  - `card_type: effect`、`tribe: general`、`cost: 0`、`level: 1`、`era: prehistoric`（星级与时代不参与商店与发现，仅为满足字段规范）、`skills: [trigger-discover-tribe-minion]`、`match_text: 同左`。
  - `## 效果` 与技能文本一致：发现一张本局流派的随从。
  - `## 出处`：本作原创的三连奖励结算，无外部出处。
- 不新增技能：奖励牌的发现复用 `trigger-discover-tribe-minion`，等级来源由 §3.1 的上下文决定。
- `card-system.md` §3 计数改为 240 / 效果 37，并补一句：奖励牌不进商店、不进任何候选、不参与三连、不从共享牌池扣除；`cards/README.md` 增加一行索引。
- 文本表自动生成 `card.triple-reward.name` 与 `card.triple-reward.match_text`；客户端另加 `ui.game.reward_card.text`（「发现一张 %d 星随从」）。

## 7. 客户端

- 手牌卡面改为读视图数值：随从牌用 `attack` / `health`，不再读 `CardDB` 的卡面值；`golden` 为真时带金色标识（沿用棋盘 `★`）。
- 奖励牌：卡面显示锁定星级（取 `rewardLevel`）与「发现一张 N 星随从」，不显示时代；普通效果牌展示不变。
- `HandCard.payload` 增加 `golden`，供落点与拖拽预览判断，不参与意图。
- 冒烟测试补充：手牌金色显示继承后的数值；打出金色后手牌出现奖励牌；打出奖励牌后出现三选一浮层。

## 8. 机器人

- `sim/src/ai.rs`：手牌策略按实例状态选牌（金色优先）；`rewardLevel` 非空的奖励牌在棋盘有空位时优先打出，保证端到端覆盖奖励路径。
- 意图与报表结构不变。

## 9. 状态哈希与确定性

- 商店槽哈希加 `instance_id`；手牌改为哈希实例：编号、卡牌、金色、`stats` 全层、`breed`、`breed_applied`、`contagion`、`granted`、`shield_used`、`reborn_used`；效果牌加 `reward_level`。
- `sources` 不参与哈希。实例编号单调递增，重放一致；`make m0-determinism` 必须通过。

## 10. 兼容与迁移

- 无持久化数据，无迁移。
- 协议 `v` 保持 1，视图新增必填字段；旧客户端不存在于线上，不做兼容层。
- 新增效果牌不改变商店抽取口径（已排除），牌池份数不变。

## 11. 验证

| 项 | 方式 |
| --- | --- |
| 实例编号贯穿 | 引擎测试：商店槽编号唯一、买入保留编号、打出后棋盘编号不变、冻结保留、刷新换号 |
| 手牌状态 | 引擎测试：买入随从在手牌带状态；复制品按卡面数值 |
| 三连落点 | 引擎测试：三种来源组合（手牌 / 棋盘 / 混合）金色都进手牌；手牌满时留棋盘且不发奖励 |
| 状态继承 | 引擎测试：成长、繁衍、传染、护盾、已授予关键词都随金色入场；基选取最早编号 |
| 奖励牌 | 引擎测试：打出金色后手牌出现奖励牌；星级 = 来源 +1 封顶 6；打出的奖励牌候选全为该星级；无候选时消耗且不产生待选 |
| 排除 | 引擎测试：商店与发现候选永远不含奖励牌 |
| 哈希 | `make m0-determinism` |
| 协议漂移 | `make protocol-check`、`make protocol-rs-check`、`make data-check` |
| 客户端 | `make client-check`、`make client-smoke` |
| 端到端 | `make m1b-e2e`（机器人会打出奖励牌） |
| 风格 | `make lint`、`make test` |

## 12. 任务拆分

| 阶段 | 任务 | 落点 |
| --- | --- | --- |
| 1 | 实例编号与手牌结构 | `state.rs`、`hash.rs` |
| 1 | 商店、买入、打出、发现、复制、战后回写接入编号 | `recruit.rs`、`effects/*`、`settle.rs` |
| 1 | 视图与协议字段 | `view.rs`、`room-service/src/view.rs`、`protocol/schema`、生成物 |
| 1 | 阶段 1 测试 | `tests/view.rs`、`tests/recruit.rs`、`tests/triple.rs` 适配 |
| 2 | 三连合成与状态继承 | `triple.rs` |
| 2 | 奖励牌发放与发现等级 | `recruit.rs`、`effects/exec.rs`、`triple.rs` |
| 2 | 卡牌数据与排除 | `content/cards/triple-reward.md`、`content/card-system.md`、`content/cards/README.md`、数据生成 |
| 2 | 客户端 | `game.gd`、`game_card_face.gd`、文本表 |
| 2 | 机器人与端到端 | `sim/src/ai.rs` |
| 2 | 文档回写与验证矩阵 | `docs/current/*`、`docs/versions/v0.1/*` |
