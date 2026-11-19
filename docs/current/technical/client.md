# Godot 客户端

> 状态：已实现（v0.1 M1b + 首版剩余四项：赛后回顾、房间生命周期、首局引导、macOS 与 iOS 导出）
> 代码：`client/`
> 验收：`make client-check`、`make client-smoke`、`make m1b-e2e`、`make client-export`、`make client-export-ios`

本文记录 v0.1 已实现的 Godot 客户端现状：工程结构、全局状态与网络层、六个场景、生成物与运行方式；不描述服务端房间服务（见[房间服务与联网](room-service/README.md)）与产品规则（见[规则引擎与模拟器](rule-engine-and-simulator.md)）。

## 1. 工程结构

```
client/
  project.godot              # 主场景 login.tscn；autoload：ScreenProfile、WindowSizer、GameStore、CardDB、Net
  export_presets.cfg         # macOS 预设（Universal、未签名）与 iOS 预设（开发签名）
  icon.png                   # 应用图标 1024×1024；iOS 导出按尺寸缩放生成
  .gitignore                 # 忽略 .godot/ 与 *.uid
  autoload/
    screen_profile.gd        # 设备类型、逻辑基准、pt→画布换算、安全区与键盘边距
    window_sizer.gd          # 桌面端启动时按屏幕可用区域 90% 设置窗口并居中；触控端跳过
    game_store.gd            # 认证、房间、PlayerView、赛后统计与引导进度的全局状态
    ws_client.gd             # WebSocketPeer、自动重连、信封编解码与分发
    card_db.gd               # 只读卡牌/技能展示表与科学分类树；展示文本按文本 id 从文本表取
  text/
    zh_CN.csv                # 唯一的运行时中文文本表（生成物 + 人工 key）
  assets/
    fonts/NotoSansSC-Regular-subset.ttf  # 默认主题字体（中文子集）；美术素材见 §10
  ui/
    ui.gd                    # 场景共用控件工具、共享构件与统一导航入口（class_name UI）
    safe_area.gd             # 页面安全区包裹（class_name SafeAreaMargin），触控端叠加键盘边距
    theme.gd                 # 三层视觉令牌与 StyleBox 工厂（class_name UITheme）
  scenes/
    login.tscn/.gd           # 登录
    lobby.tscn/.gd           # 大厅
    invite_popup.gd          # 邀请弹层：订阅 social_changed、逐条接受/拒绝（class_name InvitePopup）
    room.tscn/.gd            # 房间
    game.tscn/.gd            # 对局页面装配入口：顶部工具条、左栏与中央舞台，门控与意图
    game_ui_state.gd         # 一次刷新的页面快照：视图数据与门控结果
    game_card_face.gd        # 卡面：介绍、费用/时代角标、冻结蒙层与随从详情分层取值；类型行显示玩法
    game_detail_layer.gd     # 详情层：商店、座位、售货员与英雄四类详情及定位；随从详情含分类、关联与食物链
    game_top_bar.gd          # 顶部工具条：状态文本与图鉴、提示、退出对局（class_name GameTopBar）
    game_players_rail.gd     # 左栏座位列：头像、名次、血条与战报展开
    game_shop_view.gd        # 商店：售货员两侧的图标操作行、6 槽与出售落点（class_name GameShopView）
    game_icon_button.gd      # 40×40 图标按钮，费用角标在右上、次数角标在左上（class_name GameIconButton）
    game_portrait.gd         # 64×64 无图头像块：玩法色底与名字首字（class_name GamePortrait）
    game_river.gd            # 河道行：回合数、招募倒计时/结束招募、战斗结果（class_name GameRiver）
    game_hero_row.gd         # 英雄行：身高 84，英雄卡、金币与英雄技能列（class_name GameHeroRow）
    game_hero_card.gd        # 英雄卡：64×84，封面、名字与攻击/时代/生命三格（class_name GameHeroCard）
    game_hand_row.gd         # 贴底手牌行：80 步进重叠、整行居中（class_name GameHandRow）
    game_opponent_view.gd    # 战斗对手身份：头像、名字、英雄、血量条与幽灵（class_name GameOpponentView）
    game_recruit_view.gd     # 招募视图：商店、河道与我方棋盘
    game_combat_view.gd      # 战斗视图：对手身份、对手随从行、河道与我方随从行
    hand_card.gd             # 手牌卡：拖拽数据与预览（class_name HandCard）
    hand_slot.gd             # 手牌槽：80 步进重叠、悬停缩放与层级（class_name HandSlot）
    board_drop.gd            # 我方棋盘落点：命中判定与插入下标（class_name BoardDrop）
    combat_playback.gd       # 战斗动作落地器：节奏与状态应用（class_name CombatPlayback）
    combat_fx.gd             # 战斗演出覆盖层：碰撞往返、闪光、飘字、淡出与出现（class_name CombatFx）
    result.tscn/.gd          # 结算：名次、本人本局数据、最终阵容、种子与返回入口
    first_game_guide.gd      # 首局非阻塞引导：目标高亮、底部提示条与本地 seen 记录（class_name FirstGameGuide）
    gallery.tscn/.gd         # 卡牌图鉴页：大厅入口，浏览全部卡牌与 Token
    card_gallery.gd          # 图鉴主体：头部、章节栏、目录、网格与常驻详情（class_name CardGallery）
    gallery_chapter_bar.gd   # 图鉴时代章节栏：全部 + 7 时代单选与计数（class_name GalleryChapterBar）
    gallery_toc.gd           # 图鉴界级分类目录：全部 + 8 界单选与计数（class_name GalleryToc）
    gallery_grid.gd          # 图鉴卡面网格与分页：5 列、每页 40（class_name GalleryGrid）
    gallery_card_tile.gd     # 图鉴卡面单元：复用 GameCardFace，选中/悬停描边（class_name GalleryCardTile）
    gallery_detail.gd        # 图鉴常驻详情：大卡面、分类、玩法、数值、技能与效果（class_name GalleryDetail）
    card_pool.gd             # 本局卡池复算：与服务端 card_in_pool 同规则，排除 Token（class_name CardPool）
  tests/
    ui_smoke.tscn/.gd        # 主冒烟：五场景实例化、拖拽/浮层/门控/结算等主流程
    ui_smoke_side.tscn/.gd   # 舞台结构：商店操作行、英雄行、河道与手牌
    ui_smoke_seats.tscn/.gd  # 座位行、名次角标、血量条与战报展开
    ui_smoke_combat.tscn/.gd # 战斗视图三段与结果横幅
    ui_smoke_cards.tscn/.gd  # 商店卡费用/时代角标与冻结蒙层
    ui_smoke_text.tscn/.gd   # 文本表取用、枚举展示与错误文案回退
    ui_smoke_social.tscn/.gd # 大厅好友区、邀请弹窗与房间邀请入口
    ui_smoke_gallery.tscn/.gd # 图鉴页与对局覆盖层：计数、筛选、分页、详情与入口
    ui_smoke_guide.tscn/.gd  # 首局引导：首次展示、步骤推进与条件跳过、跳过落盘、手动重看、输入穿透、重连续接
    ui_smoke_room_ops.tscn/.gd # 房间管理：房主移除/解散、非房主隐藏、RoomClosed 回大厅、离线点击
    ui_smoke_touch.tscn/.gd  # 触控：长按详情、抬手隐藏、位移取消、手牌槽触摸复位
    ui_smoke_layout.tscn/.gd # 安全区包裹与命中尺寸；960×540 下六页面不越界
    ui_smoke_platform.tscn/.gd # 设备换算：画布缩放、pt→画布、安全区与键盘（固定输入断言）
    smoke_case.gd            # 冒烟公共断言与场景装载
    fixture.gd               # 冒烟用最小 PlayerView
    headless_game.tscn/.gd   # 无头对局驱动（1 客户端 + 7 机器人）
  generated/
    protocol.gd              # schema 生成的 GDScript 常量
    cards.json               # 产品文档生成的卡牌展示表
    skills.json              # 技能展示表（effects 为结构化数组，仅展示不解析）
    tokens.json              # Token 展示表
    taxonomy.json            # 分类树生成物：{nodes, playstyles}
```

