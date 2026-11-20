# 设计令牌

> 状态：已实现
> 定义：`client/ui/theme.gd`
> 使用：页面与组件通过 `UITheme.metric` 与 `UI` 构件取值，不直接写视觉值。

## 1. 分层

| 层 | 内容 | 引用方向 |
| --- | --- | --- |
| 原子（primitive） | 色板原始值、字号、间距、圆角刻度 | 不引用其他层 |
| 语义（semantic） | 按用途命名的颜色、字体、间距、圆角 | 只引用原子层 |
| 组件与场景（metric） | 组件尺寸与页面骨架宽度 | 只引用语义层 |

- 单主题：产品只有深色一套，令牌只维护深色取值，不新增浅色取值。

## 2. 语义颜色

| 令牌 | 取值 | 用途 |
| --- | --- | --- |
| `color-bg` | `#070a0f` | 页面背景、弹层遮罩基色 |
| `color-surface` | `#0c1118` | 区块面板 |
| `color-surface-raised` | `#121925` | 卡片、座位行、商店槽 |
| `color-surface-sunken` | `#1a2331` | 下沉容器 |
| `color-border` | `#243044` | 常规描边 |
| `color-border-strong` | `#33435c` | 可交互卡片描边 |
| `color-text` | `#e9eef7` | 主文本 |
| `color-text-dim` | `#b7c3d6` | 次要文本、状态行 |
| `color-text-muted` | `#8494ac` | 元信息、禁用文本 |
| `color-accent` | `#f0c66b` | 金色强调、本人席位、计时 |
| `color-accent-strong` | `#b98a2f` | 主按钮底色 |
| `color-success` | `#5fce86` | 胜利、已准备 |
| `color-danger` | `#e2666b` | 失败、生命、断线 |
| `color-info` | `#62a8e8` | 时代、中性提示 |

## 3. 字体

| 令牌 | 字号 | 用途 |
| --- | --- | --- |
| `font-micro` | 8 | 图标按钮的费用与次数角标、英雄卡的数值行 |
| `font-caption` | 11 | 元信息、关键词、标记 |
| `font-body-sm` | 12 | 手牌、顶部工具条状态文本与对手身份行 |
| `font-body` | 14 | 区块标题、按钮 |
| `font-title` | 16 | 随从攻防、英雄名 |
| `font-title-lg` | 18 | 图鉴详情的卡名 |
| `font-display` | 22 | 战斗结果横幅、弹层标题 |
| `font-hero` | 28 | 预留，当前页面未使用 |

## 4. 间距与圆角

| 令牌 | 取值 | | 令牌 | 取值 |
| --- | --- | --- | --- | --- |
| `space-2xs` | 2 | | `radius-sm` | 4 |
| `space-xs` | 4 | | `radius-md` | 8 |
| `space-sm` | 6 | | `radius-lg` | 12 |
| `space-md` | 8 | | `radius-pill` | 16 |
| `space-lg` | 12 | | | |
| `space-xl` | 16 | | | |
| `space-2xl` | 24 | | | |
| `space-3xl` | 32 | | | |

语义透明度：`opacity-dim` = 0.45，用于被淘汰的座位行与幽灵身份块，经 `UI.apply_dim` 应用；`freeze-overlay-alpha` = 0.22，用于冻结蒙层自身，经 `self_modulate.a` 应用，不向下压暗冰晶与「冻」字。

## 5. 组件与场景度量

| 令牌 | 取值 | 用途 |
| --- | --- | --- |
| `rail-width` | 88 | 左栏座位列宽度 |
| `top-bar-height` | 28 | 对局页顶部工具条高度 |
| `river-height` | 32 | 对局页河道行高度，商店与棋盘、双方棋盘之间 |
| `hero-row-height` | 84 | 对局页英雄行高度，容纳英雄卡与金币、技能列 |
| `hero-meta-row-height` | 24 | 英雄行金币与被动技能标记所在行的高度，也是被动标记的边长 |
| `hero-card-width` / `-height` | 64 / 84 | 英雄卡尺寸，与 `card-width` 同宽高比 |
| `hero-cover-height` | 40 | 英雄卡玩法色封面高度 |
| `hero-name-height` | 14 | 英雄卡名字行高度 |
| `hero-stats-height` | 30 | 英雄卡底部时代与生命行高度 |
| `shop-icon-size` | 40 | 商店升时代、刷新、冻结与英雄技能图标按钮边长 |
| `recruit-shop-gap` | 18 | 招募组合中商店操作与商店整体到河道的间距 |
| `opponent-hp-width` | 360 | 战斗阶段对手身份块与血量条宽度 |
| `report-width` | 236 | 左栏战报浮层宽度 |
| `avatar-sm` / `-md` / `-lg` | 22 / 30 / 38 | 头像块边长 |
| `avatar-rail` | 44 | 左栏座位行头像边长 |
| `portrait-size` | 64 | 售货员与对手英雄头像块边长 |
| `seat-hp-bar-height` | 6 | 座位行与战斗对手身份块的血量条高度 |
| `rank-badge-size` | 18 | 座位行名次角标边长 |
| `freeze-crystal-gap` | 14 | 冻结冰晶细线间距 |
| `hero-tile-width` | 150 | 英雄小卡宽度（大厅与房间的英雄选择） |
| `card-width` / `-height` | 112 / 146 | 商店、棋盘、手牌与战斗只读卡（含拖拽占位） |
| `card-cover-inset` | 3 | 卡面美术向内收的距离，保证方形美术的四角落在卡面圆角以内 |
| `card-info-height` | 90 | 有介绍时的卡面信息区基础高度，容纳名称与数值；手牌介绍超出时信息区向上扩展 |
| `card-info-compact-height` | 56 | 无介绍时的卡面信息区高度，只容纳名称与数值 |
| `hand-card-overlap` | 32 | 手牌相邻卡的交叠宽度；槽位步进 = `card-width - hand-card-overlap` = 80，十张单行 |
| `hand-hover-scale` | 1.12 | 手牌悬停放大倍数，以底边中点为轴；小数令牌用 `UITheme.factor` 读取，不要用 `metric` |
| `gallery-toc-width` | 140 | 图鉴分类目录宽度；手机 120 |
| `gallery-detail-width` | 280 | 图鉴常驻详情宽度 |
| `gallery-preview-width` / `-height` | 200 / 260 | 图鉴详情卡面美术图尺寸；手机为 160 / 208 |
| `gallery-page-size` | 18 | 图鉴每页卡面张数（6 列 × 3 行）；手机 8（4 列 × 2 行） |
| `gallery-grid-columns` | 6 | 图鉴卡面网格列数；手机 4 |
| `aim-arc-width` | 2.5 | 选目标弧线与角标的线宽 |
| `aim-dash` / `aim-gap` | 10 / 6 | 弧线虚线段长与间隔 |
| `aim-dot-radius` | 2.5 | 弧线上流动光点的半径 |
| `aim-flow-speed` | 0.55 | 光点沿弧线循环的速度（每秒圈数） |
| `aim-source-radius` | 12 | 弧线起点圆环半径，起点为技能按钮或入场随从中心 |
| `aim-mark-length` / `aim-mark-inset` | 8 / 3 | 候选卡四角角标的臂长与向内收的距离 |
| `aim-outline-width` | 2 | 命中候选呼吸描边的线宽 |
| `aim-pulse` | 1.2 | 命中候选描边的呼吸周期（秒） |
| `aim-cross` | 6 | 无效落点叉号的半边长 |

