# Godot 客户端

> 状态：已实现（v0.1 M1b）
> 代码：`client/`
> 验收：`make client-check`、`make client-smoke`、`make m1b-e2e`

本文记录 v0.1 M1b 已实现的 Godot 客户端现状：工程结构、全局状态与网络层、五个场景、生成物与运行方式；不描述服务端房间服务（见[房间服务与联网](room-service.md)）与产品规则（见[规则引擎与模拟器](rule-engine-and-simulator.md)）。

## 1. 工程结构

```
client/
  project.godot              # 主场景 login.tscn；autoload：GameStore、CardDB、Net
  .gitignore                 # 忽略 .godot/ 与 *.uid
  autoload/
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
| 对局 | 酒馆战棋式三栏布局：顶栏回合/计时/属性，左栏 8 人积分榜，中区招募视图（自上而下为商店、7 槽棋盘、手牌）或战斗视图（双方英雄、双方随从、结果横幅），右栏我的英雄/英雄技能/结束招募/战报；十种意图对应操作；`pendingChoices` 弹层；目标缺失时按 `NO_VALID_TARGET` 补选目标 |
| 结算 | 按服务端 `ranking` 展示名次与 `seed`；返回大厅时用令牌重新认证 |

- 对局界面整块读取 `GameStore.view`，每次 `view_changed` 重建；按钮可用性只取 `affordances` / `playable` / `can*`，不做本地推导。冻结与左右移位没有对应的服务端 affordance，按 `phase`、`self.recruited` 与下标做本地门控，与服务端拒绝条件保持一致。
- 操作还要求视图自身的 `turn` 与 `GameStore.turn` 一致：新回合的 `PhaseChanged` 会先于新视图到达，旧视图只展示、不可操作，避免用上一回合的商店发意图。
- 倒计时由 `phaseDeadline` 计算；归零后结束招募、刷新、购买、出牌、出售与重排按钮一并禁用，只等服务端推进阶段。
- 已结束招募（`self.recruited`）后只展示界面，不再发起意图，也不弹待选弹层。
- 状态栏文案按错误码取 `UI.error_text` 的固定中文；协议 `message` 只进 tooltip，不作为 UI 文案。
- `UI.error_route` 决定错误后的去向：认证类错误回登录并清空会话，`NOT_IN_ROOM` / `ROOM_NOT_FOUND` 回大厅并清空房间；其余错误留在当前页面。
- 未连接时点击操作不会静默丢弃：状态栏提示「未连接，操作未发出」，发送失败不登记目标重试。目标重试表按回合清理并设上限。
- 页面切换统一走 `UI.navigate(page, path)`：`change_scene_to_file` 会立即把当前场景摘下，而页面仍连着 `GameStore` 等 autoload 信号；摘下后的回调再取 `get_tree()` 会得到 `null`。`UI.navigate` 以「页面仍在树上」为唯一判定，离树页面的导航直接丢弃并返回 `false`，场景脚本不再直接调用 `get_tree().change_scene_to_file`。
- 房间场景同时监听 `view_changed`：重连或晚到的视图显示对局已开始时直接进入对局；房间不存在或已不在房间时回大厅。
- 商店与手牌使用 `HFlowContainer` 自动换行，左侧内容放入 `ScrollContainer`，1280×720 下不会因手牌或商店变长而溢出。
- 战斗只展示服务端 `CombatResult` 的胜者、伤害与双方随从数，不本地重放结算。
- 返回大厅不使用 `LeaveRoom`（非大厅阶段会被拒绝），而是清除房间状态后重新连接并以令牌认证。
- 对局页的三栏结构、页面状态与数据来源见[对局页](../product/ui/pages/game.md)；颜色、字号、间距与圆角来自 `UITheme`，页面不写死视觉值。招募阶段与战斗阶段共用同一页面，战斗与结算阶段隐藏招募视图、英雄技能与结束招募，节点保留以便阶段恢复。
- 棋盘随从详情面板（`game.gd` 的 `MinionDetailPanel`）：招募棋盘与战斗双方的随从卡统一挂详情，悬停进入、鼠标左键按下或触摸按下时显示；指针移出卡片矩形后隐藏，松开鼠标左键或触摸时按指针所在卡片矩形更新（由 `_input` 按卡片全局矩形做命中判断，避免卡内子控件截断悬停通知）。数据取自协议 `BoardMinion.stats`，按初始、永久、回合、光环、受击、当前六行展示，有符号项带 `+`，受击的攻防都为 0 时省略该行；`stats` 缺失时退化为只显示顶层 `attack` / `health` 的当前攻防。面板只读、忽略鼠标输入，位置以卡片矩形为锚点并按视口边界夹紧；每次 `_refresh` 重建前先隐藏。
- 冒烟测试额外断言对局页的 8 座积分榜、7 槽棋盘（含空位）、招募/战斗视图互斥、双方随从行与结果横幅，以及详情面板节点存在且默认隐藏。

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

- 客户端创建房间后把 6 位房间码发给 7 个机器人；8 人全部选好英雄并准备后服务端自动开局。
- 机器人参数与协议顺序见[房间服务与联网](room-service.md) §11。
- 无头验收用 `make m1b-e2e`：客户端自动建房并写入房间码，脚本再拉起 7 个机器人，结束后输出 `M1B_GAME_OVER place=<n> seed=<seed> ranking=<n> intents=<n> errors=<n> soft=<n>`；脚本校验名次、8 人排名、种子、意图数，并要求错误全部落在预期白名单（`NO_SPACE`、`NOT_ENOUGH_GOLD`、`SHOP_SLOT_EMPTY` 与补选目标后的 `NO_VALID_TARGET`），其余错误为 0，服务端与机器人日志无 panic。
- 无头驱动不等上一条意图确认就按新视图继续决策，服务端按最新状态拒绝属预期；因此 `errors` 与 `soft` 必须相等。驱动另外对同一回合的同一商店槽位只提交一次购买（刷新后允许再次购买），避免重复购买同一槽位。
- 驱动按 `clientIntentId` 找回被拒意图，收到 `NO_VALID_TARGET` 时补上棋盘第一个己方随从重发；`sim bot` 的处理方式相同（见[房间服务与联网](room-service.md) §11）。
- 默认端口 `18080` 被其他进程占用时，脚本会因监听失败退出；用 `M1B_PORT=<端口> make m1b-e2e` 覆盖，脚本与客户端读取同一个变量。

## 6. 与设计的偏差

1. 设计 §5.6 只要求“1 个客户端 + 7 个机器人人工验收”；实现额外提供 `client/tests/headless_game.*` 与 `make m1b-e2e`，用真实客户端栈（Net + GameStore）自动打完一局，作为协议与视图消费的回归证据。该驱动不点击 UI，五场景可读性仍由 `make client-smoke` 与手动运行保证。
2. 卡牌展示表新增 `display_cost`：随从固定 3、效果牌取文档 `cost`，仅用于显示；规则判定仍全部在服务端。
3. `PlayCard` 的 `targetMinionId` 不预先询问：先按无目标发送，收到 `NO_VALID_TARGET` 后再弹出己方随从列表并带目标重发同一意图。服务端在存在合法目标而未指定时返回该错误；主动技能在池为空时也返回该错误（技能不可用），随从战吼与效果牌则放行并让效果落空（见[房间服务与联网](room-service.md) §7）。`BuyMinion` 只携带槽位，买入一律入手牌，不登记补选目标。
4. 英雄技能在己方有随从时先弹出可选目标列表，含「不指定目标」；己方无随从时直接无目标发送。

## 相关文档

- [房间服务与联网](room-service.md)
- [规则引擎与模拟器](rule-engine-and-simulator.md)
- [技术现状索引](README.md)
- [客户端技术基线](../../versions/v0.1/technical/architecture/client-stack.md)