窗口启动时由 `WindowSizer` 按当前屏幕可用区域（`screen_get_usable_rect`，已排除菜单栏与 Dock）的 90% 设置尺寸并居中；无头模式下跳过。逻辑视口固定 1280×720，窗口尺寸变化由 `canvas_items` 等比缩放，页面不感知实际窗口大小。

## 2. 全局状态与网络层

- `GameStore`：保存 `playerId`、`playerCode`、`token`、`roomId`、`roomCode`、房间成员、`PlayerView`、`lastSeq`、排名、种子、本人赛后统计 `self_stats`、`RoomClosed` 原因 `room_closed_reason`、引导步骤 `guide_step`，以及社交视图 `friends` / `incoming_requests` / `outgoing_requests` / `recent_players` / `pending_invites`。`handle(type, payload)` 只做字段搬运，发出 `auth_changed`、`room_changed`、`view_changed`、`game_started`、`game_over`、`room_closed`、`error_received`、`social_changed`、`invite_received`、`invite_result_received` 信号；`FriendUpdate` 整段替换社交列表，`FriendInvite` 按 `inviteId` 去重后入队，`InviteAccepted` 移除邀请并自动补发 `JoinRoom{roomCode}`。`GameOver` 写入 `ranking`、`game_seed` 与 `selfStats`；`RoomClosed` 先暂存 `reason`，`reset_room()` 后写回 `room_closed_reason`、补发 `LeaveRoom` 并发出 `room_closed`。`reset_room()` 清空房间与对局状态，`clear_game_view()` 只清本局视图（保留房间与社交），`reset_social()` 清空社交，`reset_session()` 两者都清并清空身份与令牌。
- `Net`：持有一个 `WebSocketPeer`，`_process` 中轮询；连接建立后先认证，断线每 3 秒自动重连。重连时以令牌发 `AuthRequest`，若仍在房间再发 `Reconnect{roomId, lastSeq}`，由服务端回全量 `Snapshot`。
- 下行信封只校验 `v == 1` 与 `type`，`payload` 原样交给 `GameStore`；未知类型忽略。上行信封为 `{v, id, type, ts, payload}`，`id` 由时间戳与计数器生成。
- `CardDB`：启动时读取 `client/generated/{cards,skills,tokens,taxonomy}.json`。卡名、技能文本与时代名不放在生成数据里，而是按文本 id 从 `client/text/zh_CN.csv` 取：`display_name` 取 `card.<id>.name`（Token 取 `token.<id>.name`），`display_text` 取 `skill.<id>.text`，`era_name` 取 `enum.era.<id>`。不参与任何规则判定。
- `CardDB` 的玩法与分类接口：
  - `taxonomy`：节点 id → `{id, parent, rank}` 的字典，来自 `generated/taxonomy.json` 的 `nodes`；
  - `playstyle_name(id)`：取 `enum.playstyle.<id>`，空 id 返回空串，缺键回退为 id；
  - `taxon_name(id)`：取 `enum.taxon.<id>`，空 id 返回空串，缺键回退为 id；
  - `taxon_path(id)`：从叶子沿 `parent` 上溯到根（最多 7 层），返回「界 → 科」顺序的展示名数组；未知节点或空 id 返回空数组。
- `skill_name(id)`：取 `skill.<id>.name`，空 id 或缺键返回空串，避免内部技能 id 泄露到界面；卡牌详情与图鉴详情据此跳过没有展示名的技能。
- `cards.json` 含 `playstyle` / `taxon` / `specialty`；`tokens.json` 的条目并入 `CardDB.cards`，因此 Token 卡面同样可按自身玩法与分类展示。

## 3. 六个场景

| 场景 | 职责 |
| --- | --- |
| 登录 | 输入服务器地址、手机号、固定验证码 `123456` 与显示名；认证成功后进入大厅 |
| 大厅 | 展示本人 6 位玩家编号；创建房间或凭房间码加入；按编号添加好友、处理收到的申请、删除好友、从最近队友回加；有待处理邀请时弹出邀请浮层，接受后自动入房；收到 `RoomUpdate` 后进入房间 |
| 房间 | 展示成员确认状态、选择英雄并一键确认；列出在线好友供一键邀请（对局中/离线置灰）；开局完全由服务端决定（全员准备后自动开始）；房主且等待区时每名成员行带 `Kick_<playerId>`，底部 `DissolveButton`，收到 `RoomClosed` 后按原因提示并回大厅 |
| 对局 | 酒馆战棋式布局：顶部工具条左侧为状态文本（断线、未发出、错误），右侧为 `GalleryButton`、`HintButton`、`LeaveGameButton`；左栏 8 行座位（按服务端 `rank` 升序，头像、名次角标、血量条，点击展开战报）；中央舞台自上而下为招募组合或战斗组合、英雄行与贴底手牌。招募组合 418 高：商店操作行（升时代图标—售货员—刷新/冻结图标）、商店 6 槽、河道、按容量棋盘；战斗组合同样 418 高：对手身份、对手随从行、河道（回合与结果）、我方随从行。英雄行 84 高：64×84 英雄卡、金币格与英雄技能图标，淘汰后显示「已淘汰 · 观战中」；手牌行 146 高贴底。十一种意图对应操作；`pendingChoices` 弹层（发现/进化/战吼目标，候选由视图下发）；英雄技能按 `requiresTarget` / `targetCandidates` 直接释放或弹出候选浮层 |
| 结算 | 按服务端 `ranking` 展示名次行（名次、玩家名、存活回合、造成伤害）、`selfStats` 的本局数据区与最终阵容、`seed`；`ConfirmButton`（「确定」）只清本地房间并重新认证回大厅，不发 `LeaveRoom`（房间已在对局结束时销毁） |
| 卡牌图鉴 | 只读浏览卡牌与 Token：时代章节 + 界级分类两维筛选、5 列网格每页 40 张、常驻详情；大厅入口用全量卡表（当前 409 卡 + 23 Token），对局入口以覆盖层只列本局卡池 |

