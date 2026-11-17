# Godot 客户端

> 状态：已实现（v0.1 M1b）
> 代码：`client/`
> 验收：`make client-check`、`make client-smoke`、`make m1b-e2e`

本文记录 v0.1 M1b 已实现的 Godot 客户端现状：工程结构、全局状态与网络层、五个场景、生成物与运行方式；不描述服务端房间服务（见[房间服务与联网](room-service.md)）与产品规则（见[规则引擎与模拟器](rule-engine-and-simulator.md)）。

## 1. 工程结构

```
client/
  project.godot              # 主场景 login.tscn；autoload：WindowSizer、GameStore、CardDB、Net
  .gitignore                 # 忽略 .godot/ 与 *.uid
  autoload/
    window_sizer.gd          # 启动时按当前屏幕可用区域 90% 设置窗口并居中
    game_store.gd            # 认证、房间与 PlayerView 的全局状态
    ws_client.gd             # WebSocketPeer、自动重连、信封编解码与分发
    card_db.gd               # 只读卡牌/技能展示表；展示文本按文本 id 从文本表取
  text/
    zh_CN.csv                # 唯一的运行时中文文本表（生成物 + 人工 key）
  ui/
    ui.gd                    # 场景共用控件工具、共享构件与统一导航入口（class_name UI）
    theme.gd                 # 三层视觉令牌与 StyleBox 工厂（class_name UITheme）
  scenes/
    login.tscn/.gd           # 登录
    lobby.tscn/.gd           # 大厅
    room.tscn/.gd            # 房间
    game.tscn/.gd            # 对局页面装配入口：三栏骨架、门控与意图
    game_ui_state.gd         # 一次刷新的页面快照：视图数据与门控结果
    game_card_face.gd        # 卡面：介绍、费用/时代角标、冻结蒙层与随从详情分层取值
    game_detail_layer.gd     # 详情层：商店、座位与随从三类详情及定位
    game_players_rail.gd     # 左栏座位列：头像、名次、血条与战报展开
    game_side_panel.gd       # 右栏：身份与商店操作、回合与倒计时、英雄与操作
    game_recruit_view.gd     # 招募视图：商店与我方棋盘（操作按钮在右栏）
    game_combat_view.gd      # 战斗视图：对手随从行、结果横幅与我方随从行
    hand_card.gd             # 手牌卡：拖拽数据与预览（class_name HandCard）
    hand_slot.gd             # 手牌槽：80 步进重叠、悬停缩放与层级（class_name HandSlot）
    board_drop.gd            # 我方棋盘落点：命中判定与插入下标（class_name BoardDrop）
    combat_playback.gd       # 战斗动作落地器：节奏与状态应用（class_name CombatPlayback）
    combat_fx.gd             # 战斗演出覆盖层：碰撞往返、闪光、飘字、淡出与出现（class_name CombatFx）
    result.tscn/.gd          # 结算
  tests/
    ui_smoke.tscn/.gd        # 主冒烟：五场景实例化、拖拽/浮层/门控/结算等主流程
    ui_smoke_side.tscn/.gd   # 右栏身份、回合与倒计时
    ui_smoke_seats.tscn/.gd  # 座位行、名次角标、血量条与战报展开
    ui_smoke_combat.tscn/.gd # 战斗视图三段与结果横幅
    ui_smoke_cards.tscn/.gd  # 商店卡费用/时代角标与冻结蒙层
    ui_smoke_text.tscn/.gd   # 文本表取用、枚举展示与错误文案回退
    smoke_case.gd            # 冒烟公共断言与场景装载
    fixture.gd               # 冒烟用最小 PlayerView
    headless_game.tscn/.gd   # 无头对局驱动（1 客户端 + 7 机器人）
  generated/
    protocol.gd              # schema 生成的 GDScript 常量
    cards.json               # 产品文档生成的卡牌展示表
    skills.json              # 技能展示表（effects 为结构化数组，仅展示不解析）
    tokens.json              # Token 展示表
```

