# 大厅

> 状态：已实现
> 场景：`client/pages/lobby/lobby.tscn`
> 脚本：`client/pages/lobby/lobby.gd`；玩家卡 `lobby_player_card.gd`（`LobbyPlayerCard`）、段位卡 `lobby_rank_card.gd`（`LobbyRankCard`）、排队卡 `lobby_queue_card.gd`（`LobbyQueueCard`）、快捷操作 `lobby_quick_actions.gd`（`LobbyQuickActions`）、左栏底栏 `lobby_left_actions.gd`（`LobbyLeftActions`）、大厅图 `lobby_hall_image.gd`（`LobbyHallImage`）、座位列表 `lobby_room_list.gd`（`LobbyRoomList`）、房间操作 `lobby_room_actions.gd`（`LobbyRoomActions`）、加入弹窗 `lobby_join_dialog.gd`（`LobbyJoinDialog`）、提示弹窗 `lobby_notice_dialog.gd`（`LobbyNoticeDialog`）
> 公共组件：`FriendsButton`、`FriendsPanel`（`client/ui/social/`）、`ConnectionDialog`（`client/ui/connection_dialog.gd`）
> 验证：`make client-smoke SMOKE=lobby`、`make ui-preview PAGE=lobby`

## 1. 目的与入口

- 目的：登录后的主界面。内容区按 7:3 分为左右两栏：左栏承载大厅图与图鉴 / 排行榜 / 好友入口，右栏承载玩家身份、段位与积分、天梯排队、匹配与房间入口。
- 入口：登录成功后进入；从图鉴、排行榜返回时回到本页；房间等待区被解散、本人被移出，或加入失败时回到本页。
- 出口：匹配成功或开局后进入对局页；宾客创建 / 加入房间成功后进入房间等待区；房主创建房间后留在本页右栏的座位列表；图鉴与排行榜为整页切换；认证类错误回登录页。
- 页面不出现「文明战旗」与「大厅」字样，不做页面标题；连接状态只由断线弹窗表达。

## 2. 结构层级

```text
页面根 Control
├─ PageBackground：软件背景，内容框外整窗铺满
└─ ContentFrame → ContentRoot（设计画布）→ PageBody（PageSurface 语义面板）
   └─ SafeAreaMargin
      ├─ MarginContainer：内边距 space-2xl
      │  └─ LobbyColumns：横向两栏，间距 space-2xl，左右 7:3
      │     ├─ LobbyLeft（7）
      │     │  ├─ LobbyHallImage：大厅图，占满剩余高度，cover 裁切
      │     │  └─ LobbyLeftActions：好友入口（图标 + 在线数）· 图鉴 · 排行榜
      │     └─ LobbyRight（3）
      │        ├─ LobbyPlayerCard：头像缩写 + 名字 + 玩家长编号
      │        ├─ LobbyMiddle：占满剩余高度
      │        │  ├─ LobbyRankCard：段位与积分（默认模式）
      │        │  ├─ LobbyQueueCard：等待时长与排队人数（排队中替换段位卡）
      │        │  └─ LobbyRoomList：8 行座位列表（创建房间模式替换上面两张）
      │        ├─ LobbyQuickActions：开始匹配（主按钮）· 创建房间 / 加入房间
      │        └─ LobbyRoomActions：房主 开始 / 解散；宾客 准备 / 离开（仅创建房间模式）
      └─ 浮层：FriendsPanel（左侧全高抽屉）、LobbyJoinDialog、LobbyNoticeDialog、ConnectionDialog、InvitePopup
页面外挂：LobbyLeaveConfirm（ConfirmationDialog，挂在 PageBody，不参与大厅排版）
```

