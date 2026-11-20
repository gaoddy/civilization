# 冻结跨回合保留，解冻只在当回合（技术）

> 状态：已确认（2026-11-20）
> 适用版本：v0.1
> 产品规则：[product.md](product.md)

## 1. 结论

把「冻结状态」与「解冻权」拆开：

- 冻结状态保存在各槽位 `ShopSlot::frozen` 上，跨回合保留，只由解冻或重抽清除；回合开始刷新沿用原实例编号、卡牌与冻结标记。
- 解冻权保存在玩家状态 `PlayerState::froze_this_turn` 上，冻结时置真、回合开始清零；`UnfreezeShop` 以它为预检条件。
- 视图新增 `affordances.canUnfreeze`，客户端冻结按钮按它决定意图、激活态与 tooltip，不再按「是否存在冻结槽位」推导。

## 2. 规则引擎

`server/crates/rule-engine/src/state.rs`：

- `PlayerState` 新增 `froze_this_turn: bool`，文档注明「解冻只在冻结的当回合可用」。

`server/crates/rule-engine/src/engine.rs`：

- 构造玩家时初始化为 `false`。
- `start_game` 与 `start_turn` 的玩家循环中清零，与 `recruit_done` 同层。

`server/crates/rule-engine/src/recruit.rs`：

- `roll_player(..., preserve_frozen = true)` 的保留分支继续沿用 `existing.instance_id`、`card_id` 与 `frozen = true`；文档注释明确「不解冻就一直保留，刷新换新卡才清除」。
- `freeze` 在置位槽位后把 `player.froze_this_turn` 置真；重复冻结同样是当回合冻结，会重新取得解冻权。
- `check_unfreeze` 在槽位预检之后要求 `froze_this_turn`，否则返回 `INVALID_STAGE`（消息「本回合尚未冻结商店，不能解冻」）。`unfreeze` 不额外处理。
- 手动刷新与升时代仍走 `roll_player(false)`，先清冻结标记再重抽；新卡牌不带冻结状态。

## 3. 视图与协议

`server/crates/rule-engine/src/view.rs`：

- `Affordances` 新增 `can_unfreeze: bool`。
- 计算：`gate.is_none() && player.froze_this_turn && 存在 frozen 槽位`。`gate` 已含阶段、淘汰、`recruit_done` 与队首目标待选，故不可操作时恒为 `false`。

`protocol/schema/v1/view.schema.json`：

- `Affordances` 的 `required` 与 `properties` 增加 `canUnfreeze`，描述为「本回合冻结过商店且仍有冻结槽位；跨回合保留的冻结不算」。

生成物：

- `make protocol-rs` 更新 `protocol/generated/protocol.rs`。
- `make protocol` 更新 `protocol/generated/protocol.gd`、`client/generated/protocol.gd` 与 `docs/versions/v0.1/technical/architecture/protocol-messages.md`（本次仅视图字段表变化，意图不变）。

`server/crates/room-service/src/view.rs`：

- `Affordances` 映射补充 `can_unfreeze: view.affordances.can_unfreeze`。

## 4. 客户端

`client/scenes/game.gd`：

- `_on_freeze()` 读取 `self.affordances.canUnfreeze`：为真发 `INTENT_UNFREEZE_SHOP`，否则发 `INTENT_FREEZE_SHOP`；两者都带 `slot: null`。

`client/scenes/game_shop_view.gd`：

- `_refresh_actions()` 用 `canUnfreeze` 设置按钮激活态与 tooltip；禁用门控（`blocked`、阶段与 `recruited`）不变。
- 商店卡冻结蒙层仍按各槽位 `frozen` 绘制，跨回合保留的卡继续显示。

`client/tests/fixture.gd`：`affordances` 增加 `canUnfreeze`。

## 5. 测试

- `server/crates/rule-engine/tests/recruit.rs`：
  - `frozen_cards_persist_until_manually_thawed`：冻结后过回合，卡牌与实例编号保留且 `frozen` 仍为真、`froze_this_turn` 为假；新回合解冻返回 `INVALID_STAGE`；重新冻结后解冻成功，再下一回合重抽。
  - `reroll_clears_frozen_mark_on_new_cards`：整店冻结后手动刷新，所有槽位 `frozen == false`。
  - 既有 `unfreeze_*` 用例在冻结后解冻，语义不变。
- `server/crates/room-service/tests/game.rs`：
  - `frozen_cards_survive_next_recruit_and_empty_slots_refill`：整店冻结、买一张、倒计时过回合，断言保留卡牌仍冻结、空槽补满、`canUnfreeze` 为假、直接解冻返回 `INVALID_STAGE`；重新冻结后 `canUnfreeze` 为真并解冻成功，再下一回合重抽。
  - `frozen_cards_survive_next_recruit_when_all_end_recruit`：全员手动结束招募路径断言保留卡牌仍冻结、`canUnfreeze` 为假。
- `client/tests/ui_smoke_side.gd`：`canUnfreeze` 为真发 `UnfreezeShop`；跨回合保留（`frozen` 为真、`canUnfreeze` 为假）与无冻结两种情况发 `FreezeShop`，tooltip 同步。

## 6. 文档同步

- `docs/versions/v0.1/product/rules/core-rules.md` §4.1：冻结状态一直保留；解冻只在冻结当回合可用；刷新换新卡清除冻结。
- `docs/versions/v0.1/product/overview/game-intro.md`：同步玩家口径。
- `docs/current/technical/rule-engine-and-simulator.md`：`UnfreezeShop` 预检与实例编号表述。
- `docs/current/technical/client.md`、`docs/current/product/ui/pages/game.md`：按钮按 `canUnfreeze` 切换意图与 tooltip。

## 7. 验证

- `make protocol-rs-check`
- `make protocol-check`
- `make test`
- `make lint`
- `make client-check`
- `make client-smoke`
- `make m1-check`
