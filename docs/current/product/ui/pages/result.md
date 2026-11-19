# 结算页

> 状态：已实现
> 场景：`client/scenes/result.tscn`
> 脚本：`client/scenes/result.gd`
> 验证：`make client-smoke`（`client/tests/ui_smoke.tscn` 的 `_check_result`）

## 1. 目的、入口与返回

- 目的：展示服务端下发的本局名次、本人本局统计与对局种子；数据只读，客户端不做推算。
- 入口：对局页收到 `GameOver` 后由 `GameStore.game_over` 信号触发，导航到 `res://scenes/result.tscn`。
- 返回：
  - `ConfirmButton`（文案「确定」）：房间仍在时先发送 `LeaveRoom` 释放座位，再 `GameStore.reset_room()`、`Net.relogin()` 并导航大厅；房间已在结算期间解散时不发离开请求，直接回大厅。
  - 本页只有一个返回入口，不提供「返回房间」。
- 本页只在进入时构建一次；展示期间房间回到等待区不会清掉已展示的结果。

## 2. 结构层级

```text
背景（全屏，深色）
└─ CenterContainer（全屏）
   └─ 面板（最小宽 760）
      └─ 外边距 24
         └─ 纵向容器（间距 12）
            ├─ ResultScroll（纵向滚动，占满剩余高度，禁用横向滚动）
            │  └─ ResultContent（间距 10）
            │     ├─ 标题「对局结束」
            │     ├─ 本人名次「你的名次：第 N 名」/「你的名次：未记录」
            │     ├─ 分隔线
            │     ├─ RankingList：按 `place` 升序的 8 人名次行，本人行高亮
            │     ├─ 分隔线
            │     ├─ 小标题「本局数据」
            │     ├─ StatsSection：12 个 `StatRow_*`、分组标签与 BoardList
            │     └─ 对局种子行
            └─ ResultActions（固定在滚动区之外）
               └─ ConfirmButton：「确定」，离开房间并回大厅
```

- 内容再长也不会把返回入口挤出屏幕；返回按钮始终固定在面板底部。
- `RankingList` 的每一行文案为「第 N 名 · 玩家名 · 存活 N 回合 · 伤害 N」，缺失字段按 0 展示。
- `StatsSection` 共 12 个 `StatRow_*`：存活回合、最终生命、最终时代、剩余金币、购买、出售、刷新、三连、造成伤害、承受伤害、战斗（胜/平/负三格并排）、英雄技能使用。
- `BoardList` 逐项显示最终阵容「卡名（金色加 ★） 攻击/生命」；为空时只显示一行「无随从」占位。

## 3. 状态清单

| 状态 | 触发 | 表现 | 出口 |
| --- | --- | --- | --- |
| 完整数据 | 收到 `GameOver` 且 `selfStats` 完整 | 名次、12 个统计行、最终阵容、种子全部展示 | 点「确定」回大厅 |
| 字段缺失 | `selfStats` 缺字段或为 `null` | 对应数值按 0 处理；`finalEra` 缺失时 `CardDB.era_name(0)` 回退为 `0`，页面不报错 | 点「确定」回大厅 |
| 最终阵容为空 | `selfStats.board` 为空数组 | `BoardList` 只显示「无随从」占位 | 点「确定」回大厅 |
| 本人名次缺失 | `ranking` 中没有本人条目 | 顶部显示「你的名次：未记录」；名次列表照常展示已有条目 | 点「确定」回大厅 |
| 名次列表缺失 | `ranking` 为空 | `RankingList` 为空，页面其余区域照常展示 | 点「确定」回大厅 |
| 房间已不在 | 结算期间房间被解散 | 结果保留展示并显示关闭原因；点「确定」不发离开请求，直接回大厅 | 回大厅 |
| 离线 | `Net` 未连接 | 本页数据来自 `GameStore`，展示不受影响；返回大厅时重新连接并认证 | 重连后进入大厅 |
| 无权限 | — | 页面没有按角色区分的操作，也不提供数据修改 | — |
| 达到上限 | — | 名次最多 8 行；最终阵容最多 7 项（服务端限制） | — |

## 4. 数据来源

| 数据 | 来源 |
| --- | --- |
| 本人名次 | `GameStore.ranking[]` 中 `playerId` 为本人条目的 `place` |
| 8 人名次列表 | `GameStore.ranking[]`：`place`、`playerId`、`survivedTurns`、`damageDealt` |
| 本人统计 | `GameStore.self_stats`：`place`、`survivedTurns`、`finalHp`、`finalEra`、`remainingGold`、`damageDealt`、`damageTaken`、`buys`、`sells`、`rerolls`、`triples`、`combatWins`、`combatDraws`、`combatLosses`、`heroPowerUses`、`board` |
| 最终阵容 | `self_stats.board[]`：`cardId`、`golden`、`attack`、`health`，按服务端站位顺序 |
| 对局种子 | `GameStore.game_seed` |
| 卡名与时代名 | `CardDB.display_name`、`CardDB.era_name` |

- 所有数值与名次直接取服务端下发值；客户端不做四舍五入、二次计算或重新排序（只按 `place` 升序展示）。缺失字段统一经 `_int_of` 回退为 0，`finalEra` 为 0 时 `CardDB.era_name` 原样返回 `0`。
- `GameStore` 在 `GameOver` 中同时保存 `ranking`、`seed` 与 `selfStats`；本页点「确定」调用 `reset_room()` 连房间一起清空，不再使用 `clear_game_view()`。

## 5. 重连与返回大厅

- 对局已结束、座位仍在房间时，服务端在快照后补发本人 `GameOver`；客户端按同一路径写入 `GameStore`，对局页收到 `game_over` 后进入本页。
- 已回到等待区时，服务端在 `RoomUpdate` 之后补发缓存的上一局 `GameOver`；缓存按座位保存，新一局开始时清空。
- 本页不重放战斗演示，也不请求额外数据；重连后仍通过「确定」离开房间回大厅。

## 6. 适配与性能

- 逻辑尺寸固定 1280×720；面板最小宽 760，内容区纵向滚动，返回按钮不随内容滚动。
- 页面只构建一次；名次最多 8 行、统计 12 行、阵容最多 7 行，不随对局刷新重建。
- 不使用图片与字体资源，颜色、字号与间距来自 `UI` 与 `UITheme`。

## 7. 关联

- 对局页：[game.md](game.md)
- 通用交互：[interactions.md](../interactions.md)
- 组件：[button.md](../components/button.md)、[card-panel.md](../components/card-panel.md)
- 客户端实现：[Godot 客户端](../../../technical/client.md)
