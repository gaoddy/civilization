# Godot 客户端

> 状态：已实现（v0.1 M1b + 首版剩余四项：赛后回顾、房间生命周期、首局引导、macOS 与 iOS 导出；v0.2 起加入 Android 导出与系统返回路由）
> 代码：`client/`
> 验收：`make client-check`、`make client-smoke`、`make client-env-check`、`make client-e2e`、`make client-export`、`make client-export PLATFORM=ios`、`make client-export PLATFORM=android`、`make client-export PLATFORM=android AAB=1`

本文记录 v0.1 已实现的 Godot 客户端现状：工程结构、全局状态与网络层、六个场景、生成物与运行方式；不描述服务端房间服务（见[房间服务与联网](../room-service/README.md)）与产品规则（见[规则引擎与模拟器](../rule-engine-and-simulator.md)）。

## 1. 工程结构

```
client/
  project.godot              # 主场景 login.tscn；autoload：ScreenProfile、WindowSizer、GameStore、CardDB、Net、Sfx、Platform、LocalServer、SystemBack
  export_presets.cfg         # macOS 预设（Universal、未签名）、macOS 本地调试预设（embedded_server）、iOS 预设（开发签名）与 Android APK / AAB 预设（release 签名）
  icon.png                   # 应用图标 1024×1024；iOS 与 Android 导出按尺寸缩放生成
  icon.svg                   # 应用图标矢量源；改动图标先改此处再导出 PNG
  .gitignore                 # 忽略 .godot/ 与 *.uid
  autoload/
    screen_profile.gd        # 设备类型、逻辑基准、内容框比例、pt→画布换算、安全区与键盘边距
    window_sizer.gd          # 桌面端启动时按屏幕可用区域 90% 设置窗口，收成 16:9 并居中；触控端跳过
    game_store.gd            # 认证、房间、PlayerView、赛后统计与引导进度的全局状态
    ws_client.gd             # WebSocketPeer、自动重连、信封编解码与分发
    card_db.gd               # 只读卡牌/技能展示表与科学分类树；展示文本按文本 id 从文本表取
    sfx.gd                   # 对局音效播放池、场景/卡牌资源路径与默认音效回落（autoload Sfx）
    local_server.gd          # 单机调试包：启动内嵌 room-service 与 sim、读取房间码并交给登录/大厅自动流程（autoload LocalServer）
    system_back.gd           # Android 系统返回键与返回手势路由：先收可关闭浮层，再交给页面 system_back()（autoload SystemBack）
  text/
    zh_CN.csv                # 唯一的运行时中文文本表（生成物 + 人工 key）
  assets/
    fonts/NotoSansSC-Regular-subset.ttf  # 默认主题字体（中文子集）；美术素材见 §10
    audio/sfx/                           # 场景音效：刷新、冻结、解冻、商店入场、战斗开始/结束；见 §11
    audio/cards/default/                 # 卡牌默认音效：购入、攻击、死亡；逐卡覆盖见 §11
  ui/
    theme.gd                 # 三层视觉令牌与 StyleBox 工厂（class_name UITheme）
    safe_area.gd             # 页面安全区包裹（class_name SafeAreaMargin），触控端叠加键盘边距；整屏背景留在包裹层外
    layout.gd                # 安全区、内容框、设计画布页面容器与共享图层（class_name UILayout）
    navigation.gd            # 页面路径常量、统一导航与错误码去向（class_name UINavigation）
    wording.gd               # 枚举展示与错误文案取用（class_name UIWording）
    format.gd                # RFC3339 时间格式化（class_name UIFormat）
    controls.gd              # 共用控件构造：label、button、panel、margin、胶囊、背景、页面容器背景、区块标题与空态（class_name UIControls）
    pill_button.gd           # 胶囊按钮：neutral/primary/danger 三变体（class_name PillButton）
    popup_layer.gd           # 弹出层外壳：遮罩、居中/锚点/四向贴边定位与点击遮罩关闭（class_name PopupLayer）
    art.gd                   # 图片取用与铺层（class_name UIArt）
    connection_dialog.gd     # 断线弹窗：说明、重新连接与重连中（class_name ConnectionDialog）
    social/                  # 好友入口与抽屉：FriendsButton、FriendsPanel、FriendRow、FriendRequestRow、RecentPlayerRow
    drag_preview.gd          # 拖拽预览定位（class_name UIDrag）
    long_press.gd            # 长按判定：450ms 与 8 画布单位位移取消（class_name LongPressTracker），详情层与选英雄覆盖层共用
    avatar.gd                # 头像块：继承 AvatarFrame 的圆角头像，贴图或玩法色封面 + 首字（class_name Avatar）
    avatar_frame.gd          # 圆角头像框：头像按框圆角裁切、描边叠在头像之上、业务内容走内容叠层（class_name AvatarFrame）
    hp_bar.gd                # 血量条：按 hp / hpCap 比例填充，可选圆角（class_name HpBar）
    rank_badge.gd            # 名次角标：rank 为空时不渲染（class_name RankBadge）
    skill_list.gd            # 英雄技能列表：技能名、触发类型、完整文本与费用/次数（class_name SkillList）
    selectable_box.gd        # 可选项框：内容容器 + 常态/悬停/选中描边与框内点击（class_name SelectableBox）
    icons.gd                 # 代码绘制的矢量图标（class_name Icons）
    stat_plate.gd            # 攻防底牌（class_name StatPlate）
    card_frame.gd            # 卡框三条嵌套描边（class_name CardFrame）
    card_top_row.gd          # 卡面顶栏：时代在左、等级星居中、费用在右（class_name CardTopRow）
    frame_material.gd        # 框体材质：给控件叠 shader 倒角边框与内侧阴影（class_name FrameMaterial）
    frame_material.gdshader  # 黄铜浮雕边框：时代合金色、三连足金与引导流光
    era_glyph.gd             # 时代象形（class_name EraGlyph）
    card_face_base.gd        # 卡面共享绘制：圆角封面、美术内缩、顶栏、自适应信息区、冻结与潜行蒙版、就地改身材
    card_text.gd             # 卡牌与卡面周边的显示文本格式化
    detail_layer.gd          # 详情层（class_name DetailLayer）：命中、定位与装配；内容由 detail_sections/ 渲染
    detail_kit.gd            # 详情内容公共辅助：标题、文本块、分类与关联行（class_name DetailKit）
    detail_sections/         # 商店、座位、售货员、英雄与随从详情各一文件
    card_gallery.gd          # 图鉴主体：头部、视图分段、筛选下拉、网格与详情浮层装配（class_name CardGallery）
    gallery_mode_bar.gd      # 图鉴视图分段：随从 / 英雄两枚按钮（class_name GalleryModeBar）
    gallery_filter_select.gd # 图鉴筛选触发器：显示「维度 · 当前值」，点击交给页面开面板（class_name GalleryFilterSelect）
    gallery_filter_panel.gd  # 图鉴筛选面板：挂在触发器下沿的条目与计数列表（class_name GalleryFilterPanel）
    gallery_grid.gd          # 图鉴卡面网格：按可用宽度分列、滚动分批加载（class_name GalleryGrid）
    gallery_card_tile.gd     # 图鉴卡面单元：复用 GalleryCardFace，继承 SelectableBox 的内联选中描边（class_name GalleryCardTile）
    gallery_card_face.gd     # 图鉴卡面：攻防、玩法、费用/时代与名称，不含效果文本
    gallery_detail_layer.gd  # 图鉴详情浮层：遮罩 + 右侧贴右整高抽屉（class_name GalleryDetailLayer）
    gallery_detail.gd        # 图鉴详情内容：卡面美术图/占位、分类、玩法、数值、技能与效果（class_name GalleryDetail）
  pages/
    login/
      login.tscn/.gd         # 登录
      login_field.gd         # 表单行：标题 + 输入框（class_name LoginField）
    lobby/
      lobby.tscn/.gd         # 大厅：内容区 7:3 两栏；左栏大厅图与入口，右栏玩家卡、中部卡片与操作区
      lobby_hall_image.gd    # 大厅图：位图 cover 铺满，缺图回退下沉面板（class_name LobbyHallImage）
      lobby_player_card.gd   # 玩家卡：头像缩写、显示名与编号（class_name LobbyPlayerCard）
      lobby_rank_card.gd     # 段位卡：段位名与积分（class_name LobbyRankCard）
      lobby_queue_card.gd    # 排队卡：等待时长与已凑人数（class_name LobbyQueueCard）
      lobby_left_actions.gd  # 左栏底栏：好友入口、卡牌图鉴、排行榜与 Steam 账号入口（class_name LobbyLeftActions）
      lobby_quick_actions.gd # 右栏快捷操作：匹配 / 取消与创建房间（class_name LobbyQuickActions）
      lobby_room_list.gd     # 创建模式座位列表：8 行、准备圆点、邀请行与 Steam 状态行，行区可滚动（class_name LobbyRoomList）
      lobby_room_actions.gd  # 创建模式操作：开始 / 解散与可见原因行（class_name LobbyRoomActions）
      lobby_notice_dialog.gd # 大厅提示弹窗：标题、说明与关闭（class_name LobbyNoticeDialog）
      invite_popup.gd        # 邀请弹层：订阅 social_changed、逐条接受/拒绝（class_name InvitePopup）
    room/
      room.tscn/.gd          # 房间等待区：装配座位矩阵、邀请与操作栏，收到 HeroPickStarted 后导航到对局页
      room_seat_grid.gd      # 8 座矩阵：4 列 2 行，按 seat 填座（class_name RoomSeatGrid）
      room_seat_cell.gd      # 座位格：头像、名字、房主/准备徽标与移除按钮（class_name RoomSeatCell）
      room_invite_row.gd     # 邀请入口：在线好友可邀请，其余置灰（class_name RoomInviteRow）
      room_action_bar.gd     # 底部操作栏：准备/离开/补位/开始/解散与状态行（class_name RoomActionBar）
    game/
      game.tscn/.gd          # 对局页面装配入口：右上角工具组、左栏与中央舞台，门控与意图
      game_ui_state.gd       # 一次刷新的页面快照：视图数据与门控结果
      shop_card.gd             # 商店卡拖拽源：拖到手牌区购买，点击不发意图（class_name ShopCard）
      shop_card_face.gd        # 商店卡面：费用/时代象形与冻结表现，不显示介绍
      hand_card_face.gd        # 手牌卡面：费用/时代、★ 或奖励星级与完整介绍（字号自适应）
      board_card_face.gd       # 棋盘与战斗只读卡面：★/潜行与当前身材
      game_aim_layer.gd        # 技能瞄准层：技能与战吼的选目标弧线、候选角标与命中；效果牌拖拽共用视觉（class_name GameAimLayer）
      game_choice_overlay.gd   # 选择浮层：发现候选卡面与进化选项块横排，点击即选中（class_name GameChoiceOverlay）
      game_tool_buttons.gd     # 右上角工具组：图鉴、提示、退出对局三枚胶囊按钮（class_name GameToolButtons）
      game_status_modal.gd     # 状态模态：等待、断线/重连、错误与未发出，居中且不锁对局（class_name GameStatusModal）
      game_players_rail.gd     # 左栏座位列：头像、名次、血条与座位详情
      game_shop_view.gd        # 商店：售货员两侧的图标操作行、6 槽与出售落点（class_name GameShopView）
      game_icon_button.gd      # 边长取 shop-icon-size 的图标按钮（桌面 56、手机 48），费用角标在右上、次数角标在左上（class_name GameIconButton）
      game_portrait.gd         # 圆角头像块（CombatIntro 等使用）：继承 AvatarFrame，可选头像贴图 + 玩法色封面与名字首字回退，名字条在裁切内叠层（class_name GamePortrait）
      game_river.gd            # 河道行：回合数、招募倒计时/结束招募、战斗结果（class_name GameRiver）
      game_hero_row.gd         # 英雄行：身高 84，英雄卡、被动标记与全部主动技能列（class_name GameHeroRow）
      game_hero_card.gd        # 英雄卡：84×84，继承 AvatarFrame，头像按圆角裁切铺满、描边压在头像上、右下角只显示当前生命（class_name GameHeroCard）
      game_hand_row.gd         # 贴底手牌行：80 步进重叠（手机 46）、整行居中（class_name GameHandRow）
      game_opponent_view.gd    # 战斗对手身份：头像、名字、英雄、血量条与幽灵（class_name GameOpponentView）
      game_recruit_view.gd     # 招募视图：商店、河道与我方棋盘
      game_combat_view.gd      # 战斗视图：对手身份、对手随从行、河道与我方随从行
      hand_card.gd             # 手牌卡：拖拽数据与预览，兼作商店卡落点（class_name HandCard）
      hand_slot.gd             # 手牌槽：80 步进重叠、悬停缩放与层级（class_name HandSlot）
      hand_drop.gd             # 手牌带底部购买落点：只接受可购买的商店卡（class_name HandDrop）
      board_drop.gd            # 我方棋盘落点：命中判定与插入下标（class_name BoardDrop）
      combat_playback.gd       # 战斗动作落地器：节奏与状态应用（class_name CombatPlayback）
      combat_fx.gd             # 战斗演出覆盖层：碰撞往返、闪光、飘字、淡出与出现（class_name CombatFx）
      combat_intro.gd          # 战斗开场覆盖层：双方头像、VS 与「战斗开始」，帧驱动淡入淡出（class_name CombatIntro）
      combat_hero_clash.gd     # 战斗结束覆盖层：双方英雄卡缓慢靠岸、猛击对冲、顶退、闪白与伤害飘字（class_name CombatHeroClash）
      first_game_guide.gd      # 首局非阻塞引导：目标高亮与底部提示条（class_name FirstGameGuide）
      hero_pick_overlay.gd     # 选英雄覆盖层：标题、倒计时、已选进度、4 张候选、8 座位点与确认（class_name HeroPickOverlay）
      hero_pick_list.gd        # 选英雄候选行：单行居中的 4 张候选（class_name HeroPickList）
      hero_pick_card.gd        # 选英雄候选卡：继承 SelectableBox 的外扩选中框（class_name HeroPickCard）
      hero_pick_detail_layer.gd # 选英雄悬停详情层：继承 DetailLayer，贴卡外侧且不显示图片（class_name HeroPickDetailLayer）
      card_pool.gd             # 本局卡池复算：与服务端 card_in_pool 同规则，排除非池卡（class_name CardPool）
    result/
      result.tscn/.gd          # 结算：名次、本人本局数据、最终阵容、种子与返回入口
      result_ranking.gd        # 名次行：名次、玩家名、存活回合与造成伤害（class_name ResultRanking）
      result_stats_section.gd  # 本局数据区与最终阵容（class_name ResultStatsSection）
      result_board.gd          # 最终阵容行（class_name ResultBoard）
    gallery/
      gallery.tscn/.gd         # 卡牌图鉴页：大厅入口，只做整屏背景、图鉴组件装配与返回大厅
  tests/
    ui_smoke.tscn/.gd        # 主冒烟：五场景实例化、拖拽/浮层/门控/结算等主流程
    ui_smoke_side.tscn/.gd   # 舞台结构：商店操作行、英雄行、河道与手牌
    ui_smoke_seats.tscn/.gd  # 座位行、名次角标、血量条与座位详情
    ui_smoke_combat.tscn/.gd # 战斗视图三段、河道回合数与英雄对冲演出
    ui_smoke_cards.tscn/.gd  # 商店卡费用/时代象形、卡框星级、攻防底牌与冻结蒙层
    ui_smoke_text.tscn/.gd   # 文本表取用、枚举展示与错误文案回退
    ui_smoke_social.tscn/.gd # 大厅好友浮层、邀请弹窗与房间邀请入口
    ui_smoke_gallery.tscn/.gd # 图鉴页与对局覆盖层：计数、筛选、滚动加载、详情与入口
    ui_smoke_guide.tscn/.gd  # 首局引导：默认隐藏、提示按钮打开、步骤推进、跳过、输入穿透
    ui_smoke_room_ops.tscn/.gd # 房间管理：房主移除/解散、非房主开始与解散隐藏而补位禁用、RoomClosed 回大厅、离线点击
    ui_smoke_touch.tscn/.gd  # 触控：长按详情、抬手隐藏、位移取消、手牌槽触摸复位
    ui_smoke_layout.tscn/.gd # 安全区包裹与命中尺寸；960×540、1280×800/1280×960 下六页面不越界，八人对局积分榜不滚动，手机随从卡放大而手牌与英雄卡不变
    ui_smoke_platform.tscn/.gd # 设备换算：画布缩放、pt→画布、安全区与键盘（固定输入断言）
    ui_smoke_local_server.tscn/.gd # 本地调试：房间码行解析、sidecar 路径与无 embedded_server 特性时不启用
    smoke_case.gd            # 冒烟公共断言与场景装载
    fixture.gd               # 冒烟用最小 PlayerView
    headless_game.tscn/.gd   # 无头对局驱动（1 客户端 + 7 机器人）
    wireframe_probe.tscn/.gd # 线框图探针：导出可见 Control 的全局矩形（`make ui-wireframe`）
    view_preview.tscn/.gd    # 运行态视图预览：真实窗口装载页面/场景并注入夹具（`make ui-preview`）
    preview_fixture.gd       # 预览夹具：`game`（八人对局 + 假连接）与 `none`（空态）
    preview_focus.gd         # 预览聚焦层：目标控件外侧描边，不改页面树
    previews/game_rail.gd    # 左栏积分榜单视图的 `SETUP` 构建脚本
  generated/
    protocol.gd              # schema 生成的 GDScript 常量
```

