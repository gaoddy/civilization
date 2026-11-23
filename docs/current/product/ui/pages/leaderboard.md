# 排行榜页

> 状态：已实现
> 场景：`client/pages/leaderboard/leaderboard.tscn`
> 脚本：`client/pages/leaderboard/leaderboard.gd`
> 验证：`make client-smoke SMOKE=ranked`（`client/tests/ui_smoke_ranked.tscn`）

## 1. 目的、入口与返回

- 目的：展示服务端实时下发的全服积分前 100 名（名次、昵称、段位、积分），点击任意一行进入该玩家的个人主页。
- 入口：大厅底栏「排行榜」按钮，导航到 `pages/leaderboard/leaderboard.tscn`。
- 返回：页头「返回大厅」按钮或 Android 系统返回，回到大厅。

## 2. 结构层级

```text
软件背景 PageBackground
└─ 外边距 space-2xl
   └─ 纵向根容器（间距 space-lg）
      ├─ 头部：标题「全服排行榜」 + 弹性空隙 + BackButton「返回大厅」
      ├─ LeaderboardStatus：加载 / 空态提示
      └─ LeaderboardScroll（纵向滚动，禁用横向滚动）
         └─ LeaderboardList：每个上榜玩家一个 Button 行
```

- 行文案为「第 N 名  昵称 · 段位 · M 分」；点击行调用 `GameStore.open_profile()` 并以本页为返回页导航个人主页。
- 页面进入时先发 `RequestLeaderboard`；收到 `Leaderboard` 后重建列表，不在本地排序或算分。

## 3. 状态清单

| 状态 | 触发 | 表现 |
| --- | --- | --- |
| 加载中 | 进入页面且尚未收到 `Leaderboard` | 列表为空，显示「正在加载排行榜」 |
| 有数据 | 收到非空 `Leaderboard` | 按服务端顺序逐行展示，状态提示隐藏 |
| 空榜 | 收到空 `Leaderboard` | 显示「暂无排行数据」 |
| 请求失败 | 服务端返回错误 | 保持加载态并沿用既有错误提示；页面结构不变 |

## 4. 数据来源

| 数据 | 来源 |
| --- | --- |
| 榜单条目 | `GameStore.leaderboard`，来自 `Leaderboard.entries` |
| 名次、昵称、段位、积分 | 条目字段 `rank`、`displayName`、`tier`、`rating`；段位经 `ui.tier.<id>` 取展示名 |
| 目标玩家 | 条目字段 `playerId` |

- 排序、前 100 截断、段位与积分全部由服务端决定，客户端只做字段搬运。
- 缺失数值按 0 处理；`tier` 缺键时回退为段位 id。

## 5. 适配

- 内容随窗口宽度铺满，头部固定，列表纵向滚动；手机档位不裁切。
- 只构建一次头部，列表在每次收到 `Leaderboard` 时重建。

## 6. 关联

- 个人主页：[profile.md](profile.md)
- 结算页：[result.md](result.md)
- 通用交互：[interactions.md](../interactions.md)
