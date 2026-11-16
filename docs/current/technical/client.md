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
    card_db.gd               # 只读卡牌/技能展示表
  ui/
    ui.gd                    # 场景共用控件工具、共享构件与统一导航入口（class_name UI）
    theme.gd                 # 三层视觉令牌与 StyleBox 工厂（class_name UITheme）
  scenes/
    login.tscn/.gd           # 登录
    lobby.tscn/.gd           # 大厅
    room.tscn/.gd            # 房间
    game.tscn/.gd            # 对局
    hand_card.gd             # 手牌卡：拖拽数据与预览（class_name HandCard）
    board_drop.gd            # 我方棋盘落点：命中判定与插入下标（class_name BoardDrop）
    combat_playback.gd       # 战斗动作落地器：节奏与状态应用（class_name CombatPlayback）
    result.tscn/.gd          # 结算
  tests/
    ui_smoke.tscn/.gd        # 五场景无头实例化冒烟与离树导航守卫
    fixture.gd               # 冒烟用最小 PlayerView
    headless_game.tscn/.gd   # 无头对局驱动（1 客户端 + 7 机器人）
  generated/
    protocol.gd              # schema 生成的 GDScript 常量
    cards.json               # 产品文档生成的卡牌展示表
    skills.json              # 技能展示表
    tokens.json              # Token 展示表
