# 整桌断线的对局不结束，房间与账号被占 2–37 分钟

状态：已修复（本地测试通过，待 dev 部署后验证压测样本消失）

## 现象与复现

- 现象：`sim load --connect` 压测补位时，建房机器人收到 `进入房间失败: ALREADY_IN_ROOM 已在房间中`；启动失败率 85–215‰，失败集中在一次压测的前 0.1–94 秒。
- 复现：同一服务端连续两次执行 `sim load --connect ws://… --rooms 10 --duration-ms 120000`，被判阻塞的补位序号逐条一致：`#6, #14–#23, #26, #35, #46–#50, #74, #75`（共 20 个）。
- 触发条件：对局进行中整桌连接同时断开（`ssh -L` 隧道中断、进程被杀、玩家强制退出），且没有任何一方发出 `LeaveRoom`。

## 影响范围

- 残留房间占用房间码与 8 个座位 2–37 分钟（`game_over` 观察日志里 132 局超过 2 分钟，最长 2239 秒、全部 `turns=20`），期间这 8 个账号调用 `CreateRoom` 一律返回 `ALREADY_IN_ROOM`。
- 压测侧：每个新进程的身份序号从 0 重新开始，必然撞上历史残留账号，启动失败率随压测次数累积上升；dev 环境实测残留 20 个房间、160 个账号被占。
- 真实玩家：整桌都退出后，30 秒宽限已过对局仍不结束，想重开一局会被拒，直到原局磨满 20 回合（最长约 37 分钟）。单人掉线时保留座位到对局结束是 v0.1 产品基线（`rooms-and-invites.md`），不属于本缺陷。
- 脏数据：无。房间只存在于服务端内存，进程重启即清空。

## 根因

- `game.rs` 的 `mark_disconnect` 只记录 `disconnected_at_ms`；`expire_disconnects` 到期只把座位标成离线（`connected = false`），不结算、不结束对局。
- `room.rs` 的 `tick_disconnects` 只处理 `Phase::Lobby`，对局中的房间不释放座位。
- 房间只在 `GameOver` 时由 `net.rs` 的投递路径 `rooms.remove()` 删除；没有任何路径判断「整桌都不在线」，对局不会因无人而收束。
- 因此整桌掉线后没人提交意图，对局只能靠阶段截止（招募 60–120 秒/回合）逐个回合推进到第 20 回合收束；期间 `room_of_player` 一直命中，`CreateRoom` 返回 `ALREADY_IN_ROOM`。
- 证据：服务端全程 0 重启，两次独立探测的阻塞序号完全一致；12 分钟服务端日志里 11 个发生过弃权的房间全部是「7 次 `forfeit` + `game_over`」，没有「有弃权无结束」的房间，说明残留房间从未收到过 `LeaveRoom`；`game_over` 观察日志里大量 `durationMs` 达数分钟且 `turns=20`，与「无意图、纯靠阶段截止推进」一致。

## 修复方案

- 修复层次：对局驱动（根因所在层），不改连接层、不在入口加兜底。
- 不改变 v0.1 产品规则：单人断线超过 `RECONNECT_GRACE_MS`（30 秒）仍只标记离线，座位保留到对局结束。
- `GameTask::tick_inner` 每 100ms 检查一次：宽限期到期后若全部座位都不在线（整桌断线且无人重连），`end_abandoned_game` 按规则引擎的弃权结算逐个淘汰存活座位，存活降到 1 时进入 `GameOver`，房间由既有投递路径销毁并向其余座位下发 `PlayerEliminated` 与视图；只要还有一人在线就不收束。
- 主动离房与整桌断线收束共用 `settle_forfeit`，区别只在主动离房登记 `departed`（不再接收任何消息）。
- 兼容：无。当前处于开发阶段，不保留旧行为。

## 验证方式

- 失败用例（修复前失败、修复后通过）：`game::tests::all_disconnects_end_abandoned_game`。
- 回归点：`game::tests::expired_disconnect_keeps_seat`（单人掉线到期不淘汰）、`game::tests::reconnect_within_grace_keeps_seat`（宽限期内重连保持在线、对局不收束）、`cargo test -p room-service`（含 e2e 与断线重连一致性用例）。
- 需要检查的日志：整桌掉线的房间应在约 30 秒后出现 `game_over` 观察日志，不再出现数十分钟的长局；dev 重启后同一 10 房间探测应 0 启动失败。