- 瞄准相关令牌都是小数，统一用 `UITheme.factor` 读取；颜色只用语义层的 `color-accent`、`color-text-muted`、`color-danger`。

### 5.1 手机基准紧凑值

iPhone 以 960×540 为基准，对局页通过 `UITheme.stage_metric(name)` 优先取 `<name>-compact`；桌面与 iPad 取上表原值。

| 令牌 | 取值 | 用途 |
| --- | --- | --- |
| `card-width-compact` / `card-height-compact` | 84 / 92 | iPhone 商店、棋盘、手牌与战斗只读卡 |
| `card-info-height-compact` | 60 | iPhone 有介绍时的卡面信息区（手牌） |
| `avatar-rail-compact` | 40 | iPhone 座位行头像边长 |
| `seat-hp-bar-height-compact` | 4 | iPhone 座位行血量条高度 |
| `seat-row-gap-compact` | 2 | iPhone 座位行间距 |
| `rank-badge-size-compact` | 14 | iPhone 座位名次角标边长 |
| `river-height-compact` | 24 | iPhone 河道行高度 |
| `recruit-shop-gap-compact` | 10 | iPhone 商店整体到河道的间距 |
| `shop-actions-height-compact` | 52 | iPhone 商店操作行高度 |
| `hero-row-height-compact` | 64 | iPhone 英雄行高度 |
| `top-bar-height-compact` | 26 | iPhone 顶部工具条高度 |
| `portrait-size-compact` | 48 | iPhone 售货员与对手英雄头像边长 |

- 紧凑座位行（头像 40 + 血量条 4 + 行内边距 4 + 行距 2）让 8 席在 960×540 下一屏放下，左栏不出现滚动。

## 6. 玩法色

`UITheme.PLAYSTYLE_COLOR` 共 14 个键：13 个玩法与 1 个 Token/缺省兜底色，全部是原子色令牌。

| 玩法 | 名称 | 令牌 | 取值 |
| --- | --- | --- | --- |
| `mammal` | 哺乳纲 | `blue-400` | `#62a8e8` |
| `bird` | 鸟纲 | `cyan-400` | `#3fd0e0` |
| `reptile` | 爬行纲 | `green-400` | `#5fce86` |
| `fish` | 鱼纲 | `teal-400` | `#3fc7ac` |
| `insect` | 昆虫纲 | `lime-400` | `#a3cf61` |
| `cephalopod` | 头足纲 | `purple-400` | `#b28ce6` |
| `cnidarian` | 水螅纲 | `rose-400` | `#d9799b` |
| `plant` | 植物界 | `olive-400` | `#9aa83c` |
| `fungus` | 真菌界 | `amber-400` | `#e0b04a` |
| `bacteria` | 细菌域 | `orange-400` | `#e57c4a` |
| `protozoa` | 原生生物界 | `magenta-400` | `#d979c4` |
| `virus` | 病毒 | `red-400` | `#e2666b` |
| `machine` | 机械 | `slate-400` | `#93a6bd` |
| `neutral` | 中立（仅缺省兜底，不是卡牌分类） | `sand-400` | `#cdb98c` |

- 新增原子色令牌：`cyan-400`、`teal-400`、`olive-400`、`amber-400`、`orange-400`、`magenta-400`；`blue-400`、`green-400`、`purple-400`、`rose-400`、`red-400`、`slate-400`、`sand-400` 沿用。
- 14 个键的颜色互不重复（`red-400` 与语义 `color-danger` 同值，但中立色不与任何玩法色重复）；卡牌不再使用 `neutral`，该键只作未知玩法的兜底色；未知玩法色彩回退 `snow-500`。
- 取值入口：`UITheme.playstyle_color(id)` 返回玩法色；`playstyle_cover(id)` 返回压暗 0.62 的玩法色，作为卡面与房间英雄候选卡的封面底色。未知 id 回退 `snow-500`。
- 玩法色用于卡面封面、英雄头像块描边与名字首字、英雄小卡描边，不改变布局与结构。