窗口启动时由 `WindowSizer` 按当前屏幕可用区域（`screen_get_usable_rect`，已排除菜单栏与 Dock）的 90% 设置尺寸并居中；无头模式下跳过。逻辑视口固定 1280×720，窗口尺寸变化由 `canvas_items` 等比缩放，页面不感知实际窗口大小。

## 2. 全局状态与网络层

- `GameStore`：保存 `playerId`、`token`、`roomId`、`roomCode`、房间成员、`PlayerView`、`lastSeq`、排名与种子。`handle(type, payload)` 只做字段搬运，发出 `auth_changed`、`room_changed`、`view_changed`、`game_started`、`game_over`、`error_received` 信号；`reset_room()` 清空房间，`reset_session()` 再清空身份与令牌。
- `Net`：持有一个 `WebSocketPeer`，`_process` 中轮询；连接建立后先认证，断线每 3 秒自动重连。重连时以令牌发 `AuthRequest`，若仍在房间再发 `Reconnect{roomId, lastSeq}`，由服务端回全量 `Snapshot`。
- 下行信封只校验 `v == 1` 与 `type`，`payload` 原样交给 `GameStore`；未知类型忽略。上行信封为 `{v, id, type, ts, payload}`，`id` 由时间戳与计数器生成。
- `CardDB`：启动时读取 `client/generated/{cards,skills,tokens}.json` 的规则与展示费用字段；卡名、技能文本与时代名不放在生成数据里，而是按文本 id 从 `client/text/zh_CN.csv` 取：`display_name` 取 `card.<id>.name`（Token 取 `token.<id>.name`），`display_text` 取 `skill.<id>.text`，`era_name` 取 `enum.era.<id>`。不参与任何规则判定。

## 3. 五个场景

| 场景 | 职责 |
| --- | --- |
| 登录 | 输入服务器地址、手机号、固定验证码 `123456` 与显示名；认证成功后进入大厅 |
| 大厅 | 创建房间或凭 6 位房间码加入；收到 `RoomUpdate` 后进入房间 |
| 房间 | 展示房间码与成员、选择英雄、准备/取消准备、离开房间；开局完全由服务端决定 |
| 对局 | 酒馆战棋式三栏布局：左栏 8 行座位（按服务端 `rank` 升序，头像、名次角标、血量条，点击展开战报），中区招募视图（自上而下为商店、按容量棋盘）与战斗视图（对手随从、结果横幅、我方随从）互斥，手牌行常驻在两者之下，右栏为身份（售货员/对手英雄/幽灵/已淘汰观战）与售货员下方的商店操作、垂直居中的回合与倒计时、自己英雄介绍与贴底的英雄技能；十一种意图对应操作；`pendingChoices` 弹层（发现/进化/战吼目标）；英雄技能目标缺失时按 `NO_VALID_TARGET` 补选目标 |
| 结算 | 按服务端 `ranking` 展示名次与 `seed`；返回大厅时用令牌重新认证 |

