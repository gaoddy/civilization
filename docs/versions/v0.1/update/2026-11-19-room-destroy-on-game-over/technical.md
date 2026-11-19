# 对局结束即销毁房间（技术）

> 状态：已确认（2026-11-19）
> 适用版本：v0.1
> 产品规则：[product.md](product.md)

## 1. 结论

对局结束不再把房间重置回等待区，而是删除注册表房间并让 `RoomTask` 自行退出。结算页只读本局已下发的结果；点「确定」不发 `LeaveRoom`，直接清本地状态、重连并回大厅。

## 2. 服务端

`server/crates/room-service/src/net.rs`：

- `deliver_excluding`：`GameOver` 投递完成后
  1. `self.game.take()`；
  2. 取参与者列表后 `self.senders.clear()`；
  3. `rooms.remove(&self.room_id)` 删除房间（房间码随之失效）；
  4. `presence_end_notified` 未置位时通知一次好友在线状态。
  下一轮 `retire_if_done` 检测注册表无房间，摘 `hub` 条目并退出 task。
- 删除 `last_game_over`、`round`、`game_seed()`、`resend_cached_game_over()`、`send_game_over_to()`，以及 `attach`/`reconnect` 大厅分支里的补发调用。
- `start_game` 用 `self.seed`；删除 `last_game_over.clear()`，保留 `presence_end_notified = false`。
- `retire_if_done` 注释改为「房间已销毁即退出」。

`server/crates/room-service/src/room.rs`：

- 删除 `reset_for_next_game` 与其单测；`remove_slot` 仍供 `kick`/`dissolve`/`leave` 使用。
- `Room.seed` 保留：创建时按房间码生成玩法与候选。

`server/crates/room-service/src/game.rs`：

- `departed` 集合与 `is_departed` 保留（弃权后停止推送）；删除无调用者的 `departed_player_ids`。

## 3. 客户端

`client/scenes/result.gd`：

- `_on_confirm_pressed()` 不再发送 `LeaveRoom`；直接 `GameStore.reset_room()`、`Net.relogin()`、导航大厅。
- 删除 `RoomClosedNotice` 节点、`GameStore.room_closed` 连接与 `_on_room_closed()`：房间在对局结束时已销毁，结算页不会收到 `RoomClosed`。

`client/tests/ui_smoke.gd`：

- `_check_result` 删除 `RoomClosedNotice` 断言；按钮与「确定后回大厅」断言保留。

## 4. 测试

- `tests/e2e.rs`：把「结束后回大厅再开下一局」改为「对局结束房间从注册表删除、旧房间码加入失败、重连不补发结果」。
- `tests/reconnect.rs`：结束时重连改为断言 `NOT_IN_ROOM`/`ROOM_NOT_FOUND` 而非缓存 `GameOver`。
- `tests/game.rs`：删除 `departed_player_ids` 断言，保留「已淘汰/终局后弃权返回空更新」。
- `tests/room.rs`：删除 `reset_for_next_game` 三个用例；踢人/解散/离开用例保留。

**验证**：`cargo test -p room-service`、`make protocol-rs-check`、`cargo clippy --workspace --all-targets -- -D warnings`、`make client-smoke`、`make client-check`、`python3 scripts/validate_docs.py`。
