# 对局页

> 状态：已实现
> 场景：`client/scenes/game.tscn`
> 脚本：`client/scenes/game.gd`
> 验证：`make client-smoke`

## 1. 目的、入口与返回

- 目的：在酒馆战棋式的三栏布局中呈现服务端 `PlayerView` 与 `CombatResult`，并把玩家操作转成协议意图。
- 入口：房间页在 `RoomUpdate` 阶段离开 `Lobby` 或收到 `GameStarted` 后导航进入；重连后收到视图也会进入。
- 返回：对局结束收到 `GameOver` 后进入结算页；认证类错误回登录；`NOT_IN_ROOM` / `ROOM_NOT_FOUND` 回大厅。

## 2. 结构层级

```text
背景
└─ 外边距
   └─ 纵向根容器
      ├─ 顶栏：回合与阶段 · 断线提示 · 生命/时代/金币胶囊 · 倒计时胶囊
      ├─ 主体（横向）
      │  ├─ 左栏 积分榜（196）：8 个座位行，本人金框
      │  ├─ 中区（自适应）
      │  │  ├─ 招募视图
      │  │  │  ├─ 商店（6 槽 + 刷新/升时代/冻结）
      │  │  │  ├─ 我方棋盘（7 个槽位：随从卡或空位）
      │  │  │  └─ 手牌（横向滚动）
      │  │  └─ 战斗视图
      │  │     ├─ 对手英雄条
      │  │     ├─ 对手随从行
      │  │     ├─ 结果横幅
      │  │     ├─ 我方随从行
      │  │     └─ 我方英雄条
      │  └─ 右栏（272）：我的英雄卡 · 英雄技能 · 结束招募 · 战报
      └─ 状态行
```

- 招募视图与战斗视图互斥显示：战斗与结算阶段隐藏招募视图、英雄技能与结束招募，节点保留以便状态恢复。
- 招募中区自上而下固定为商店、我方棋盘、手牌；对手信息只出现在左栏积分榜与战斗视图。
- 左栏、右栏与状态行在两个阶段都常驻。

## 3. 状态清单

| 状态 | 触发 | 表现 | 出口 |
| --- | --- | --- | --- |
| 加载中 | `GameStore.view` 为空 | 状态行「等待对局状态…」 | 收到视图后自动恢复 |
| 空棋盘 | `self.board` 为空 | 棋盘显示 7 个「空位」槽 | 从手牌打出随从 |
| 空商店槽 | 槽位 `cardId` 为 `null` | 按钮显示「空槽」并禁用 | 刷新商店 |
| 空手牌 | `self.hand` 为空 | 手牌区「手牌为空」 | 购买或发现获得 |
| 战斗结果未到 | 战斗阶段 `last_combat` 为空 | 横幅「等待战斗结果…」 | 收到 `CombatResult` 后自动恢复 |
| 错误 | `Error` 消息 | 状态行中文文案，tooltip 为协议原文 | 按错误码留页、回大厅或回登录 |
| 无权限 | 页面没有按角色区分的操作；越权请求由服务端拒绝 | 按错误状态展示，不本地隐藏操作 | 按错误码处理 |
| 离线 | `Net` 未连接 | 顶栏「未连接 · 正在重连」；操作提示「未连接，操作未发出」 | 3 秒自动重连，收到 `Snapshot` 恢复 |
| 视图过期 | `view.turn` 与 `GameStore.turn` 不一致 | 全部操作禁用 | 收到新回合视图 |
| 倒计时归零 | `phaseDeadline` 已过 | 全部招募操作禁用 | 服务端推进阶段 |
| 已结束招募 | `self.recruited` 为真 | 只读；结束按钮显示「已结束招募」，冻结禁用 | 服务端推进阶段 |
| 达到上限 | 商店 6 槽、棋盘 7 槽 | 槽位固定，空位以「空位」占位 | 服务端判定手牌与棋盘上限，拒绝原因进 tooltip |

- 同一时刻只呈现一种主状态；错误文本不覆盖正在显示的待选弹层，弹层由独立的视图门槛控制。

## 4. 数据来源

| 数据 | 来源 |
| --- | --- |
| 回合、阶段、阶段截止时间 | `view.turn`、`view.phase`、`view.phaseDeadline`；`GameStore.phase` 作为空视图时的兜底 |
| 生命、时代、金币、英雄、技能次数 | `view.self` |
| 商店、手牌、棋盘、可用性 | `view.self.shop/hand/board/affordances`，按钮可用性只取 `buyable`、`playable`、`sellable`、`can*` |
| 积分榜与对手信息 | `view.players`（座位号、名字、生命、时代、存活、随从数、英雄、准备、名次、托管） |
| 战斗双方与结果 | `CombatResult`：`ownBoard`、`opponentBoard`、`damage`、`winnerId`、`mirror`；当前对手为 `view.opponentPlayerId` |
| 战报 | `CombatResult` 摘要与 `GameStore.event_log` 最近 8 条 |

- 页面不做本地规则推导：不计算伤害、不推断可用性、不重放 `CombatResult.actions`。
- 重排（左移/右移）与冻结没有对应 affordance，按阶段、`recruited` 与下标本地门控，与服务端拒绝条件一致。

## 5. 适配与性能

- 逻辑尺寸固定 1280×720，窗口按 `canvas_items` 缩放；没有竖屏与超宽布局。
- 三栏中只有中区自适应，左栏与右栏固定宽；棋盘、商店、手牌使用 `HFlowContainer` 自动换行，手牌与积分榜使用滚动容器。
- 每次 `view_changed` 重建左栏与中区，节点上限为 8 个座位、7 个随从、6 个商店槽、10 张手牌；不使用图片与字体资源。

## 6. 关联

- 令牌：[tokens.md](../tokens.md)
- 通用交互：[interactions.md](../interactions.md)
- 组件：[hero-tile.md](../components/hero-tile.md)、[avatar.md](../components/avatar.md)、[pill.md](../components/pill.md)、[button.md](../components/button.md)、[card-panel.md](../components/card-panel.md)