```

窗口启动时由 `WindowSizer` 按当前屏幕可用区域（`screen_get_usable_rect`，已排除菜单栏与 Dock）的 90% 设置尺寸并居中；无头模式下跳过。逻辑视口固定 1280×720，窗口尺寸变化由 `canvas_items` 等比缩放，页面不感知实际窗口大小。

## 2. 全局状态与网络层

- `GameStore`：保存 `playerId`、`token`、`roomId`、`roomCode`、房间成员、`PlayerView`、`lastSeq`、排名与种子。`handle(type, payload)` 只做字段搬运，发出 `auth_changed`、`room_changed`、`view_changed`、`game_started`、`game_over`、`error_received` 信号；`reset_room()` 清空房间，`reset_session()` 再清空身份与令牌。
- `Net`：持有一个 `WebSocketPeer`，`_process` 中轮询；连接建立后先认证，断线每 3 秒自动重连。重连时以令牌发 `AuthRequest`，若仍在房间再发 `Reconnect{roomId, lastSeq}`，由服务端回全量 `Snapshot`。
- 下行信封只校验 `v == 1` 与 `type`，`payload` 原样交给 `GameStore`；未知类型忽略。上行信封为 `{v, id, type, ts, payload}`，`id` 由时间戳与计数器生成。
- `CardDB`：启动时读取 `client/generated/{cards,skills,tokens}.json`，提供卡名、技能文本与展示费用；不参与任何规则判定。

## 3. 五个场景

| 场景 | 职责 |
| --- | --- |
| 登录 | 输入服务器地址、手机号、固定验证码 `123456` 与显示名；认证成功后进入大厅 |
| 大厅 | 创建房间或凭 6 位房间码加入；收到 `RoomUpdate` 后进入房间 |
| 房间 | 展示房间码与成员、选择英雄、准备/取消准备、离开房间；开局完全由服务端决定 |
| 对局 | 酒馆战棋式三栏布局：顶栏回合/计时/属性，左栏 8 人积分榜，中区招募视图（自上而下为商店、7 槽棋盘）与战斗视图（双方英雄、双方随从、结果横幅）互斥，手牌行常驻在两者之下，右栏我的英雄/英雄技能/结束招募/战报；十一种意图对应操作；`pendingChoices` 弹层（发现/进化/战吼目标）；英雄技能目标缺失时按 `NO_VALID_TARGET` 补选目标 |
| 结算 | 按服务端 `ranking` 展示名次与 `seed`；返回大厅时用令牌重新认证 |

- 对局界面整块读取 `GameStore.view`，每次 `view_changed` 重建；按钮可用性只取 `affordances` / `playable` / `can*`，不做本地推导。冻结与棋盘卡拖拽换位没有对应的服务端 affordance，按 `phase`、`self.recruited` 与下标做本地门控，与服务端拒绝条件保持一致。
- 操作还要求视图自身的 `turn` 与 `GameStore.turn` 一致：新回合的 `PhaseChanged` 会先于新视图到达，旧视图只展示、不可操作，避免用上一回合的商店发意图。
- 倒计时由 `phaseDeadline` 计算；归零后结束招募、刷新、购买、出牌、出售与换位一并禁用，只等服务端推进阶段。
- 已结束招募（`self.recruited`）后只展示界面，不再发起意图，也不弹待选弹层。
- 状态栏文案按错误码取 `UI.error_text` 的固定中文；协议 `message` 只进 tooltip，不作为 UI 文案。
- `UI.error_route` 决定错误后的去向：认证类错误回登录并清空会话，`NOT_IN_ROOM` / `ROOM_NOT_FOUND` 回大厅并清空房间；其余错误留在当前页面。
- 未连接时点击操作不会静默丢弃：状态栏提示「未连接，操作未发出」，发送失败不登记目标重试。目标重试表按回合清理并设上限。
- 页面切换统一走 `UI.navigate(page, path)`：`change_scene_to_file` 会立即把当前场景摘下，而页面仍连着 `GameStore` 等 autoload 信号；摘下后的回调再取 `get_tree()` 会得到 `null`。`UI.navigate` 以「页面仍在树上」为唯一判定，离树页面的导航直接丢弃并返回 `false`，场景脚本不再直接调用 `get_tree().change_scene_to_file`。
- 房间场景同时监听 `view_changed`：重连或晚到的视图显示对局已开始时直接进入对局；房间不存在或已不在房间时回大厅。
- 商店与手牌使用 `HFlowContainer` 自动换行，左侧内容放入 `ScrollContainer`，1280×720 下不会因手牌或商店变长而溢出。
- 手牌只能拖拽出牌（`HandCard._get_drag_data`），点击不产生意图；落点由 `BoardDrop` 判定：随从牌按指针位置换算插入下标、棋盘已满时任何落点都不接收，需要目标的效果牌落在随从卡上即取该卡为目标，其它落点静默取消。拖拽数据带 `epoch`（`_view_epoch`），视图重建后松手按取消处理。
- 手牌拖拽期间 `BoardDrop` 逐帧按指针位置复查落点（`hand_drop_accepted`）：空位仍可放置；棋盘外、棋盘已满或需要目标却悬停在空白处判为无效，指针切换为禁止光标（`CURSOR_FORBIDDEN`），松手不产生意图，卡牌留在手牌。
- 棋盘随从卡也使用拖拽：`BoardCard._get_drag_data` 交出 `{epoch, minionId, fromIndex, sellable, golden}`；`BoardDrop` 在 `NOTIFICATION_DRAG_BEGIN` 时隐藏原卡并插入同尺寸透明占位，指针移动时实时移动占位、其余随从让位，松手按「移除后被拖随从剩余序列中的插入下标」发 `RearrangeBoard`，拖拽结束恢复原顺序。商店面板是 `SellDrop`，只接受可出售的棋盘卡：拖入时金色描边并提示「松手出售 +N 金」（`golden ? 3 : 1`），松手发 `SellMinion`；手牌与不可出售随从静默取消。卡内不再生成出售与左右移位按钮。
- 拖放冒泡依赖控件的鼠标过滤：Godot 在「`_can_drop_data` 为假且 `mouse_filter == STOP`」的控件上中止沿父链的落点查找。棋盘空位（`EmptySlot`）设 `IGNORE`；棋盘随从卡（`BoardCard`）与商店槽位按钮、商店标题按钮设 `PASS`，保留自身点击/拖拽起手与悬停详情，同时把落点交给父级 `BoardDrop` / `SellDrop`；商店占位 `shop_spacer` 设 `IGNORE`。新增落点控件或改其鼠标过滤时必须保证落点仍能到达父级。
- 时代文案由 `CardDB.era_name` 把 1–7 映射为「史前、原始、古典、封建、工业、现代、未来」；顶栏胶囊、积分榜座位、右侧属性行与战斗双方信息条统一显示「时代 X」。
- 棋盘容量取自 `PlayerView.boardSlots`（缺失时回退 7），满场标记、提示文案与空位数量用同一数值；`BoardDrop` 仍由页面设置 `board_full`，控件本身不感知容量。
- 落点回调在 `epoch` 校验之外还要重新过一遍与拖拽起始同源的门控（`drop_gate`）：阶段为招募、未结束招募、视图未过期、倒计时未归零、`Net` 已连接、队首待选不是 `target`。拖拽途中换回合、开战或断线时松手不会发出意图。
- 手牌可拖动的前提是阶段为招募、视图未过期、倒计时未归零、`Net` 已连接、队首待选不是 `target`、且该卡 `playable`；任一不满足时 `_get_drag_data` 返回 `null`，拖拽根本不开始。
- 队首待选为 `target` 时弹出战吼目标浮层：只列我方棋盘随从、高亮触发者，没有取消与跳过；期间购买/刷新/升时代/冻结/结束招募/英雄技能/出售/换位/拖拽全部禁用，选中后发 `ChooseTarget`。
- 战斗演示由 `CombatPlayback` 驱动（`PACE_MS = 320`、`TAIL_MS = 2000`，与服务端一致）：第 i 条动作在 `(i + 1) × 320ms` 应用，数值只取 `source`/`target`/`summon` 的服务端下发值，客户端不做加减；写入哪一行按动作的 `sourceSide`/`targetSide`（缺失时退回按 `minionId` 双侧查找），因此镜像战双方实例编号相同也能落到正确一侧。动作播完并经过 `TAIL_MS` 后才显示胜负横幅。
- 战斗行只渲染 `alive` 为真的随从：阵亡卡立即离场；播放器仍保留阵亡条目作为顺序锚点，复生把 `alive` 置回真后卡片在原相对位置重现。召唤按 `summon.position`（存活序列下标）插入，换算用 `CombatPlayback.alive_insert_index`，不会把阵亡锚点当计数。
- 战报归属回合：`GameStore.last_combat_turn` 记录收到 `CombatResult` 时的 `turn`，`combat_is_current()` 只在它与当前 `turn` 一致时为真；战报行、战斗横幅与演示都以此为准。`_apply_view` 在阶段离开 `Combat` 时清空 `last_combat` 并复位 `last_combat_turn`，未配对或已出局的座位不会把上一场当成新一场重播。
- 演示中的重连按 `phaseDeadline` 反推进度（`elapsed = total − (deadline − now)`）直接 `seek`，已播动作不重播；演示期间没有任何跳过或加速入口。
- 返回大厅不使用 `LeaveRoom`（非大厅阶段会被拒绝），而是清除房间状态后重新连接并以令牌认证。
- 对局页的三栏结构、页面状态与数据来源见[对局页](../product/ui/pages/game.md)；颜色、字号、间距与圆角来自 `UITheme`，页面不写死视觉值。招募阶段与战斗阶段共用同一页面，战斗与结算阶段隐藏招募视图、英雄技能与结束招募，节点保留以便阶段恢复。
- 棋盘随从详情面板（`game.gd` 的 `MinionDetailPanel`）：招募棋盘与战斗双方的随从卡统一挂详情，悬停进入、鼠标左键按下或触摸按下时显示；指针移出卡片矩形后隐藏，松开鼠标左键或触摸时按指针所在卡片矩形更新（由 `_input` 按卡片全局矩形做命中判断，避免卡内子控件截断悬停通知）。数据取自协议 `BoardMinion.stats`，按初始、永久、回合、光环、受击、当前六行展示，有符号项带 `+`，受击的攻防都为 0 时省略该行；`stats` 缺失时退化为只显示顶层 `attack` / `health` 的当前攻防。面板只读、忽略鼠标输入，位置以卡片矩形为锚点并按视口边界夹紧；每次 `_refresh` 重建前先隐藏，同一张卡重复触发显示时只校正位置、不重建内容。
- 冒烟测试额外断言对局页的 8 座积分榜、按视图容量的棋盘槽位（含空位）、招募/战斗视图互斥、手牌行位于中区战斗视图之下、双方随从行与结果横幅，以及详情面板节点存在且默认隐藏；并覆盖真实鼠标事件拖拽（按下、越过阈值、移到落点、松手）落盘与商店出售、拖拽落点意图（首卡 0、次卡 1、末尾空白为随从数）、棋盘卡换位（真实 `force_drag` 通知、原卡隐藏、落点发 `fromIndex/toIndex`）、商店出售落点（可卖与不可卖、手牌拒绝、普通/金色返还提示）、时代名称在顶栏/积分榜/属性行的展示、棋盘满时不接收、需要目标的效果牌落点规则、四个不可拖动条件、落点门控（换回合/开战/结束招募/断线时不发意图）、目标待选浮层与控件阻塞、战斗逐动作播放（含双方实例编号相同时按侧别落地、阵亡卡离场、复生回原位、召唤按存活下标插入）、横幅停留、按截止时间续播、无跳过入口、战斗阶段不弹待选弹层、回合号不匹配时不显示旧战报，以及离开战斗后清空战报。

## 4. 生成物与命令

| 命令 | 内容 |
| --- | --- |
| `make protocol` | 由 `protocol/schema/v1/` 生成 `protocol/generated/protocol.gd` 与 `client/generated/protocol.gd` |
| `make protocol-check` | 校验协议样例，并比对两处 GDScript 生成物漂移 |
| `make data` | 生成服务端数据与 `client/generated/{cards,skills,tokens}.json` |
| `make data-check` | 比对服务端与客户端生成数据，运行效果审计 |
| `make client-import` | `godot --headless --path client --import`，检查工程可导入 |
| `make client-check` | 协议/数据漂移 + 客户端边界断言 + 工程导入 |
| `make client-smoke` | 无头实例化五个场景，要求输出 `UI_SMOKE_OK` 且无 `SCRIPT ERROR` |
| `make m1b-e2e` | 启动服务端、Godot 无头客户端与 7 个 `sim bot`，要求输出 `M1B_E2E_OK` |
| `make dev-bots` | 启动服务端与 7 个机器人并打印房间码，供开发者用桌面客户端作为第 8 人手动联调 |

Godot 可执行文件通过 `GODOT` 变量覆盖，默认自动探测 `godot` 或 `/Applications/Godot.app/Contents/MacOS/Godot`。

## 5. 运行方式

手动联调的最省事用法是 `make dev-bots`：

```
make dev-bots                                       # 终端 1，起服务端与 7 个机器人
/Applications/Godot.app/Contents/MacOS/Godot --path client   # 终端 2，大厅填房间码
```

命令打印两行 `ROOM_CODE=<6 位>` 与提示后等待，开发者在客户端大厅输入该房间码、选英雄并准备，8 人齐自动开局；整局结束后命令自行退出并释放端口，Ctrl-C 也可一次退出全部进程。端口被占用时立即失败，用 `make dev-bots DEV_PORT=8081` 换端口。需要自定义机器人名字或英雄时改用下面的手动方式。

手动方式：

```
make m1b-build
cd server && cargo run -q -p room-service          # 终端 1，监听 127.0.0.1:8080
godot --path client                                 # 终端 2，手动登录并创建房间
cargo run -q -p sim -- bot --connect ws://127.0.0.1:8080/ws \
  --room <房间码> --name Bot1 --hero tesla          # 终端 3 起，共 7 个
