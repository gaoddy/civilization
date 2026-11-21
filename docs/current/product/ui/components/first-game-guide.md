# 首局引导

> 状态：已实现
> 入口：`client/pages/game/first_game_guide.gd`（`class_name FirstGameGuide`，`extends CanvasLayer`）
> 验证：`make client-smoke`（`client/tests/ui_smoke_guide.tscn`）

## 用途

同一玩家编号在本机第一次进入对局时，用非阻塞高亮与手牌上方的提示条介绍购买、出战、结束招募与战斗；不暂停对局、不拦截输入、不强制完成步骤。玩家可在对局顶部工具条点「提示」手动重看。

## 挂载与接口

- 由 `client/pages/game/game.gd` 在对局页装配完成后创建，`target_root` 指向对局页自身，节点名为 `FirstGameGuide`。
- `start()`：优先恢复 `GameStore.guide_step`；否则检查本机 `seen` 记录，首次从第 1 步开始。
- `show_manual()`：从第 1 步开始手动重看，不读写 `seen`。
- `set_phase(phase, self_view)`：对局页每次刷新时同步阶段与本人视图，用于阶段门控和步骤 2/3 的条件跳过。
- `hide()`：对局结束或离开对局时收起，不写 `seen`。

## 结构层级与节点

```text
FirstGameGuide（CanvasLayer，layer 100）
└─ GuideOverlay（全屏，忽略鼠标）
   ├─ GuideShade0..3（四块压暗矩形，围出高亮窗口，忽略鼠标）
   ├─ GuideBarAnchor（全屏边距容器，忽略鼠标）
   │  └─ GuideBar（手牌上方的提示条，接收点击）
   │     ├─ GuideTitle（步骤标题）
   │     ├─ GuideStep（「N/4」胶囊）
   │     ├─ GuideBody（步骤说明）
   │     ├─ GuideSkip（跳过）
   │     └─ GuideNext（下一步；最后一步为完成）
```

- 压暗层与锚点容器全部 `MOUSE_FILTER_IGNORE`，只有 `GuideBar` 接收点击；点击、拖拽、倒计时与其它对局操作照常到达底层控件。
- 提示条停靠在贴底手牌上方（下边距 = `card-height` + `space-md` + `space-xl`），为手牌悬停放大的越界留出余量，不遮挡手牌与拖拽。
- 高亮矩形取当前步骤目标节点的全局矩形并集，外扩 6px；目标缺失、离树或不可见时只隐藏压暗层，提示条与文案仍可展示。

## 步骤表

| 步骤 | 阶段 | 目标节点 | 展示条件 | 文案键 |
| --- | --- | --- | --- | --- |
| 1 商店与金币 | `Recruit` | `ShopPanel`、`ShopRow`、`GoldTile` | 无 | `ui.guide.step1.title` / `ui.guide.step1.body` |
| 2 出战 | `Recruit` | `HandRow`、`BoardRow` | 本人手牌非空 | `ui.guide.step2.title` / `ui.guide.step2.body` |
| 3 结束招募 | `Recruit` | `RecruitEndButton`、`RecruitTimerLabel` | 本人棋盘非空 | `ui.guide.step3.title` / `ui.guide.step3.body` |
| 4 战斗 | `Combat` | `CenterPanel`、`ReportLayer` | 无 | `ui.guide.step4.title` / `ui.guide.step4.body` |

- 步骤 2/3 的条件不满足时自动跳到下一步，不阻塞后续步骤。
- 阶段与步骤不匹配时隐藏引导但保留当前步骤；回到匹配阶段重新显示。
- `GoldTile` 是英雄行技能列顶部的金币格，供步骤 1 定位；步骤 3 的 `RecruitEndButton` 在倒计时悬停翻转后才可见，高亮取 `RecruitTimerLabel` 与翻转区的并集。

## 本地记录与会话恢复

- 文件：`user://first_game_guide.cfg`，`ConfigFile` 格式；段名 `seen`，键为 `playerId`，值为 `true`。
- 读写失败只打印警告，不影响本局：读取失败按「未看过」处理，写入失败只影响下次是否再次自动展示。
- `GameStore.player_id` 为空时不读写、不自动展示。
- 当前步骤写回 `GameStore.guide_step`；`-1` 表示未在引导中。断线重连导致场景重建时，`start()` 从该值恢复当前步骤。
- 跳过或完成时写入 `seen` 并清空 `guide_step`；手动重看不写 `seen`，关闭后同样清空会话步骤。
- 对局结束只收起，不写 `seen`；未完成时下次进入对局仍会从第 1 步自动展示。

## 禁用场景

- 不用于教学局、强制任务、步骤奖励或服务端进度保存。
- 不在结算页、房间页或大厅挂载。
- 不遮挡商店、手牌、棋盘与结束招募按钮的点击；高亮层不得改为 `STOP`。提示条停靠在手牌上方，不与手牌悬停区域重叠。

## 使用者

- 对局页：[game.md](../pages/game.md)
- 结算页（不挂载）：[result.md](../pages/result.md)