窗口启动时由 `WindowSizer` 按当前屏幕可用区域（`screen_get_usable_rect`，已排除菜单栏与 Dock）的 90% 设置尺寸，再用 `ScreenProfile.launch_window_size` 收成 16:9 并居中；无头模式下固定为设计基准尺寸，保证冒烟按 16:9 复现。`stretch/aspect` 为 `expand`：逻辑视口按设计基准（桌面/iPad 1280×720、iPhone 960×540）等比缩放后铺满整个窗口，宽度与高度都随窗口比例扩展，比例不匹配时内容框外的背景带由页面背景填满，不露引擎黑边。页面内容由 `UILayout.safe_area(page, page_texture)` 包成多层：`PageBackground` 留在包裹层外，铺满整个视口并按等比裁切（`STRETCH_KEEP_ASPECT_COVERED`），作为所有页面共用的软件背景；`ContentFrame` 由 `ScreenProfile.content_frame(viewport, base)` 按 contain 算出，固定 16:9、受限一侧贴满、另一侧居中留背景；`ContentRoot` 固定为设计基准尺寸，按 `frame_h / base_h` 整体缩放；`ContentRoot` 内是设计画布尺寸的 `PageBody`，其子节点 `PageSurface` 承载页面自己的背景（对局页传设备档位分区图，其余页面用统一语义面板），`SafeAreaMargin` 只约束内容。内容始终完整、居中、不裁切——窗口比 16:9 更宽时左右露背景，更窄时上下露背景。窗口尺寸只在启动时设定一次，拖动缩放后不回弹，全屏、最大化与触控端不介入。

## 2. 全局状态与网络层

- `GameStore`：保存 `playerId`、`playerCode`、`token`、`roomId`、`roomCode`、房间成员、`PlayerView`、`lastSeq`、排名、种子、本人赛后统计 `self_stats`、`RoomClosed` 原因 `room_closed_reason`，以及社交视图 `friends` / `incoming_requests` / `outgoing_requests` / `recent_players` / `pending_invites`。`handle(type, payload)` 只做字段搬运，发出 `auth_changed`、`room_changed`、`view_changed`、`events_received`、`game_started`、`game_over`、`room_closed`、`error_received`、`social_changed`、`invite_received`、`invite_result_received` 信号；每封 `StateUpdate` 的 `events` 在视图写入后原样转发一次，`Snapshot` 重连补发不转发；`FriendUpdate` 整段替换社交列表，`FriendInvite` 按 `inviteId` 去重后入队，`InviteAccepted` 移除邀请并自动补发 `JoinRoom{roomCode}`。`GameOver` 写入 `ranking`、`game_seed` 与 `selfStats`；`RoomClosed` 先暂存 `reason`，`reset_room()` 后写回 `room_closed_reason`、补发 `LeaveRoom` 并发出 `room_closed`。`reset_room()` 清空房间与对局状态，`clear_game_view()` 只清本局视图（保留房间与社交），`reset_social()` 清空社交，`reset_session()` 两者都清并清空身份与令牌。
- `Net`：持有一个 `WebSocketPeer`，`_process` 中轮询；连接建立后先认证，断线每 3 秒自动重连。重连时以令牌发 `AuthRequest`，若仍在房间再发 `Reconnect{roomId, lastSeq}`，由服务端回全量 `Snapshot`。连接前用 `prefer_ipv4_url` 把 `ws://` URL 主机名解析成 IPv4（`wss://` 保留域名交给 TLS 证书校验）：`WebSocketPeer` 只尝试解析结果的第一条地址，双栈主机名常先拿到 IPv6。
- 下行信封只校验 `v == 1` 与 `type`，`payload` 原样交给 `GameStore`；未知类型忽略。上行信封为 `{v, id, type, ts, payload}`，`id` 由时间戳与计数器生成。
- `CardDB`：启动时从服务端 `GET /content` 下载展示内容，包内不再包含卡牌数据。数据按内容版本缓存到 `user://content_cache.json`：启动先读缓存保证离线可用，再用 `If-None-Match` 请求；`304` 保留缓存，`200` 覆盖缓存。请求地址由 WebSocket 地址推导（`ws://` → `http://`、`wss://` → `https://`，去掉 `/ws` 后追加 `/content`）。无缓存且请求失败时 `has_failed()` 为真；页面监听 `content_changed` / `content_failed` 重建或提示。本地调试、冒烟、预览与线框图通过 `CIVILIZATION_CONTENT_DIR` 直接读 `server/data/v0.1/client-content.json`，不依赖服务端。卡名、技能文本与时代名不放在下发数据里，而是按文本 id 从 `client/text/zh_CN.csv` 取：`display_name` 取 `card.<id>.name`，`hero_introduction` 取 `card.<id>.introduction`，`display_text` 取 `skill.<id>.text`，`era_name` 取 `enum.era.<id>`。不参与任何规则判定。
- `CardDB` 的玩法与分类接口：
  - `taxonomy`：节点 id → `{id, parent, rank}` 的字典，来自下发内容的 `taxonomy.nodes`；
  - `playstyle_name(id)`：取 `enum.playstyle.<id>`，空 id 返回空串，缺键回退为 id；
  - `taxon_name(id)`：取 `enum.taxon.<id>`，空 id 返回空串，缺键回退为 id；
  - `taxon_path(id)`：从叶子沿 `parent` 上溯到根（最多 7 层），返回「界 → 科」顺序的展示名数组；未知节点或空 id 返回空数组；
  - `in_shop_pool(entry)`：读取卡牌 `in_shop_pool`，缺省 `true`；`false` 表示该卡不进商店卡池、共享牌池、三连与发现。
- `skill_name(id)`：取 `skill.<id>.name`，空 id 或缺键返回空串，避免内部技能 id 泄露到界面；卡牌详情与图鉴详情据此跳过没有展示名的技能。
- `heroes.json` 单独载入 `CardDB.heroes`，不与 `cards` 合并：英雄与同名随从卡 id 相同但分属两个集合。`CardDB.hero(id)` / `hero_text(id)` 只读英雄池；英雄卡没有 `card_type`、`taxon`、时代与数值，界面按需只展示介绍、玩法、关联与技能。
- `cards.json` 含 `categories`（`[{kind, value}]`，`kind` 为 `play` / `prey` / `host`）、`taxon` 与 `in_shop_pool`，不再含 `is_hero`；`in_shop_pool: true` 的卡至少 1 条 `play:`，没有 `neutral`；非池卡允许没有 `play:`，`card_type` 按玩法需要取 `minion` 或 `effect`，卡面照常展示名称、分类与数值。
- `Sfx`：8 路 `AudioStreamPlayer` 轮转播放，场景音效按 cue 读 `assets/audio/sfx/`，卡牌音效优先读 `assets/audio/cards/<cardId>/` 并回落 `assets/audio/cards/default/`；缺失资源静默，不阻塞对局。`cue_log` 只供无头冒烟断言，不参与运行时逻辑。资源与触发规则见 §11。

## 3. 六个场景

| 场景 | 职责 |
| --- | --- |
| 登录 | 输入手机号、固定验证码 `123456` 与显示名；服务器地址由构建配置决定，页面不提供输入；认证成功后进入大厅 |
| 大厅 | 展示本人 6 位玩家编号；创建房间或接受好友邀请入房；左下角「好友」入口打开好友浮层（按编号添加、处理申请、删除好友、从最近队友回加），右下角进入卡牌图鉴；有待处理邀请时弹出邀请浮层，接受后自动入房；收到 `RoomUpdate` 后进入房间；客户端不展示房间码，也没有手动输入入口 |
| 房间 | 8 座位等待区：顶部状态行（已准备进度、机器人补位开/关、座位数）与 4×2 座位矩阵（本人/已准备/未准备/空位四态，房主可见成员移除按钮，座位带准备徽标）；邀请行列出在线好友供一键邀请（对局中/离线置灰）；底部操作栏为准备、Steam 邀请、离开、机器人补位、开始、解散与中间状态行；开始由房主调用 `GameStore.request_start()` 显式发送 `StartGame`，收到 `HeroPickStarted` 后进入对局页；房间页不显示房间码；收到 `RoomClosed` 后按原因提示并回大厅 |
| 对局 | 酒馆战棋式布局：右上角浮层工具组为 `GalleryButton`、`HintButton`、`LeaveGameButton`，页面不再有顶部状态条，等待、断线/重连、错误与未发出统一用居中状态模态（`StatusModal`）显示；左栏 8 行座位（按服务端 `rank` 升序，头像、名次角标、血量条，悬停详情含身份与对战记录）；中央舞台自上而下为招募组合或战斗组合、英雄行与贴底手牌。招募组合 438 高：商店操作行 84（升时代图标—84×84 售货员—刷新/冻结图标）、商店 6 槽、河道（金币在左、回合数居中、倒计时在右）、按容量棋盘；战斗组合 438 高：对手英雄行 84（84×84 英雄卡 + 84 宽身份列）、对手随从行、河道上下各 6（金币在左、回合与结果居中、「战斗中」在右）、我方随从行。英雄行 84 高：84×84 英雄卡、24 高被动技能标记行与全部主动技能图标（主动技能按视图顺序横排），淘汰后显示「已淘汰 · 观战中」；金币已移入河道，英雄行不再显示。棋盘随从卡右上角的技能角标显示费用与剩余次数，点击或按住即可发动主动技能，需要目标时复用同一瞄准层。手牌行 146 高贴底，允许被压缩且卡底可越过页面底部。十四种意图对应操作，冻结按钮按 `affordances.canUnfreeze` 发送 `FreezeShop` 或 `UnfreezeShop`；`pendingChoices` 弹层只保留发现与进化，以一排卡面或选项块呈现，点击即发 `ChooseDiscover`；战吼目标待选与需要目标的英雄技能改为瞄准层（从技能按钮或入场随从拉出弧线，松手命中候选生效，落空或 `Esc` 取消/放弃）；`GameAimLayer` 同时为需要目标的效果牌拖拽绘制同一套视觉 |
| 结算 | 按服务端 `ranking` 展示名次行（名次、玩家名、存活回合、造成伤害）、`selfStats` 的本局数据区与最终阵容、`seed`；`ConfirmButton`（「确定」）只清本地房间并重新认证回大厅，不发 `LeaveRoom`（房间已在对局结束时销毁） |
| 卡牌图鉴 | 只读浏览卡牌（含非池卡）：不显示页面标题，头部一行放「随从 / 英雄」分段、时代 / 玩法 / 类型三个筛选下拉、结果计数与返回。随从视图三维交叉筛选；英雄视图隐藏时代与类型，按英雄玩法筛选。两套视图共用按可用宽度分列的滚动网格（桌面与 iPad 基准 9 列、手机基准 7 列，按批加载），点击卡面打开详情浮层抽屉（桌面与 iPad 440、手机 400，同为贴右整高抽屉），预览显示卡面美术封面或占位。大厅入口用全量卡表（当前 432 卡，其中 24 张非池卡）与全部 32 张英雄，对局入口以覆盖层只列本局卡池与本局可用英雄 |

