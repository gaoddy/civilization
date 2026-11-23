# 个人主页

> 状态：已实现
> 场景：`client/pages/profile/profile.tscn`
> 脚本：`client/pages/profile/profile.gd`
> 验证：`make client-smoke SMOKE=ranked`（`client/tests/ui_smoke_ranked.tscn`）

## 1. 目的、入口与返回

- 目的：展示玩家的当前段位、当前积分、历史最高段位、前 4 名比例与最近 10 局名次。
- 入口：
  - 大厅顶栏的段位与积分按钮，查看本人主页，返回页为大厅。
  - 排行榜页点击任意一行，查看该玩家主页，返回页为排行榜。
- 返回：页头「返回」按钮或 Android 系统返回，回到进入时的页面。

## 2. 结构层级

```text
软件背景 PageBackground
└─ 外边距 space-2xl
   └─ 纵向根容器（间距 space-lg）
      ├─ 头部：标题「个人主页」 + 弹性空隙 + BackButton「返回」
      └─ ProfileScroll（纵向滚动，禁用横向滚动）
         ├─ ProfileStatus：加载态提示
         └─ ProfileContent
            ├─ 昵称
            ├─ ProfileTier / ProfileRating / ProfileHighestTier
            ├─ ProfileTop4
            ├─ 小标题「最近 10 局」
            └─ ProfileRecent（逐行名次、积分变化与结算后积分）或不可见 / 空态提示
```

- 每条最近战绩为「第 N 名  +Δ 分 → 结算后积分」，负数变化不带加号。
- 最近战绩不可见时只显示「最近战绩仅本人与好友可见」，前 4 名比例显示占位「—」。

## 3. 状态清单

| 状态 | 触发 | 表现 |
| --- | --- | --- |
| 加载中 | 进入页面且尚无目标玩家资料 | 显示「正在加载个人主页」 |
| 本人或好友 | `recentVisible = true` | 展示当前段位、积分、历史最高段位、前 4 名比例与最近 10 局 |
| 非好友 | `recentVisible = false` | 展示基础段位与积分；最近战绩隐藏，前 4 名比例显示占位 |
| 无最近战绩 | `recentVisible = true` 且 `recent` 为空 | 显示「暂无最近战绩」 |
| 收到他人旧结果 | `PlayerProfile.playerId` 与目标不一致 | 保持加载态，不串号展示 |

## 4. 数据来源

| 数据 | 来源 |
| --- | --- |
| 目标玩家 | `GameStore.profile_target`；为空时服务端按本人返回 |
| 昵称、积分、段位、最高段位 | `PlayerProfile.displayName`、`rating`、`tier`、`highestTier` |
| 最近战绩可见性 | `PlayerProfile.recentVisible` |
| 前 4 名比例 | `PlayerProfile.top4Ratio`（0–1），页面按百分数展示 |
| 最近战绩 | `PlayerProfile.recent[]`：`placement`、`ratingDelta`、`ratingAfter`、`finishedAt`（页面展示前三项） |

- 统计口径、可见性与最近 10 局截断全部由服务端决定；客户端只做字段搬运与百分数格式化。
- 段位展示经 `ui.tier.<id>` 取文案；历史最高段位按服务端按当前配置计算的结果下发。

## 5. 适配

- 内容随窗口宽度铺满，头部固定，资料与战绩纵向滚动；手机档位不裁切。
- 资料在每次收到 `PlayerProfile` 时重建；返回页由进入入口写入并保持不变。

## 6. 关联

- 排行榜：[leaderboard.md](leaderboard.md)
- 结算页：[result.md](result.md)
- 通用交互：[interactions.md](../interactions.md)
