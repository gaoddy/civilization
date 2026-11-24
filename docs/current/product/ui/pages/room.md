# 房间等待区

> 状态：已实现
> 场景：`client/pages/room/room.tscn`
> 脚本：`client/pages/room/room.gd`；座位矩阵 `room_seat_grid.gd`（`RoomSeatGrid`）、座位格 `room_seat_cell.gd`（`RoomSeatCell`）、底部操作栏 `room_action_bar.gd`（`RoomActionBar`）、邀请行 `room_invite_row.gd`（`RoomInviteRow`）
> 验证：`make client-smoke`

## 1. 目的、入口与返回

- 目的：在房主开始前承载 8 座等待区。房主默认已准备、不显示准备按钮；其他成员准备或取消准备。房主控制开始、机器人补位、移除与解散；所有成员可离开。选英雄不在本页，开始后进入对局页的独立选英雄阶段。
- 入口：大厅创建房间或加入房间成功后收到 `RoomUpdate` 进入；断线重连回到 `Lobby` 阶段时同样进入本页。
- 返回：收到 `HeroPickStarted` 后导航到对局页；`NOT_IN_ROOM` / `ROOM_NOT_FOUND` 回大厅；被房主移出或房间解散时按 `RoomClosed.reason` 提示原因后回大厅；认证类错误回登录。本页提供主动离开房间入口。
- 本页不显示房间码，也不展示任何英雄候选；房间码只在大厅的创建与加入入口使用。

## 2. 结构层级

```text
软件背景 PageBackground
└─ 页面容器 PageBody → PageSurface（语义面板，设计画布尺寸）
   └─ RoomPanel：卡片面板（surface，铺满整屏）
      └─ RoomColumn：纵向容器（内边距 space-lg，间距 space-lg）
         ├─ RoomTopBar：标题 + RoomMeta（「已准备 N/8 · 机器人补位 开/关」）
         ├─ RoomSeatGrid：4 列 2 行固定 8 格
         │  └─ RoomSeatCell ×8：头像 + 名字 + 徽标行
         ├─ BotFillHint：仅房主可见，显示「开始后补入 N 个机器人」
         ├─ RoomInviteRow：「邀请好友」标签 + 在线好友按钮行
         └─ RoomActionBar：左组 准备/取消准备（仅宾客）·（Steam 构建）邀请 Steam 好友 · 离开房间；中间状态行；右组 机器人补位 · 开始 · 解散房间
离开确认：ConfirmationDialog（标题「离开房间」，确定「离开」，取消「关闭」）
```

- 8 个座位固定按座位号 0–7 升序填充，4 列 2 行；客户端不重排座位。
- 空位显示虚线边框、加号头像与「等待加入」胶囊；已加入成员显示名字首字头像、名字与徽标。
- 顶部状态栏只表达准备进度与补位开关状态，不显示房间码。
- 底部操作栏左右分组，中间状态行吸收剩余宽度；状态行同时承载错误、邀请结果、移除/解散提示与开始禁用原因。
- 机器人不提前占座：机器人只在房主开始后由服务端补入，开局后显示在对局页；本页不会出现机器人座位。

## 3. 座位格

- 头像为 26×26 圆盘，显示名字首字；本人头像与头像底色使用 `color-accent`，他人使用 `color-surface-sunken`，空位为透明底加号。
- 本人座位整格使用 2px `color-accent` 描边与 8% 金色底的抬升面板；他人使用普通抬升面板；空位不铺底色、边框为虚线。
- 已加入成员的名字单行居中，超出省略。
- 徽标行：房主显示「房主」胶囊（accent），所有已加入成员显示「已准备」（success）或「未准备」（neutral）胶囊；房主由服务端保证默认已准备，因此房主徽标始终为「已准备」；空位不显示徽标行。
- 房主且阶段为 `Lobby` 时，其他已加入成员的座位格底部出现「移除」小按钮 `Kick_<playerId>`；本人座位、空位与非房主不显示。
- 座位状态共四种：本人、已准备、未准备、空位；本人与其他状态可叠加，本人仍显示准备徽标。