- 结算页节点：`ResultScroll` / `ResultContent` / `RankingList` / `StatsSection` / `BoardList` / `ResultActions` / `ConfirmButton`。`StatsSection` 共 12 个 `StatRow_*`（战斗胜/平/负合为 `StatRow_combat`），缺字段按 0、空阵容显示一条「无随从」占位；页面只读取 `GameStore.ranking` / `self_stats` / `game_seed`，不推算；不监听 `RoomClosed`，房间已在对局结束时销毁。
- 房间管理：`room.gd` 仅在 `_is_host()` 且 `GameStore.phase == PHASE_LOBBY` 时渲染 `Kick_<playerId>`，`DissolveButton` 随房主身份显隐；点击发 `KickMember{targetPlayerId}` / `DissolveRoom{}`。收到 `RoomClosed` 时 `GameStore` 清房间并补发 `LeaveRoom`，`room.gd` 把 `kicked` / `dissolved` 写成 `ui.room.kicked_notice` / `ui.room.dissolved_notice` 后导航大厅。
- 对局内退出：顶部工具条 `LeaveGameButton` 发 `leave_game_pressed`，`game.gd` 发送 `LeaveRoom`、收起引导、`reset_room()`、`Net.relogin()` 并回大厅；服务端按弃权结算，退出座位保留到本局结束。
- 首局引导：`FirstGameGuide` 由 `game.gd` 在对局页装配后创建（`target_root` 为对局页），`_refresh` 调 `set_phase(phase, self_state)`，顶部工具条 `HintButton` 调 `show_manual()`；步骤、节点名与本地存储见[首局引导组件](../product/ui/components/first-game-guide.md)。
- 对局界面整块读取 `GameStore.view`，每次 `view_changed` 重建；按钮可用性只取 `affordances` / `playable` / `can*`，不做本地推导。冻结与棋盘卡拖拽换位没有对应的服务端 affordance，按 `phase`、`self.recruited` 与下标做本地门控，与服务端拒绝条件保持一致。
- 操作还要求视图自身的 `turn` 与 `GameStore.turn` 一致：新回合的 `PhaseChanged` 会先于新视图到达，旧视图只展示、不可操作，避免用上一回合的商店发意图。
- 倒计时由 `phaseDeadline` 计算；归零后结束招募、刷新、购买、出牌、出售与换位一并禁用，只等服务端推进阶段。
- 已结束招募（`self.recruited`）后只展示界面，不再发起意图，也不弹待选弹层。
- 状态栏文案按错误码取 `UI.error_text` 的文本表条目（`error.<CODE>`）；悬停详情优先用服务端下发的 `messageId` 与 `messageParams`（`UI.error_tooltip` → `UI.error_text_by_id`），缺失时回退错误码文案。协议 `message` 只面向日志，不作为 UI 文案。
- `UI.error_route` 决定错误后的去向：认证类错误回登录并清空会话，`NOT_IN_ROOM` / `ROOM_NOT_FOUND` 回大厅并清空房间；其余错误留在当前页面。
- 未连接时点击操作不会静默丢弃：状态栏提示「未连接，操作未发出」。客户端不维护目标重试表：目标只能取自视图候选，被拒后只提示错误并等待新视图。
- 页面切换统一走 `UI.navigate(page, path)`：`change_scene_to_file` 会立即把当前场景摘下，而页面仍连着 `GameStore` 等 autoload 信号；摘下后的回调再取 `get_tree()` 会得到 `null`。`UI.navigate` 以「页面仍在树上」为唯一判定，离树页面的导航直接丢弃并返回 `false`，场景脚本不再直接调用 `get_tree().change_scene_to_file`。
- 房间场景同时监听 `view_changed`：重连或晚到的视图显示对局已开始时直接进入对局；房间不存在或已不在房间时回大厅。
- 大厅好友区读取 `GameStore` 的社交列表，不做本地排序：好友顺序、在线状态与最近队友次序都由服务端给出。添加好友按 6 位编号（`FriendRequest{toPlayerCode}`）；申请条目提供同意/拒绝，好友条目提供删除；编号或输入非法时只在状态栏提示，不发请求。
- 邀请弹层是独立组件 `InvitePopup`（`class_name InvitePopup`），由大厅挂载；它自己订阅 `GameStore.social_changed`，按 `live_invites()` 过滤已过期条目并渲染遮罩 + 居中面板，条目显示邀请人名字、编号与房间码。接受与拒绝都发 `RespondInvite`（`accept` 为 `true`/`false`）并立即禁用该条按钮，结果清掉该条邀请后弹层自动隐藏；大厅只监听 `responded` 信号做状态文案。收到 `InviteAccepted` 后由 `GameStore` 自动补发 `JoinRoom{roomCode}`。
- 房间页的邀请入口只列 `status == "online"` 的好友，其余按钮禁用；点击发 `InviteToRoom{friendId}`，结果由 `FriendInviteResult` 回写状态栏（已接受/未接受）。邀请成功的提示不改变房间成员，受邀者按普通 `JoinRoom` 入房后才出现在座位列表。
- 商店使用 `HFlowContainer` 自动换行，左侧内容放入 `ScrollContainer`，1280×720 下不会因商店变长而溢出；手牌行同样使用 `HFlowContainer`，但槽位按 `card-width - hand-card-overlap` = 80 步进排列、行内 `h_separation` 为 0，1–10 张保持单行，卡片自身 112×146 向右溢出并与下一槽重叠。
- 手牌卡使用与商店/棋盘一致的 `GameCardFace` 卡面（介绍、费用、时代角标与底部名称、攻击/类型/生命信息区），只能拖拽出牌（`HandCard._get_drag_data`），点击不产生意图；落点由 `BoardDrop` 判定：随从牌按指针位置换算插入下标、棋盘已满时任何落点都不接收，需要目标的效果牌只有落在服务端下发的候选（`targetCandidates`，区域为棋盘或手牌）上才提交，落点候选即目标；其它落点静默取消。拖拽数据带 `epoch`（`_view_epoch`）与 `targets`（候选的 `{zone, id}` 列表），视图重建后松手按取消处理。
- 手牌的随从数值读服务端视图 `attack` / `health`（当前值，含成长等全部层数），不读 `CardDB` 卡面值；金色随从带「★」，奖励牌在角标行显示锁定的发现星级，并用「发现一张 N 星随从」作为原生 tooltip。拖拽 payload 带 `instanceId` 与 `golden` 供落点与预览判断，不参与意图。
- 手牌悬停由 `HandSlot` 承担：进入时槽位 `z_index = 1`，卡片以底边中点为轴放大 `hand-hover-scale`（1.12）；放大后的矩形包含原矩形，不会在卡片下缘反复触发 `mouse_exited`。悬停不读取 `drag_allowed`，禁用与战斗阶段同样生效；不改顺序、`handIndex` 或 payload。`HandCard` 在拖拽真正开始时发出 `drag_started`，槽位同步复位缩放与层级后才复制拖拽预览，保证预览为原始尺寸。`HandScroll` 关闭 `clip_contents`，让放大后的卡顶越过 154 高的滚动行继续绘制与命中；悬停卡会盖住「手牌」标题，属已接受的视觉取舍。Godot 的 GUI 拾取按树序逆序、不比较 `z_index`，因此悬停期间 `HandSlot` 把右侧槽位卡片的 `mouse_filter` 临时设为 `IGNORE`、复位时恢复 `STOP`：否则放大卡被揭示出来的右侧交叠带仍会命中并拖起下一张卡。左侧槽位不用处理，它在树序中本来就晚于本槽被拾取。拖拽取消或视图刷新后不自动恢复悬停，指针需移出再移入。
- 手牌拖拽期间 `BoardDrop` 逐帧按指针位置复查落点（`hand_drop_accepted`）：空位仍可放置；棋盘外、棋盘已满或需要目标却不在候选上判为无效，指针切换为禁止光标（`CURSOR_FORBIDDEN`），松手不产生意图，卡牌留在手牌。指针离开棋盘矩形时不设置禁止光标，交还给手牌卡的 `_can_drop_data`。
- 需要目标的效果牌也可以落在手牌候选上：`HandCard` 实现 `_can_drop_data` / `_drop_data`，只接受「需要目标的效果牌」且本卡实例编号出现在拖拽数据的 `targets` 中（区域 `hand`），拖到自己身上拒绝；接受时发出 `target_dropped(payload, target)`。
- 棋盘随从卡也使用拖拽：`BoardCard._get_drag_data` 交出 `{epoch, minionId, fromIndex, sellable, golden}`；`BoardDrop` 在 `NOTIFICATION_DRAG_BEGIN` 时隐藏原卡并插入同尺寸透明占位，指针移动时实时移动占位、其余随从让位，松手按「移除后被拖随从剩余序列中的插入下标」发 `RearrangeBoard`，拖拽结束恢复原顺序。商店面板是 `SellDrop`，只接受可出售的棋盘卡：拖入时金色描边并提示「松手出售 +N 金」（`golden ? 3 : 1`），松手发 `SellMinion`；手牌与不可出售随从静默取消。卡内不再生成出售与左右移位按钮。
- 拖放冒泡依赖控件的鼠标过滤：Godot 在「`_can_drop_data` 为假且 `mouse_filter == STOP`」的控件上中止沿父链的落点查找。棋盘空位（`EmptySlot`）设 `IGNORE`；棋盘随从卡（`BoardCard`）与商店槽位设 `PASS`，保留自身点击/拖拽起手与悬停详情，同时把落点交给父级 `BoardDrop` / `SellDrop`。新增落点控件或改其鼠标过滤时必须保证落点仍能到达父级。
- 时代文案由 `CardDB.era_name` 把 1–7 映射为英文 id（`prehistoric`…`future`）后按 `enum.era.<id>` 取文本；玩法、卡牌类型、进化选项与攻防分层同理走 `UI.enum_text(group, id)`；英雄卡底部的时代格、对手身份块的英雄行与座位详情统一显示「时代 X」。
- 左栏固定宽 88，座位行只渲染头像（`avatar-rail` = 44）、名次角标（`rank` 为 `null` 时不渲染）与血量条（高 6，按 `hp / hpCap` 比例填充，无数字）；行顺序按 `players[].rank` 升序（第 1 名在最上，`rank` 为 `null` 垫底、同序按座位号），每次视图重建重排，客户端不推导名次；已淘汰行经 `UI.apply_dim` 整体压暗但保留名次且仍可点击。点击展开 `view.combatLog` 中按 `participantIds` 过滤的战报，浮层挂在全屏 `ReportLayer` 上、从座位行右侧覆盖中央舞台，同一时刻只展开一块，展开座位按 `playerId` 在 `GamePlayersRail.refresh` 整块重建后恢复；无条目显示「无战报」。
- 招募舞台的身份只由商店操作行承担：`SellerTile`（`GamePortrait`，64×64，名字「售货员」贴底）居中，左侧 `UpgradeButton`、右侧 `RerollButton` 与 `FreezeButton` 均为 40×40 `GameIconButton`，费用角标在右上，随 `affordances` 门控；营业说明只在悬停时由 `GameDetailLayer` 的 `seller` 详情显示，不再有「已淘汰观战」身份拦截。
- 河道行分两种模式：招募河道 `RecruitRiver` 中央是回合数 `RecruitTurnLabel`，右端 `RecruitCountdownFlip` 内为金色胶囊 `RecruitTimerLabel`「剩余 N 秒」，悬停翻成 `RecruitEndButton`「结束招募」，移出恢复，点击只发一次 `EndRecruit`；归零时 `RecruitTimerLabel` 显示「等待结算」并禁用翻转。战斗河道 `CombatRiver` 中央是 `CombatBanner`（演示前「第 N 回合」，播完后追加结果），右端是 `CombatStatusPill`「战斗中」。两套节点名不重名，避免 `find_child` 命中隐藏实例。
- 我方英雄在英雄行：`HeroCard` 64×84 自上而下为玩法色封面、名字行、攻击/时代/生命三格，生命取服务端当前值与上限；右侧 `HeroSide` 列放 `GoldTile` 金币与英雄技能图标（费用角标右上、剩余次数角标左上，剩 1 次时隐藏次数、0 次时置灰）。本人淘汰后英雄行改显示「已淘汰 · 观战中」并隐藏技能图标；战斗阶段隐藏技能图标，英雄卡与金币保留。玩法、时代与技能文本只在悬停时由 `hero` 详情显示。
- 战斗舞台的对手身份由 `GameOpponentView` 渲染：普通战为 64×64 头像、名字/英雄/HP 行与 360 宽血量条，幽灵战换成幽灵归属者身份并加「幽灵」角标、整块压暗、不显示血量条，`opponentPlayerId` 为空时显示「等待配对」。身份只随战斗视图出现，招募阶段不渲染。
- 商店卡左上角渲染 `CostBadge`（`display_cost`）、右上角渲染 `EraBadge`（`era`）；冻结由 `FreezeOverlay` 整卡蒙层（`freeze-overlay-alpha` = 0.22，经 `self_modulate` 只压蒙层自身）、间距 14 的 45° 冰晶细线与右下角「冻」字 `FreezeGlyph` 共同表达。
- 棋盘容量取自 `PlayerView.boardSlots`（缺失时回退 7），满场标记、提示文案与空位数量用同一数值；`BoardDrop` 仍由页面设置 `board_full`，控件本身不感知容量。
- 落点回调在 `epoch` 校验之外还要重新过一遍与拖拽起始同源的门控（`drop_gate`）：阶段为招募、未结束招募、视图未过期、倒计时未归零、`Net` 已连接、队首待选不是 `target`。拖拽途中换回合、开战或断线时松手不会发出意图。
- 手牌可拖动的前提是阶段为招募、视图未过期、倒计时未归零、`Net` 已连接、队首待选不是 `target`、且该卡 `playable`；任一不满足时 `_get_drag_data` 返回 `null`，拖拽根本不开始。
- 队首待选为 `target` 时弹出战吼目标浮层：按区域分组列出 `pendingChoices[0].candidates`（可含手牌随从），高亮触发者，没有取消与跳过；期间购买/刷新/升时代/冻结/结束招募/英雄技能/出售/换位/拖拽全部禁用，选中后发 `ChooseTarget { target }`。
- 英雄技能浮层只服务主动技能：`heroPower.requiresTarget` 为假时点击直接发 `UseHeroPower`；为真且候选非空时弹出候选浮层，取消不发意图、不扣金币与次数；为真且候选为空时不弹浮层，按钮禁用原因为 `NO_VALID_TARGET`。
- 战斗演示由 `CombatPlayback` 驱动（`PACE_MS = 320`、`TAIL_MS = 2000`，节奏只在客户端，服务端不估算演示时长）：第 i 条动作在 `(i + 1) × 320ms` 应用，数值只取 `source`/`target`/`summon` 的服务端下发值，客户端不做加减；动作序列只含主动攻击，反击伤害已并入该条动作下发的快照。写入哪一行按动作的 `sourceSide`/`targetSide`（缺失时退回按 `minionId` 双侧查找）。幽灵战改为随机出局者的最终阵容，实例编号只在克隆内与己方重号时按需改写，因此两侧编号仍可判。动作播完并经过 `TAIL_MS` 后才显示胜负横幅。
- 战斗行只渲染 `alive` 为真的随从：阵亡卡立即离场；播放器仍保留阵亡条目作为顺序锚点，复生把 `alive` 置回真后卡片在原相对位置重现。召唤按 `summon.position`（存活序列下标）插入，换算用 `CombatPlayback.alive_insert_index`，不会把阵亡锚点当计数。
- 战斗视图的占位在演示全程固定：对手身份固定 `portrait-size`（64）、两条随从行的 `custom_minimum_size.y` 取 `card-height`，一方全灭后空行不塌陷；`CombatRiver` 恒取 `river-height`（32），`CombatBanner` 常驻显示回合数，播完追加结果文案，横幅切换不改变任何行高，整个战斗组合固定 418。
- 战报归属回合：`GameStore.last_combat_turn` 记录收到 `CombatResult` 时的 `turn`，`combat_is_current()` 只在它与当前 `turn` 一致时为真；战报行、战斗横幅与演示都以此为准。`_apply_view` 在阶段离开 `Combat` 时清空 `last_combat` 并复位 `last_combat_turn`，未配对或已出局的座位不会把上一场当成新一场重播。
- 演示进度以 `CombatResult.startedAt`（服务端开战时刻）为锚点：首帧按 `now − startedAt` 直接 `seek`，已播动作不重播；演示期间没有任何跳过或加速入口。
- 碰撞演出由 `GameCombatView` 在 `advance` 里驱动 `CombatFx` 覆盖层（`CombatFxLayer`，`mouse_filter = IGNORE`）：播放器返回本帧到期动作，视图先按服务端数值更新数据，再演出：
  - `attack`：隐藏攻方原卡并在原位抓取矩形，克隆同尺寸卡 130ms 飞向目标；命中时目标闪光、飘出「-伤害」；再 130ms 飞回原位，克隆释放、原卡恢复。`attack` 只应用 `attack`/`health`，不改 `alive`。
  - `death`：受影响卡按原位淡出 160ms 后移除；若攻方本帧阵亡，克隆已回到原位才消失。
  - `reborn`/`summon`：卡片按存活序列位置出现并缩放缓入 160ms。
  - `contagion`：更新数值后卡片短暂变色。
