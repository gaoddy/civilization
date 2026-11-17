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
| `font-caption` | 11 | 元信息、关键词、标记 |
| `font-body-sm` | 12 | 右栏身份与属性行、手牌、状态行 |
| `font-body` | 14 | 区块标题、按钮 |
| `font-title` | 16 | 随从攻防、英雄名 |
| `font-title-lg` | 18 | 右栏回合数 |
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
| `side-width` | 272 | 右栏宽度 |
| `report-width` | 236 | 左栏战报浮层宽度 |
| `avatar-sm` / `-md` / `-lg` | 22 / 30 / 38 | 头像块边长 |
| `avatar-rail` | 44 | 左栏座位行头像边长 |
| `portrait-size` | 200 | 售货员与我方英雄大图宽度 |
| `portrait-height` | 150 | 售货员与我方英雄大图高度 |
| `seat-hp-bar-height` | 6 | 座位行血量条高度，与 `avatar-rail` 同宽；右栏对手身份块的血量条复用该高度 |
| `rank-badge-size` | 18 | 座位行名次角标边长 |
| `freeze-crystal-gap` | 14 | 冻结冰晶细线间距 |
| `hero-tile-width` | 150 | 英雄小卡宽度（右栏按栏宽覆盖） |
| `card-width` / `-height` | 112 / 146 | 商店、棋盘、手牌与战斗只读卡（含空位与拖拽占位） |
| `combat-banner-height` | 30 | 战斗视图结果横幅的固定占位高度；横幅出现不推动上下两行 |
| `card-info-height` | 90 | 有介绍时的卡面信息区高度，容纳两行介绍、名称与数值 |
| `card-info-compact-height` | 56 | 无介绍时的卡面信息区高度，只容纳名称与数值 |
| `hand-card-overlap` | 32 | 手牌相邻卡的交叠宽度；槽位步进 = `card-width - hand-card-overlap` = 80，十张单行 |
| `hand-hover-scale` | 1.12 | 手牌悬停放大倍数，以底边中点为轴；小数令牌用 `UITheme.factor` 读取，不要用 `metric` |

## 6. 部落色

| 部落 | 令牌 | 取值 |
| --- | --- | --- |
| 人类 | `blue-400` | `#62a8e8` |
| 学者 | `purple-400` | `#b28ce6` |
| 机械 | `slate-400` | `#93a6bd` |
| 生物 | `green-400` | `#5fce86` |
| 病毒 | `rose-400` | `#d9799b` |
| 植物 | `lime-400` | `#a3cf61` |
| 通用 | `sand-400` | `#cdb98c` |
| 未取到部落 | `color-border-strong` | `#33435c` |

部落色只用于头像块的描边与名字首字，不改变布局与结构。