- 结算页节点：`ResultScroll` / `ResultContent` / `RankingList` / `StatsSection` / `BoardList` / `ResultActions` / `ConfirmButton`。`StatsSection` 共 12 个 `StatRow_*`（战斗胜/平/负合为 `StatRow_combat`），缺字段按 0、空阵容显示一条「无随从」占位；页面只读取 `GameStore.ranking` / `self_stats` / `game_seed`，不推算；不监听 `RoomClosed`，房间已在对局结束时销毁。
- 房间管理：`room.gd` 仅在 `_is_host()` 且 `GameStore.phase == PHASE_LOBBY` 时渲染 `Kick_<playerId>`，`DissolveButton` 随房主身份显隐；点击发 `KickMember{targetPlayerId}` / `DissolveRoom{}`。收到 `RoomClosed` 时 `GameStore` 清房间并补发 `LeaveRoom`，`room.gd` 把 `kicked` / `dissolved` 写成 `ui.room.kicked_notice` / `ui.room.dissolved_notice` 后导航大厅。
- 对局内退出：右上角工具组 `LeaveGameButton` 发 `leave_game_pressed`，`game.gd` 发送 `LeaveRoom`、收起引导、`reset_room()`、`Net.relogin()` 并回大厅；服务端按弃权结算，退出座位保留到本局结束。
- 首局引导：`FirstGameGuide` 由 `game.gd` 在对局页装配后创建（`target_root` 为对局页），`_refresh` 调 `set_phase(phase, self_state)`，右上角工具组 `HintButton` 调 `show_manual()`；步骤与节点名见[首局引导组件](../../product/ui/components/first-game-guide.md)。
- 对局界面整块读取 `GameStore.view`，每次 `view_changed` 重建；按钮可用性优先取 `affordances` / `playable` / `can*`。点击类入口（刷新、升时代、冻结、英雄技能、结束招募、待选选择）另在 `game.gd` 用 `_click_allowed()` 复查阶段、视图新鲜度、倒计时、待选、已结束招募与本人出局，避免阶段切换瞬间按钮仍可点而把必然被拒的意图发给服务端；该门控不含连接状态，断线仍由 `_send_intent` 提示「未发出」。冻结与棋盘卡拖拽换位没有对应的槽位 affordance，按 `phase`、`self.recruited`、出局与下标做本地门控，与服务端拒绝条件保持一致；冻结按钮按 `affordances.canUnfreeze`（本回合冻结过且仍有冻结槽位）选择发送 `FreezeShop` 或 `UnfreezeShop`，冻结标记跨回合保留而解冻权不跨回合。
- 操作还要求视图自身的 `turn` 与 `GameStore.turn` 一致：新回合的 `PhaseChanged` 会先于新视图到达，旧视图只展示、不可操作，避免用上一回合的商店发意图。
- 倒计时由 `phaseDeadline` 计算；归零后结束招募、刷新、购买、出牌、出售与换位一并禁用，只等服务端推进阶段。
- 已结束招募（`self.recruited`）后只展示界面，不再发起意图，也不开启目标待选瞄准。
- 状态栏文案按错误码取 `UIWording.error_text` 的文本表条目（`error.<CODE>`）；悬停详情优先用服务端下发的 `messageId` 与 `messageParams`（`UIWording.error_tooltip` → `UIWording.error_text_by_id`），缺失时回退错误码文案。协议 `message` 只面向日志，不作为 UI 文案。错误文案停留 `ERROR_NOTICE_SECONDS`（4 秒）后清空，阶段或回合变化后的下一次刷新也清空；连接提示优先，写入连接提示或非错误提示时取消错误计时，避免一次拒绝常驻整局。
- `UINavigation.error_route` 决定错误后的去向：认证类错误回登录并清空会话，`NOT_IN_ROOM` / `ROOM_NOT_FOUND` 回大厅并清空房间；其余错误留在当前页面。
- 未连接时点击操作不会静默丢弃：状态栏提示「未连接，操作未发出」。客户端不维护目标重试表：目标只能取自视图候选，被拒后只提示错误并等待新视图。
- 页面切换统一走 `UINavigation.navigate(page, path)`：`change_scene_to_file` 会立即把当前场景摘下，而页面仍连着 `GameStore` 等 autoload 信号；摘下后的回调再取 `get_tree()` 会得到 `null`。`UINavigation.navigate` 以「页面仍在树上」为唯一判定，离树页面的导航直接丢弃并返回 `false`，场景脚本不再直接调用 `get_tree().change_scene_to_file`。
- 房间场景同时监听 `view_changed`：重连或晚到的视图显示对局已开始时直接进入对局；房间不存在或已不在房间时回大厅。
- 大厅好友浮层是独立组件 `FriendsPanel`（`class_name FriendsPanel`），继承 `PopupLayer`，由大厅挂载，左下角 `FriendsButton` 打开、显示待处理申请数；组件自己订阅 `GameStore.social_changed`，读取 `GameStore` 的社交列表，不做本地排序：好友顺序、在线状态与最近队友次序都由服务端给出。添加好友按 6 位编号（`FriendRequest{toPlayerCode}`）；申请条目提供同意/拒绝，好友条目提供删除；编号或输入非法时只在浮层状态行提示，不发请求。服务端错误在浮层可见时就地显示，不跳回主页。
- 邀请弹层是独立组件 `InvitePopup`（`class_name InvitePopup`），由大厅挂载；它自己订阅 `GameStore.social_changed`，按 `live_invites()` 过滤已过期条目并渲染居中面板（遮罩与居中由 `PopupLayer` 承担），条目显示邀请人名字、编号与房间码。接受与拒绝都发 `RespondInvite`（`accept` 为 `true`/`false`）并立即禁用该条按钮，结果清掉该条邀请后弹层自动隐藏；大厅只监听 `responded` 信号做状态文案。收到 `InviteAccepted` 后由 `GameStore` 自动补发 `JoinRoom{roomCode}`。
- 弹出层外壳统一由 `PopupLayer`（`client/ui/popup_layer.gd`）承担：全屏遮罩、内容宿主、居中 / 锚点 / 四向贴边定位与点击遮罩关闭。`InvitePopup`、`FriendsPanel`、`GameChoiceOverlay`、`CombatIntro` 使用居中定位，图鉴筛选面板 `GalleryFilterPanel` 使用锚点定位、透明遮罩与点遮罩关闭；各组件只提供面板内容与遮罩参数，不重复搭建遮罩和居中容器。首局引导的四向挖空与详情层的跟随指针不属于该外壳。
- 房间页的邀请入口只列 `status == "online"` 的好友，其余按钮禁用；点击发 `InviteToRoom{friendId}`，结果由 `FriendInviteResult` 回写状态栏（已接受/未接受）。邀请成功的提示不改变房间成员，受邀者按普通 `JoinRoom` 入房后才出现在座位列表。
- 商店使用 `HFlowContainer` 自动换行，左侧内容放入 `ScrollContainer`，1280×720 下不会因商店变长而溢出；手牌行同样使用 `HFlowContainer`，但槽位按 `card-width - hand-card-overlap` 步进排列（桌面 80、手机 46）、行内 `h_separation` 为 0，1–10 张保持单行，卡片自身桌面 112×146、手机 70×91 向右溢出并与下一槽重叠；棋盘与商店的随从卡改用 `minion-card-*`（桌面 112×146、手机 78×102），手机比手牌大一档。
- 手牌卡使用与商店/棋盘一致的卡面语言（由 `HandCardFace` 构建；介绍、费用、时代象形与底部名称、攻击/类型/生命信息区），介绍不截断：信息区按文本向上扩展，字号在 11–8 之间逐档缩小、行距为 0，直到全文放下，角标最后添加并压在信息区之上；只能拖拽出牌（`HandCard._get_drag_data`），点击不产生意图；落点由 `BoardDrop` 判定：随从牌按指针位置换算插入下标、棋盘已满时任何落点都不接收，需要目标的效果牌只有落在服务端下发的候选（`targetCandidates`，区域为棋盘或手牌）上才提交，落点候选即目标；其它落点静默取消。拖拽数据带 `epoch`（`_view_epoch`）与 `targets`（候选的 `{zone, id}` 列表），视图重建后松手按取消处理。
- 手牌的随从数值读服务端视图 `attack` / `health`（当前值，含成长等全部层数），不读 `CardDB` 卡面值；金色随从以金框与金色等级星表达并显示金色技能文案（`skill.<id>.text.golden`，缺键回落普通文案），奖励牌在顶栏费用左侧显示锁定的发放星级，并用「获得一张 N 星随从」作为原生 tooltip。拖拽 payload 带 `instanceId` 与 `golden` 供落点与预览判断，不参与意图。
- 手牌悬停由 `HandSlot` 承担：进入时槽位 `z_index = 1`，卡片以底边中点为轴放大 `hand-hover-scale`（1.12）；放大后的矩形包含原矩形，不会在卡片下缘反复触发 `mouse_exited`。悬停不读取 `drag_allowed`，禁用与战斗阶段同样生效；不改顺序、`handIndex` 或 payload。`HandCard` 在拖拽真正开始时发出 `drag_started`，槽位同步复位缩放与层级后才复制拖拽预览，保证预览为原始尺寸。`HandScroll` 关闭 `clip_contents`，让放大后的卡顶越过 154 高的滚动行继续绘制与命中；悬停卡会盖住「手牌」标题，属已接受的视觉取舍。Godot 的 GUI 拾取按树序逆序、不比较 `z_index`，因此悬停期间 `HandSlot` 把右侧槽位卡片的 `mouse_filter` 临时设为 `IGNORE`、复位时恢复 `STOP`：否则放大卡被揭示出来的右侧交叠带仍会命中并拖起下一张卡。左侧槽位不用处理，它在树序中本来就晚于本槽被拾取。拖拽取消或视图刷新后不自动恢复悬停，指针需移出再移入。
- 拖拽预览定位统一走 `UIDrag.preview(card, at_position)`：Godot 的 `Viewport` 在拖拽中每帧把预览控件的位置设成指针位置，直接偏移预览会被覆盖；helper 用包裹 `Control` 承载被拖卡片，并按起手点反向偏移（手牌先把 `at_position` 换算到画布，再按复位悬停后的卡片取起手点），指针因此停在按下时的卡片位置。手牌、棋盘与商店三种卡的 `_get_drag_data` 都使用这一路径。预览由 `duplicate()` 生成，只复制导出属性：卡面里依赖脚本变量自绘的控件（`StatPlate.kind`、`EraGlyph.era_id`、`CardFrame.level`）必须声明为 `@export`，否则拖拽预览与战斗克隆会回退默认样式；原卡不能直接放进预览层，Godot 会在发出 `NOTIFICATION_DRAG_END` 之前连同预览层一起释放。
- 手牌拖拽期间 `BoardDrop` 逐帧按指针位置复查落点（`hand_drop_accepted`）：棋盘行空白仍可放置；棋盘外、棋盘已满或需要目标却不在候选上判为无效，指针切换为禁止光标（`CURSOR_FORBIDDEN`），松手不产生意图，卡牌留在手牌。指针离开棋盘矩形时不设置禁止光标，交还给手牌卡的 `_can_drop_data`。
- 手牌随从牌拖拽时 `BoardDrop` 同时逐帧更新落点占位（`hand_preview_index` → `_place_hand_preview`）：在 `_resolve` 给出的插入下标处放 `HandDragSpacer`（`card-width` × `card-height`，透明、`MOUSE_FILTER_IGNORE`），其余随从实时让位；效果牌、棋盘外、棋盘已满或 `_resolve` 拒绝时由 `_clear_hand_preview()` 移除。占位状态 `_hand_spacer` 与棋盘卡换位的 `_drag_id` / `_spacer` 互不复用；视图重建导致占位失效时按空引用重建，拖拽结束或落点提交后清除。无头冒烟不更新真实指针，测试用 `force_drag` 建立拖拽状态后按显式坐标调用 `_update_hand_preview(pointer)`。
- 需要目标的效果牌也可以落在手牌候选上：`HandCard` 实现 `_can_drop_data` / `_drop_data`，只接受「需要目标的效果牌」且本卡实例编号出现在拖拽数据的 `targets` 中（区域 `hand`），拖到自己身上拒绝；接受时发出 `target_dropped(payload, target)`。信号由作为落点的目标卡发出，因此 `GameHandRow` 对每张手牌都连接该信号，不能只连接效果牌自身。
- 棋盘随从卡也使用拖拽：`BoardCard._get_drag_data` 交出 `{epoch, minionId, fromIndex, sellable, golden}`；`BoardDrop` 在 `NOTIFICATION_DRAG_BEGIN` 时隐藏原卡并插入同尺寸透明占位，指针移动时实时移动占位、其余随从让位，松手按「移除后被拖随从剩余序列中的插入下标」发 `RearrangeBoard`，拖拽结束恢复原顺序。商店面板是 `SellDrop`，只接受可出售的棋盘卡：拖入时金色描边并提示「松手出售 +N 金」（`golden ? 3 : 1`），松手发 `SellMinion`；手牌与不可出售随从静默取消。卡内不再生成出售与左右移位按钮。
- 拖放冒泡依赖控件的鼠标过滤：Godot 在「`_can_drop_data` 为假且 `mouse_filter == STOP`」的控件上中止沿父链的落点查找。棋盘随从卡（`BoardCard`）与商店槽位（`ShopCard`）设 `PASS`，保留自身点击/拖拽起手与悬停详情，同时把落点交给父级 `BoardDrop` / `SellDrop`。购买落点用独立控件 `HandDrop`（`STOP`）承载卡带空白区域；手牌卡子自身接住落在卡上的拖拽并转发给 `GameHandRow`。新增落点控件或改其鼠标过滤时必须保证落点仍能到达父级。
- 时代文案由 `CardDB.era_name` 把 1–7 映射为英文 id（`prehistoric`…`future`）后按 `enum.era.<id>` 取文本；玩法、卡牌类型、进化选项与攻防分层同理走 `UIWording.enum_text(group, id)`；对手身份块的英雄行与座位详情统一显示「时代 X」，英雄卡不再显示时代。
- 左栏固定宽 88，8 个座位行是 `SelectableBox`（`INLINE` 描边、`radius-md` 圆角、2px 边框、内容边距 2），边长取 `PlayersRail` 内宽与人均可用高度的较小值（布局前回退 `avatar-rail` = 44 / compact 40；桌面受栏内宽 64 限制），`PlayersRail` 是普通 `Control`，行按边长手动定位、整组在栏内上下左右居中、间距取 `seat-row-gap`，行最小尺寸保持 0，不参与左栏最小高度（避免行高反过来把页面撑出画布），不使用 `ScrollContainer`；头像为 `Avatar` 铺满行内容区，边长 = 行边长 − 4，`Avatar.create` 的 `border_color` 传透明色，外框由行底板绘制（本人行 accent 12% 背景与 accent 描边，普通行 surface 背景与 border 描边），悬停时描边换成 `color-border-strong`，不做选中态；名次角标（`rank` 为 `null` 时不渲染）收在头像内左上、圆形深色半透明底白字，血量条叠在头像底部内侧（左右与底部各内缩 `space-2xs` = 2、宽 `头像边长 - 4`、高 6、圆角取高度一半的胶囊，按 `hp / hpCap` 比例填充，无数字），不额外占用行高；`PlayersRail` 监听自身 `resized`，边长变化时下一帧整体重建；iPhone 基准改用 `seat-hp-bar-height-compact` = 4、`seat-row-gap-compact` = 2，座位行边长随可用高度约 58（头像 54，含 Home 指示条安全区时），8 席一屏放下且不滚动；行顺序按 `players[].rank` 升序（第 1 名在最上，`rank` 为 `null` 垫底、同序按座位号），每次视图重建重排，客户端不推导名次；已淘汰行经 `UIControls.apply_dim` 整体压暗但保留名次且仍可悬停。座位行不接点击：悬停时经 `detail.track` 把 `player`、`hpCap` 与整份 `view` 交给详情层，由 `SeatDetailSection` 渲染「名字 + 名次 / 英雄 · 时代 · 随从 N / 生命条（当前/上限数字叠在条上） / 对战记录（无标题）」，身份元信息压成一行，不再单列「生命」字样与存活、招募状态；详情生命条取 `seat-detail-hp-bar-height`（16），`HpBar` 的 `label_text` 用白字加 1px 深色阴影居中叠在条上；面板宽度按 `DetailLayer.DETAIL_WIDTHS` 取 `seat-detail-width`（236），其余详情仍用 `detail-panel-width`。对战记录按 `participantIds` 过滤 `view.combatLog`，一行一条，胜负分段着色，对手名过长省略，无条目显示「无战报」；服务端按回合升序下发并只保留最近 5 回合，客户端展示时按回合倒序（最近一场在最上）。
- 招募舞台的身份只由商店操作行承担：`SellerTile` 复用 `GameHeroCard`（桌面 84×84、手机 64×64，只显示立绘、不显示名字条），与我方英雄卡同组件、同尺寸；左侧 `UpgradeButton`、右侧 `RerollButton` 与 `FreezeButton` 均为 `shop-icon-size` 边长（桌面 56、手机 48）的 `GameIconButton`，费用角标在右上，随 `affordances` 门控；营业说明只在悬停时由 `DetailLayer` 的 `seller` 详情显示，不再有「已淘汰观战」身份拦截。`GameHeroCard.present_portrait(display_name, glyph, tint, art_path)` 设置封面/描边/贴图/首字并隐藏生命牌，`SellerTile` 传入 `GameShopView.SELLER_ART`（`res://assets/characters/avatars/seller_ava.png`，256² 方形头像，源立绘 `characters/portraits/seller_full.png`）；资源存在时叠加 `AvatarFrameArt`（位于头像框裁切层索引 0、忽略鼠标）并隐藏首字，路径为空或缺图时回退首字色块。
- 河道行分两种模式：招募河道 `RecruitRiver` 左端是金币胶囊 `RecruitGoldTile`（桌面 72×24、手机 64×20，淘汰后隐藏），中央是回合数 `RecruitTurnLabel`，右端 `RecruitCountdownFlip` 内为金色胶囊 `RecruitTimerLabel`「剩余 N 秒」，悬停翻成 `RecruitEndButton`「结束招募」，移出恢复，点击只发一次 `EndRecruit`；归零时 `RecruitTimerLabel` 显示「等待结算」并禁用翻转。战斗河道 `CombatRiver` 左端是 `CombatGoldTile`，中央是 `CombatTurnLabel`（始终只显示「第 N 回合」），右端是 `CombatStatusPill`「战斗中」；战斗阶段金币保持可见且静态。两套节点名不重名，避免 `find_child` 命中隐藏实例。
- 我方英雄在英雄行：`HeroCard` 为 84×84 方形，继承 `AvatarFrame`，整块头像按 cover 铺满并由框的圆角裁切，玩法色 2px 圆角描边叠在头像之上（无素材回退玩法色与首字），右下角只叠当前生命数字，生命牌挂在内容叠层、不被裁切，生命取服务端当前值；右侧 `HeroSide` 列自上而下为 `HeroMeta` 行（仅 `HeroPassives` 被动标记，行高取 `hero-meta-row-height`）和 `HeroPowers` 横排。主动技能对应 `HeroPowerButton<下标>`（费用角标右上、剩余次数角标左上，剩 1 次时隐藏次数、0 次时置灰），按 `self.heroPowers` 顺序重建，tooltip 首行为技能名；被动技能取 `CardDB.hero(heroId).skills` 中 `cost` 为空的项，按卡面顺序生成 `HeroPassive<下标>`，只读、不可点击、不置灰，tooltip 为「被动技能：名称 + 效果」，英雄切换时整组重建。本人淘汰后英雄行改显示「已淘汰 · 观战中」并隐藏英雄卡与技能列；战斗阶段隐藏 `HeroPowers` 按钮，英雄卡与被动标记保留。玩法、时代、介绍与技能文本只在悬停时由 `hero` 详情显示。
- 战斗舞台的对手身份由 `GameOpponentView` 渲染：普通战为 84×84 `GameHeroCard` + 84 宽身份列（展示名/名次、HP 行与血量条），英雄名由英雄卡 tooltip 与 `hero` 悬停详情承担；幽灵战英雄卡隐藏生命牌，身份列加「幽灵」角标与时代/随从数、不显示血量条，整块压暗；`opponentPlayerId` 为空时显示「等待配对」。整行宽 84 + 8 + 84 = 176，与我方英雄行镜像。身份只随战斗视图出现，招募阶段不渲染。
- 商店卡左上角渲染 `CostBadge`（`display_cost`）、右上角渲染无框 `EraGlyph`（`era`）；冻结由 `FreezeOverlay` 整卡蒙层（`freeze-overlay-alpha` = 0.22，经 `self_modulate` 只压蒙层自身）、间距 14 的 45° 冰晶细线与右下角「冻」字 `FreezeGlyph` 共同表达。
- 商店卡由 `ShopCard` 承载：`_get_drag_data` 交出 `{epoch, shopSlot, cardId, buyable}`，`drag_allowed` 取 `GameUiState.can_drag(buyable)`，不可拖时整卡压暗；点击不发购买意图。`GameShopView` 在 `NOTIFICATION_DRAG_BEGIN` 收到商店数据时隐藏原卡并在原位插入同尺寸透明占位（`ShopDragSpacer`），`NOTIFICATION_DRAG_END` 或视图重建时移除占位并恢复原卡，起手表现与棋盘卡一致。手牌带底部的 `HandDrop`（节点 `HandDropBand`，高 `card-height`、贴底铺满）是购买落点，只接受 `buyable` 的商店数据并发出 `shop_dropped`；手牌卡自身也接受同一数据并转发给 `GameHandRow`。商店卡拖拽期间 `GameHandRow` 逐帧按指针位置把购买带切为 `accent-bare` 描边，拖拽结束或移出即恢复。
- 棋盘容量取自 `PlayerView.boardSlots`（缺失时回退 7），满场标记用该数值；棋盘只渲染实际随从，不足满槽时按 `FlowContainer.alignment` 居中，行宽按满槽、行高保持 `card-height` 以保留整行拖放区，空棋盘也能接住首次上场。`BoardDrop` 仍由页面设置 `board_full`，控件本身不感知容量。
- 落点回调在 `epoch` 校验之外还要重新过一遍与拖拽起始同源的门控（`drop_gate`）：阶段为招募、未结束招募、视图未过期、倒计时未归零、`Net` 已连接、队首待选不是 `target`，本人未出局。拖拽途中换回合、开战或断线时松手不会发出意图。
- 手牌可拖动的前提是阶段为招募、视图未过期、倒计时未归零、`Net` 已连接、队首待选不是 `target`、且该卡 `playable`；任一不满足时 `_get_drag_data` 返回 `null`，拖拽根本不开始。
- 队首待选为 `target` 时由 `GameAimLayer` 进入待选模式：候选卡（棋盘、手牌、商店）靠建卡时写入的 `target_zone` / `target_id` 元数据被命中，候选显示金色角标（触控端按下前也常驻，并画出入场随从的起点圆环）；按住拖出弧线，松手命中发 `ChooseTarget { target }`，落空或 `Esc` 发 `AbandonTarget`；期间购买/刷新/升时代/冻结/结束招募/主动技能/出售/换位/拖拽全部禁用。页面每次刷新按队首待选调用 `begin_pending`，视图过期、归零、已结束招募、非招募阶段或离线时关闭瞄准。
- 英雄技能瞄准：`GameHeroRow` 每个主动技能按钮的 `button_down` 发出 `hero_power_down(skill_id)`；页面按 id 取对应 `heroPowers` 条目，`requiresTarget` 为假时 `pressed` 直接发 `UseHeroPower { skillId }`；为真且候选非空时页面调用 `begin_skill(power_rect(skill_id), candidates)`，命中候选后补 `target` 发 `UseHeroPower`，落空或 `Esc` 取消、不发意图、不扣金币与次数；候选为空时该按钮禁用，原因为 `NO_VALID_TARGET`。多技能各自计次，一个技能用尽不影响另一个。
- 棋盘随从技能角标：`GameRecruitView._minion_card` 把 `BoardAbilityBadge` 叠在卡片外层（`AbilityBadges`，右上纵向排列，避开车面顶栏与底部攻血数值），文本 `◆<费用>×<剩余次数>`，悬停显示技能名与文本；可用性取服务端 `abilities[].canUse` 且与拖拽同批的 `can_rearrange`。`minion_ability_pressed` / `minion_ability_down` 由页面处理：`requiresTarget` 为假时点击直接发 `UseMinionAbility { minionId, skillId }`；为真且可用时按下缓存含 `minionId` / `skillId` 的意图，从 `ability_rect()` 起手复用 `GameAimLayer.begin_skill`，命中候选后补 `target` 提交；拖动仍从卡身开始，角标不参与拖拽。`HERO_POWER_LIMIT` 在随从技能语境取 `error.HERO_POWER_LIMIT.minion`，其它错误回退通用文案。
- 效果牌拖拽的瞄准视觉由 `GameAimLayer` 的 `DRAG` 模式提供：`_process` 读 `gui_is_dragging()` 与 `gui_get_drag_data()`，拖拽数据带 `targets` 时按手牌实例编号定位起点并绘制；只画视觉，落点提交仍由 `BoardDrop` 与 `HandCard` 完成。拖拽数据里的 `targets` 必须复制后持有——瞄准层 `clear()` 会清空数组，直接引用会污染卡片的载荷。
- 战斗开场与开始音效同一判定：`_start_combat` 按 `startedAt` 算出 `elapsed_ms`，小于 `CombatPlayback.PACE_MS` 才显示 `CombatIntro` 并播开始音；续播进入中段不补。`CombatIntro`（`INTRO_MS = 1200`、`ENTER_MS = EXIT_MS = 160`）与 `CombatFx` 一样按帧驱动，身份取 `players` 中的本人条目与 `opponentPlayerId`，圆角头像与底色复用 `CardDB` 与 `GamePortrait`，幽灵战压暗并加角标。身份列放在 `CombatIntroCard` 底板上（`panel_style("raised")`），避免与河道回合数、随从卡文字重叠；底板随整体缩放淡入淡出。开场显示期间 `GameCombatView.advance` 只推进开场并直接返回，播放器与碰撞演出都停在起点，首条动作从开场结束的下一帧开始计数。覆盖层最后挂载、铺满战斗组合、`mouse_filter = IGNORE`，`reset()` 收起。
- 战斗演示由 `CombatPlayback` 驱动（`PACE_MS = 320`、`TAIL_MS = 2000`，节奏只在客户端，服务端不估算演示时长）：第 i 条动作在 `(i + 1) × 320ms` 应用，数值只取 `source`/`target`/`summon` 的服务端下发值，客户端不做加减；动作序列只含主动攻击，反击伤害已并入该条动作下发的快照。写入哪一行按动作的 `sourceSide`/`targetSide`（缺失时退回按 `minionId` 双侧查找）。幽灵战改为随机出局者的最终阵容，实例编号只在克隆内与己方重号时按需改写，因此两侧编号仍可判。动作播完并经过 `TAIL_MS` 后进入英雄对冲演出，胜负与伤害由演出表达，河道不再追加文字。
- 战斗行只渲染 `alive` 为真的随从：阵亡卡立即离场；播放器仍保留阵亡条目作为顺序锚点，复生把 `alive` 置回真后卡片在原相对位置重现。召唤按 `summon.position`（存活序列下标）插入，换算用 `CombatPlayback.alive_insert_index`，不会把阵亡锚点当计数。
- 战斗视图的占位在演示全程固定：对手英雄卡固定 `hero-card-size`（桌面 84、手机 64）、两条随从行的 `custom_minimum_size.y` 取 `card-height`，一方全灭后空行不塌陷；`CombatRiver` 恒取 `river-height`（44/30），上下间距各取 `_stage_gap()`（6/4），`CombatTurnLabel` 常驻显示回合数，结束对冲演出不改变任何行高，整个战斗组合固定 438（手机 288）。
- 战报归属回合：`GameStore.last_combat_turn` 记录收到 `CombatResult` 时的 `turn`，`combat_is_current()` 只在它与当前 `turn` 一致时为真；战报行、战斗视图与演示都以此为准。`_apply_view` 在阶段离开 `Combat` 时清空 `last_combat` 并复位 `last_combat_turn`，未配对或已出局的座位不会把上一场当成新一场重播。
- 演示进度以 `CombatResult.startedAt`（服务端开战时刻）为锚点：首帧按 `now − startedAt` 直接 `seek`，已播动作不重播；演示期间没有任何跳过或加速入口。
- 碰撞演出由 `GameCombatView` 在 `advance` 里驱动 `CombatFx` 覆盖层（`CombatFxLayer`，`mouse_filter = IGNORE`）：播放器返回本帧到期动作，视图先按服务端数值更新数据，再演出：
  - `attack`：隐藏攻方原卡并在原位抓取矩形，克隆同尺寸卡 130ms 飞向目标；命中时目标闪光、飘出「-伤害」；再 130ms 飞回原位，克隆释放、原卡恢复。`attack` 只应用 `attack`/`health`，不改 `alive`。
  - `death`：受影响卡按原位淡出 160ms 后移除；若攻方本帧阵亡，克隆已回到原位才消失。
  - `reborn`/`summon`：卡片按存活序列位置出现并缩放缓入 160ms。
  - `contagion`：更新数值后卡片短暂变色。