- 演出用逐帧推进而不是 `Tween`，便于无头冒烟按帧断言；`reset()` 与 `_start_combat()` 清理覆盖层与进行中的动画。
- 演示播完后页面为该回合发送一次 `CombatPlayed`（`_combat_ack_turn` 去重），由服务端在全部参战座位回报后结算。
- 对局中主动退出使用 `LeaveRoom`：服务端在对局阶段按弃权结算，客户端收起引导、清空房间并重新连接、以令牌认证回大厅。结算页返回大厅同样清空房间并重新认证；对局结束即销毁房间，结算页不再有返回房间入口，也不显示房间关闭提示。
- 对局页的顶部工具条 + 左栏 + 中央舞台结构、页面状态与数据来源见[对局页](../product/ui/pages/game.md)；颜色、字号、间距与圆角来自 `UITheme`，页面不写死视觉值。左栏宽 88；中央舞台自上而下为招募组合（418）或战斗组合（418）、英雄行（84）与贴底手牌（146），四个视图都按 `SIZE_SHRINK_BEGIN` 顶部对齐；商店、棋盘、手牌与英雄行都不带标题。招募阶段与战斗阶段共用同一页面，战斗与结算阶段隐藏招募视图、商店操作、英雄技能、倒计时与结束招募，节点保留以便阶段恢复；战斗组合在招募阶段整块隐藏。
- 详情层（`client/scenes/game_detail_layer.gd` 的 `GameDetailLayer`，面板节点名 `MinionDetailPanel`）统一承载商店、座位、售货员与随从四类详情，我方英雄详情亦由其 `hero` 类别渲染：商店卡、座位行与招募/战斗随从卡在悬停进入、鼠标左键按下或触摸按下时显示；指针移出锚点矩形后隐藏，松开鼠标左键或触摸时按指针所在目标矩形更新（由 `_input` 按全局矩形做命中判断，避免子控件截断悬停通知）。随从详情先显示名称、科学分类（完整路径）、玩法、关联玩法（仅中立非学者）、食物链（捕食/寄生）与 `CardDB.display_text` 介绍，再取协议 `BoardMinion.stats` 按初始、永久、回合、光环、受击、当前六行展示，有符号项带 `+`，受击的攻防都为 0 时省略该行；`stats` 缺失时仍显示名称与介绍，并退化为只显示顶层 `attack` / `health` 的当前攻防。商店详情同样先给分类与关联/食物链行，再接费用、原始数值、时代、技能文本与不可购买原因；英雄详情给科学分类、玩法、擅长玩法、时代与英雄技能。分类路径缺失显示「未定」，关联或食物链为空时省略对应行。座位与商店字段见[对局页](../product/ui/pages/game.md)。面板只读、忽略鼠标输入，位置以锚点矩形为准并按视口边界夹紧；每次 `_refresh` 重建前先隐藏，同一目标重复触发显示时只校正位置、不重建内容。
- 冒烟测试按 `client/tests/ui_smoke*.tscn` 拆为十三个场景，`make client-smoke` 用 glob 逐个运行并要求各自输出 `UI_SMOKE_OK` 且无 `SCRIPT ERROR`。主冒烟覆盖五场景实例化、手牌重叠布局（十张 80 步进单行、相邻交叠、末卡不越界、滚动行不裁剪）、手牌显示服务端身材与奖励牌星级角标、手牌悬停（置顶、底边中点放大 1.12、越出滚动行仍命中、右侧交叠带仍归本卡且拖拽拾起本卡、移出复位、战斗阶段可查看）与拖拽开始复位、真实鼠标事件拖拽（按下、越过阈值、移到落点、松手）落盘与商店出售、拖拽落点意图（首卡 0、次卡 1、末尾空白为随从数）、棋盘卡换位（真实 `force_drag` 通知、原卡隐藏、落点发 `fromIndex/toIndex`）、商店出售落点（可卖与不可卖、手牌拒绝、普通/金色返还提示）、棋盘满时不接收、需要目标的效果牌落点规则、四个不可拖动条件、落点门控（换回合/开战/结束招募/断线时不发意图）、目标待选浮层与控件阻塞、战斗逐动作播放（阵亡卡离场、复生回原位、召唤按存活下标插入）、横幅停留、按截止时间续播、无跳过入口、战斗阶段不弹待选弹层、回合号不匹配时不显示旧战报、离开战斗后清空战报，以及离树导航守卫。`ui_smoke_side` 断言舞台结构：无右栏与旧手牌节点，中央舞台顺序为招募组合—战斗组合—英雄行—手牌，招募阶段只显示招募组合；商店操作顺序为升时代—售货员—刷新—冻结且均为 40×40，售货员 64×64，河道位于商店与棋盘之间且回合数居中、倒计时贴右端；英雄行 84、英雄卡 64×84（名字在时代上方、攻击在左、时代居中、生命在右）、技能图标 40×40 与金币格；手牌在英雄行下方且底边为 712；倒计时悬停翻成结束招募与归零文案；战斗对手的普通/幽灵/等待配对三种身份，以及淘汰与幽灵舞台；`ui_smoke_seats` 断言 8 行座位、头像尺寸、名次角标缺失与压暗、血量条比例、战报展开与刷新保持、无战报文案与座位详情；`ui_smoke_combat` 断言战斗组合子节点顺序为对手身份块—对手随从行—河道—我方随从行且无旧英雄条，横幅播前只显示回合数、播后追加结果，以及固定占位（两行按 `card-height` 取行高、河道按 `river-height`、一方全灭后行高与位置不变），并以平局夹具断言平局横幅文案。`ui_smoke_cards` 断言商店卡费用与时代角标、冻结蒙层/冰晶/「冻」字、卡面介绍的两行省略、无介绍时的紧凑信息区与名称/数值基线，以及卡牌统一 112×146；卡面类型行显示玩法（中立为「中立」、哺乳纲随从为「哺乳纲」）且封面取 `UITheme.playstyle_cover`；`UITheme.PLAYSTYLE_COLOR` 14 个玩法的颜色互不重复、封面色比原色更暗；详情面板含科学分类与玩法行，机械工具卡与学者卡分别为「机械」与「中立」、英雄详情含科学分类行。`ui_smoke_text` 断言 `CardDB.display_name`（含 Token）、`display_text`、`era_name` 与 `UI.enum_text` 的取用，`CardDB.playstyle_name`（含 `neutral`）、`CardDB.taxon_name` 与 `CardDB.taxon_path`（完整路径、空 id、未知节点），`UI.error_text` 的错误码文案，`UI.error_text_by_id` 的 `messageId` 细分文案，以及缺键时 `UI.error_text_by_id` 返回空串。`ui_smoke_gallery` 断言大厅图鉴页 432 项与「全部」章节计数、每页 40 张与 11 页、常驻详情非空、目录与详情宽度取令牌且整体不溢出视口；时代与分类两维筛选（史前 63、未来下真菌项禁用、未来 + 动物界 14 张、清空后回 432）；空目录显示空态且无详情名；翻页到第 2 页；点击卡面后详情名称与类型/分类行同步；大厅「卡牌图鉴」入口文案；对局覆盖层为 198 张且 `alan-turing`、`airplane` 在池、`bruce-lee`、`ammonite-token`、`aristotle` 不在池，覆盖层不切换场景、返回按钮与 `Esc` 都能收起。`ui_smoke_guide` 断言首次进入自动展示第 1 步、下一步推进与手牌/棋盘条件跳过、跳过写入 `seen` 后不再自动展示、`show_manual()` 从第 1 步开始且不写 `seen`、场景重建后从 `GameStore.guide_step` 恢复、引导存在时点击与拖拽仍到达底层控件、存储缺失时按首次处理。`ui_smoke_room_ops` 断言房主可见 `DissolveButton` 与 `Kick_<playerId>`、非房主全部隐藏、`RoomClosed` 的 `kicked` / `dissolved` 提示后回大厅、离线时房主操作不可用。`ui_smoke_touch` 断言触控端长按 450ms 显示详情、短按不显示、位移取消、多指与阻塞态不触发，以及手牌悬停残留复位；`ui_smoke_layout` 断言六个页面在 960×540 下不越界，并断言安全区与键盘边距取较大值、桌面边距为零；`ui_smoke_platform` 断言桌面、iPhone 与 iPad 三档逻辑基准、44pt/32pt 命中尺寸与安全区、键盘像素换算。

