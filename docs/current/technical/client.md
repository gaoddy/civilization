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
  ui/ui.gd                   # 场景共用控件工具（class_name UI）
  scenes/
    login.tscn/.gd           # 登录
    lobby.tscn/.gd           # 大厅
    room.tscn/.gd            # 房间
    game.tscn/.gd            # 对局
    result.tscn/.gd          # 结算
  tests/
    ui_smoke.tscn/.gd        # 五场景无头实例化冒烟
    fixture.gd               # 冒烟用最小 PlayerView
    headless_game.tscn/.gd   # 无头对局驱动（1 客户端 + 7 机器人）
  generated/
    protocol.gd              # schema 生成的 GDScript 常量
    cards.json               # 产品文档生成的卡牌展示表
    skills.json              # 技能展示表
    tokens.json              # Token 展示表
```

## 2. 全局状态与网络层

- `GameStore`：保存 `playerId`、`token`、`roomId`、`roomCode`、房间成员、`PlayerView`、`lastSeq`、排名与种子。`handle(type, payload)` 只做字段搬运，发出 `auth_changed`、`room_changed`、`view_changed`、`game_started`、`game_over`、`error_received`、`combat_received` 信号。
- `Net`：持有一个 `WebSocketPeer`，`_process` 中轮询；连接建立后先认证，断线每 3 秒自动重连。重连时以令牌发 `AuthRequest`，若仍在房间再发 `Reconnect{roomId, lastSeq}`，由服务端回全量 `Snapshot`。
- 下行信封只校验 `v == 1` 与 `type`，`payload` 原样交给 `GameStore`；未知类型忽略。上行信封为 `{v, id, type, ts, payload}`，`id` 由时间戳与计数器生成。
- `CardDB`：启动时读取 `client/generated/{cards,skills,tokens}.json`，提供卡名、技能文本与展示费用；不参与任何规则判定。

## 3. 五个场景

| 场景 | 职责 |
| --- | --- |
| 登录 | 输入服务器地址、手机号、固定验证码 `123456` 与显示名；认证成功后进入大厅 |
| 大厅 | 创建房间或凭 6 位房间码加入；收到 `RoomUpdate` 后进入房间 |
| 房间 | 展示房间码与成员、选择英雄、准备/取消准备、离开房间；开局完全由服务端决定 |
| 对局 | 商店、手牌、棋盘、玩家列表、倒计时、战报与事件；十种意图对应操作；`pendingChoices` 弹层；目标缺失时按 `NO_VALID_TARGET` 补选目标 |
| 结算 | 按服务端 `ranking` 展示名次与 `seed`；返回大厅时用令牌重新认证 |

- 对局界面整块读取 `GameStore.view`，每次 `view_changed` 重建；按钮可用性只取 `affordances` / `playable` / `can*`，不做本地推导。冻结与左右移位没有对应的服务端 affordance，按 `phase`、`self.recruited` 与下标做本地门控，与服务端拒绝条件保持一致。
- 操作还要求视图自身的 `turn` 与 `GameStore.turn` 一致：新回合的 `PhaseChanged` 会先于新视图到达，旧视图只展示、不可操作，避免用上一回合的商店发意图。
- 倒计时由 `phaseDeadline` 计算；归零后结束招募、刷新、购买、出牌、出售与重排按钮一并禁用，只等服务端推进阶段。
- 已结束招募（`self.recruited`）后只展示界面，不再发起意图，也不弹待选弹层。
- 战斗只展示服务端 `CombatResult` 的胜者、伤害与双方随从数，不本地重放结算。
- 返回大厅不使用 `LeaveRoom`（非大厅阶段会被拒绝），而是清除房间状态后重新连接并以令牌认证。

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

Godot 可执行文件通过 `GODOT` 变量覆盖，默认自动探测 `godot` 或 `/Applications/Godot.app/Contents/MacOS/Godot`。

## 5. 运行方式

```
make m1b-build
cd server && cargo run -q -p room-service          # 终端 1，监听 127.0.0.1:8080
godot --path client                                 # 终端 2，手动登录并创建房间
cargo run -q -p sim -- bot --connect ws://127.0.0.1:8080/ws \
  --room <房间码> --name Bot1 --hero tesla          # 终端 3 起，共 7 个
```

- 客户端创建房间后把 6 位房间码发给 7 个机器人；8 人全部选好英雄并准备后服务端自动开局。
- 机器人参数与协议顺序见[房间服务与联网](room-service.md) §11。
- 无头验收用 `make m1b-e2e`：客户端自动建房并写入房间码，脚本再拉起 7 个机器人，结束后输出 `M1B_GAME_OVER place=<n> seed=<seed> ranking=<n> intents=<n> errors=<n>`；脚本校验名次、8 人排名、种子、意图数与客户端协议错误数为 0，并要求服务端与机器人日志无 panic。

## 6. 与设计的偏差

1. 设计 §5.6 只要求“1 个客户端 + 7 个机器人人工验收”；实现额外提供 `client/tests/headless_game.*` 与 `make m1b-e2e`，用真实客户端栈（Net + GameStore）自动打完一局，作为协议与视图消费的回归证据。该驱动不点击 UI，五场景可读性仍由 `make client-smoke` 与手动运行保证。
2. 卡牌展示表新增 `display_cost`：随从固定 3、效果牌取文档 `cost`，仅用于显示；规则判定仍全部在服务端。
3. `BuyMinion` / `PlayCard` 的 `targetMinionId` 不预先询问：先按无目标发送，收到 `NO_VALID_TARGET` 后再弹出己方随从列表并带目标重发同一意图。当前服务端尚未在无目标时返回该错误码（见[待确认事项](../../versions/v0.1/open-questions.md)），实际表现为效果不结算；该重试路径已实现但没有端到端触发条件。
4. 英雄技能在己方有随从时先弹出可选目标列表，含「不指定目标」；己方无随从时直接无目标发送。

## 相关文档

- [房间服务与联网](room-service.md)
- [规则引擎与模拟器](rule-engine-and-simulator.md)
- [技术现状索引](README.md)
- [客户端技术基线](../../versions/v0.1/technical/architecture/client-stack.md)