```

- 客户端创建房间后把 6 位房间码发给 7 个机器人；8 人全部从各自候选中选好英雄并准备后服务端自动开局。
- 机器人参数与协议顺序见[房间服务与联网](room-service.md) §11。
- 无头验收用 `make m1b-e2e`：客户端自动建房并写入房间码，脚本再拉起 7 个机器人，结束后输出 `M1B_GAME_OVER place=<n> seed=<seed> ranking=<n> intents=<n> errors=<n> soft=<n>`；脚本校验名次、8 人排名、种子、意图数，并要求错误全部落在预期白名单（`NO_SPACE`、`NOT_ENOUGH_GOLD`、`SHOP_SLOT_EMPTY` 与补选目标后的 `NO_VALID_TARGET`），其余错误为 0，服务端与机器人日志无 panic。
- 无头驱动不等上一条意图确认就按新视图继续决策，服务端按最新状态拒绝属预期；因此 `errors` 与 `soft` 必须相等。驱动另外对同一回合的同一商店槽位只提交一次购买（刷新后允许再次购买），避免重复购买同一槽位。
- 驱动按 `clientIntentId` 找回被拒意图，收到 `NO_VALID_TARGET` 时补上棋盘第一个己方随从重发；队首待选为 `target` 时改发 `ChooseTarget`（同样取棋盘第一个随从），不再一律当发现处理；`sim bot` 的处理方式相同（见[房间服务与联网](room-service.md) §11）。
- 默认端口 `18080` 被其他进程占用时，脚本会因监听失败退出；用 `M1B_PORT=<端口> make m1b-e2e` 覆盖，脚本与客户端读取同一个变量。

## 6. 与设计的偏差

1. 设计 §5.6 只要求“1 个客户端 + 7 个机器人人工验收”；实现额外提供 `client/tests/headless_game.*` 与 `make m1b-e2e`，用真实客户端栈（Net + GameStore）自动打完一局，作为协议与视图消费的回归证据。该驱动不点击 UI，五场景可读性仍由 `make client-smoke` 与手动运行保证。
2. 卡牌展示表新增 `display_cost`：随从固定 3、效果牌取文档 `cost`，仅用于显示；规则判定仍全部在服务端。
3. `PlayCard` 的目标由拖拽落点直接给出（随从牌带 `position`，需要目标的效果牌带 `targetMinionId`），不再先发无目标再补选；英雄技能仍先在己方有随从时弹目标列表，只按 `NO_VALID_TARGET` 补选。`BuyMinion` 只携带槽位，买入一律入手牌，不登记补选目标。
4. 英雄技能在己方有随从时先弹出可选目标列表，含「不指定目标」；己方无随从时直接无目标发送。

## 相关文档

- [房间服务与联网](room-service.md)
- [规则引擎与模拟器](rule-engine-and-simulator.md)
- [技术现状索引](README.md)
- [客户端技术基线](../../versions/v0.1/technical/architecture/client-stack.md)