- 图鉴页大厅入口由 `lobby.gd` 的「卡牌图鉴」按钮经 `UI.navigate` 进入 `res://scenes/gallery.tscn`，目录取 `CardDB.cards.keys()`（已含 Token）；对局入口是顶部工具条右侧的 `GalleryButton`，`game.gd._open_gallery` 首次创建 `CardGallery` 覆盖节点（`GalleryOverlay`，铺满视口，`closed` 只收起 `visible`），不切换场景、不暂停视图刷新，拖拽、待选与战斗演示状态保留；`_refresh_overlay` 在选择浮层可见时调用 `_close_gallery`。
- 图鉴两维筛选交叉生效：章节计数按当前分类过滤、目录计数按当前时代过滤，零计数且未选中的项禁用；筛选与翻页后网格只构建当前页（`gallery-page-size` = 40，`gallery-grid-columns` = 5），选中卡不在结果内时改为结果首张，结果为空时显示空态。排序按时代序号（未知排最后）再按本地化名称。
- 本局卡池由 `CardPool.card_ids` 复算：遍历 `CardDB.cards` 并跳过 Token；非中立卡按 `playstyle` 是否在 `enabledFactions` 判定，中立卡取 `specialty` 中 `kind = "play"` 的关联玩法、至少一个启用才入池且无关联时始终入池，与服务端 `card_in_pool` 一致；结果只用于展示，不影响出牌与商店合法性。