- 对局界面整块读取 `GameStore.view`，每次 `view_changed` 重建；按钮可用性只取 `affordances` / `playable` / `can*`，不做本地推导。冻结与棋盘卡拖拽换位没有对应的服务端 affordance，按 `phase`、`self.recruited` 与下标做本地门控，与服务端拒绝条件保持一致。
- 操作还要求视图自身的 `turn` 与 `GameStore.turn` 一致：新回合的 `PhaseChanged` 会先于新视图到达，旧视图只展示、不可操作，避免用上一回合的商店发意图。
- 倒计时由 `phaseDeadline` 计算；归零后结束招募、刷新、购买、出牌、出售与换位一并禁用，只等服务端推进阶段。
- 已结束招募（`self.recruited`）后只展示界面，不再发起意图，也不弹待选弹层。
- 状态栏文案按错误码取 `UI.error_text` 的文本表条目（`error.<CODE>`）；悬停详情优先用服务端下发的 `messageId` 与 `messageParams`（`UI.error_tooltip` → `UI.error_text_by_id`），缺失时回退错误码文案。协议 `message` 只面向日志，不作为 UI 文案。
- `UI.error_route` 决定错误后的去向：认证类错误回登录并清空会话，`NOT_IN_ROOM` / `ROOM_NOT_FOUND` 回大厅并清空房间；其余错误留在当前页面。
- 未连接时点击操作不会静默丢弃：状态栏提示「未连接，操作未发出」，发送失败不登记目标重试。目标重试表按回合清理并设上限。
- 页面切换统一走 `UI.navigate(page, path)`：`change_scene_to_file` 会立即把当前场景摘下，而页面仍连着 `GameStore` 等 autoload 信号；摘下后的回调再取 `get_tree()` 会得到 `null`。`UI.navigate` 以「页面仍在树上」为唯一判定，离树页面的导航直接丢弃并返回 `false`，场景脚本不再直接调用 `get_tree().change_scene_to_file`。
- 房间场景同时监听 `view_changed`：重连或晚到的视图显示对局已开始时直接进入对局；房间不存在或已不在房间时回大厅。
- 商店使用 `HFlowContainer` 自动换行，左侧内容放入 `ScrollContainer`，1280×720 下不会因商店变长而溢出；手牌行同样使用 `HFlowContainer`，但槽位按 `card-width - hand-card-overlap` = 80 步进排列、行内 `h_separation` 为 0，1–10 张保持单行，卡片自身 112×146 向右溢出并与下一槽重叠。
- 手牌卡使用与商店/棋盘一致的 `GameCardFace` 卡面（介绍、费用、时代角标与底部名称、攻击/类型/生命信息区），只能拖拽出牌（`HandCard._get_drag_data`），点击不产生意图；落点由 `BoardDrop` 判定：随从牌按指针位置换算插入下标、棋盘已满时任何落点都不接收，需要目标的效果牌落在随从卡上即取该卡为目标，其它落点静默取消。拖拽数据带 `epoch`（`_view_epoch`），视图重建后松手按取消处理。
- 手牌的随从数值读服务端视图 `attack` / `health`（当前值，含成长等全部层数），不读 `CardDB` 卡面值；金色随从带「★」，奖励牌在角标行显示锁定的发现星级，并用「发现一张 N 星随从」作为原生 tooltip。拖拽 payload 带 `instanceId` 与 `golden` 供落点与预览判断，不参与意图。
- 手牌悬停由 `HandSlot` 承担：进入时槽位 `z_index = 1`，卡片以底边中点为轴放大 `hand-hover-scale`（1.12）；放大后的矩形包含原矩形，不会在卡片下缘反复触发 `mouse_exited`。悬停不读取 `drag_allowed`，禁用与战斗阶段同样生效；不改顺序、`handIndex` 或 payload。`HandCard` 在拖拽真正开始时发出 `drag_started`，槽位同步复位缩放与层级后才复制拖拽预览，保证预览为原始尺寸。`HandScroll` 关闭 `clip_contents`，让放大后的卡顶越过 154 高的滚动行继续绘制与命中；悬停卡会盖住「手牌」标题，属已接受的视觉取舍。Godot 的 GUI 拾取按树序逆序、不比较 `z_index`，因此悬停期间 `HandSlot` 把右侧槽位卡片的 `mouse_filter` 临时设为 `IGNORE`、复位时恢复 `STOP`：否则放大卡被揭示出来的右侧交叠带仍会命中并拖起下一张卡。左侧槽位不用处理，它在树序中本来就晚于本槽被拾取。拖拽取消或视图刷新后不自动恢复悬停，指针需移出再移入。
- 手牌拖拽期间 `BoardDrop` 逐帧按指针位置复查落点（`hand_drop_accepted`）：空位仍可放置；棋盘外、棋盘已满或需要目标却悬停在空白处判为无效，指针切换为禁止光标（`CURSOR_FORBIDDEN`），松手不产生意图，卡牌留在手牌。
- 棋盘随从卡也使用拖拽：`BoardCard._get_drag_data` 交出 `{epoch, minionId, fromIndex, sellable, golden}`；`BoardDrop` 在 `NOTIFICATION_DRAG_BEGIN` 时隐藏原卡并插入同尺寸透明占位，指针移动时实时移动占位、其余随从让位，松手按「移除后被拖随从剩余序列中的插入下标」发 `RearrangeBoard`，拖拽结束恢复原顺序。商店面板是 `SellDrop`，只接受可出售的棋盘卡：拖入时金色描边并提示「松手出售 +N 金」（`golden ? 3 : 1`），松手发 `SellMinion`；手牌与不可出售随从静默取消。卡内不再生成出售与左右移位按钮。
- 拖放冒泡依赖控件的鼠标过滤：Godot 在「`_can_drop_data` 为假且 `mouse_filter == STOP`」的控件上中止沿父链的落点查找。棋盘空位（`EmptySlot`）设 `IGNORE`；棋盘随从卡（`BoardCard`）与商店槽位设 `PASS`，保留自身点击/拖拽起手与悬停详情，同时把落点交给父级 `BoardDrop` / `SellDrop`。新增落点控件或改其鼠标过滤时必须保证落点仍能到达父级。
- 时代文案由 `CardDB.era_name` 把 1–7 映射为英文 id（`prehistoric`…`future`）后按 `enum.era.<id>` 取文本；流派、卡牌类型、进化选项与攻防分层同理走 `UI.enum_text(group, id)`；右栏自己英雄的属性行、右栏对手身份块的元信息与座位详情统一显示「时代 X」。
- 左栏固定宽 88，座位行只渲染头像（`avatar-rail` = 44）、名次角标（`rank` 为 `null` 时不渲染）与血量条（高 6，按 `hp / hpCap` 比例填充，无数字）；行顺序按 `players[].rank` 升序（第 1 名在最上，`rank` 为 `null` 垫底、同序按座位号），每次视图重建重排，客户端不推导名次；已淘汰行经 `UI.apply_dim` 整体压暗但保留名次且仍可点击。点击展开 `view.combatLog` 中按 `participantIds` 过滤的战报，浮层挂在全屏 `ReportLayer` 上、从座位行右侧覆盖中区与右栏，同一时刻只展开一块，展开座位按 `playerId` 在 `GamePlayersRail.refresh` 整块重建后恢复；无条目显示「无战报」。
- 右栏身份判定以阶段优先：招募阶段固定显示售货员大图（名字「售货员」贴底，营业说明只在悬停时由 `GameDetailLayer` 的 `seller` 详情显示），刷新、升时代与冻结三个商店操作横排在大图正下方，随视图 `affordances` 门控；战斗阶段依次为自身已淘汰「已淘汰 · 观战中」、幽灵战（幽灵归属者英雄 +「幽灵」角标、无血量条、整块压暗）、对手英雄、等待配对，商店操作随招募操作一并隐藏。`SideHero` 的我方英雄同样使用 200×150 大图与贴底名字，部落、时代与技能文本只在悬停时由 `hero` 详情显示，生命/时代/金币三格仍常驻贴底。`SideIdentity` 与 `SideHero` 按两者最小高度的较大值等高并平分状态块之外的剩余高度，`SideStatus` 因此垂直居中；`SideStatus` 把回合数与 `TimerLabel` 剩余时间放在同一行，归零显示「等待结算」；招募阶段悬停倒计时翻成 `EndButton`「结束招募」，移出恢复，点击发 `EndRecruit`。
- 商店卡左上角渲染 `CostBadge`（`display_cost`）、右上角渲染 `EraBadge`（`era`）；冻结由 `FreezeOverlay` 整卡蒙层（`freeze-overlay-alpha` = 0.22，经 `self_modulate` 只压蒙层自身）、间距 14 的 45° 冰晶细线与右下角「冻」字 `FreezeGlyph` 共同表达。
- 棋盘容量取自 `PlayerView.boardSlots`（缺失时回退 7），满场标记、提示文案与空位数量用同一数值；`BoardDrop` 仍由页面设置 `board_full`，控件本身不感知容量。
- 落点回调在 `epoch` 校验之外还要重新过一遍与拖拽起始同源的门控（`drop_gate`）：阶段为招募、未结束招募、视图未过期、倒计时未归零、`Net` 已连接、队首待选不是 `target`。拖拽途中换回合、开战或断线时松手不会发出意图。
- 手牌可拖动的前提是阶段为招募、视图未过期、倒计时未归零、`Net` 已连接、队首待选不是 `target`、且该卡 `playable`；任一不满足时 `_get_drag_data` 返回 `null`，拖拽根本不开始。
- 队首待选为 `target` 时弹出战吼目标浮层：只列我方棋盘随从、高亮触发者，没有取消与跳过；期间购买/刷新/升时代/冻结/结束招募/英雄技能/出售/换位/拖拽全部禁用，选中后发 `ChooseTarget`。
- 战斗演示由 `CombatPlayback` 驱动（`PACE_MS = 320`、`TAIL_MS = 2000`，节奏只在客户端，服务端不估算演示时长）：第 i 条动作在 `(i + 1) × 320ms` 应用，数值只取 `source`/`target`/`summon` 的服务端下发值，客户端不做加减；动作序列只含主动攻击，反击伤害已并入该条动作下发的快照。写入哪一行按动作的 `sourceSide`/`targetSide`（缺失时退回按 `minionId` 双侧查找）。幽灵战改为随机出局者的最终阵容，实例编号只在克隆内与己方重号时按需改写，因此两侧编号仍可判。动作播完并经过 `TAIL_MS` 后才显示胜负横幅。
- 战斗行只渲染 `alive` 为真的随从：阵亡卡立即离场；播放器仍保留阵亡条目作为顺序锚点，复生把 `alive` 置回真后卡片在原相对位置重现。召唤按 `summon.position`（存活序列下标）插入，换算用 `CombatPlayback.alive_insert_index`，不会把阵亡锚点当计数。
- 战斗视图的占位在演示全程固定：两行 `custom_minimum_size.y` 取 `card-height`，一方全灭后空行不塌陷；`CombatBannerSlot` 以 `combat-banner-height`（30）常驻占位，`CombatBanner` 锚点铺满槽位并垂直居中，横幅显示与隐藏都不改变三段高度。
- 战报归属回合：`GameStore.last_combat_turn` 记录收到 `CombatResult` 时的 `turn`，`combat_is_current()` 只在它与当前 `turn` 一致时为真；战报行、战斗横幅与演示都以此为准。`_apply_view` 在阶段离开 `Combat` 时清空 `last_combat` 并复位 `last_combat_turn`，未配对或已出局的座位不会把上一场当成新一场重播。
- 演示进度以 `CombatResult.startedAt`（服务端开战时刻）为锚点：首帧按 `now − startedAt` 直接 `seek`，已播动作不重播；演示期间没有任何跳过或加速入口。
- 碰撞演出由 `GameCombatView` 在 `advance` 里驱动 `CombatFx` 覆盖层（`CombatFxLayer`，`mouse_filter = IGNORE`）：播放器返回本帧到期动作，视图先按服务端数值更新数据，再演出：
  - `attack`：隐藏攻方原卡并在原位抓取矩形，克隆同尺寸卡 130ms 飞向目标；命中时目标闪光、飘出「-伤害」；再 130ms 飞回原位，克隆释放、原卡恢复。`attack` 只应用 `attack`/`health`，不改 `alive`。
  - `death`：受影响卡按原位淡出 160ms 后移除；若攻方本帧阵亡，克隆已回到原位才消失。
  - `reborn`/`summon`：卡片按存活序列位置出现并缩放缓入 160ms。
  - `contagion`：更新数值后卡片短暂变色。