## 4. 底部操作栏

- 左组：
  - 准备/取消准备：仅宾客显示；房主默认已准备由服务端保证，等待区不渲染房主准备按钮，也不发送 `Ready`。宾客按钮按 `ready` 切换文案，断线时禁用；每次点击发送 `Ready{ready: !my_ready}`。
  - 邀请 Steam 好友：只在 Steam 构建创建；已连接、等待区、大厅可用且没有进行中的邀请时可点。点击先向服务端请求邀请 token，再创建 Steam 大厅并打开好友邀请；失败时提示改用其他方式。
  - 离开房间：所有成员可见；点击先弹二次确认，确认后发送 `LeaveRoom`、清空房间状态并回大厅。
- 右组：
  - 机器人补位：所有成员可见，显示「机器人补位 开/关」，使用金色描边按钮；仅房主可点，非房主禁用。点击发送 `SetBotFill`。
  - 开始：仅房主可见；满足开始条件且已连接时可用。点击调用 `GameStore.request_start()` 发送 `StartGame`，不做二次确认。
  - 解散房间：仅房主可见；发送 `DissolveRoom`。
- 中间状态行：
  - 断线时显示「未连接服务器」。
  - 有真人未准备时显示「等待所有玩家准备」。
  - 补位关闭且真人不足 8 时显示「补位已关闭，还差 N 名玩家」。
  - 开始按钮的禁用原因同时写入状态行与按钮 tooltip：触屏端靠状态行，桌面端可悬停查看。
  - 错误文案最后写入并在下一次房间状态变化前保持；错误详情通过 tooltip 给出协议原文。
- 邀请结果、被移出与房间解散提示也写入状态行，然后执行对应导航。

## 5. 状态清单

| 状态 | 触发 | 表现 | 出口 |
| --- | --- | --- | --- |
| 未连接 | `Net` 未连接 | 状态行「未连接服务器」；准备、开始、补位、Steam 邀请禁用 | 自动重连后恢复 |
| 空房间 | 只有房主或成员陆续加入 | 其余座位显示「等待加入」；补位开启时开始可用 | 等待成员加入或房主开始 |
| 本人未准备 | 宾客且 `my_ready` 为假 | 准备按钮显示「准备」；本人徽标为「未准备」 | 点准备或房主开始 |
| 本人已准备 | `my_ready` 为真；房主始终为该状态 | 宾客准备按钮显示「取消准备」；本人徽标为「已准备」；房主不显示准备按钮 | 点取消准备或房主开始 |
| 有人未准备 | 至少一名真人未准备 | 房主「开始」禁用，状态行「等待所有玩家准备」 | 该成员准备后恢复 |
| 补位关闭且人数不足 | 真人少于 8 且补位关闭 | 「开始」禁用，状态行「补位已关闭，还差 N 名玩家」 | 打开补位或等待成员加入 |
| 可开始 | 真人全部准备且补位开启或已有 8 名真人 | 房主「开始」可用；补位提示显示将补入人数 | 点开始进入对局页选英雄 |
| 非房主 | 本人不是房主 | 不渲染开始与解散按钮；补位开关可见但禁用；其他成员座位无移除按钮 | 等待房主操作 |
| 邀请好友 | 点击在线好友按钮 | 状态行「已邀请 X」；在线好友可点，游戏中/离线置灰；无好友时显示「暂无好友可邀请」 | 服务端回写接受或拒绝结果 |
| Steam 邀请 | Steam 构建点击邀请入口 | 状态行「正在创建邀请…」，按钮置灰；成功后打开 Steam 好友邀请，失败提示不可用 | 邀请完成或失败后恢复 |
| 被移出 | 收到 `RoomClosed{reason:"kicked"}` | 状态行「你已被房主移出房间」，随后回大厅 | 大厅 |
| 房间解散 | 收到 `RoomClosed{reason:"dissolved"}` | 状态行「房间已解散」，随后回大厅 | 大厅 |
| 开始游戏 | 收到 `HeroPickStarted` | 离开 Steam 大厅并导航到对局页 | 对局页选英雄阶段 |
| 错误 | `Error` 消息 | 状态行中文文案，tooltip 为协议原文 | 按错误码留页、回大厅或回登录 |

