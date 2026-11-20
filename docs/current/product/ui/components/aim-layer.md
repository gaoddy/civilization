# 技能瞄准层

> 状态：已实现
> 入口：`client/scenes/game_aim_layer.gd`（`class_name GameAimLayer`，`extends Control`）
> 验证：`make client-smoke`（`client/tests/ui_smoke.gd` 的目标待选、放弃、技能瞄准与效果牌拖拽用例）

## 用途

选目标不再弹出候选列表：按住英雄技能按钮或战吼待选中的任意位置，从技能出处或入场随从拉出一条指向指针的弧线，松手即对指针指向的候选生效。效果牌拖拽时复用同一套视觉，只做提示、不参与提交。

## 挂载与接口

- 由 `client/scenes/game.gd` 在 `ChooserOverlay` 之前创建并挂载，节点名 `AimLayer`，全屏铺满、`MOUSE_FILTER_IGNORE`，本身不拦截鼠标。
- `begin_skill(source_rect, candidates)`：按下的主动技能按钮开启技能模式，起点取该按钮全局矩形；候选为空时不开。
- `begin_pending(source_minion_id, candidates)`：服务端下发目标待选时开启待选模式，起点按 `minion_id` 找到棋盘卡；同一来源与同一候选重复调用不重置手势。
- `clear()`：关闭瞄准，保留的只是一次操作，不是页面状态。
- `aiming()` / `skill_active()` / `pending_active()`：页面据此决定悬停详情是否暂停。
- 信号：`target_chosen(target)` 松手命中候选；`target_abandoned` 待选模式松手落空或按 `Esc`；`aim_cancelled` 技能模式松手落空或按 `Esc`。

## 模式

| 模式 | 开启 | 输入 | 松手命中 | 松手落空 / `Esc` |
| --- | --- | --- | --- | --- |
| `SKILL` | 英雄技能按钮按下，技能 `requiresTarget` 且候选非空 | 本层接收并吞掉按下、移动、抬起 | 发 `target_chosen` | 发 `aim_cancelled`，不发意图 |
| `PENDING` | 视图队首待选 `kind = target` | 按下开始瞄准；桌面端未按下时弧线跟随指针 | 发 `target_chosen` | 发 `target_abandoned` |
| `DRAG` | 检测到 Godot 手牌拖拽且数据带 `targets` | 只读拖拽数据，不接收输入 | 由 `BoardDrop` / `HandCard` 原有落点提交 | 拖拽取消后收起视觉 |
| `NONE` | 默认 / 关闭 | 不绘制、不接收输入 | — | — |

- 技能与待选模式在按下与抬起时调用 `get_viewport().set_input_as_handled()`，避免触发底层按钮与拖拽。
- 鼠标与触控分别处理：触控端用 `InputEventScreenTouch` / `InputEventScreenDrag`，以触点编号跟踪；坐标统一换算到画布。技能模式由按钮按下开启，本层没见到起始触点，第一段 `ScreenDrag` 接管该触点编号，弧线随手指移动。
- `Esc` 在技能模式取消、待选模式放弃，与松手落空同义。

## 候选命中

- 候选是服务端下发的 `{zone, id}`，区域取 `board`（棋盘随从的 `minionId`）、`hand`（手牌实例编号）、`shop`（商店实例编号）。
- 命中的唯一依据是页面内带 `target_zone` / `target_id` 元数据的可见控件：`board_card_face.gd`、`game_hand_row.gd`、`game_shop_view.gd` 在建卡时写入；拖拽预览不在页面树下，不会被命中。
- 命中判定集中在本组件：按候选查控件、取全局矩形，指针落在矩形内即为命中；按住期间逐帧重判，视图重建移动候选后仍然正确。

## 绘制

| 元素 | 规则 |
| --- | --- |
| 弧线 | 二次贝塞尔，控制点在中垂线向上弯曲，弯曲比例 `aim-bend` = 0.22；金色虚线（`aim-dash` / `aim-gap`），宽度 `aim-arc-width` |
| 流动光点 | 沿弧线等距 3 个，按 `aim-flow-speed` 循环前进，半径 `aim-dot-radius` |
| 起点圆环 | 技能按钮或入场随从中心，半径 `aim-source-radius` |
| 候选角标 | 每张候选卡四角向内收 `aim-mark-inset` 的 L 形角标，长 `aim-mark-length`；进入瞄准即全部显示 |
| 命中描边 | 指针所在候选外框呼吸描边，周期 `aim-pulse`，宽度 `aim-outline-width` |
| 无效终点 | 按住但未命中时候选弧线转 `color-text-muted`，指针处画 `color-danger` 叉（`aim-cross`） |

- 颜色只用 `color-accent`、`color-text-muted`、`color-danger`；尺寸与节奏全部取令牌，见 [tokens.md](../tokens.md) §5。
- 弧线只在本层 `_draw()` 内绘制，不建子节点；瞄准期间每帧 `queue_redraw()`。

## 与页面的协作

- 技能：`GameHeroRow` 某个主动技能按钮的 `button_down` 带技能 id 触发页面 `_on_hero_power_down(skill_id)`，页面按 id 取对应 `heroPowers` 条目，判断 `requiresTarget`、候选非空与门控后进入瞄准；`pressed` 对需要目标的技能不再发意图。
- 战吼待选：页面每次刷新按队首待选调用 `begin_pending`；视图过期、倒计时归零、已结束招募、非招募阶段或离线时关闭。
- 提交都在页面里完成：技能模式补 `target` 发 `UseHeroPower`；待选模式发 `ChooseTarget` 或 `AbandonTarget`；取消只清本地待发意图。
- 瞄准期间页面暂停悬停详情（`GameDetailLayer.blocked`），关闭图鉴；发现与进化待选仍走 `ChooserOverlay`。

## 边界

- 不负责候选计算与下发，不自行从棋盘、手牌或商店推导候选。
- 不改效果牌与随从牌的落点提交路径；`DRAG` 模式只画视觉。
- 不做自动改选：提交前失效的目标由服务端拒绝，待选保留并允许重新瞄准。
- Godot 4 没有读取系统「减少动态效果」的接口，弧线与光点按固定节奏绘制。

## 使用者

- 对局页：[game.md](../pages/game.md)
- 通用交互：[interactions.md](../interactions.md) §3.1