- 演出用逐帧推进而不是 `Tween`，便于无头冒烟按帧断言；`reset()` 与 `_start_combat()` 清理覆盖层与进行中的动画。
- 演示播完后页面为该回合发送一次 `CombatPlayed`（`_combat_ack_turn` 去重），由服务端在全部参战座位回报后结算。
- 返回大厅不使用 `LeaveRoom`（非大厅阶段会被拒绝），而是清除房间状态后重新连接并以令牌认证。
- 对局页的三栏结构、页面状态与数据来源见[对局页](../product/ui/pages/game.md)；颜色、字号、间距与圆角来自 `UITheme`，页面不写死视觉值。左栏宽 88，右栏身份块与英雄块取相同高度、平分中部状态块之外的剩余高度，商店操作与英雄技能因此分列售货员大图与英雄大图下方，生命/时代/金币三格贴底且逐项高对比展示；中区商店与棋盘不带标题，棋盘面板按内容高度收缩。招募阶段与战斗阶段共用同一页面，战斗与结算阶段隐藏招募视图、商店操作、英雄技能与结束招募，节点保留以便阶段恢复。
- 详情层（`client/scenes/game_detail_layer.gd` 的 `GameDetailLayer`，面板节点名 `MinionDetailPanel`）统一承载商店、座位与随从三类详情：商店卡、座位行与招募/战斗随从卡在悬停进入、鼠标左键按下或触摸按下时显示；指针移出锚点矩形后隐藏，松开鼠标左键或触摸时按指针所在目标矩形更新（由 `_input` 按全局矩形做命中判断，避免子控件截断悬停通知）。随从详情先显示名称与 `CardDB.display_text` 介绍，再取协议 `BoardMinion.stats` 按初始、永久、回合、光环、受击、当前六行展示，有符号项带 `+`，受击的攻防都为 0 时省略该行；`stats` 缺失时仍显示名称与介绍，并退化为只显示顶层 `attack` / `health` 的当前攻防；座位与商店字段见[对局页](../product/ui/pages/game.md)。面板只读、忽略鼠标输入，位置以锚点矩形为准并按视口边界夹紧；每次 `_refresh` 重建前先隐藏，同一目标重复触发显示时只校正位置、不重建内容。
- 冒烟测试按 `client/tests/ui_smoke*.tscn` 拆为六个场景，`make client-smoke` 用 glob 逐个运行并要求各自输出 `UI_SMOKE_OK` 且无 `SCRIPT ERROR`。主冒烟覆盖五场景实例化、手牌重叠布局（十张 80 步进单行、相邻交叠、末卡不越界、滚动行不裁剪）、手牌显示服务端身材与奖励牌星级角标、手牌悬停（置顶、底边中点放大 1.12、越出滚动行仍命中、右侧交叠带仍归本卡且拖拽拾起本卡、移出复位、战斗阶段可查看）与拖拽开始复位、真实鼠标事件拖拽（按下、越过阈值、移到落点、松手）落盘与商店出售、拖拽落点意图（首卡 0、次卡 1、末尾空白为随从数）、棋盘卡换位（真实 `force_drag` 通知、原卡隐藏、落点发 `fromIndex/toIndex`）、商店出售落点（可卖与不可卖、手牌拒绝、普通/金色返还提示）、棋盘满时不接收、需要目标的效果牌落点规则、四个不可拖动条件、落点门控（换回合/开战/结束招募/断线时不发意图）、目标待选浮层与控件阻塞、战斗逐动作播放（阵亡卡离场、复生回原位、召唤按存活下标插入）、横幅停留、按截止时间续播、无跳过入口、战斗阶段不弹待选弹层、回合号不匹配时不显示旧战报、离开战斗后清空战报，以及离树导航守卫。`ui_smoke_side` 断言右栏三段的身份判定（含阶段优先、幽灵角标、已淘汰观战）、商店操作位于售货员下方、身份块与英雄块等高且状态块垂直居中、倒计时悬停翻成结束招募与归零文案；`ui_smoke_seats` 断言 8 行座位、头像尺寸、名次角标缺失与压暗、血量条比例、战报展开与刷新保持、无战报文案与座位详情；`ui_smoke_combat` 断言战斗视图只有对手行/横幅/我方行三段且无英雄条，以及固定占位（两行按 `card-height` 取行高、一方全灭后行高与两行位置不变、横幅出现前后两行不动），并以平局夹具断言平局横幅文案。`ui_smoke_cards` 断言商店卡费用与时代角标、冻结蒙层/冰晶/「冻」字、卡面介绍的两行省略、无介绍时的紧凑信息区与名称/数值基线，以及卡牌统一 112×146；`ui_smoke_text` 断言 `CardDB.display_name`（含 Token）、`display_text`、`era_name` 与 `UI.enum_text` 的取用，`UI.error_text` 的错误码文案，`UI.error_text_by_id` 的 `messageId` 细分文案，以及缺键时 `UI.error_text_by_id` 返回空串。

