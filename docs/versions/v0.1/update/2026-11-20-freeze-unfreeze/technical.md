# 冻结后可再次按下解冻（技术）

> 状态：已确认（2026-11-20）
> 适用版本：v0.1
> 产品规则：[product.md](product.md)

## 1. 结论

新增协议意图 `UnfreezeShop { slot }`，与 `FreezeShop` 对称，不把 `FreezeShop` 改成取反。客户端冻结按钮按视图中的 `frozen` 状态选择发送 `FreezeShop` 或 `UnfreezeShop`，两者都发 `slot: null`；服务端与规则引擎对称处理。

## 2. 协议

`protocol/schema/v1/intents.schema.json`：在 `FreezeShop` 分支后新增 `oneOf` 分支。

- `kind`：`"UnfreezeShop"`。
- `slot`：`integer | null`，范围 0–5，`null` 表示全部槽位；与 `FreezeShop` 一致，`slot` 不放进 `required`。

生成物：

- `make protocol-rs` 更新 `protocol/generated/protocol.rs`。
- `make protocol` 更新 `protocol/generated/protocol.gd` 与 `client/generated/protocol.gd`，新增 `INTENT_UNFREEZE_SHOP`。

## 3. 规则引擎

`server/crates/rule-engine/src/intent.rs`：

- 新增 `Intent::UnfreezeShop { slot: Option<u8> }`。
- 枚举文档的意图数量改为十三种。

`server/crates/rule-engine/src/recruit.rs`：

- `recruit::apply` 分派 `Intent::UnfreezeShop { slot } => unfreeze(state, seat, slot)`。
- 抽出与 `check_freeze` 共用的槽位校验：指定槽位必须非空，未指定槽位不校验。`check_freeze` 与新增的 `check_unfreeze` 都调用它，保持只读、不消耗随机流。
- `unfreeze`：指定槽位时把该槽位 `frozen` 置 `false`；未指定时遍历 `shop.iter_mut().flatten()` 全部置 `false`。不重抽、不改实例编号、不发事件。
- 阶段、`recruit_done`、淘汰与目标待选的统一守卫沿用 `recruit::apply`，不在 `unfreeze` 内重复。

## 4. room-service

`server/crates/room-service/src/game.rs`：`Protocol::UnfreezeShop { slot }` 映射为 `EngineIntent::UnfreezeShop { slot: slot.map(to_u8).transpose()? }`。

集成测试 `server/crates/room-service/tests/game.rs` 覆盖冻结后解冻、视图 `frozen` 回到 `false`。

## 5. 模拟器

`server/crates/sim/src/report.rs`：

- `intent_to_json` 输出 `{"kind": "unfreeze_shop", "slot": ...}`。
- `intent_from_json` 解析 `"unfreeze_shop"`。

机器人策略不产生该意图，确定性基线不受影响。

## 6. 客户端

`client/scenes/game.gd`：

- `_on_freeze()` 读取 `GameStore.view.self.shop`：任一槽位 `frozen` 为真时发送 `INTENT_UNFREEZE_SHOP`，否则发送 `INTENT_FREEZE_SHOP`；两者都带 `slot: null`。
- 现有按钮禁用门控（`blocked`、阶段与 `recruited`）不变。

`client/scenes/game_shop_view.gd`：

- 激活态继续按「任一槽位冻结」判断。
- tooltip 在存在冻结槽位时用 `ui.game.side_panel.unfreeze`，否则用 `ui.game.side_panel.freeze`。

`client/text/zh_CN.csv`：新增 `ui.game.side_panel.unfreeze,解冻`；`make client-import` 重新导入翻译。

## 7. 文档同步

- `docs/versions/v0.1/product/rules/core-rules.md`：§3 意图表新增 `UnfreezeShop`；§4.1 补再按一次解冻的规则。
- `docs/versions/v0.1/technical/architecture/protocol.md`：意图数量改为十三种。
- `docs/current/technical/rule-engine-and-simulator.md`：意图数量、清单与测试矩阵同步。
- `docs/current/technical/client.md`：意图数量与冻结按钮行为同步。
- `docs/current/product/ui/pages/game.md`：冻结按钮状态与 tooltip 同步。
- `docs/versions/v0.1/product/overview/game-intro.md`：补充再次按下解冻的说明。

## 8. 测试

- `rule-engine/tests/recruit.rs`：
  - 冻结多个槽位后 `UnfreezeShop { slot: None }`，全部 `frozen == false`，卡牌与实例编号不变。
  - 冻结两个槽位后只解冻 0 号，1 号仍冻结。
  - 解冻空槽位返回 `SHOP_SLOT_EMPTY`，状态哈希不变。
  - 冻结后解冻再 `start_turn`，原槽位不再保留。
  - 预检表新增解冻空槽用例。
- `room-service/tests/game.rs`：冻结后解冻，视图 `frozen == false`。
- `client/tests/ui_smoke_side.gd`：冻结夹具按按钮发 `INTENT_UNFREEZE_SHOP`，无冻结夹具发 `INTENT_FREEZE_SHOP`；tooltip 随状态切换；冻结激活态断言保留。

## 9. 验证

- `make protocol-rs-check`
- `make protocol-check`
- `make test`
- `make lint`
- `make client-check`
- `make client-smoke`
- `make m1-check`