- 演出用逐帧推进而不是 `Tween`，便于无头冒烟按帧断言；`reset()` 与 `_start_combat()` 清理覆盖层与进行中的动画。
- 战斗结束后 `GameCombatView` 在 `banner_visible()` 首次为真时播一次结束音并发一次 `hero_clash_requested(winner_side, damage)`；页面把请求暂存到下一次 `_process`，待布局完成再取双方英雄卡与河道矩形，启动页面级 `CombatHeroClash`（`APPROACH_MS = 650`、`BREATH_MS = 120`、`STRIKE_MS = 140`、`PUSH_MS = 140`、`HOLD_MS = 80`、`RETURN_MS = 300`，`HIT_MS = 910`、`TOTAL_MS = 1430`）：双方克隆卡先缓慢移到河道上下两岸蓄势，再加速冲进河道中心相撞，胜方顶进、败方后撤并闪白飘出「-伤害」，平局双方互顶后弹回；结束后克隆释放、原卡恢复。英雄卡克隆走 `GameHeroCard.replica()` 按状态重建，避免 `duplicate()` 把 `AvatarFrame._init()` 建的默认子层叠在复制层上、盖住头像与描边。覆盖层铺满内容框、`mouse_filter = IGNORE`，位于详情 / 瞄准 / 状态 / 选择层之下。
- 演示播完且英雄对冲演出结束后，页面才为该回合发送一次 `CombatPlayed`（`_combat_ack_turn` 去重；`_pending_clash` 与 `playing()` 双重门控），由服务端在全部参战座位回报后结算；避免服务端提前推进到招募截断演出。
- 对局中主动退出使用 `LeaveRoom`：服务端在对局阶段按弃权结算，客户端收起引导、清空房间并重新连接、以令牌认证回大厅。结算页返回大厅同样清空房间并重新认证；对局结束即销毁房间，结算页不再有返回房间入口，也不显示房间关闭提示。
- 对局页的右上角工具组 + 左栏 + 中央舞台结构、页面状态与数据来源见[对局页](../../product/ui/pages/game.md)；颜色、字号、间距与圆角来自 `UITheme`，页面不写死视觉值。左栏宽 88；中央舞台自上而下为招募组合（438）或战斗组合（438）、英雄行（84）与贴底手牌（桌面 146 / 手机 91），桌面四个视图都按 `SIZE_SHRINK_BEGIN` 顶部对齐、手牌吸收富余高度，手机手牌改为 `SIZE_SHRINK_END` 且 `CenterPanel` 对齐 `ALIGNMENT_END`，整组贴底、富余高度留在舞台顶部；商店、棋盘、手牌与英雄行都不带标题。招募阶段与战斗阶段共用同一页面，战斗与结算阶段隐藏招募视图、商店操作、主动技能、倒计时与结束招募，被动技能标记保留，节点保留以便阶段恢复；战斗组合在招募阶段整块隐藏。
- 详情层（`client/ui/detail_layer.gd` 的 `DetailLayer`，面板节点名 `MinionDetailPanel`）统一承载商店、座位、售货员与随从四类详情，我方英雄详情亦由其 `hero` 类别渲染：商店卡、座位行与招募/战斗随从卡在悬停进入、鼠标左键按下或触摸按下时显示；指针移出锚点矩形后隐藏，松开鼠标左键或触摸时按指针所在目标矩形更新（由 `_input` 按全局矩形做命中判断，避免子控件截断悬停通知）。拖拽期间不显示任何详情：`NOTIFICATION_DRAG_BEGIN` 立即收起已打开的面板，`_show_detail` 在 `gui_is_dragging()` 为真时直接返回，购买经过棋盘、出售经过商店卡都不会弹出；松手或取消后指针移入才由悬停重新触发。随从详情先显示名称、科学分类（完整路径）、玩法、关联分类（仅 `prey:` / `host:`）与 `CardDB.display_text` 介绍，再显示对局中获得的关键词：取 `BoardMinion.keywords` 减去卡面 `skills` 的 `keyword` 派生值，按固定关键词顺序生成「获得」摘要行，并逐条给出 `ui.game.detail.keyword.<id>` 的规则说明（层数词代入 `%d`）；没有获得项或卡表缺少该卡时整组省略。之后再取协议 `BoardMinion.stats` 按初始、永久、回合、光环、受击、当前六行展示，有符号项带 `+`，受击的攻防都为 0 时省略该行；`stats` 缺失时仍显示名称与介绍，并退化为只显示顶层 `attack` / `health` 的当前攻防。商店详情同样先给分类与关联分类行，再接费用、原始数值、时代、技能文本与不可购买原因；英雄详情给介绍、玩法、关联分类与英雄技能。分类路径缺失显示「未定」，关联分类为空时省略对应行。座位与商店字段见[对局页](../../product/ui/pages/game.md)。面板只读、忽略鼠标输入，位置以锚点矩形为准并按视口边界夹紧；每次 `_refresh` 重建前先隐藏，同一目标重复触发显示时只校正位置、不重建内容。选英雄覆盖层由 `HeroPickDetailLayer` 继承该层，只覆盖内容与定位：面板名 `HeroPickDetail`、宽度取 `hero-detail-width`（手机 120），贴悬停卡外侧挂载（间隙 `space-sm`、竖直与卡片居中对齐；夹紧安全区后若压住悬停卡则换到另一侧），详情不含图片，只显示名字、介绍与技能；悬停与长按状态机与对局详情层完全共用。
- 冒烟测试按 `client/tests/ui_smoke*.tscn` 拆分场景，`make client-smoke` 用 glob 逐个运行并要求各自输出 `UI_SMOKE_OK` 且无 `SCRIPT ERROR`。主冒烟覆盖五场景实例化、手牌重叠布局（十张 80 步进单行、相邻交叠、末卡不越界、滚动行不裁剪）、手牌显示服务端身材与奖励牌星级角标、手牌悬停（置顶、底边中点放大 1.12、越出滚动行仍命中、右侧交叠带仍归本卡且拖拽拾起本卡、移出复位、战斗阶段可查看）与拖拽开始复位、真实鼠标事件拖拽（按下、越过阈值、移到落点、松手）落盘与商店出售、拖拽落点意图（首卡 0、次卡 1、末尾空白为随从数）、棋盘卡换位（真实 `force_drag` 通知、原卡隐藏、落点发 `fromIndex/toIndex`）、商店出售落点（可卖与不可卖、手牌拒绝、普通/金色返还提示）、商店卡拖到手牌带购买（卡带空白与手牌卡上都可落、点击不发意图、不可购买不可拖、过期 `epoch` 与无效落点不发意图、拖起隐藏原卡并插入同尺寸占位、取消后恢复、三种卡拖拽预览保持起手点且血量底牌、时代图标与卡框等级与原卡一致）、拖拽期间不显示悬停详情（购买经过棋盘、出售经过商店卡，取消后悬停恢复）、棋盘满时不接收、需要目标的效果牌落点规则、四个不可拖动条件、落点门控（换回合/开战/结束招募/断线时不发意图）、目标待选瞄准（候选角标、松手命中、落空与 `Esc` 放弃）与控件阻塞、技能瞄准（按下进入、命中提交、落空与 `Esc` 取消）、效果牌拖拽的瞄准视觉、战斗逐动作播放（阵亡卡离场、复生回原位、召唤按存活下标插入）、横幅停留、按截止时间续播、无跳过入口、战斗阶段不开启目标待选瞄准、回合号不匹配时不显示旧战报、离开战斗后清空战报，以及离树导航守卫、点击门控（非招募、已结束招募、旧视图、出局与目标待选时点击各入口不发意图，出局冻结按钮禁用）与错误文案生命周期（到时清空、回合变化清空）。`ui_smoke_side` 断言舞台结构：无右栏与旧手牌节点，中央舞台顺序为招募组合—战斗组合—英雄行—手牌，招募阶段只显示招募组合；商店操作顺序为升时代—售货员—刷新—冻结且均为 `shop-icon-size` 边长，售货员 84×84 且复用 `GameHeroCard`（头像框圆角裁切、描边层在头像之后、生命牌隐藏），冻结按钮按 `canUnfreeze` 与跨回合保留两种情况分别发 `UnfreezeShop` / `FreezeShop` 且 tooltip 同步，河道位于商店与棋盘之间且回合数居中、倒计时贴右端；英雄行 84、英雄卡 84×84（头像按框圆角裁切铺满、描边层在头像之后、右下角只显示当前生命）、主动技能图标 `shop-icon-size` 边长、被动技能标记 24×24（金币已移入河道），被动标记悬停文案含效果文本、主动技能禁用时仍不置灰、战斗阶段保留、无被动技能时不残留；手牌在英雄行下方且底边为 712；倒计时悬停翻成结束招募与归零文案；战斗对手的普通/幽灵/等待配对三种身份，以及淘汰与幽灵舞台；`ui_smoke_seats` 断言 8 个正方形座位行等边长、行边长等于头像边长加两侧描边、头像圆角裁切（封面绘制开启、描边层在裁切层之后）、名次角标缺失与压暗、名次角标位于头像内左上且为圆形白字、血量条比例、悬停座位详情含名字/名次/英雄 · 时代 · 随从、生命与对战记录、缩放内容框下详情仍贴座位行、幽灵/无战报文案、点击不再产生独立战报；`ui_smoke_combat` 断言战斗组合子节点顺序为对手身份块—对手随从行—河道—我方随从行且无旧英雄条，对手英雄卡 84×84 且复用 `GameHeroCard`、`CombatGoldTile` 可见，河道在播前与播后都只显示回合数，以及固定占位（两行按 `card-height` 取行高、河道按 `river-height`、一方全灭后行高与位置不变）；英雄对冲断言胜局双方克隆出现且保留头像与描边、原卡隐藏、败方飘出 `-5` 且回位后恢复，负局伤害落在我方一侧、平局无伤害飘字、幽灵战同样播放，并断言对冲结束前不发 `CombatPlayed`。`ui_smoke_cards` 断言商店卡与棋盘卡的顶栏时代角标与费用、冻结蒙层/冰晶/「冻」字、商店卡不渲染介绍且信息区为紧凑高度 56，以及卡面统一 112×146；卡面封面为圆角面板、美术向内收 3px、卡面容器只让出 1px 描边；图鉴卡显示玩法、不显示效果文本；手牌最长介绍不限制行数、不省略、超过两行且缩到 8px 后完整显示，信息区向上扩展且不压名称与角标；紧凑手牌不溢出、不压名称；卡框铺满卡面并绘制在角标之下、星级取卡表 `level`、右上角为 `EraGlyph` 且 `era_id` 取卡表、攻防为 `StatPlate`（空值与「—」不画装饰）、页面背景为整屏拉伸纹理；`AvatarFrame` 直测封面绘制、圆角裁切、描边层次与首字回退。卡面类型行显示玩法（哺乳纲随从为「哺乳纲」、多归属用「、」分隔）且封面取 `UITheme.playstyle_cover`；`UITheme.PLAYSTYLE_COLOR` 14 个键的颜色互不重复、封面色比原色更暗；详情面板含科学分类与玩法行，机械工具卡与机械随从均显示「机械」，不显示「中立」与关联分类行；随从详情显示对局中获得的关键词摘要与逐条规则说明，卡面已有词与没有获得项时不显示该组；英雄详情显示介绍与技能，不含类型「英雄」与科学分类行；选英雄覆盖层悬停详情不含图片，贴着悬停卡外侧（间隙不超过 `space-md`）。`ui_smoke_text` 断言 `CardDB.display_name`（含非池卡）、`display_text`、`era_name` 与 `UIWording.enum_text` 的取用，`CardDB.playstyle_name`（含 `neutral`）、`CardDB.taxon_name` 与 `CardDB.taxon_path`（完整路径、空 id、未知节点），`UIWording.error_text` 的错误码文案，`UIWording.error_text_by_id` 的 `messageId` 细分文案，以及缺键时 `UIWording.error_text_by_id` 返回空串。`ui_smoke_gallery` 断言大厅图鉴页无标题与左栏、头部为视图分段 + 三个筛选下拉 + 计数 + 返回（`GalleryTitle` / `GalleryToc` / `GalleryCategoryPanel` 均不存在）、默认选中随从分段、432 张计数、`GalleryGridRows` 按可用宽度铺满（桌面基准 9 列）且至少加载一批 27 张、存在 `GalleryScroll`、无分页条、底部显示已加载张数与继续下滑提示、进入时预选首张且 `GalleryDetailLayer` 保持收起、头部不横向溢出；筛选下拉「时代 / 玩法 / 类型」初始均为「全部」，零计数项禁用（未来章下鱼纲 `TaxonFish`），史前 86、未来 47、未来 + 哺乳纲 15、清除时代保留哺乳纲 122、全部回到 432；哺乳纲 122、类型随从 97 / 效果 25，切换玩法后类型回到「全部」；英雄分段切入 32 张、时代与类型下拉隐藏、玩法下拉换成 `HeroCategory*`（全部 32、哺乳纲 20），英雄卡面无角标、技能文本、费用与攻防，点击打开详情并显示名称与介绍且不含类型 / 科学分类 / 时代行，切回随从恢复 432 与时代下拉；切换视图保留史前筛选（86 → 英雄 32 → 86）；空英雄池时英雄分段禁用并保持随从。详情预览有美术时显示 `GalleryDetailArt`、无美术显示 `GalleryDetailArtPlaceholder`，预览框 160×209 且英雄与随从共用同一张卡面；空图鉴显示空态、计数 0 且详情收起；滚到底部追加下一批、卡面按下冒泡到滚动区且拖动不切换详情；桌面点击卡面打开 `GalleryDetailLayer`、点遮罩收起、重新打开后 `Esc` 只收详情并保留选中描边，抽屉无关闭按钮，元信息一行含类型、费用与带标签攻防并含科学分类、无时代象形，效果标题下逐行显示效果文本；技能缺展示名时不泄露内部 id；手机 960×540 下按宽度分列（基准 7 列）、列数随容器变窄而减少、头部控件与详情抽屉不越界、首屏至少 12 张、点击卡面打开贴右抽屉、`Esc` 只收抽屉；大厅「卡牌图鉴」入口文案；对局覆盖层为 177 张且 `alan-turing`、`airplane` 在池、`bruce-lee`、`ammonite-token`、`aristotle` 不在池，不切换场景、返回按钮与 `Esc` 分层收起（先详情后覆盖层）。`ui_smoke_guide` 断言进入对局默认隐藏、点右上角工具组「提示」后从第 1 步打开、「下一步」按顺序推进、「跳过」收起、战斗阶段从战斗步骤开始、阶段变化切换步骤，以及引导显示时拖拽购买仍到达底层控件。`ui_smoke_room_ops` 断言房主视图渲染 8 个 `RoomSeatCell`、房主可见 `StartButton` / `BotFillButton` / `DissolveButton` 与其他成员的 `Kick_<playerId>`、非房主开始与解散隐藏而补位开关可见但禁用、离线时管理操作不可用、`RoomClosed` 的 `kicked` / `dissolved` 提示后回大厅；`ui_smoke_hero_pick` 断言 `HeroPickOverlay` 渲染 4 张候选与 8 个座位点（已选绿点、本人金圈）、点击选中与确认后锁定、断线提示且确认不锁定、重连后可确认、超时停在 `00:00` 并禁用确认、`GameStarted` 后释放覆盖层、覆盖层拦截背景输入且手机档位 4 张候选不越界；`ui_smoke_touch` 断言触控端长按 450ms 显示详情、短按不显示、位移取消、多指与阻塞态不触发，以及手牌悬停残留复位、选英雄覆盖层长按详情/短按选中，和触控待选按下前的候选角标与起点圆环；`ui_smoke_layout` 断言六个页面在 960×540 下不越界、八人对局视图下积分榜不使用滚动容器且 8 个正方形座位行等边长并落在左栏内（含底部安全区偏移）、棋盘与手牌不越界且手牌贴底、棋盘与商店卡不渲染介绍而手牌保留、iPhone 紧凑卡保持桌面 112:146 卡形、手机对局手牌按 hand-card 尺寸放大且舞台富余高度留在顶部、对局英雄卡封面按圆角裁切铺满方形卡，并断言安全区与键盘边距取较大值、桌面边距为零、背景置底且详情层在缩放内容框下的坐标换算正确；`ui_smoke_platform` 断言桌面、iPhone 与 iPad 三档逻辑基准、44pt/32pt 命中尺寸与安全区、键盘像素换算。