## 4. 生成物与命令

| 命令 | 内容 |
| --- | --- |
| `make protocol` | 由 `protocol/schema/v1/` 生成 `protocol/generated/protocol.gd` 与 `client/generated/protocol.gd` |
| `make protocol-check` | 校验协议样例，并比对两处 GDScript 生成物漂移 |
| `make data` | 生成服务端数据、`client/generated/{cards,skills,tokens}.json` 与文本表 `client/text/zh_CN.csv` |
| `make data-check` | 比对服务端与客户端生成数据、文本表漂移，运行效果审计 |
| `make client-import` | `godot --headless --path client --import`，检查工程可导入 |
| `make client-run` | 先导入再启动桌面客户端；新增或改名 `class_name` 脚本后必须走这个入口，否则沿用旧全局类缓存 |
| `make client-check` | 协议/数据/文本表漂移 + 客户端边界断言 + 国际化断言 + 工程导入 |
| `make client-smoke` | 用 glob 逐个运行六个 `ui_smoke*.tscn` 冒烟场景，要求各自输出 `UI_SMOKE_OK` 且无 `SCRIPT ERROR` |
| `make m1b-e2e` | 启动服务端、Godot 无头客户端与 7 个 `sim bot`，要求输出 `M1B_E2E_OK` |
| `make dev-bots` | 启动服务端与 7 个机器人并打印房间码，供开发者用桌面客户端作为第 8 人手动联调 |