## 4. 生成物与命令

| 命令 | 内容 |
| --- | --- |
| `make protocol` | 由 `protocol/schema/v1/` 生成 `protocol/generated/protocol.gd` 与 `client/generated/protocol.gd` |
| `make protocol-check` | 校验协议样例，并比对两处 GDScript 生成物漂移 |
| `make data` | 生成服务端数据（含 `taxonomy.json`）、`client/generated/{cards,skills,tokens,taxonomy}.json` 与文本表 `client/text/zh_CN.csv` |
| `make data-check` | 比对服务端与客户端生成数据、文本表漂移，运行效果审计 |
| `make assets` | 按 `ASSET_MAP` 对源立绘去水印，派生头像与卡面并重写 `client/assets/asset_manifest.json`；见 §10 |
| `make font-subset` | 由文本表 + ASCII + GB2312 生成内置中文字体子集 `client/assets/fonts/NotoSansSC-Regular-subset.ttf`；见 §6 |
| `make client-import` | `godot --headless --path client --import`，检查工程可导入 |
| `make client-run` | 先导入再启动桌面客户端；新增或改名 `class_name` 脚本后必须走这个入口，否则沿用旧全局类缓存 |
| `make client-check` | 协议/数据/文本表漂移 + 客户端边界断言 + 国际化断言 + 工程导入 |
| `make client-smoke` | 用 glob 逐个运行十三个 `ui_smoke*.tscn` 冒烟场景，要求各自输出 `UI_SMOKE_OK` 且无 `SCRIPT ERROR` |
| `make client-export-ios` | 导出 Xcode 工程并用 `xcodebuild` 自动签名，打包 `output/ios/Civilization-ios.ipa`；见 §9 |
| `make m1b-e2e` | 启动服务端、Godot 无头客户端与 7 个 `sim bot`，要求输出 `M1B_E2E_OK` |
| `make dev-bots` | 启动服务端（监听 `0.0.0.0`，手机可直连）与 7 个机器人并打印房间码，供开发者用客户端作为第 8 人手动联调 |

Godot 可执行文件通过 `GODOT` 变量覆盖，默认自动探测 `godot` 或 `/Applications/Godot.app/Contents/MacOS/Godot`。

## 5. 运行方式

手动联调的最省事用法是 `make dev-bots`：

```
make dev-bots                                       # 终端 1，起服务端与 7 个机器人
make client-run                                     # 终端 2，大厅填房间码
```

客户端一律用 `make client-run` 启动：它会先 `--import` 刷新全局类缓存。直接跑 `Godot --path client` 时，若工程新增或改名过 `class_name` 脚本而缓存未更新，依赖它的脚本编译失败，对局页会缺整块区域（例如中央舞台整体不构建，只剩顶部工具条与左栏）。

命令打印两行 `ROOM_CODE=<6 位>` 与提示后等待，开发者在客户端大厅输入该房间码、选英雄并准备，8 人齐自动开局；整局结束后命令自行退出并释放端口，Ctrl-C 也可一次退出全部进程。端口被占用时立即失败，用 `make dev-bots DEV_PORT=8081` 换端口。需要自定义机器人名字或英雄时改用下面的手动方式。

`make dev-bots` 把服务端监听在 `0.0.0.0:8080`，手机与开发机同一局域网时可直接连接：移动端登录页默认地址为 `ws://<开发机 Bonjour 主机名>.local:8080/ws`（当前 `cooter.local`），桌面端仍为 `ws://127.0.0.1:8080/ws`。

手动方式：

```
make m1b-build
cd server && cargo run -q -p room-service          # 终端 1；手机联调加 --listen 0.0.0.0:8080
make client-run                                     # 终端 2，手动登录并创建房间
cargo run -q -p sim -- bot --connect ws://127.0.0.1:8080/ws \
  --room <房间码> --name Bot1 --hero tesla          # 终端 3 起，共 7 个
```

- 客户端创建房间后把 6 位房间码发给 7 个机器人；8 人全部从各自候选中选好英雄并准备后服务端自动开局。
- 机器人参数与协议顺序见[房间服务与联网](room-service/README.md) §12。
- 无头验收用 `make m1b-e2e`：客户端自动建房并写入房间码，脚本再拉起 7 个机器人，结束后输出 `M1B_GAME_OVER place=<n> seed=<seed> ranking=<n> intents=<n> errors=<n> soft=<n>`；脚本校验名次、8 人排名、种子、意图数，并要求错误全部落在预期白名单（`NO_SPACE`、`NOT_ENOUGH_GOLD`、`SHOP_SLOT_EMPTY`，以及目标候选与视图偏差导致的 `NO_VALID_TARGET`），其余错误为 0，服务端与机器人日志无 panic。
- 无头驱动不等上一条意图确认就按新视图继续决策，服务端按最新状态拒绝属预期；因此 `errors` 与 `soft` 必须相等。驱动另外对同一回合的同一商店槽位只提交一次购买（刷新后允许再次购买），避免重复购买同一槽位。
- 驱动按 `clientIntentId` 找回被拒意图，收到 `NO_VALID_TARGET` 时沿服务端下发的候选（`targetCandidates` 或待选 `candidates`，键为 `zone:id`）换下一个重发；队首待选为 `target` 时发 `ChooseTarget { target }`，不再一律当发现处理；`sim bot` 的处理方式相同（见[房间服务与联网](room-service/README.md) §12）。候选遍历完仍无合法目标时，本回合跳过该手牌（连续被拒 3 张且可结束时直接结束招募）；`ChooseTarget` 被拒则换下一个候选。否则会形成「重试 → 拒绝 → 再重试」的空转，直到演示超时。
- 默认端口 `18080` 被其他进程占用时，脚本会因监听失败退出；用 `M1B_PORT=<端口> make m1b-e2e` 覆盖，脚本与客户端读取同一个变量。