- 图鉴页大厅入口由 `lobby.gd` 的「卡牌图鉴」按钮经 `UINavigation.navigate` 进入 `res://pages/gallery/gallery.tscn`，随从视图取 `CardDB.cards.keys()`（含非池卡），英雄视图取 `CardDB.heroes_array()`；对局入口是右上角工具组里的 `GalleryButton`，`game.gd._open_gallery` 首次创建 `CardGallery` 覆盖节点（`GalleryOverlay`，铺满内容框，`closed` 只收起 `visible`），英雄集合按本局 `CardPool.hero_ids(view.enabledFactions)` 过滤，不切换场景、不暂停视图刷新，拖拽、待选与战斗演示状态保留；`_refresh_overlay` 在选择浮层可见时调用 `_close_gallery`。
- 图鉴头部只占一行，依次为 `GalleryModeBar`、`GalleryEraSelect` / `GalleryPlaySelect` / `GalleryKindSelect`、结果计数 `GalleryCount` 与返回按钮；不渲染标题或副标题。`GalleryModeBar` 并排两个 `ButtonGroup` 分段（`ModeMinion` / `ModeHero`），当前视图使用 `primary` 样式，点击 emit 另一视图，默认 `minion`；英雄集合为空时英雄分段禁用并保持随从。三个筛选触发器都是 `GalleryFilterSelect`，文字为「维度 · 当前值 ▾」，选中非「全部」的条目时文字转 `color-accent`；点击由 `CardGallery` 在按钮下沿打开 `GalleryFilterPanel`（`PopupLayer` 的 `ANCHORED` 定位、透明遮罩、点外部关闭），条目为 ghost 按钮，选中项用 `accent-row`，零计数且未选中时禁用。随从视图三维筛选交叉生效：时代、玩法与类型；每个维度的计数都按另外两维过滤。玩法按 `CardDB.playstyles` 的固定顺序展示「全部」与 13 个玩法，不含「中立」；卡牌按 `categories` 中的全部 `play:` 归属归入对应玩法，带多条归属的卡在多个玩法中同时出现，「全部」按唯一卡牌 id 计数。效果牌与英雄不作为顶层玩法；「类型」下拉提供「全部 / 随从 / 效果」，改玩法时类型回到「全部」。英雄视图隐藏时代与类型下拉，玩法下拉按同一顺序展示英雄玩法，英雄按全部 `play:` 归属计数，带多条归属的英雄在多个玩法中同时出现，零计数玩法显示并禁用；两套视图各自保留筛选，切换后回到首批并选中结果首张。非池卡随 `cards.json` 自带 `card_type`，按自身类型参与「类型」筛选。网格为 `GalleryScroll → GalleryScrollBody → GalleryGridRows` 的滚动结构：`set_cards` 重建首批，滚到距底部一张卡高时追加下一批，首批不足一屏时自动补足；列数在首次布局后按可用宽度计算一次并扣除纵向滚动条宽度（桌面与 iPad 基准 9 列、手机基准 7 列，安全区收窄时按实际宽度减少），卡宽按列数平分整行并保持 112:146（不小于 `card-width`），余量由网格按列均分、整行贴到右缘；桌面与 iPad 列距 `space-lg`、手机 `space-md`；每批张数取 `gallery-batch-size` = 27、手机 12（`gallery-batch-size-compact`），不再与列数绑定。卡面 `mouse_filter` 为 `PASS`，按下与拖动冒泡到 `GalleryScroll`，由 Godot 内建触屏拖拽滚动；卡面在松手且位移不超过 12 像素时才发出选中，位移按画布坐标比较、滚动量不计入，避免触屏滚动误开详情。底部 `GalleryLoadStatus` 显示「已显示 N / M 张 · 继续下滑加载 / 已全部加载」。进入页面只预选首张并保留描边，不打开详情；点击卡面时 `GalleryDetailLayer` 在头部以下显示遮罩并打开右侧抽屉，`Esc` 按「筛选面板 → 详情抽屉 → 返回」逐层关闭。选中卡不在结果内时改为结果首张，结果为空时显示空态。普通卡排序按时代序号（未知排最后）再按本地化名称，英雄按本地化名称。
- 图鉴详情抽屉桌面与 iPad 宽 `gallery-drawer-width` = 440、手机 `gallery-drawer-width-compact` = 400，贴右整高：顶部对齐头部下沿、底部贴页面底边，右侧外边距桌面 24、手机 16；不含关闭按钮，只靠点遮罩与 `Esc` 收起。遮罩从头部下沿开始，透明度取 `scrim-alpha-gallery`（0.62）。`GalleryDetail` 是抽屉内的滚动内容，内边距桌面 `space-xl`、手机 `space-md`。首行为左封面右信息列：封面宽度取 `gallery-preview-width` = 160（手机 `-compact` 112），高度由 `UITheme.card_face_height(width)` 按 112:146 计算；右列为名称、元信息一行（类型 · 时代 · 费用 · 攻/血）、玩法、关联分类与科学分类，卡片下方为分隔线与效果逐行文本。预览区以 `CardDB.art_path(id, "card_face")` 取卡面美术，英雄视图与随从视图共用同一张图。无美术时用 `UITheme.playstyle_cover` 底色与 `CardText.initial` 占位，不再实例化 `GalleryCardFace`。
- 本局卡池由 `CardPool.card_ids` 复算：遍历 `CardDB.cards` 并跳过 `in_shop_pool: false` 的非池卡；卡牌任一条 `play:` 命中 `enabledFactions` 即入池，池内卡没有 `play:` 在加载期已被拒绝，与服务端 `card_in_pool` 一致；结果只用于展示，不影响出牌与商店合法性。

## 4. 生成物与命令

| 命令 | 内容 |
| --- | --- |
| `make protocol` | 由 `protocol/schema/v1/` 生成 `protocol/generated/protocol.gd` 与 `client/generated/protocol.gd` |
| `make protocol-check` | 校验协议样例，并比对两处 GDScript 生成物漂移 |
| `make data` | 生成服务端数据（含 `taxonomy.json`）、客户端下发内容 `server/data/v0.1/client-content.json` 与文本表 `client/text/zh_CN.csv` |
| `make data-check` | 比对服务端与客户端生成数据、文本表漂移，运行效果审计 |
| `make assets` | 按 `ASSET_MAP` 对源立绘去水印，派生头像与卡面并更新 `client/assets/asset_manifest.json`（保留清单内其它条目）；需安装 Pillow，见 §10 |
| `make assets-covers` | 对 `ASSET_COVERS`（默认 `output/effect-covers/generated`，临时素材目录，处理完可删）下 `<card_id>/*.png` 去水印，缩放到 448×584 写入 `client/assets/cards/faces/`，只登记 `card_face` 与 `display.card_face`；需安装 Pillow，见 §10 |
| `make sfx` | 生成 9 个默认合成占位音效到 `client/assets/audio/`；逐卡覆盖见 §11 |
| `make font-subset` | 由文本表 + ASCII + GB2312 生成内置中文字体子集 `client/assets/fonts/NotoSansSC-Regular-subset.ttf`；见 §6 |
| `make client-import` | 导入客户端工程，默认用 `godot --headless --path client --import`（CPU 压缩）；`CLIENT_IMPORT_GPU=1` 改走非无头导入（仅 S3TC/HDR 纹理能用 GPU 压缩），`auto` 按图形会话自动判断。PNG 纹理导入本身走线程池，见 §10 |
| `make client-cache-copy` | 从主检出复制 `client/.godot/imported` 与源文件仍存在的 `*.import` 旁车到当前 worktree；`client-import` 已在新 worktree 首次运行时自动播种，本目标保留严格语义：在主检出执行时为空跑，主检出缺缓存时失败并提示先执行 `client-import`，见 §10 |
| `make client-run` | 先导入再启动桌面客户端；新增或改名 `class_name` 脚本后必须走这个入口，否则沿用旧全局类缓存。`DEVICE` 取 `phone`、`tablet`、`desktop` 时按该设备档位运行，取 `iphone`、`ipad` 时同时套用验收机横屏尺寸；同时传 `WIDTH=<宽> HEIGHT=<高>` 时按指定像素开窗并覆盖预设尺寸。显式传入 `DEVICE` 且环境为 `local` 时默认内嵌启动本机服务端与 7 个机器人（`LOCAL_SERVER=1`），`dev`/`prod` 与不传 `DEVICE` 时保持关闭；也可用 `LOCAL_SERVER=0/1` 显式覆盖。运行前按 `ENV`/`env` 写入连接地址：`local` 为空配置，`dev`/`prod` 临时写入对应 `BuildConfig.SERVER_URL` 并在退出后恢复。`PLATFORM=ios` 改为导出并安装到真机，见 §5、§9 |
| `make client-run PLATFORM=ios` | 导出 iOS 签名包后用 `xcrun devicectl` 安装到真机并启动；默认第一台已连接的 iPhone，`IOS_DEVICE_TYPE=ipad` 改选第一台已连接的 iPad，`IOS_DEVICE` 指定具体设备（设备名或 UDID），`IOS_LAUNCH=0` 只安装 |
| `make run-<设备>-<环境>` | `client-run` 的设备与环境别名：设备取 `desktop` / `iphone` / `ipad` / `phone` / `tablet`，环境取 `local` / `dev` / `prod`，共 15 个（如 `run-desktop-dev`、`run-iphone-local`）。设备含义同 `DEVICE`；`local` 默认内嵌本地服务，`dev` / `prod` 默认不启本地服务，并在启动前临时写入对应地址、退出后恢复。`make run` 等价于不带参数的 `client-run` |
| `make run-iphone` / `run-ipad` / `run-desktop` / `run-phone` / `run-tablet` | 旧设备别名，保留为对应 `local` 快捷入口（等价于 `run-<设备>-local`）；`run-iphone` 与 `run-ipad` 同时套用验收机尺寸 |
| `make run-ios-<设备>-<环境>` | 同时固定 `PLATFORM=ios`、设备类型与 `ENV`：设备取 `iphone` / `ipad`，环境取 `local` / `dev` / `prod`，共 6 个（如 `run-ios-ipad-dev`）。设备类型交给脚本自动选已连接的同类型设备，`IOS_DEVICE` 仍可覆盖具体设备；`local` 写空配置，`dev` 写开发公网入口，`prod` 需要先配置 `CLIENT_PROD_SERVER_URL` |
| `make run-ios` | 旧真机别名，保留为 `run-ios-iphone-local`（默认 iPhone） |
| `make export-macos` / `export-local` / `export-steam` / `export-ios` / `export-android` / `export-aab` | `client-export` 的平台别名，分别固定 `PLATFORM=`；`export-aab` 同时设 `AAB=1`。`make export` 等价于默认 `client-export`（macOS）。`ENV`、`SERVER_URL`、`STEAM_APP_ID` 等参数照常透传 |
| `make export-<平台>-<环境>` | 同时固定 `PLATFORM` 与 `ENV`：平台取 `macos` / `ios` / `android`，环境取 `local` / `dev` / `prod`，共 9 个（如 `export-ios-dev`、`export-android-prod`）。`local` 写空配置，`dev` 写开发公网入口，`prod` 需要先配置 `CLIENT_PROD_SERVER_URL`。Android AAB 仍用 `make export-aab ENV=…` |
| `make export-ipad-<环境>` | iPad 命名入口，固定 `PLATFORM=ios` 与 `ENV`，环境取 `local` / `dev` / `prod`，共 3 个（如 `export-ipad-dev`）。iOS 预设是 iPhone & iPad 通用包（`targeted_device_family=2`），产物与 `export-ios-<环境>` 完全相同，只用于与 `run-ios-ipad-<环境>` 对齐；地址规则同上 |
| `make client-check` | 协议/数据/文本表漂移 + 客户端边界断言 + 国际化断言 + 工程导入 |
| `make client-env-check` | 校验客户端构建环境只能是 `local`、`dev` 或 `prod`，默认 `local`；`ENV` 与 `env` 等价，同时传且值不同直接失败；`dev`/`prod` 分别要求 `CLIENT_DEV_SERVER_URL` / `CLIENT_PROD_SERVER_URL`，未配置直接失败 |
| `make client-smoke` | 用 glob 逐个运行 `ui_smoke*.tscn` 冒烟场景，要求各自输出 `UI_SMOKE_OK` 且无 `SCRIPT ERROR`；任一场景断言失败时不输出 `UI_SMOKE_OK`，目标以非 0 退出；`SMOKE=<名称>` 只跑 `ui_smoke_<名称>.tscn` |
| `make client-export PLATFORM=local` | 依赖 `client-export-templates`、`client-import` 与 `server-build`；导出调试用 `output/client/Civilization-local-<构建号>.app`，并把 debug 的 `room-service`、`sim` 复制进 `Contents/MacOS/`；仅 macOS、仅本机调试，见 §8 |
| `make ui-preview` | 真实窗口装载指定页面/场景并注入夹具，`FOCUS` 高亮子节点；不截图，窗口保持打开；见 §5 |
| `make ui-preview-check` | 与 `ui-preview` 参数相同但自动退出，按 `VIEW_PREVIEW_OK` / `VIEW_PREVIEW_FAIL` 判定；只用于本机 GUI 冒烟 |
| `make client-export PLATFORM=ios` | 按 `ENV`/`env` 写入对应环境地址（默认 `local` 写空配置）后导出 Xcode 工程，并用 `xcodebuild` 自动签名，打包 `output/client/Civilization-ios-<构建号>.ipa`；见 §9 |
| `make client-export PLATFORM=android` | 导出 arm64 APK 到 `output/client/Civilization-android-<构建号>.apk`；签名参数从 `ANDROID_KEYSTORE_*` 环境变量读取，缺失时失败；见 §9 |
| `make client-export PLATFORM=android AAB=1` | 导出 arm64 AAB 到 `output/client/Civilization-android-<构建号>-aab.aab`；同一 keystore 签名，走 Gradle 构建；见 §9 |
| `make client-e2e` | 启动服务端、Godot 无头客户端与 7 个 `sim bot`，要求输出 `CLIENT_E2E_OK` |
| `make dev-bots` | 启动服务端（监听 `0.0.0.0`，手机可直连）与 7 个机器人并打印房间码，一局结束后自动准备下一局，供开发者用客户端作为第 8 人反复联调 |