Godot 可执行文件通过 `GODOT` 变量覆盖，默认自动探测 `godot` 或 `/Applications/Godot.app/Contents/MacOS/Godot`。

## 5. 运行方式

手动联调的最省事用法是 `make dev-bots`：

```
make dev-bots                                       # 终端 1，起服务端与 7 个机器人
make client-run                                     # 终端 2，大厅填房间码
```

客户端一律用 `make client-run` 启动：它会先 `--import` 刷新全局类缓存。直接跑 `Godot --path client` 时，若工程新增或改名过 `class_name` 脚本而缓存未更新，依赖它的脚本编译失败，对局页会缺整块区域（例如中区整体不构建，只剩左栏与右栏）。

命令打印两行 `ROOM_CODE=<6 位>` 与提示后等待，开发者在客户端大厅输入该房间码、选英雄并准备，8 人齐自动开局；整局结束后命令自行退出并释放端口，Ctrl-C 也可一次退出全部进程。端口被占用时立即失败，用 `make dev-bots DEV_PORT=8081` 换端口。需要自定义机器人名字或英雄时改用下面的手动方式。

手动方式：

```
make m1b-build
cd server && cargo run -q -p room-service          # 终端 1，监听 127.0.0.1:8080
make client-run                                     # 终端 2，手动登录并创建房间
cargo run -q -p sim -- bot --connect ws://127.0.0.1:8080/ws \
  --room <房间码> --name Bot1 --hero tesla          # 终端 3 起，共 7 个
```