## 6. 国际化与文本表

- **唯一运行时中文文件**：`client/text/zh_CN.csv`（表头 `keys,zh_CN`）。客户端 `.gd` 代码不写中文 UI 文案；协议和生成数据只传 id，展示文本全部按 id 从文本表取。
- **配置**：`client/project.godot` 的 `[internationalization]` 设置 `locale/fallback="zh_CN"` 与 `locale/translations=PackedStringArray("res://text/zh_CN.zh_CN.translation")`。Godot 在导入时把 CSV 编译为 `zh_CN.zh_CN.translation`；`.translation` 与 `.csv.import` 被 `.gitignore` 忽略，CSV 本身随源提交。
- **取文本接口**：
  - 实例方法与 autoload 内取静态 key 用 `tr(key)`，带参数时用 `tr(key) % params`；
  - 静态函数（如 `GameCardFace.era_text`、`UI.enum_text`）不能用 `tr()`（`tr()` 是 Node 的实例方法），必须用 `TranslationServer.translate(key)`；实例方法与 autoload 内用 `tr(key)`，需要运行时拼接 key 时同样可以调用 `tr(key)`，带参数时用格式化结果；
  - 缺 key 时 Godot 原样返回 key，因此封装函数按「返回 key 即缺键」回退：枚举回退为 id，错误码回退为 code，错误详情回退为空串。
- **key 命名**：
  - 内容文本：`card.<id>.name`、`card.<id>.match_text`、`skill.<id>.name`、`skill.<id>.text`、`skill.<id>.match_text`、`token.<id>.name`；
  - 枚举：`enum.card_type.<id>`、`enum.playstyle.<id>`、`enum.taxon.<id>`、`enum.era.<id>`、`enum.keyword.<id>`、`enum.subtype.<slug>`、`enum.evolve.<id>`、`enum.stat_layer.<id>`；
  - 错误与界面：`error.<CODE>`（默认文案）、`error.<CODE>.<detail>`（服务端 `messageId` 的细分文案）、`ui.*`（页面静态文案）。
- **所有权**：`card.*`、`skill.*`、`token.*` 与 `enum.card_type|playstyle|taxon|era|keyword|subtype.*` 是生成 key，由 `scripts/gen_text.py` 从实体文档与 `taxonomy.md` 整体重算；`ui.*`、`error.*`、`enum.evolve.*`、`enum.stat_layer.*` 是人工 key，原样保留；出现未知前缀直接报错。`make data` 会写入 CSV，`make data-check` 检测漂移。
- **前置条件**：修改 CSV 或任何实体文本后必须运行 `make client-import`（或任何以它为前提的目标，如 `client-run`、`client-smoke`、`m1b-e2e`），让 Godot 重新导入并生成 `.translation`；只在编辑器外改 CSV 而不导入，运行时仍读旧翻译。`make client-check` 已包含 `client-import`。
- **服务端错误**：`ErrorMessage.messageId` 默认 `error.<CODE>`，`messageParams` 按 `%s` 填充；客户端状态栏先按 `code` 取 `error.<CODE>`，悬停详情优先用 `messageId`/`messageParams`，`message` 仅面向日志。
- **新增语言**：一个语言一个文件。新增语言需要：新增 `client/text/<locale>.csv`，并扩展 `scripts/gen_text.py` 以支持按语言文件读写与产出（每个语言文件独立生成内容 key 与枚举 key）；用 `make client-import` 导入后把对应 `.translation` 加入 `[internationalization]` 的 `locale/translations`。业务代码（`CardDB`、`UI.*`、`tr()` 调用）与实体文档、协议均无需改动。
- **内置字体**：`client/assets/fonts/NotoSansSC-Regular-subset.ttf`（Noto Sans SC 子集，OFL 许可见同目录 `OFL.txt`）由 `project.godot` 的 `gui/theme/custom_font` 设为默认主题字体，所有文本不再依赖系统字体回退；iOS 真机因此不再缺字。子集把垂直度量对齐 PingFang SC（`hhea` 1060/−340、`OS/2` typo 860/−140/400），切换前后行高一致，现有布局不需要按字体调整。
- **字体生成**：`make font-subset` 取 `client/text/zh_CN.csv` 全部字符 + 可打印 ASCII + GB2312 常用字（覆盖玩家自填显示名），输出上述子集。源字体放 `FONT_SOURCE`（默认 `output/fonts/NotoSansSC[wght].ttf`，Google Fonts 的 Noto Sans SC 变量字体），解释器需装 fontTools：先 `python3 -m venv output/fonttools-venv` 并 `output/fonttools-venv/bin/pip install fonttools`，再用 `make font-subset PYTHON=output/fonttools-venv/bin/python`。文案出现子集外生僻字时重跑该目标；`client/tests/ui_smoke_text.gd` 断言子集覆盖文本表全部字符。

## 7. 与设计的偏差

1. 设计 §5.6 只要求“1 个客户端 + 7 个机器人人工验收”；实现额外提供 `client/tests/headless_game.*` 与 `make m1b-e2e`，用真实客户端栈（Net + GameStore）自动打完一局，作为协议与视图消费的回归证据。该驱动不点击 UI，五场景可读性仍由 `make client-smoke` 与手动运行保证。
2. 卡牌展示表新增 `display_cost`：随从固定 3、效果牌取文档 `cost`，仅用于显示；规则判定仍全部在服务端。
3. `PlayCard` 的目标由拖拽落点直接给出（随从牌带 `position`，需要目标的效果牌带 `target: {zone, id}`，取自视图候选），不再先发无目标再补选；英雄技能按 `heroPower.requiresTarget` 与候选决定直接释放还是弹浮层。`BuyMinion` 只携带槽位，买入一律入手牌，不登记补选目标。
4. 英雄技能需要目标且候选非空时先弹出候选浮层，可取消；不需要目标时直接发送，候选为空时按钮禁用、不发送。

## 8. 桌面端导出

`client/export_presets.cfg` 提供 macOS 预设 `macOS`：Universal（`x86_64` + `arm64`）、bundle id `com.civilization.banner`、应用名 `Civilization`、不签名（`codesign/codesign=0`、`codesign/identity=""`）。预设的 `export_path` 为 `../output/desktop/Civilization.app`；`make client-export` 以命令行传入绝对路径，二者一致。

| 命令 | 内容 |
| --- | --- |
| `make client-export-templates` | 读取 `$(GODOT) --version` 前三段（当前 `4.7.2`），检查 `$HOME/Library/Application Support/Godot/export_templates/4.7.2.stable/`；缺失时下载并解压官方 `Godot_v4.7.2-stable_export_templates.tpz`，只安装其中的 `templates/` |
| `make client-export` | 依赖 `client-export-templates` 与 `client-import`；清空旧产物后导出 `output/desktop/Civilization.app`，校验 `Contents/MacOS/Civilization` 可执行，再用 `ditto` 打包 `output/desktop/Civilization-macos.zip` |