- 左栏只有大厅图与底栏入口两块；大厅图吸收剩余高度，底栏固定为 `lobby-left-action-height`。
- 右栏从上到下为玩家卡、中部卡片区、快捷操作、房间操作；中部卡片区吸收剩余高度，三种卡片互斥显示。
- 创建房间模式不切页：房主留在本页，右栏中部换成座位列表，底部换成房间操作，快捷操作整体隐藏。
- 宾客加入房间或被邀请接受后进入房间等待区，不在本页显示座位。

## 3. 结构层与模式

### 3.1 默认模式

- 中部显示段位卡：大字段位名与「N 分」，未收到账号视图时显示「暂无段位 / 0 分」，不隐藏卡片。
- 快捷操作显示「开始匹配」主按钮，下一行「创建房间」「加入房间」等宽。
- 左栏底栏显示好友入口、图鉴与排行榜。

### 3.2 匹配模式

- 点「开始匹配」发送入队；排队中主按钮变为「取消匹配」，创建与加入禁用。
- 中部段位卡换成排队卡：等待时长按分:秒刷新（每 0.5 秒），并显示「已凑齐 N/8」；不展示积分匹配范围。
- 匹配成功后排队卡停表显示成功文案，页面随后切到对局页。
- 断线时自动退队并收起排队态；重连后不自动重新排队。

### 3.3 创建房间模式

- 触发条件：阶段为 `Lobby`、房间成员非空且本人是房主。房主创建房间成功后由 `RoomUpdate` 进入本模式。
- 中部座位列表固定 8 行，按座位号 0–7 排列；空位显示「等待加入」占位。
- 每行座位：名字首字头像、显示名、房主标记胶囊、准备圆点；本人行使用金色底强调。
- 准备状态只用圆点表达：已准备为实心成功色，未准备为空心描边；不显示「玩家列表」「已准备 N/8」「已准备 / 未准备」等文字。圆点 tooltip 保留「已准备 / 未准备」说明。
- 房主行显示「房主」胶囊；房主对其他成员行显示「移除」入口，本人行与宾客不显示。
- 房主可见补位提示「开始后补入 N 个机器人」；宾客不显示。
- 列表底部「邀请好友」入口只在 `seat_count < 8` 时存在；满员时直接移除节点。点击以邀请模式打开好友抽屉。
- 底部房间操作：房主只有「开始」与「解散」；宾客只有「准备 / 取消准备」与「离开」。房主默认已准备由服务端保证，本页不给房主准备按钮。
- 开始按钮在未连接、有成员未准备、或补位关闭且不足 8 人时禁用；原因写入按钮 tooltip：未连接服务器、等待所有玩家准备、还差 N 名玩家。
- 点「离开」弹二次确认，确认后发送 `LeaveRoom`、清空房间状态并回到默认模式。

### 3.4 加入弹窗

- 点「加入房间」打开居中弹窗：标题、6 位房间码输入框、错误行、取消与加入按钮；打开时输入框获得焦点。
- 客户端只校验 `^[A-Z0-9]{6}$`；不合法时就地显示「请输入 6 位房间码」且不发送消息。合法时统一转大写并发送 `JoinRoom`。
- 发送后加入按钮与输入框禁用；服务端错误回到弹窗的错误行，页面留在弹窗内，不弹提示弹窗。
- 取消或 Esc 关闭弹窗；点遮罩不关闭，避免误触丢失已输入的房间码。本地联调启动器给出房间码时自动带入并直接加入。

### 3.5 好友抽屉

- 好友入口为公共组件 `FriendsButton`：左侧好友图标、右侧在线好友数，点击打开 `FriendsPanel`。
- 抽屉贴左整高，遮罩透明度 `scrim-alpha-friends`，点遮罩关闭；列表在剩余高度内滚动。
- 邀请模式下每行好友出现邀请按钮，只有在线好友可用；发送 `InviteToRoom` 后状态行显示已邀请对象。普通模式不出现邀请按钮。
- 抽屉与邀请弹窗的具体内容见[组件索引](../components/README.md)与[弹出层外壳](../components/popup-layer.md)。

### 3.6 断线