### 构建号与产物复用

- 构建号 = `<git 短号>[-dirty]-<环境>`：环境取 `CLIENT_ENV`（`local` / `dev` / `prod`），Steam 固定 `steam`，`PLATFORM=local` 调试包固定 `local`，Android AAB 追加 `-aab`。
- 直接产物统一在 `output/client/`，文件名 `Civilization-<平台>-<构建号>.<扩展名>`；中间产物在 `output/client/build/<平台>/<构建号>/`；戳记在 `output/client/stamps/<产物基名>.json`。
- 戳记记录提交、分支、dirty、源码指纹、`SERVER_URL`、`STEAM_APP_ID`、Godot 版本与产物清单。macOS 系 `.app` 还把戳记复制进 `Contents/Resources/build-stamp.json`，iOS 包内的 `Civilization.app` 为 app 根的 `build-stamp.json`。
- `make export-*` 与 `make run-ios-*` 每次先比对戳记：源码指纹（HEAD + `git status` + 变更文件大小/mtime）与配置一致且产物齐全时直接复用，打印「复用 …」并跳过 Godot 导出与 `xcodebuild`；iOS 的 `client-import` 排在复用判定之后，命中时一并跳过，`make run-ios-*` 直接进入设备安装；否则重建并覆盖同名产物。dirty 工作区同样可复用，内容一变戳记即失效。
- `BuildConfig` 同时写入 `BUILD_ENV`、`GIT_COMMIT`、`GIT_DIRTY`、`BUILD_TIME`，安装包内可直接读到构建来源；提交版 `client/generated/build_config.gd` 保留这些常量的空默认值。

Godot 可执行文件通过 `GODOT` 变量覆盖，默认自动探测 `godot` 或 `/Applications/Godot.app/Contents/MacOS/Godot`。引擎补丁版本固定为 `4.7.2`（`Makefile` 的 `GODOT_VERSION`）；`make client-import` 及所有依赖它的客户端目标会先校验本机引擎，版本不符直接失败。

`make data`、`make data-check`、`make protocol`、`make protocol-check`、`make test`、`make client-check` 与 `make check-core`、`make check` 中互不依赖的步骤由 `scripts/run_parallel.py` 并行执行：默认并行度取 CPU 数（上限 8），`JOBS=<n>` 可覆盖，`JOBS=1` 退回串行。任一命令失败时其余命令仍执行完毕，目标以非 0 退出并汇总失败命令与退出码；单条命令输出超过 12,000 字符时保留末尾。

导入压缩：`make client-import` 默认 `CLIENT_IMPORT_GPU=0`，走 `--headless` 并用 CPU 压缩。原因是当前卡面为 `compress/high_quality=true` 的 RGBA8，导入选择 BPTC（BC7），而 Godot 4.7 的 Betsy GPU 压缩只覆盖 S3TC（BC1/BC3）与 BC6H，BC7 明确不支持；实测非无头冷导入与 headless 耗时一致，且没有 `Betsy: Encoding` 日志。headless 显示服务固定拒绝创建 RenderingDevice，所以 headless 导入只能 CPU 压缩。需要 GPU 时设 `CLIENT_IMPORT_GPU=1` 去掉 `--headless`：Betsy 能创建 RenderingDevice 时用 GPU 压缩 BC1/BC3/BC4/BC5/BC6（即改用 `compress/high_quality=false` 的 S3TC 或 HDR 纹理后才会生效），否则回落 CPU；无图形会话时由命令自身失败并回退 headless。`CLIENT_IMPORT_GPU=auto` 用 `DISPLAY` / `WAYLAND_DISPLAY` 或 macOS 的 `launchctl print gui/<uid>` 判断图形会话。非无头导入会按引擎格式重写 `client/project.godot`（去掉默认值设置、重排 section），命令会提示检查 diff。ETC2/ASTC 始终不走 GPU。

## 5. 运行方式

手动联调的最省事用法是 `make dev-bots`：

```
make dev-bots                                       # 终端 1，起服务端与 7 个机器人
make client-run                                     # 终端 2，大厅填房间码
```

需要检查不同尺寸下的布局时，用 `WIDTH` 与 `HEIGHT` 指定窗口像素尺寸，两者需同时传入：

```
make client-run WIDTH=960 HEIGHT=540                 # 16:9 小窗
make client-run WIDTH=2556 HEIGHT=1179               # iPhone 横屏物理像素
```

指定尺寸只改变窗口大小并居中，不改变设备档位；不传时窗口按屏幕可用区自动收拢。尺寸不合法或只传一个时命令直接报错，不启动客户端。

在桌面模拟移动端布局与触控交互时，用 `DEVICE` 指定设备档位：`phone` / `tablet` / `desktop`，也接受整机预设 `iphone` / `ipad`（见下）。其余值直接报错，不再按平台自动回落。手机档使用 960×540 逻辑基准与紧凑卡面，触控端控件尺寸生效，并在启动时开启鼠标模拟触摸，卡牌长按详情、拖拽与瞄准走真实触控路径。非真机触控平台不读系统屏幕安全区与虚拟键盘，避免桌面整屏安全区算出错误边距。设备档位不改变默认连接地址：`local` 环境仍连 `ws://127.0.0.1:8080/ws`；`dev` / `prod` 由 `run-<设备>-<环境>` 在启动前临时写入对应 `BuildConfig.SERVER_URL`，退出后恢复。

常用整机预设同时给出验收机横屏尺寸与设备档位：

```
make client-run DEVICE=iphone                      # 2622×1206，phone 档位
make client-run DEVICE=ipad                        # 2360×1640，tablet 档位
make client-run DEVICE=desktop                     # 窗口按屏幕可用区自动收拢
```

显式传入 `DEVICE` 且环境为 `local` 时默认带本地服务（等价于追加 `LOCAL_SERVER=1`）：客户端启动后由 `LocalServer` 在 `server/target/debug` 下内嵌拉起 `room-service` 与 7 个机器人，服务端就绪后自动登录，拿到房间码后自动入房，无需另开终端跑 `make dev-bots`。`dev` / `prod` 环境与不传 `DEVICE` 的 `make client-run` 不启动本地服务；可用 `LOCAL_SERVER=0` 或 `LOCAL_SERVER=1` 显式覆盖。客户端正常退出时一并回收它自己启动的服务端与机器人（`Ctrl-C` 同进程组一并结束）；若 8080 上已有可复用的服务端则直接复用，既不再启动也不回收。内嵌服务端只监听 `127.0.0.1:8080`，真机联调仍需用 `make dev-bots`。

客户端一律用 `make client-run` 启动：它会先 `--import` 刷新全局类缓存。直接跑 `Godot --path client` 时，若工程新增或改名过 `class_name` 脚本而缓存未更新，依赖它的脚本编译失败，对局页会缺整块区域（例如中央舞台整体不构建，只剩右上角工具组与左栏）。

命令打印两行 `ROOM_CODE=<6 位>` 与提示后等待，开发者在客户端大厅输入该房间码、入房并准备；8 名真人全部准备后，房主机器人自动发送 `StartGame`，随后进入对局页选英雄阶段（补位与开始条件由真人房主的客户端与服务端校验负责，机器人房主只等满 8 名真人）。一局结束后服务端不重启，7 个机器人自动建下一局并打印新的 `ROOM_CODE`（房间码按服务端计数器递增）；在结算页点「确定」回大厅、输入新房间码即可继续。每局的机器人意图种子自动 +1（`DEV_INTENT_SEED` 指定第 1 局的起始值，缺省 0），重开时机器人决策不会一成不变。`Ctrl-C` 一次退出全部进程并释放端口；`DEV_ROUNDS=<正整数>` 限定局数，缺省一直循环。端口被占用时立即失败，用 `make dev-bots DEV_PORT=8081` 换端口。需要自定义机器人名字或英雄时改用下面的手动方式。

`make dev-bots` 把服务端监听在 `0.0.0.0:8080`，手机与开发机同一局域网时可直接连接：移动端登录页默认地址为 `ws://<开发机 Bonjour 主机名>.local:8080/ws`（当前 `cooter.local`），桌面端仍为 `ws://127.0.0.1:8080/ws`。客户端连接前对 `ws://` 地址把主机名解析成 IPv4 再连（`Net.prefer_ipv4_url`）：`WebSocketPeer` 只尝试解析结果的第一条地址，双栈主机名会先拿到 IPv6，而服务端只监听 IPv4；`wss://` 保留域名，避免换成 IP 后证书校验失败。

手动方式：

```
make server-build
cd server && cargo run -q -p room-service          # 终端 1；手机联调加 --listen 0.0.0.0:8080
make client-run                                     # 终端 2，手动登录并创建房间
cargo run -q -p sim -- bot --connect ws://127.0.0.1:8080/ws \
  --room <房间码> --name Bot1 --hero tesla          # 终端 3 起，共 7 个
```

- 客户端创建房间后把 6 位房间码发给 7 个机器人；开发者入房并准备、8 名真人全部准备后，房主机器人发送 `StartGame`，服务端进入选英雄阶段。
- 机器人参数与协议顺序见[房间服务与联网](../room-service/README.md) §12。
- 无头验收用 `make client-e2e`：客户端自动建房并写入房间码，脚本再拉起 7 个机器人，结束后输出 `CLIENT_E2E_GAME_OVER place=<n> seed=<seed> ranking=<n> intents=<n> errors=<n> soft=<n>`；脚本校验名次、8 人排名、种子、意图数，并要求错误全部落在预期白名单（`NO_SPACE`、`NOT_ENOUGH_GOLD`、`SHOP_SLOT_EMPTY`，以及目标候选与视图偏差导致的 `NO_VALID_TARGET`），其余错误为 0，服务端与机器人日志无 panic。
- 无头驱动不等上一条意图确认就按新视图继续决策，服务端按最新状态拒绝属预期；因此 `errors` 与 `soft` 必须相等。驱动另外对同一回合的同一商店槽位只提交一次购买（刷新后允许再次购买），避免重复购买同一槽位。
- 驱动按 `clientIntentId` 找回被拒意图，收到 `NO_VALID_TARGET` 时沿服务端下发的候选（`targetCandidates` 或待选 `candidates`，键为 `zone:id`）换下一个重发；队首待选为 `target` 时发 `ChooseTarget { target }`，不再一律当发现处理；`sim bot` 的处理方式相同（见[房间服务与联网](../room-service/README.md) §12）。候选遍历完仍无合法目标时，本回合跳过该手牌（连续被拒 3 张且可结束时直接结束招募）；`ChooseTarget` 被拒则换下一个候选。否则会形成「重试 → 拒绝 → 再重试」的空转，直到演示超时。
- 默认端口 `18080` 被其他进程占用时，脚本会因监听失败退出；用 `CIVILIZATION_E2E_PORT=<端口> make client-e2e` 覆盖，脚本据此拼出客户端连接地址。

### 运行态视图预览

`make ui-preview` 用真实窗口装载指定视图，注入本地夹具，不截图、不导出 PNG：

```
make ui-preview PAGE=game FIXTURE=game FOCUS=PlayersPanel   # 对局页，左栏积分榜描边
make ui-preview PAGE=game DEVICE=phone                      # 对局页手机档位
make ui-preview PAGE=gallery                                # 其他页面空态
make ui-preview SCENE=res://pages/game/game.tscn            # 任意场景
make ui-preview SETUP=res://tests/previews/game_rail.gd     # 无 .tscn 的组件单视图
```

- `PAGE` 取 `client/pages` 下的页面名；`SCENE` 直接加载任意 `res://` 场景；`FOCUS` / `FOCUS_PATH` 在真实页面树里高亮目标并打印 `get_global_rect()`，不隐藏兄弟、不 reparent。
- `FIXTURE=game` 注入八人对局视图与假连接；不传时按页面名自动选（`game`），其余页面为 `none` 空态。夹具只写 `GameStore` / `Net`，不连接服务端。
- 页面脚本实现 `static func preview(opts: Dictionary = {}) -> Node` 时优先调用；没有 `.tscn` 的组件由 `SETUP` 脚本的 `static func build(opts)` 构建。
- `make ui-preview-check` 与 `ui-preview` 参数相同，自动退出并按日志判定：`VIEW_PREVIEW_OK` 通过，`VIEW_PREVIEW_FAIL` 或 `SCRIPT ERROR` 失败；可用 `EXPECT=SeatRow=8` 断言节点数量。需要 GUI 会话，不接入 CI。
- 预览只做查看与轻交互；点击返回/退出等真实导航会切走预览场景。真实联调仍用 `make dev-bots` + `make client-run`。

## 6. 国际化与文本表

- **唯一运行时中文文件**：`client/text/zh_CN.csv`（表头 `keys,zh_CN`）。客户端 `.gd` 代码不写中文 UI 文案；协议和生成数据只传 id，展示文本全部按 id 从文本表取。
- **配置**：`client/project.godot` 的 `[internationalization]` 设置 `locale/fallback="zh_CN"` 与 `locale/translations=PackedStringArray("res://text/zh_CN.zh_CN.translation")`。Godot 在导入时把 CSV 编译为 `zh_CN.zh_CN.translation`；`.translation` 与 `.csv.import` 被 `.gitignore` 忽略，CSV 本身随源提交。
- **取文本接口**：
  - 实例方法与 autoload 内取静态 key 用 `tr(key)`，带参数时用 `tr(key) % params`；
  - 静态函数（如 `CardText.era_text`、`UIWording.enum_text`）不能用 `tr()`（`tr()` 是 Node 的实例方法），必须用 `TranslationServer.translate(key)`；实例方法与 autoload 内用 `tr(key)`，需要运行时拼接 key 时同样可以调用 `tr(key)`，带参数时用格式化结果；
  - 缺 key 时 Godot 原样返回 key，因此封装函数按「返回 key 即缺键」回退：枚举回退为 id，错误码回退为 code，错误详情回退为空串。