- 两个目标只在 macOS 上可用，非 Darwin 主机直接报错退出；模板版本读不到时同样退出非 0，不猜测版本。
- Universal 导出要求 `project.godot` 启用 `rendering/textures/vram_compression/import_etc2_astc` 与 `import_s3tc_bptc`，缺失时 Godot 会拒绝导出。
- 产物未做正式签名与公证：`codesign/codesign=0` 禁用导出时的重签，但可执行文件仍保留 Godot 模板内置的 ad-hoc 签名。首次打开需右键「打开」，或在「系统设置 → 隐私与安全性」中放行；经 zip 下载后解压的副本同样会被 Gatekeeper 拦截。
- 导出物内置默认服务器地址：桌面 `ws://127.0.0.1:8080/ws`，移动端 `ws://cooter.local:8080/ws`；登录页「服务器」字段仍可手改。

## 9. iOS 移动端

### 设备与布局

- `ScreenProfile`（autoload）在启动时按设备设置 `content_scale_size`：iPhone 为 960×540，iPad 与桌面为 1280×720，均保持 16:9 等比缩放；`project.godot` 的 `display/window/handheld/orientation=4`（`SCREEN_SENSOR_LANDSCAPE`）锁定左右横屏。
- 设备判定：iOS 上按 `OS.get_model_name()` 前缀区分 iPad 与 iPhone；机型名不可用时按屏幕长宽比兜底。非 iOS 与无头环境一律按桌面处理。
- 换算公式是纯函数：`canvas_scale`、`canvas_hit_min`、`canvas_from_pixels`、`canvas_insets`。`DisplayServer.get_display_safe_area()` 与 `virtual_keyboard_get_height()` 返回像素，按内容矩形（含等比缩放留白）换算为画布单位，落在留白内的边距记零。
- `UI.safe_area(page)` 把页面子节点整体移入 `SafeAreaMargin`：桌面四边为零；触控端取安全区四边，底部取安全区与键盘高度的较大值。键盘边距只在页面内 `LineEdit` 获得焦点期间逐帧读取。`UI.safe_layer(page)` 供对局内图鉴覆盖层等动态全屏节点挂载。
- `UITheme.hit_primary()` / `hit_small()` 把 44pt / 32pt 换算为画布单位；`UI.button` 与 `UI.styled_button` 在触控端按此抬升最小高度，桌面端保留原值。
- 图鉴在 iPhone 基准下收窄目录（`gallery-toc-width-compact`）、详情（`gallery-detail-width-compact`）并改为 4 列（`gallery-grid-columns-compact`）；桌面与 iPad 保持原令牌。
- 对局页与内部组件统一用 `UITheme.stage_metric(name)` 取场景令牌：iPhone 优先取 `<name>-compact`（卡面 84×100、英雄行 64、河道 26、商店操作行 52、顶部工具条 26、头像 48、图标按钮 36 等），其余设备取原值；页面边距与舞台间距在 iPhone 下同步收紧。桌面端仍为 720 高的完整舞台，iPhone 为 540 高，纵向顺序与操作不变。

### 触控交互

- 详情层（`GameDetailLayer`）在触控端由长按状态机驱动：按下只登记第一个触点，450ms 显示，抬手隐藏；位移超过 8 个画布单位或模态浮层打开时取消。触控端忽略触摸模拟出的鼠标悬停与按下，桌面鼠标路径不变。
- `HandSlot` 在触摸释放、拖拽结束与重建卡片时显式复位缩放、`z_index` 与右侧槽位的 `mouse_filter`，弥补触控端没有 `mouse_exited` 的问题。
- 单击、拖拽继续依赖 Godot 的 `emulate_mouse_from_touch`；`Esc` 返回在触控端由常驻返回按钮替代。

### 联调地址

- 移动端登录页默认 `ws://cooter.local:8080/ws`（`Net.MOBILE_URL`）：`cooter` 取开发机 `scutil --get LocalHostName`，换机器时改该常量并重新导出；iOS 通过 Bonjour 解析 `.local`，开发机 IP 变化不影响。
- 桌面端默认 `ws://127.0.0.1:8080/ws`（`Net.DESKTOP_URL`）。`Net.default_url_for(mobile)` 是纯函数，冒烟直接断言两端取值；登录页字段始终可手改，`connect_to("")` 回落到当前平台默认值。
- 服务端默认只监听 `127.0.0.1:8080`；真机联调必须用 `make dev-bots`（监听 `0.0.0.0`）或手动 `cargo run -p room-service -- --listen 0.0.0.0:8080`。首次连接会触发 iOS 本地网络权限弹窗，描述文案已写入导出预设的 `NSLocalNetworkUsageDescription`。

### iOS 导出

`client/export_presets.cfg` 的 `iOS` 预设：bundle id `com.civilization.banner`、团队 `Q62EFJPV3C`、`development` 签名、iPhone & iPad（`targeted_device_family=2`）、最低 iOS 15.0、只导出 Xcode 工程（`export_project_only=true`）、`Apple Development` 签名身份，并通过 `additional_plist_content` 写入 `UIRequiresFullScreen` 与 `NSLocalNetworkUsageDescription`。

| 命令 | 内容 |
| --- | --- |
| `make client-export-ios` | 依赖 `client-export-templates` 与 `client-import`；导出 `output/ios/Civilization.xcodeproj`，用 `xcodebuild`（scheme `Civilization`、`-allowProvisioningUpdates`）构建 Release，再把 `Civilization.app` 装入 `Payload/` 打包为 `output/ios/Civilization-ios.ipa` |

- 只在 macOS 上可用；模板目录缺少 `ios.zip` 时直接退出非 0。
- 导出前必须存在 `application/config/icon`（当前 `res://icon.png`）；缺少图标时 Godot 导出失败。图标由 `scripts/gen_app_icon.py` 生成，随源提交。
- 签名依赖 Xcode 登录 Apple 团队账号并登记目标真机；缺少账号或描述文件时 `xcodebuild` 失败并打印签名错误，不产出未签名包。
- 构建产物在 `output/ios/build/Build/Products/Release-iphoneos/`；`output/` 不提交。

## 10. 美术素材层

- 位置：`client/assets/`。`characters/portraits/ch_<id>_full.png` 为去水印立绘，`characters/avatars/ch_<id>_ava.png` 为圆形头像，`cards/faces/card_<id>_face.png` 为 448×584 卡面封面；`asset_manifest.json` 按卡牌 id 登记三类路径与展示尺寸。
- 加载：`CardDB` 启动时读取清单到 `art`，提供 `has_art(id)` 与 `art_path(id, kind)`；清单缺失或条目缺字段时返回空串，不报错。
- 回退：`GameCardFace.face()` 有卡面纹理时叠 `CoverArt` 并隐藏首字水印，否则保留纯色封面与首字；`UI.avatar()` 有头像纹理时叠 `AvatarArt`，否则退回玩法色块首字。纹理加载失败与无资源走同一回退。
- 稀有度：`CardDB.rarity_tier(id)` 按卡牌 `level` 映射为 `common`（1–2）、`rare`（3–4）、`epic`（5–6）、`legendary`（7），仅用于展示，不进入协议与服务端数据。
- 生成：`make assets` 调用 `scripts/process_assets.py`，按 `ASSET_MAP`（默认 `client/assets/characters/portraits/asset_map.json`，内容为 `card_id → 源立绘文件名`）处理源立绘，并重写清单；映射文件缺失时目标直接失败。
- 批次：首批试点 20 张，覆盖 14 种玩法色与 7 个时代；其余卡牌继续走程序化绘制。源立绘随仓库提交。
- 导入：图片 `.import` 由 Godot 生成并被 `client/.gitignore` 忽略；改图后运行 `make client-import`。

## 相关文档

- [房间服务与联网](room-service/README.md)
- [规则引擎与模拟器](rule-engine-and-simulator.md)
- [技术现状索引](README.md)
- [客户端技术基线](../../versions/v0.1/technical/architecture/client-stack.md)