- 客户端创建房间后把 6 位房间码发给 7 个机器人；8 人全部从各自候选中选好英雄并准备后服务端自动开局。
- 机器人参数与协议顺序见[房间服务与联网](room-service.md) §11。
- 无头验收用 `make m1b-e2e`：客户端自动建房并写入房间码，脚本再拉起 7 个机器人，结束后输出 `M1B_GAME_OVER place=<n> seed=<seed> ranking=<n> intents=<n> errors=<n> soft=<n>`；脚本校验名次、8 人排名、种子、意图数，并要求错误全部落在预期白名单（`NO_SPACE`、`NOT_ENOUGH_GOLD`、`SHOP_SLOT_EMPTY` 与补选目标后的 `NO_VALID_TARGET`），其余错误为 0，服务端与机器人日志无 panic。
- 无头驱动不等上一条意图确认就按新视图继续决策，服务端按最新状态拒绝属预期；因此 `errors` 与 `soft` 必须相等。驱动另外对同一回合的同一商店槽位只提交一次购买（刷新后允许再次购买），避免重复购买同一槽位。
- 驱动按 `clientIntentId` 找回被拒意图，收到 `NO_VALID_TARGET` 时补上棋盘第一个己方随从重发；队首待选为 `target` 时改发 `ChooseTarget`（同样取棋盘第一个随从），不再一律当发现处理；`sim bot` 的处理方式相同（见[房间服务与联网](room-service.md) §11）。
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
  - 枚举：`enum.card_type.<id>`、`enum.tribe.<id>`、`enum.era.<id>`、`enum.keyword.<id>`、`enum.subtype.<slug>`、`enum.evolve.<id>`、`enum.stat_layer.<id>`；
  - 错误与界面：`error.<CODE>`（默认文案）、`error.<CODE>.<detail>`（服务端 `messageId` 的细分文案）、`ui.*`（页面静态文案）。