- **key 命名**：
  - 内容文本：`card.<id>.name`、`card.<id>.introduction`、`skill.<id>.name`、`skill.<id>.text`；
  - 枚举：`enum.card_type.<id>`、`enum.playstyle.<id>`、`enum.taxon.<id>`、`enum.era.<id>`、`enum.keyword.<id>`、`enum.subtype.<slug>`、`enum.evolve.<id>`、`enum.stat_layer.<id>`；
  - 错误与界面：`error.<CODE>`（默认文案）、`error.<CODE>.<detail>`（服务端 `messageId` 的细分文案）、`ui.*`（页面静态文案）。
- **所有权**：`card.*`、`skill.*` 与 `enum.card_type|playstyle|taxon|era|keyword|subtype.*` 是生成 key，由 `scripts/gen_text.py` 从实体文档与 `taxonomy.md` 整体重算；`ui.*`、`error.*`、`enum.evolve.*`、`enum.stat_layer.*` 是人工 key，原样保留；出现未知前缀直接报错。`make data` 会写入 CSV，`make data-check` 检测漂移。
- **前置条件**：修改 CSV 或任何实体文本后必须运行 `make client-import`（或任何以它为前提的目标，如 `client-run`、`client-smoke`、`client-e2e`），让 Godot 重新导入并生成 `.translation`；只在编辑器外改 CSV 而不导入，运行时仍读旧翻译。`make client-check` 已包含 `client-import`。
- **服务端错误**：`ErrorMessage.messageId` 默认 `error.<CODE>`，`messageParams` 按 `%s` 填充；客户端状态栏先按 `code` 取 `error.<CODE>`，悬停详情优先用 `messageId`/`messageParams`，`message` 仅面向日志。
- **新增语言**：一个语言一个文件。新增语言需要：新增 `client/text/<locale>.csv`，并扩展 `scripts/gen_text.py` 以支持按语言文件读写与产出（每个语言文件独立生成内容 key 与枚举 key）；用 `make client-import` 导入后把对应 `.translation` 加入 `[internationalization]` 的 `locale/translations`。业务代码（`CardDB`、`UIWording` / `UIControls`、`tr()` 调用）与实体文档、协议均无需改动。
- **内置字体**：`client/assets/fonts/NotoSansSC-Regular-subset.ttf`（Noto Sans SC 子集，OFL 许可见同目录 `OFL.txt`）由 `project.godot` 的 `gui/theme/custom_font` 设为默认主题字体，所有文本不再依赖系统字体回退；iOS 真机因此不再缺字。子集把垂直度量对齐 PingFang SC（`hhea` 1060/−340、`OS/2` typo 860/−140/400），切换前后行高一致，现有布局不需要按字体调整。
- **字体生成**：`make font-subset` 取 `client/text/zh_CN.csv` 全部字符 + 可打印 ASCII + GB2312 常用字（覆盖玩家自填显示名），输出上述子集。源字体由目标按需下载到 `FONT_SOURCE`（默认 `output/fonts/NotoSansSC[wght].ttf`，Google Fonts 的 Noto Sans SC 变量字体，URL 可用 `FONT_SOURCE_URL` 覆盖）；缺少 fontTools 时自动创建 `output/fonttools-venv` 并安装，`PYTHON` 已装 fontTools 则直接复用。文案出现子集外生僻字时重跑该目标；`client/tests/ui_smoke_text.gd` 断言子集覆盖文本表全部字符。

## 7. 与设计的偏差

1. 设计 §5.6 只要求“1 个客户端 + 7 个机器人人工验收”；实现额外提供 `client/tests/headless_game.*` 与 `make client-e2e`，用真实客户端栈（Net + GameStore）自动打完一局，作为协议与视图消费的回归证据。该驱动不点击 UI，五场景可读性仍由 `make client-smoke` 与手动运行保证。
2. 卡牌展示表新增 `display_cost`：随从固定 3、效果牌取文档 `cost`，仅用于显示；规则判定仍全部在服务端。
3. `PlayCard` 的目标由拖拽落点直接给出（随从牌带 `position`，需要目标的效果牌带 `target: {zone, id}`，取自视图候选）；需要目标的随从战吼改为两段式：第一段出牌不带目标，服务端推入目标待选，第二段由瞄准层发 `ChooseTarget`，落空或 `Esc` 发 `AbandonTarget` 退回手牌。账本上的消耗与退还都由服务端待选状态决定，客户端不做本地预判。
4. 主动技能需要目标且候选非空时按住该技能按钮进入瞄准，松手命中候选发 `UseHeroPower { skillId, target }`；不需要目标时直接发送，候选为空时该按钮禁用、不发送。`pendingChoices` 的目标待选也走同一瞄准层，不再使用弹层。

## 8. 桌面端导出

`client/export_presets.cfg` 提供 macOS 预设 `macOS`：Universal（`x86_64` + `arm64`）、bundle id `com.civilization.banner`、应用名 `Civilization`、不签名（`codesign/codesign=0`、`codesign/identity=""`）。预设里的 `export_path` 只给编辑器手动导出用；`make client-export` 以命令行传入 `output/client/Civilization-macos-<构建号>.app` 的实际路径。

| 命令 | 内容 |
| --- | --- |
| `make client-export-templates` | 按 `make/client.mk` 固定的 `4.7.2` 检查 `$HOME/Library/Application Support/Godot/export_templates/4.7.2.stable/`；缺失时下载并解压官方 `Godot_v4.7.2-stable_export_templates.tpz`，只安装其中的 `templates/` |
| `make client-env-check` | 校验客户端构建环境只能是 `local`、`dev` 或 `prod`，默认 `local`；`ENV` 与 `env` 等价，同时传且值不同直接失败；`dev`/`prod` 分别要求 `CLIENT_DEV_SERVER_URL` / `CLIENT_PROD_SERVER_URL`，未配置直接失败 |
| `make client-export` | 默认 `PLATFORM=macos`，依赖 `client-env-check`、`client-export-templates` 与 `client-import`；按 `ENV`/`env` 临时写入对应环境地址后导出 `output/client/Civilization-macos-<构建号>.app`，校验 `Contents/MacOS/Civilization` 可执行，再用 `ditto` 打包同名 `.zip`。产物带 `build-stamp.json` 戳记，同号同环境且源码指纹一致时直接复用。`PLATFORM` 取 `macos`、`local`、`steam`、`ios`、`android`，其余值直接报错 |

- 客户端构建环境分 `local`、`dev`、`prod` 三档，默认 `local`；参数可写 `ENV=` 或 `env=`，两者等价，同时传且值不同直接失败。`local` 写入空配置，客户端运行时按平台回落本机调试地址；`dev` 写入 `CLIENT_DEV_SERVER_URL`，默认 `wss://civilization.satori.host/ws`；`prod` 写入 `CLIENT_PROD_SERVER_URL`，当前为空，配置前导出直接失败。`dev`/`prod` 必须提供非空地址，`prod` 还必须是 `wss://`；`dev` 可用 `ws://` 连局域网调试端口。导出过程用 `trap` 恢复仓库内的空配置，失败也不会留下脏文件。

- 两个目标只在 macOS 上可用，非 Darwin 主机直接报错退出；本机引擎版本与固定版本不符时同样退出非 0。
- Universal 导出要求 `project.godot` 启用 `rendering/textures/vram_compression/import_etc2_astc` 与 `import_s3tc_bptc`，缺失时 Godot 会拒绝导出。
- 产物未做正式签名与公证：`codesign/codesign=0` 禁用导出时的重签，但可执行文件仍保留 Godot 模板内置的 ad-hoc 签名。首次打开需右键「打开」，或在「系统设置 → 隐私与安全性」中放行；经 zip 下载后解压的副本同样会被 Gatekeeper 拦截。
- 导出物内置服务器地址由构建环境决定：release 导出按 `ENV`/`env` 写入地址（`local` 为空配置，`dev` 为 `wss://civilization.satori.host/ws`，`prod` 取 `CLIENT_PROD_SERVER_URL`）；编辑器运行按 `ENV`/`env` 生效：`local` 为空配置（默认），`dev` / `prod` 在启动前临时写入对应地址并在退出后恢复；`make client-export PLATFORM=local` 始终为空配置，回落到桌面 `ws://127.0.0.1:8080/ws`、移动端 `ws://cooter.local:8080/ws`。`PLATFORM=local` 与 `PLATFORM=steam` 不接受 `ENV`：前者始终使用空配置，后者只读 `SERVER_URL` 与 `STEAM_APP_ID`，显式传 `ENV` 直接报错。登录页不提供服务器输入，运行时始终使用构建配置或平台默认值。连接前 `ws://` 地址的主机名先解析为 IPv4，避免双栈主机名拿到 IPv6；`wss://` 保留域名以满足证书校验。

### 单机调试包

`client/export_presets.cfg` 另提供只用于本机自测的预设 `macOS Local Debug`（`preset.3`）：`platform="macOS"`、`custom_features="embedded_server"`、Universal、bundle id 与应用名同发行预设、不签名，用 `--export-debug` 导出。

- `make client-export PLATFORM=local` 导出 `output/client/Civilization-local-<构建号>.app`，再复制 `server/target/debug/room-service` 与 `server/target/debug/sim` 到 `Contents/MacOS/` 并校验可执行；不 zip、不签名。复用判定同时看 `client/` 与 `server/` 指纹，sidecar 重建后不会复用旧包。
- `LocalServer` autoload 的启用条件：调试包由导出特性 `embedded_server` 启用；编辑器运行由环境变量 `CIVILIZATION_LOCAL_SERVER=1` 启用（Godot 4.7.2 没有命令行注入自定义特性的参数）。sidecar 目录取 `CIVILIZATION_LOCAL_SERVER_DIR`，为空时回退到可执行文件同级目录；`make client-run LOCAL_SERVER=1` 传入 `server/target/debug`。
- 启用后先探测 `127.0.0.1:8080`，已有进程监听则复用（不启动自己的服务端），否则启动 `room-service --listen 127.0.0.1:8080` 并等待最多 10 秒；服务端就绪后启动 `sim dev --connect ws://127.0.0.1:8080/ws --intent-seed 0`，最多 15 秒内从 stdout 读取 `ROOM_CODE=<码>`。server 与 sim 的 stdout/stderr 写入 `user://local_debug.log`。
- 登录页在服务端就绪后用预填的开发账号（`13900000000` / `123456` / `玩家A`）自动登录；大厅拿到房间码后填入并自动加入。先到大厅后到房间码时订阅 `room_code_ready` 补发，房间码只消费一次。启动失败时状态标签显示 ASCII 失败原因（如 `server_timeout`、`sidecar_missing`），登录按钮恢复可用。
- `CIVILIZATION_LOCAL_AUTO=0` 只关闭自动登录与自动入房，服务端与机器人仍会启动，便于手测登录/大厅流程。
- 客户端正常退出时 `_exit_tree` 只 `OS.kill` 自己启动的 server 与 sim；复用已有服务端时不 kill。崩溃残留的 sim 进程不自动清理，避免误杀其他进程；下次启动若 8080 已被占用则直接复用。
- 产物为 debug 构建并含调试符号，未签名，只保证本机运行；sidecar 为宿主架构（arm64），Intel Mac 需另行构建。
- 发行链路（`macOS`、`Steam`、`iOS` 预设与 `make client-export` / `make client-export PLATFORM=steam` / `make client-export PLATFORM=ios`）不带 `embedded_server` 特性，也不复制 sidecar；冒烟与直接 `make client-run` 不设置 `CIVILIZATION_LOCAL_SERVER`，`LocalServer` 不启动任何进程。
- 当前只完成构建级与静态校验（调试产物导出成功、sidecar 可执行、`make client-export` 产物不含 sidecar）；GUI 端到端（自动登录、自动入房、完成一局、退出后进程回收）尚未在真实产物中验收，由主 agent 在 8080 空闲时执行。

## 9. 移动端（iOS 与 Android）

### 设备与布局

- `ScreenProfile`（autoload）在启动时按设备设置 `content_scale_size`：iPhone 为 960×540，iPad 与桌面为 1280×720；`expand` 下按基准等比缩放后逻辑视口铺满整个窗口，宽度与高度都随窗口比例扩展。`content_frame(viewport, base)` 给出内容框：固定 16:9 的 contain 区域（受限一侧贴满，另一侧居中留背景）；`launch_window_size(usable, base)` 给出桌面启动窗口尺寸（可用区域 90% 收成 16:9），窗口拖动后不再介入。`project.godot` 的 `display/window/handheld/orientation=4`（`SCREEN_SENSOR_LANDSCAPE`）锁定左右横屏。
- 设备判定：iOS 上按 `OS.get_model_name()` 前缀区分 iPad 与 iPhone，机型名不可用时按屏幕比例兜底；Android 上按屏幕短边（不小于 600dp）与长宽比（低于 1.5）区分平板与手机，密度取 `DisplayServer.screen_get_scale()` 与 `screen_get_dpi() / 160` 中较大的值。其它平台与无头环境一律按桌面处理。
- 换算公式是纯函数：`canvas_scale`、`canvas_hit_min`、`canvas_from_pixels`、`canvas_insets`。`DisplayServer.get_display_safe_area()` 与 `virtual_keyboard_get_height()` 返回像素，按内容矩形换算为画布单位；内容矩形即 `content_frame` 给出的 16:9 居中框，落在背景带内的边距记零。键盘高度由 `keyboard_from_pixels` 先与底部背景带抵消，只有越过内容框底边的部分才换算为让位高度。
- `UILayout.safe_area(page, page_texture)` 在页面 `_ready()` 第一句调用，把页面内容移入 `ContentFrame → ContentRoot → PageBody → SafeAreaMargin`；调用后新增的子节点按创建顺序自动移入安全区：`ContentFrame` 是 16:9 居中框，`ContentRoot` 以设计基准尺寸布局、按内容框高度缩放；`PageBody` 与设计画布同尺寸，`PageSurface` 是它承载的页面背景（`page_texture` 非空时按容器尺寸拉伸，否则为 `panel_style("page")` 语义面板）；`SafeAreaMargin` 内桌面四边为零，触控端取安全区四边，底部取安全区与键盘高度的较大值。整屏软件背景（`PageBackground`）留在包裹层外并置底，铺满视口，安全区只约束内容控件；图鉴页自带整屏背景时关闭 `CardGallery` 的组件背景。键盘边距只在页面内 `LineEdit` 获得焦点期间逐帧读取。`UILayout.safe_layer(page)` 供对局内图鉴覆盖层等动态全屏节点挂载。覆盖层（详情、瞄准）随 `ContentRoot` 整体缩放，输入坐标先经画布与全局变换换算到内容框局部，再按层缩放换算矩形尺寸。
- `UITheme.hit_primary()` / `hit_small()` 把 44pt / 32pt 换算为画布单位；`UIControls.button` 与 `UIControls.styled_button` 在触控端按此抬升最小高度，桌面端保留原值；`UIControls.line_edit` 桌面高 34，触控端同样按 `hit_primary()` 抬升。
- 图鉴在 iPhone 基准下按可用宽度分列（960 宽为 7 列，安全区收窄时按实际宽度减少），列距 `space-md`，卡宽按列数平分并等比放大、整行贴满，每批 12 张；详情为贴右整高抽屉（宽 400、右侧留 16），抽屉内边距 `space-md`、属性标题列 52，封面宽 `stage_metric("gallery-preview-width")` = 112（高度随 `card_face_height`）。头部仍为一行的两段视图、时代 / 玩法 / 类型下拉、计数与返回；筛选面板宽 `gallery-filter-panel-width` = 240，挂在触发按钮下沿。桌面与 iPad 基准 9 列、440 抽屉与 160 封面。
- 对局页与内部组件统一用 `UITheme.stage_metric(name)` 取场景令牌：iPhone 优先取 `<name>-compact`（手牌卡 70×91、随从卡 78×102、英雄行 76、河道 30、商店操作行 64、胶囊按钮 24（触控命中 32）、头像 48、图标按钮 48、座位行头像兜底 40（实际按行高解算）等），其余设备取原值；卡面紧凑值保持桌面 112:146 的卡形，只缩尺寸不改比例；页面边距与舞台间距在 iPhone 下同步收紧。桌面与 iPad 的舞台按窗口高度扩展（基准 720 高），iPhone 固定 540 高，纵向顺序与操作不变。

### 触控交互