- `Net.disconnected` 时自动退队、暂停快捷操作，并弹出 `ConnectionDialog`：标题「与服务器断开连接」、说明「未连接服务器」、可用的「重新连接」按钮。
- 点重新连接后按钮禁用、标题变为「正在重新连接…」、隐藏说明；`Net.connected` 后收起弹窗并恢复快捷操作。
- 大厅不设常驻连接状态文本；除断线弹窗外不以其他形式表达连接状态。

### 3.7 通知

- 被移出、房间解散、一般服务端错误使用居中提示弹窗：标题「提示」+ 一句说明 + 「关闭」；点遮罩也可关闭。
- 加入弹窗打开期间的服务端错误不进提示弹窗，而是回到加入弹窗；认证类错误直接回登录页。
- 房间类错误（`ROUTE_LOBBY`）先清空房间状态，再回默认模式并提示。

## 4. 状态清单

| 状态 | 触发 | 表现 | 出口 |
| --- | --- | --- | --- |
| 未连接 | `Net` 未连接 | 断线弹窗；快捷操作与房间操作禁用 | 重连成功后收起弹窗 |
| 重连中 | 点「重新连接」 | 弹窗标题变「正在重新连接…」，按钮禁用 | 连接成功或再次失败 |
| 默认 | 已连接且未排队、无创建房间 | 段位卡 + 快捷操作可见 | 排队、建房、加入或跳转 |
| 排队中 | 点「开始匹配」 | 主按钮变「取消匹配」；段位卡换排队卡；创建与加入禁用 | 取消排队、匹配成功或断线 |
| 匹配成功 | 收到 `MatchFound` | 排队卡停表，显示成功文案 | 进入对局页 |
| 创建中 | 点「创建房间」 | 创建按钮禁用，等待 `RoomUpdate` | 房主进入创建房间模式或收到错误 |
| 创建房间 | 房主 + 房间非空 + `Lobby` 阶段 | 座位列表与房间操作替换段位卡与快捷操作 | 开始、解散、离开、被移出 |
| 有成员未准备 | 房主视角，至少一人未准备 | 开始禁用，tooltip「等待所有玩家准备」 | 该成员准备后恢复 |
| 补位关闭且不足 8 人 | 房主视角，补位关闭且真人少于 8 | 开始禁用，tooltip「还差 N 名玩家」 | 打开补位或等待成员加入 |
| 满员 | 座位 0–7 全部占用 | 邀请入口移除 | 有成员离开后恢复 |
| 加入弹窗 | 点「加入房间」 | 居中弹窗；输入框聚焦 | 取消、Esc、合法提交或服务端错误 |
| 加入中 | 提交合法房间码 | 加入按钮与输入框禁用 | 成功进入房间等待区或错误回到弹窗 |
| 好友抽屉 | 点好友入口或列表邀请入口 | 左侧全高抽屉，点遮罩关闭 | 关闭抽屉或发送邀请 |
| 被移出 | 收到 `RoomClosed{reason:"kicked"}` | 提示「你已被房主移出房间」 | 关闭提示后留在默认模式 |
| 房间解散 | 收到 `RoomClosed{reason:"dissolved"}` | 提示「房间已解散」 | 关闭提示后留在默认模式 |
| 错误 | `Error` 消息 | 提示弹窗显示中文文案 | 按错误码留页、回大厅或回登录 |
| 离开确认 | 创建房间模式点「离开」 | `ConfirmationDialog` 二次确认 | 确认离开或取消 |

- 创建房间模式下不显示段位、排队与快捷操作；宾客视角不存在该模式，进入房间等待区。
- 排队与创建房间互斥：排队中创建 / 加入禁用，创建中不能排队。

## 5. 数据来源