- 同一时刻以房间状态与连接状态共同决定按钮可用性；页面不自行推断服务端会接受的操作。
- 开始条件只做本地呈现；点击后仍以服务端校验为准，拒绝时按错误码提示。

## 6. 数据来源

| 数据 | 来源 |
| --- | --- |
| 座位号、成员、名字与准备状态 | `GameStore.room_players[]` 的 `seat`、`playerId`、`displayName`、`ready` |
| 房主身份 | `room_players[].isHost`（协议同时下发 `hostId`） |
| 本人身份与准备 | `GameStore.player_id`、`GameStore.am_host()`、`GameStore.my_ready()` |
| 机器人补位开关 | `RoomUpdate.botFill` → `GameStore.room_bot_fill` |
| 当前阶段 | `GameStore.phase`；房主管理控件只在 `Lobby` 渲染 |
| 好友列表与在线状态 | `GameStore.friends`；邀请行按 `status` 置灰 |
| 房间关闭原因 | `GameStore.room_closed_reason` |
| Steam 邀请 token | `GameStore.room_invite` 的 `token`、`roomCode`、`expiresAt` |
| 连接状态 | `Net.is_open()` 与 `Net.connected` / `Net.disconnected` |

- 页面不做座位分配、准备状态或开始条件的服务端规则推导；本地只计算用于禁用按钮与显示原因的即时状态。
- 开始后页面不再显示本局对局数据，全部交给对局页。

## 7. 事件

| 操作 | 请求 |
| --- | --- |
| 点准备/取消准备 | `Ready{ready: !my_ready}`；仅宾客 |
| 点开始 | `GameStore.request_start()` → `StartGame{}`；仅房主 |
| 点机器人补位 | `GameStore.set_bot_fill(!room_bot_fill)` → `SetBotFill{enabled}`；仅房主 |
| 点离开并确认 | `LeaveRoom{}`，随后清空房间并回大厅 |
| 点移除 | `KickMember{targetPlayerId}`；仅房主、仅等待区、不能移除自己 |
| 点解散 | `DissolveRoom{}`；仅房主、仅等待区 |
| 点好友邀请 | `InviteToRoom{friendId}` |
| 点 Steam 邀请 | 创建房间邀请 token，再打开 Steam 好友邀请 |

- 开始、补位、移除、解散与邀请都要求连接可用；断线时按钮禁用，不发送请求。
- 开始后服务端推进到选英雄阶段并下发 `HeroPickStarted`，页面据此导航，不等待 `GameStarted`。

## 8. 适配

- 页面内容固定 16:9 居中，桌面与 iPad 以 1280×720、iPhone 以 960×540 为设计基准；窗口比例变化只在内容框外露出软件背景。
- 座位矩阵随可用宽高等分 4 列 2 行；手机档位沿用同一矩阵，不切换为纵向列表。
- 触屏端没有 hover：开始禁用原因写入状态行，补位数量也以 `BotFillHint` 常驻显示；桌面端同时提供 tooltip。
- 邀请行在窄屏下按可用宽度压缩，按钮保持可点尺寸。
- 本页不显示房间码，因此没有复制房间码的入口。

## 9. 关联

- 通用交互：[interactions.md](../interactions.md)
- 组件：[pill.md](../components/pill.md)、[button.md](../components/button.md)、[card-panel.md](../components/card-panel.md)
- 系统规则：[房间与邀请](../../systems/rooms-and-invites.md)
- 对局页：[game.md](game.md)
- 结算页：[result.md](result.md)