- 详情层（`DetailLayer`）在触控端由长按状态机驱动：按下只登记第一个触点，450ms 显示，抬手隐藏；位移超过 8 个画布单位或模态浮层打开时取消。触控端忽略触摸模拟出的鼠标悬停与按下，桌面鼠标路径不变。状态机由 `LongPressTracker` 承担，阈值与计时方式只有一处实现。
- 选英雄覆盖层候选卡的悬停详情复用同一套状态机：`HeroPickDetailLayer extends DetailLayer`，长按 450ms 显示、抬手隐藏；覆盖层 `_input` 复用 `LongPressTracker` 独立跟踪短按，未取消、未到 450ms、抬手仍在起点卡内时才选中，长按与位移都不会改变选中。
- 选英雄覆盖层在 `setup` 时按 `HeroPickStarted.players[]` 中 `playerId` 等于 `GameStore.player_id` 的座位定位本人，座位点金圈与已选锁定都以此为准；匹配局跨进程重连不重放 `RoomUpdate` 与 `MatchFound`，载荷是唯一可用的身份来源，解析不到时才依次回退 `room_players`、`my_seat()` 与 `match_seat`。
- `HandSlot` 在触摸释放、拖拽结束与重建卡片时显式复位缩放、`z_index` 与右侧槽位的 `mouse_filter`，弥补触控端没有 `mouse_exited` 的问题。
- 招募河道倒计时的悬停翻转在触控端只认 `InputEventScreenTouch`，桌面只认鼠标事件：真机上一次触摸会同时产生触摸与模拟鼠标按下，两个分支都放行会发出两条 `EndRecruit`，后一条在阶段推进后收到 `INVALID_STAGE`。触控平台判定统一用 `ScreenProfile.is_touch()`；`is_simulated_touch()` 只描述桌面模拟档位，不能用来区分真机。
- 冒烟用 `ScreenProfile.set_device_for_test` 覆盖设备档位、`set_native_touch_for_test` 覆盖真机触控判定，从而在桌面构造真机触控分支。
- 单击、拖拽继续依赖 Godot 的 `emulate_mouse_from_touch`；iOS 的 `Esc` 返回由常驻返回按钮替代，Android 另由 `SystemBack` 处理系统返回键与返回手势。

### 联调地址

- 空配置时移动端默认 `ws://cooter.local:8080/ws`（`Net.MOBILE_URL`）：`cooter` 取开发机 `scutil --get LocalHostName`，换机器时改该常量并重新导出；iOS 通过 Bonjour 解析 `.local`，开发机 IP 变化不影响。连接前 `Net.prefer_ipv4_url` 对 `ws://` 用 `IP.resolve_hostname(host, IP.TYPE_IPV4)` 把主机名换成 IPv4：`WebSocketPeer` 只尝试解析结果的第一条地址，双栈主机名会先拿到 IPv6，而服务端只监听 IPv4，直接连主机名会失败；无法解析时保留原地址交给连接层报错。`wss://` 不做替换，保留域名交给 TLS 证书校验。
- 桌面端默认 `ws://127.0.0.1:8080/ws`（`Net.DESKTOP_URL`）。`Net.default_url_for(mobile)` 是纯函数，冒烟直接断言两端取值；登录页不提供服务器输入，`connect_to("")` 回落到当前平台默认值。
- 服务端默认只监听 `127.0.0.1:8080`；真机联调必须用 `make dev-bots`（监听 `0.0.0.0`）或手动 `cargo run -p room-service -- --listen 0.0.0.0:8080`。首次连接会触发 iOS 本地网络权限弹窗，描述文案已写入导出预设的 `NSLocalNetworkUsageDescription`。

### iOS 导出

`client/export_presets.cfg` 的 `iOS` 预设：bundle id `com.civilization.banner`、团队 `Q62EFJPV3C`、`development` 签名、iPhone & iPad（`targeted_device_family=2`）、最低 iOS 15.0、只导出 Xcode 工程（`export_project_only=true`）、`Apple Development` 签名身份，并通过 `additional_plist_content` 写入 `UIRequiresFullScreen` 与 `NSLocalNetworkUsageDescription`。

| 命令 | 内容 |
| --- | --- |
| `make client-export PLATFORM=ios` | 依赖 `client-export-templates`；先比对戳记，未复用时才执行 `client-import`，再按 `ENV`/`env` 写入对应环境地址（默认 `local` 写空配置），导出 `output/client/build/ios/<构建号>/Civilization.xcodeproj`，用 `xcodebuild`（scheme `Civilization`、`-allowProvisioningUpdates`）构建 Release，再把 `Civilization.app` 装入 `Payload/` 打包为 `output/client/Civilization-ios-<构建号>.ipa` |
| `make client-run PLATFORM=ios` | 依赖 `client-export PLATFORM=ios`，用 `xcrun devicectl` 把 `output/client/build/ios/<构建号>/build/Build/Products/Release-iphoneos/Civilization.app` 安装到 iPhone 并启动 `com.civilization.banner`；导出被复用时跳过导入与导出，直接安装已有产物 |

- 只在 macOS 上可用；模板目录缺少 `ios.zip` 时直接退出非 0。
- 构建环境与桌面导出一致：`make client-export PLATFORM=ios` 与 `make client-run PLATFORM=ios` 接受 `ENV`/`env`（默认 `local`）。`local` 写空配置，真机按移动端回落 `ws://cooter.local:8080/ws`；`ENV=dev` 写入 dev 公网入口，`ENV=prod` 取 `CLIENT_PROD_SERVER_URL`。
- 导出前必须存在 `application/config/icon`（当前 `res://icon.png`）；缺少图标时 Godot 导出失败。图标为 AI 出图后随源提交的 `client/icon.png`，设计原则与尺寸见[客户端图片素材索引](image-asset-index.md#25-应用图标)。
- 签名依赖 Xcode 登录 Apple 团队账号并登记目标真机；缺少账号或描述文件时 `xcodebuild` 失败并打印签名错误，不产出未签名包。
- 构建产物在 `output/client/build/ios/<构建号>/`，ipa 在 `output/client/`；`output/` 不提交。
- `make client-run PLATFORM=ios` 默认取 `devicectl` 中第一台已连接（`connected`）的 iPhone，并打印所选设备名与 UDID；`IOS_DEVICE=<设备名|UDID>` 指定其他设备，`IOS_LAUNCH=0` 只安装不启动（设备锁屏或未解锁时启动失败，安装不受影响）。
- 安装要求设备已在 Xcode 中信任并开启开发者模式；没有已连接的 iPhone 时目标直接失败并列出当前设备。

### Android 端

- 工程设置：`project.godot` 的 `display/window/handheld/orientation=4`（`SCREEN_SENSOR_LANDSCAPE`）锁定左右横屏；`application/config/quit_on_go_back=false` 关闭引擎默认的「返回即退出」；`input_devices/pointing/emulate_mouse_from_touch=true` 保留触摸到鼠标的 GUI 映射，`emulate_touch_from_mouse=false`，桌面模拟触控档位仍由 `ScreenProfile` 在运行时开启。渲染继续使用 `renderer/rendering_method.mobile="gl_compatibility"` 与 ASTC。
- 设备档位：`ScreenProfile._detect_device()` 在 Android 上按屏幕短边（不小于 600dp）与长宽比（低于 1.5 归平板）判定；安全区、软键盘高度与命中尺寸沿用 iOS 的 `DisplayServer` 调用，`window_sizer.gd` 在真机触控端不调整窗口尺寸。
- 系统返回键与手势：autoload `SystemBack` 监听 `NOTIFICATION_WM_GO_BACK_REQUEST`，顺序为可关闭浮层（`FriendsPanel`、`GalleryFilterPanel`、`GalleryDetailLayer`）→ 当前场景 `system_back()` → 忽略。账号页执行常驻返回按钮动作，图鉴页回大厅，对局页收起图鉴覆盖层；大厅、房间、结算、登录与对局其它状态没有返回动作，返回键忽略且不退出应用。邀请弹窗、匹配排队浮层、发现 / 进化选择与状态模态有待处理业务状态，不随返回键关闭。
- 导出预设：`Android`（APK，`gradle_build/export_format=0`，沿用导出模板的 SDK 版本）与 `Android AAB`（`export_format=1`，Gradle 构建，`minSdk 24`、`targetSdk 35`）。两者包名 `com.civilization.banner`、应用名 `Civilization`、arm64-v8a、`package/signed=true`、`screen/immersive_mode=true`、`screen/edge_to_edge=true`。
- 签名：`make client-export PLATFORM=android` 与 `make client-export PLATFORM=android AAB=1` 只从 `ANDROID_KEYSTORE_PATH`、`ANDROID_KEYSTORE_USER`、`ANDROID_KEYSTORE_PASSWORD` 读取，缺失时目标失败，不产出未签名包。`scripts/android_export.sh` 在导出期间把参数临时写入预设，退出时恢复备份；AAB 首次导出临时安装 Gradle 构建模板并在结束后删除 `client/android/`。口令不进入仓库或命令行。
- 构建环境：两个 Android 导出路径接受 `ENV`/`env`（默认 `local`），导出期间把地址写入 `client/generated/build_config.gd` 并在退出时恢复空配置；`local` 写空配置，真机按移动端回落开发机局域网地址。
- 导出前置条件：Godot 编辑器设置 `export/android/java_sdk_path` 指向 JDK 17+，并配置 `export/android/android_sdk_path`；APK 需要 `android_release.apk` 模板，AAB 另需 `android_source.zip`。`make client-export PLATFORM=android` 不修改编辑器设置，缺失时由 Godot 报错。
- 已知问题：`client/addons/godotsteam/godotsteam.gdextension` 声明了 Android / Linux / Windows 库，但仓库只保留 macOS 二进制；Android 产物内会出现 0 字节的 `libgodotsteam*.so` 与 `libsteam_api.so`，导出日志报无法打开对应文件。产物仍可生成，Android 端运行时的扩展加载行为未验证。
- 验证记录（2026-11-23）：`make client-check` 退出码 0（目标内包含 `client-import`）；`make client-export-android` 与 `make client-export-android-aab` 在隔离 HOME（JBR 21 临时编辑设置）与临时测试 keystore 下成功产出 APK / AAB，`apksigner verify` 与 `jarsigner -verify` 通过。Android 手机与平板真机、系统返回手势与软键盘手测未验证（本机无已连接 Android 设备）。

## 10. 美术素材层

- 位置与清单：素材在 `client/assets/`，`asset_manifest.json` 按卡牌 id 登记 `portrait`、`avatar`、`card_face` 三类路径与展示尺寸；页面背景、大厅图与售货员头像不经过清单。各槽位的资源、尺寸、使用组件与回退见[纹理登记册](textures.md)。
- 加载：`CardDB` 启动时读取清单到 `art`，提供 `has_art(id)` 与 `art_path(id, kind)`；清单缺失或条目缺字段时返回空串，不报错。
- 回退：`UIArt.texture()` / `UIArt.layer()` 统一取图与铺层，路径为空、资源缺失或加载失败时返回空，由调用方回退程序化绘制；逐组件的纹理节点与回退规则见[纹理登记册](textures.md)。
- 取图范围：图鉴网格与详情、选英雄覆盖层候选卡取 `card_face`；对局英雄卡、座位头像、战斗开场与售货员取 `avatar`；对局英雄详情与选英雄覆盖层悬停详情（`HeroPickDetailLayer`）不显示图片。
- 生成：`make assets` 调用 `scripts/process_assets.py`，按 `ASSET_MAP`（`client/assets/characters/portraits/asset_map.json`，随仓库提交，内容为 `card_id → characters/portraits/ 下的立绘文件名`）处理源立绘，并更新清单中对应条目；清单内不在映射中的条目（如直接生成的随从封面）原样保留。映射文件缺失时目标直接失败。效果牌封面走 `make assets-covers`：取每个 `<card_id>/` 下最新的 PNG，去水印后缩放到 448×584，清单只写入 `card_face` 与 `display.card_face`，保留条目已有的 `portrait` / `avatar`。逐张处理按 `--jobs`（默认 CPU 数、上限 8）分给子进程并行执行，清单条目与日志顺序仍按串行顺序产出，`--jobs 1` 退回串行。运行前需安装 Pillow（`python3 -m pip install pillow`）。封面生成风格、prompt 模板、主题符号映射与去水印约束见 [卡片封面美术生成](card-cover-art.md)。
- 批次：英雄与已生成的随从、效果牌封面随仓库提交，统一按卡面 112:146 展示；尚无封面的卡牌继续走程序化绘制，由玩法色与首字回退。
- 离线产物：纹理压缩产物由 `tools/asset-compress` 生成（`make asset-compress`），不依赖 Godot。BC7 走 GPU（wgpu/Metal，`standard` 档：60 张卡面 mean 44.05 dB、min 39.58 dB），ASTC 4x4 走 CPU astcenc `PRESET_MEDIUM`（与 Godot 同档，单张产物与 Godot 逐字节一致）；写出 `.ctex`、`.import`、`.md5` 后，`make client-import` 不会重导（实测 540 张 0 变更）。质量门禁用 `make asset-compress-check`。全量 540 张在 M1 Max 上约 50s，瓶颈是 ASTC 的 CPU 编码。
- 渲染后端：`client/project.godot` 的 `rendering_method` 为 `forward_plus`（macOS 走 Metal），`rendering_method.mobile` 为 `gl_compatibility`。桌面 OpenGL 3.3 不提供 BPTC，GLES3 会把 BC7 纹理逐张解码成 RGBA8 并输出 `BPTC_RGBA not supported by hardware` 警告；Forward+ 直接使用 BC7，移动端的 GLES3 使用 ASTC。
- 导入：图片 `.import` 由 Godot 生成并被 `client/.gitignore` 忽略；`client/project.godot` 的 `[importer_defaults]` 为 `texture` 统一设置 `compress/mode=2`（VRAM 压缩）与 `compress/high_quality=true`，桌面（Forward+）走 BPTC、iOS 走 ASTC；`[editor]` 的 `import/use_multiple_threads=true` 显式固定线程池导入（引擎默认即为 true，PNG 由 `ResourceImporterTexture` 处理并支持线程导入）。448×584 卡面单张冷加载由约 6ms 降到约 0.2ms，图鉴一批 27 张的解码停顿从约 160ms 降到个位毫秒数。改图后运行 `make client-import`；调整导入默认值后需先删除对应的 `.png.import` 或 `client/.godot`，否则已有 `.import` 里的旧参数仍生效。新 worktree 首次运行 `make client-import` 时自动从主检出播种 `client/.godot/imported` 与源文件仍存在的 `*.import` 旁车（优先 clonefile/reflink，不支持时回退普通复制），随后只做增量导入；日常运行的跳过提示默认静默，`CLIENT_CACHE_COPY=0` 可关闭播种以验证冷导入。不在 git 仓库内（如源码归档）或主检出缺缓存时只告警并跳过播种，显式 `make client-cache-copy` 仍严格失败。缓存按源图 md5 校验，worktree 内改过的资源仍会重导。

## 11. 音效层

- 位置：`client/assets/audio/`。场景音效 `sfx/{reroll,freeze,unfreeze,shop_enter,combat_start,combat_end}.wav`；卡牌默认音效 `cards/default/{buy,attack,death}.wav`；逐卡覆盖为 `cards/<cardId>/{buy,attack,death}.wav`。
- 播放：`Sfx` 自动加载单例，8 路播放池轮转、统一 `-6 dB`；`play_cue(cue)` 播场景音效，`play_card(cardId, cue)` 播卡牌音效。卡牌音效先查逐卡路径，缺失回落默认目录；所有路径按需加载并缓存，文件缺失时静默跳过。
- 触发：`game.gd` 监听 `GameStore.events_received`，同批含 `TurnStarted` 播商店入场、否则 `ShopRolled` 播刷新、每条 `MinionBought` 按 `cardId` 播购入；冻结/解冻由视图 `frozen` 变化判断，带重抽事件的批次不误判。`game_combat_view.gd` 在应用 `attack` / `death` 动作时按随从实例反查卡牌 id，播放攻击/死亡音效；战斗装载时按 `startedAt` 算出 `elapsed_ms`，小于 `PACE_MS` 才播开始音，结果横幅首次出现时播结束音并置本场标志；`seek()` 续播不补播旧动作，每场开始与 `reset()` 清零结束标志。
- 资源：当前 9 个默认音效由 `make sfx` 调用 `scripts/gen_sfx.py` 合成，作为占位素材随源提交；正式素材可直接替换同名文件。`.import` 被 `client/.gitignore` 忽略，替换后运行 `make client-import`。
- 测试：`tests/test_client_boundary.py` 断言 `Sfx` 注册与 9 个默认资源存在；`client/tests/ui_smoke.gd` 的 `_check_sfx_events`、`_check_sfx_freeze`、`_check_sfx_combat` 断言事件到 cue 的映射、次数与卡牌 id，并覆盖战斗开始/结束不重复与续播不补开始音；`client/tests/ui_smoke_combat.gd` 的 `_check_started_at_seek` 覆盖续播到结束态只补一次结束音。

## 相关文档

- [Steam 接入](../steam-integration.md)
- [房间服务与联网](../room-service/README.md)
- [规则引擎与模拟器](../rule-engine-and-simulator.md)
- [技术现状索引](../README.md)
- [客户端技术基线](../architecture/client-stack.md)