| 数据 | 来源 |
| --- | --- |
| 显示名与玩家长编号 | `GameStore.display_name`、`GameStore.player_code` |
| 段位与积分 | `GameStore.my_tier()`、`GameStore.my_rating()`；账号视图变化时刷新 |
| 在线好友数 | `GameStore.friends[]` 中 `status == "online"` 的条数 |
| 排队状态、人数与等待时长 | `GameStore.queueing`、`GameStore.queue_state`、`GameStore.queue_wait_seconds()` |
| 房间成员、座位与准备状态 | `GameStore.room_players[]` 的 `seat`、`playerId`、`displayName`、`ready`、`isHost` |
| 房主身份与本人准备 | `GameStore.am_host()`、`GameStore.my_ready()` |
| 机器人补位开关 | `GameStore.room_bot_fill` |
| 当前阶段 | `GameStore.phase`；创建房间模式只在 `Lobby` 阶段成立 |
| 房间关闭原因 | `GameStore.room_closed_reason` |
| 最近一次错误 | `GameStore.last_error` |
| 连接状态 | `Net.is_open()` 与 `Net.connected` / `Net.disconnected` |
| 本地联调房间码 | `LocalServer.consume_room_code()`（仅调试包） |

- 页面只做呈现与导航：排队、准备、开始与房间规则以服务端状态为准，本地不推断服务端会接受的操作。
- 段位卡与排队卡直接读全局状态；好友抽屉由组件自己订阅 `GameStore.social_changed`。

## 6. 事件

| 操作 | 请求 / 导航 |
| --- | --- |
| 点开始匹配 | `GameStore.join_queue()` |
| 点取消匹配 | `GameStore.leave_queue()` |
| 点创建房间 | `CreateRoom{}`；成功后房主留在本页 |
| 点加入并提交房间码 | `JoinRoom{code}` |
| 点准备 / 取消准备（宾客） | `Ready{ready: !my_ready}` |
| 点开始（房主） | `GameStore.request_start()` → `StartGame{}` |
| 点解散（房主） | `DissolveRoom{}` |
| 点移除（房主） | `KickMember{targetPlayerId}` |
| 点离开并确认 | `LeaveRoom{}`，随后清空房间并回默认模式 |
| 点好友邀请 | `InviteToRoom{friendId}` |
| 点重新连接 | `Net.relogin()` |
| 点图鉴 / 排行榜 | 导航到图鉴页 / 排行榜页 |
| 收到匹配成功或开局 | 导航到对局页；`HeroPickStarted` 与 `GameStarted` 只导航一次 |

- 所有发送类操作都要求连接可用；断线时按钮禁用，不发送请求。

## 7. 适配

- 页面内容固定 16:9 居中；桌面与 iPad 以 1280×720、iPhone 以 960×540 为设计基准，比例变化只在内容框外露出软件背景。
- 两栏比例在两种档位都保持 7:3；左栏大厅图随高度缩放，按 cover 裁切，不出现信箱边。
- 手机档位右栏高度预算为 508（540 − 上下内边距 2×16）：玩家卡 58 + 间距 8 + 座位列表 319 + 间距 8 + 房间操作 112 ≈ 505，座位行取 28。
- 右栏各卡片、快捷操作与座位行统一走 `UITheme.stage_metric`：手机取 `<令牌>-compact`，桌面取原值。
- 好友抽屉在两种档位都取 600 宽，并夹紧在 `[360, 视口宽]` 内；抽屉整高，列表滚动。
- 触屏端没有 hover：开始禁用原因与准备状态说明通过 tooltip 保留，按钮点击不受影响。

## 8. 关联

- 通用交互：[interactions.md](../interactions.md)
- 设计令牌：[tokens.md](../tokens.md)；图标：[icons.md](../icons.md)
- 组件：[popup-layer.md](../components/popup-layer.md)、[button.md](../components/button.md)、[pill.md](../components/pill.md)
- 房间等待区：[room.md](room.md)
- 房间与邀请规则：[rooms-and-invites.md](../../systems/rooms-and-invites.md)
- 美术素材：[client.md](../../../technical/client.md#10-美术素材层)