- **所有权**：`card.*`、`skill.*`、`token.*` 与 `enum.card_type|tribe|era|keyword|subtype.*` 是生成 key，由 `scripts/gen_text.py` 从实体文档整体重算；`ui.*`、`error.*`、`enum.evolve.*`、`enum.stat_layer.*` 是人工 key，原样保留；出现未知前缀直接报错。`make data` 会写入 CSV，`make data-check` 检测漂移。
- **前置条件**：修改 CSV 或任何实体文本后必须运行 `make client-import`（或任何以它为前提的目标，如 `client-run`、`client-smoke`、`m1b-e2e`），让 Godot 重新导入并生成 `.translation`；只在编辑器外改 CSV 而不导入，运行时仍读旧翻译。`make client-check` 已包含 `client-import`。
- **服务端错误**：`ErrorMessage.messageId` 默认 `error.<CODE>`，`messageParams` 按 `%s` 填充；客户端状态栏先按 `code` 取 `error.<CODE>`，悬停详情优先用 `messageId`/`messageParams`，`message` 仅面向日志。
- **新增语言**：一个语言一个文件。新增语言需要：新增 `client/text/<locale>.csv`，并扩展 `scripts/gen_text.py` 以支持按语言文件读写与产出（每个语言文件独立生成内容 key 与枚举 key）；用 `make client-import` 导入后把对应 `.translation` 加入 `[internationalization]` 的 `locale/translations`。业务代码（`CardDB`、`UI.*`、`tr()` 调用）与实体文档、协议均无需改动。

## 7. 与设计的偏差

1. 设计 §5.6 只要求“1 个客户端 + 7 个机器人人工验收”；实现额外提供 `client/tests/headless_game.*` 与 `make m1b-e2e`，用真实客户端栈（Net + GameStore）自动打完一局，作为协议与视图消费的回归证据。该驱动不点击 UI，五场景可读性仍由 `make client-smoke` 与手动运行保证。
2. 卡牌展示表新增 `display_cost`：随从固定 3、效果牌取文档 `cost`，仅用于显示；规则判定仍全部在服务端。
3. `PlayCard` 的目标由拖拽落点直接给出（随从牌带 `position`，需要目标的效果牌带 `targetMinionId`），不再先发无目标再补选；英雄技能仍先在己方有随从时弹目标列表，只按 `NO_VALID_TARGET` 补选。`BuyMinion` 只携带槽位，买入一律入手牌，不登记补选目标。
4. 英雄技能在己方有随从时先弹出可选目标列表，含「不指定目标」；己方无随从时直接无目标发送。

## 相关文档

- [房间服务与联网](room-service.md)
- [规则引擎与模拟器](rule-engine-and-simulator.md)
- [技术现状索引](README.md)
- [客户端技术基线](../../versions/v0.1/technical/architecture/client-stack.md)
