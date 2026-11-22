# 首局引导

> 状态：已实现
> 入口：`client/pages/game/first_game_guide.gd`（`class_name FirstGameGuide`，`extends CanvasLayer`）
> 验证：`make client-smoke`（`client/tests/ui_smoke_guide.tscn`）

## 用途

对局页默认隐藏的非阻塞引导：玩家点右上角工具组「提示」后，用高亮与手牌上方的提示条介绍购买、出战、结束招募与战斗；不暂停对局、不拦截输入、不强制完成步骤。

## 挂载与接口

- 由 `client/pages/game/game.gd` 在对局页装配完成后创建，`target_root` 指向对局页自身，节点名为 `FirstGameGuide`；进入对局时保持隐藏。
- `show_manual()`：点右上角工具组「提示」时调用，从当前阶段对应的步骤开始展示。
- `set_phase(phase, self_view)`：对局页每次刷新时同步阶段，用于阶段门控。
- `hide()`：对局结束或离开对局时收起。

## 结构层级与节点

```text
FirstGameGuide（CanvasLayer，layer 100）
└─ GuideOverlay（全屏，忽略鼠标）
   ├─ GuideShade0..3（四块压暗矩形，围出高亮窗口，忽略鼠标）
   ├─ GuideBarAnchor（全屏边距容器，忽略鼠标）
   │  └─ GuideBar（手牌上方、宽度随内容自适应并居中的提示条，接收点击）
   │     ├─ GuideTitle（步骤标题）
   │     ├─ GuideStep（「N/4」胶囊）
   │     ├─ GuideBody（步骤说明）
   │     ├─ GuideSkip（跳过）
   │     └─ GuideNext（下一步；最后一步为完成）
```

- 压暗层与锚点容器全部 `MOUSE_FILTER_IGNORE`，只有 `GuideBar` 接收点击；点击、拖拽、倒计时与其它对局操作照常到达底层控件。
- 提示条停靠在贴底手牌上方（下边距 = `card-height` + `space-md` + `space-xl`），为手牌悬停放大的越界留出余量，不遮挡手牌与拖拽。
- 提示条宽度随文案内容自适应并水平居中，不铺满整行；说明文案保持单行，文案变长时提示条随之变宽。
- 高亮矩形取当前步骤目标节点的全局矩形并集，外扩 6px；目标缺失、离树或不可见时只隐藏压暗层，提示条与文案仍可展示。

## 步骤表

| 步骤 | 阶段 | 目标节点 | 文案键 |
| --- | --- | --- | --- |
| 1 商店与金币 | `Recruit` | `ShopPanel`、`ShopRow`、`RecruitGoldTile` | `ui.guide.step1.title` / `ui.guide.step1.body` |
| 2 出战 | `Recruit` | `HandRow`、`BoardRow` | `ui.guide.step2.title` / `ui.guide.step2.body` |
| 3 结束招募 | `Recruit` | `RecruitEndButton`、`RecruitTimerLabel` | `ui.guide.step3.title` / `ui.guide.step3.body` |
| 4 战斗 | `Combat` | `CenterPanel`、`ReportLayer` | `ui.guide.step4.title` / `ui.guide.step4.body` |

- 从「提示」打开时从当前阶段对应的步骤开始，非招募/战斗阶段回退到第 1 步；之后按步骤顺序推进，不按手牌/棋盘条件跳步。
- 引导打开期间阶段变化时切换到新阶段对应的步骤；`GameOver` 时收起。
- `RecruitGoldTile` 是招募河道左端的金币胶囊，供步骤 1 定位；步骤 3 的 `RecruitEndButton` 在倒计时悬停翻转后才可见，高亮取 `RecruitTimerLabel` 与翻转区的并集。

## 打开与关闭

- 进入对局不自动展示；只由右上角工具组「提示」打开。
- 关闭方式为「跳过」或「完成」，只收起覆盖层，不改变对局状态。
- 不读写本机记录，也不在断线重连导致场景重建后恢复引导。

## 禁用场景

- 不用于教学局、强制任务、步骤奖励或服务端进度保存。
- 不在结算页、房间页或大厅挂载。
- 不遮挡商店、手牌、棋盘与结束招募按钮的点击；高亮层不得改为 `STOP`。提示条停靠在手牌上方，不与手牌悬停区域重叠。

## 使用者

- 对局页：[game.md](../pages/game.md)
- 结算页（不挂载）：[result.md](../pages/result.md)
