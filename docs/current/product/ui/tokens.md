# 设计令牌

> 状态：已实现
> 定义：`client/ui/theme.gd`
> 使用：页面与组件通过 `UITheme.metric` 与 `UIControls` 等共享构件取值，不直接写视觉值。

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
| `color-highlight` | `#8fd3ff` | 结算页本人名次行与数据分组标题 |
| `color-combat-tint` | `#ff8c8c` | 战斗受击闪白的目标色 |
| `color-white` | `#ffffff` | 白色文字、描边与提亮基色 |
| `color-black` | `#000000` | 黑色描边与遮罩基色 |
| `color-transparent` | 透明 | StyleBox 无底色、无描边时的占位色 |

## 3. 字体

| 令牌 | 字号 | 用途 |
| --- | --- | --- |
| `font-micro` | 8 | 图标按钮的费用与次数角标 |
| `font-caption` | 11 | 元信息、关键词、标记 |
| `font-body-sm` | 12 | 手牌、胶囊按钮与对手身份行 |
| `font-body` | 14 | 区块标题、按钮 |
| `font-title` | 16 | 随从攻防、英雄名 |
| `font-title-lg` | 18 | 图鉴详情的卡名 |
| `font-title-xl` | 20 | 结算页本人名次与数据分区标题 |
| `font-display` | 22 | 战斗结果横幅、弹层标题 |
| `font-hero` | 28 | 结算页标题 |
| `font-display-lg` | 30 | 登录页主标题 |
| `font-watermark` | 72 | 选英雄覆盖层候选卡无卡面时的水印首字 |

## 4. 间距与圆角

| 令牌 | 取值 | | 令牌 | 取值 |
| --- | --- | --- | --- | --- |
| `space-none` | 0 | | `radius-sm` | 4 |
| `space-2xs` | 2 | | `radius-md` | 8 |
| `space-xs` | 4 | | `radius-lg` | 12 |
| `space-sm` | 6 | | `radius-pill` | 16 |
| `space-md` | 8 | | | |
| `space-md-plus` | 10 | | | |
| `space-lg` | 12 | | | |
| `space-xl` | 16 | | | |
| `space-2xl` | 24 | | | |
| `space-3xl` | 32 | | | |

语义透明度：`opacity-dim` = 0.45，用于被淘汰的座位行、幽灵身份块与禁用图标，经 `UIControls.apply_dim` 或 `alpha("color-white", ...)` 应用；`freeze-overlay-alpha` = 0.22，用于冻结蒙层自身，经 `self_modulate.a` 应用，不向下压暗冰晶与「冻」字；`preview-dim` = 0.7，用于卡面预览压暗；遮罩透明度 `scrim-alpha` = 0.55、`scrim-alpha-friends` = 0.58、`scrim-alpha-strong` = 0.6、`scrim-alpha-gallery` = 0.62、`scrim-alpha-choice` = 0.72，按弹层场景取值；页面容器语义面板 `page-surface-alpha` = 0.92，经 `alpha("color-surface", opacity("page-surface-alpha"))` 应用；大厅装饰透明度 `lobby-rank-ring-alpha` = 0.08（段位卡淡金环）、`lobby-room-ready-dot-halo` = 0.12（已准备圆点光晕），只作装饰，不承载状态。

## 5. 组件与场景度量

| 令牌 | 取值 | 用途 |
| --- | --- | --- |
| `rail-width` | 88 | 左栏座位列宽度 |
| `pill-button-height` | 26 | 胶囊按钮高度；触控端不低于次要操作命中高度 |
| `toolbar-button-height` | 26 | 头部工具行次级按钮高度，桌面与触控端一致，命中区与视觉一致、不外扩；上下内容边距只留 `space-2xs` |
| `status-dialog-width` | 360 | 对局状态模态的提示面板宽度 |
| `river-height` | 44 | 对局页河道行高度，商店与棋盘、双方棋盘之间 |
| `river-gold-width` | 72 | 河道左端金币胶囊宽度；手机 64 |
| `river-gold-height` | 24 | 河道左端金币胶囊高度；手机 20 |
| `hero-row-height` | 84 | 对局页英雄行高度，容纳英雄卡与技能列 |
| `hero-meta-row-height` | 24 | 英雄行被动技能标记所在行的高度，也是被动标记的边长 |
| `hero-card-size` | 84 | 对局页英雄卡边长；售货员与对手英雄复用同一尺寸，方形卡与英雄行同高，头像裁切铺满整卡 |
| `shop-icon-size` | 56 | 商店升时代、刷新、冻结与英雄技能图标按钮边长；与被动行 24、间距 4 合计填满英雄行 84 |
| `recruit-shop-gap` | 6 | 招募组合中商店整体到河道的间距，与战斗组合的舞台间距一致 |
| `opponent-hp-width` | 84 | 战斗阶段对手身份列与血量条宽度，与英雄卡同宽；手机 64 |
| `seat-detail-width` | 236 | 座位悬停详情面板宽度，比通用详情宽一档以容纳一行战报 |
| `detail-minion-width` | 280 | 棋盘与战斗随从详情单列宽度 |
| `detail-minion-width-dense` | 360 | 来源超过 8 行时的双列宽度；只有大于基础宽度时才生效 |
| `avatar-sm` / `-md` / `-lg` | 22 / 30 / 38 | 头像块边长 |
| `avatar-rail` | 44 | 左栏座位行边长兜底值，左栏尺寸未解算时使用；实际边长取栏内宽与人均可用高度的较小值，头像边长再减两侧描边 4，手机兜底 40 |
| `seat-row-gap` | 4 | 座位行间距；手机 2 |
| `portrait-size` | 64 | `CombatIntro` 等仍使用 `GamePortrait` 的头像块边长；售货员与对手英雄改用 `hero-card-size` |
| `hero-detail-width` | 160 | 对局页选英雄覆盖层的候选卡与悬停详情宽度，手机 120；高度由 `UITheme.card_face_height` 按卡面比例计算 |
| `room-hero-detail-width` | 280 | 旧房间英雄详情宽度；当前实现不使用，保留令牌，手机 200 |
| `seat-hp-bar-height` | 6 | 座位行与战斗对手身份块的血量条高度 |
| `seat-detail-hp-bar-height` | 16 | 座位详情里叠「当前/上限」数字的生命条高度，圆角取高度一半 |
| `rank-badge-size` | 18 | 座位行名次圆形角标直径，圆角取直径一半；圆心收在头像内左上 |
| `freeze-crystal-gap` | 14 | 冻结冰晶细线间距 |
| `card-width` / `-height` | 112 / 146 | 手牌、选择浮层候选与通用卡面（含拖拽占位） |
| `minion-card-width` / `-height` | 112 / 146 | 棋盘与商店的随从卡（含拖拽占位）；桌面与 `card-*` 相同，手机放大一档到 78 / 102 |
| `card-cover-inset` | 3 | 卡面美术向内收的距离，保证方形美术的四角落在卡面圆角以内 |
| `card-info-height` | 90 | 有介绍时的卡面信息区基础高度，容纳名称与数值；手牌介绍超出时信息区向上扩展 |
| `card-info-compact-height` | 56 | 无介绍时的卡面信息区高度，只容纳名称与数值 |
| `hand-card-overlap` | 32 | 手牌相邻卡的交叠宽度；槽位步进 = `card-width - hand-card-overlap`，桌面 80、手机 46，十张单行 |
| `hand-hover-scale` | 1.12 | 手牌悬停放大倍数，以底边中点为轴；小数令牌用 `UITheme.factor` 读取，不要用 `metric` |
| `gallery-drawer-width` | 440 | 图鉴详情抽屉宽度；手机取 `-compact` 400，两者都是贴右整高抽屉 |
| `gallery-filter-panel-width` | 240 | 图鉴筛选下拉展开后的条目面板宽度 |
| `gallery-preview-width` | 160 | 图鉴详情封面宽度；手机取 `-compact` 112；高度由 `UITheme.card_face_height` 按卡面比例计算 |
| `gallery-batch-size` | 27 | 图鉴每次滚动加载的卡面张数；手机 12 |
| `lobby-friends-button-width` | 76 | 大厅左栏好友入口宽度；手机 62 |
| `lobby-left-action-height` | 46 | 大厅左栏入口按钮高度；手机 44 |
| `lobby-primary-action-height` | 54 | 大厅匹配主按钮高度；手机 44 |
| `lobby-secondary-action-height` | 40 | 大厅创建 / 加入次按钮高度；手机 44 |
| `lobby-player-avatar` | 52 | 大厅玩家卡头像边长；手机 42 |
| `lobby-friends-drawer-width` | 600 | 好友抽屉宽度，贴左整高并夹紧在 `[360, 视口宽]`；手机 600 |
| `lobby-rank-tier-size` | 38 | 段位卡段位名字号；手机 30 |
| `lobby-rank-score-size` | 15 | 段位卡积分字号；手机 13 |
| `lobby-rank-ring-size` | 220 | 段位卡背景淡金环直径；手机 160 |
| `lobby-rank-ring-width` | 1 | 段位卡背景淡金环线宽 |
| `lobby-room-row-height` | 36 | 创建房间座位行高度；手机 28 |
| `lobby-room-pill-height` | 18 | 座位行房主标记胶囊高度；手机 16 |
| `lobby-room-ready-dot` | 8 | 座位行准备圆点直径；手机 7 |
| `lobby-room-ready-dot-border` | 1 | 准备圆点描边宽度，未准备为空心圆环 |
| `lobby-room-ready-dot-halo-size` | 2 | 已准备圆点的成功色光晕扩散半径 |
| `lobby-room-kick-height` | 24 | 座位行「移除」微按钮高度，按令牌固定、不按触控命中抬升；手机 22 |
| `lobby-room-invite-height` | 32 | 座位列表「邀请好友」与房间码行按钮高度；手机 34 |
| `lobby-room-action-height` | 40 | 创建房间操作按钮高度，按令牌固定、不再按触控命中抬升；手机 44 |
| `lobby-page-margin` | 24 | 大厅页面四边内边距，被安全区包裹在内；手机 14 |
| `aim-arc-width` | 2.5 | 选目标弧线与角标的线宽 |
| `aim-dash` / `aim-gap` | 10 / 6 | 弧线虚线段长与间隔 |
| `aim-dot-radius` | 2.5 | 弧线上流动光点的半径 |
| `aim-flow-speed` | 0.55 | 光点沿弧线循环的速度（每秒圈数） |
| `aim-source-radius` | 12 | 弧线起点圆环半径，起点为技能按钮或入场随从中心 |
| `aim-mark-length` / `aim-mark-inset` | 8 / 3 | 候选卡四角角标的臂长与向内收的距离 |
| `aim-outline-width` | 2 | 命中候选呼吸描边的线宽 |
| `aim-pulse` | 1.2 | 命中候选描边的呼吸周期（秒） |
| `aim-cross` | 6 | 无效落点叉号的半边长 |
| `era-glyph-size` | 14 | 卡面右上角时代象形边长 |

- 瞄准相关令牌都是小数，统一用 `UITheme.factor` 读取；颜色只用语义层的 `color-accent`、`color-text-muted`、`color-danger`。
- 攻防底牌边长不是独立令牌，按内嵌 Label 的字号推导：`边长 = 字号 + 4`（卡面默认 `font-title` 16 → 20；手牌 `font-body` 14 → 18；紧凑手牌 `font-caption` 11 → 15）。

### 5.1 手机基准紧凑值

iPhone 以 960×540 为宽度基准（高度随窗口扩展），对局页通过 `UITheme.stage_metric(name)` 优先取 `<name>-compact`；桌面与 iPad 取上表原值。

| 令牌 | 取值 | 用途 |
| --- | --- | --- |
| `card-width-compact` / `card-height-compact` | 70 / 91 | iPhone 手牌与选择浮层候选卡；保持桌面 112:146 的卡形 |
| `minion-card-width-compact` / `minion-card-height-compact` | 78 / 102 | iPhone 棋盘与商店的随从卡；比手牌大一档，保持 112:146 的卡形 |
| `card-info-height-compact` | 60 | iPhone 有介绍时的卡面信息区（手牌） |
| `avatar-rail-compact` | 40 | iPhone 座位行边长兜底值，左栏尺寸未解算时使用；实际边长按栏内宽与人均可用高度解算 |
| `seat-hp-bar-height-compact` | 4 | iPhone 座位行血量条高度 |
| `seat-row-gap-compact` | 2 | iPhone 座位行间距 |
| `rank-badge-size-compact` | 14 | iPhone 座位名次圆形角标直径 |
| `river-height-compact` | 30 | iPhone 河道行高度 |
| `river-gold-width-compact` | 64 | iPhone 河道金币胶囊宽度 |
| `river-gold-height-compact` | 20 | iPhone 河道金币胶囊高度 |
| `recruit-shop-gap-compact` | 4 | iPhone 商店整体到河道的间距 |
| `hero-row-height-compact` | 76 | iPhone 英雄行高度，由技能列 48 与被动行 24、间距 4 撑开 |
| `hero-card-size-compact` | 64 | iPhone 英雄卡、售货员与对手英雄卡边长；英雄行高于 64 时保持正方形居中 |
| `shop-icon-size-compact` | 48 | iPhone 商店升时代、刷新、冻结与英雄技能图标按钮边长 |
| `opponent-hp-width-compact` | 64 | iPhone 对手身份列与血量条宽度 |
| `pill-button-height-compact` | 24 | iPhone 胶囊按钮高度 |
| `status-dialog-width-compact` | 280 | iPhone 对局状态模态的提示面板宽度 |
| `portrait-size-compact` | 48 | iPhone `CombatIntro` 等仍使用 `GamePortrait` 的头像块边长 |
| `room-hero-detail-width-compact` | 200 | 旧房间英雄详情宽度；当前实现不使用，保留令牌 |
| `hero-detail-width-compact` | 120 | iPhone 选英雄覆盖层候选卡与悬停详情宽度；高度由 `card_face_height` 计算 |
| `detail-minion-width-compact` | 200 | iPhone 棋盘与战斗随从详情单列宽度 |
| `detail-minion-width-dense-compact` | 200 | iPhone 双列档位与基础同宽，来源超过 8 行时实际保持单列 |
| `lobby-friends-button-width-compact` | 62 | iPhone 大厅好友入口宽度 |
| `lobby-left-action-height-compact` | 44 | iPhone 大厅左栏入口按钮高度 |
| `lobby-primary-action-height-compact` | 44 | iPhone 大厅匹配主按钮高度 |
| `lobby-secondary-action-height-compact` | 44 | iPhone 大厅创建 / 加入次按钮高度 |
| `lobby-player-avatar-compact` | 42 | iPhone 大厅玩家卡头像边长 |
| `lobby-friends-drawer-width-compact` | 600 | iPhone 好友抽屉宽度，与桌面同宽并按视口夹紧 |
| `lobby-rank-tier-size-compact` | 30 | iPhone 段位卡段位名字号 |
| `lobby-rank-score-size-compact` | 13 | iPhone 段位卡积分字号 |
| `lobby-rank-ring-size-compact` | 160 | iPhone 段位卡背景淡金环直径 |
| `lobby-room-row-height-compact` | 28 | iPhone 创建房间座位行高度 |
| `lobby-room-pill-height-compact` | 16 | iPhone 座位行房主标记胶囊高度 |
| `lobby-room-ready-dot-compact` | 7 | iPhone 座位行准备圆点直径 |
| `lobby-room-kick-height-compact` | 22 | iPhone 座位行「移除」微按钮高度 |
| `lobby-room-invite-height-compact` | 44 | iPhone 座位列表「邀请好友」与房间码行按钮高度，达到次要命中尺寸 |
| `lobby-room-action-height-compact` | 44 | iPhone 创建房间操作按钮高度，按令牌固定 |
| `lobby-page-margin-compact` | 14 | iPhone 大厅页面四边内边距 |

- 随从详情按 `stats.sources` 行数选宽：不超过 8 行取 `detail-minion-width`；超过 8 行且 `detail-minion-width-dense` 大于当前档位基础宽度时取 dense，否则保持基础宽度。手机两档都是 200，因此来源再多也保持单列。
- 座位行边长取栏内宽与人均可用高度的较小值（960×540 下约 57），不再使用固定行内边距；头像边长 = 行边长 − 4，血量条叠在头像底部内侧、不额外占高；行距 2 让 8 席一屏放下，左栏不出现滚动。
- 大厅手机右栏高度预算为 508（540 − 上下内边距 2×16）：玩家卡 58 + 间距 8 + 座位列表 319 + 间距 8 + 房间操作 112 ≈ 505；座位行取 28，座位头像使用零内边距样式，避免行高被内容边距撑开。

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

- 例外：卡框的透明底（`Color(0,0,0,0)`）与底牌数字的白字黑描边（`Color.WHITE` / `Color.BLACK`）直接用 Godot 常量，不新增语义令牌。
- 新增原子色令牌：`cyan-400`、`teal-400`、`olive-400`、`amber-400`、`orange-400`、`magenta-400`；`blue-400`、`green-400`、`purple-400`、`rose-400`、`red-400`、`slate-400`、`sand-400` 沿用。
- 14 个键的颜色互不重复（`red-400` 与语义 `color-danger` 同值，但中立色不与任何玩法色重复）；卡牌不再使用 `neutral`，该键只作未知玩法的兜底色；未知玩法色彩回退 `snow-500`。
- 取值入口：`UITheme.playstyle_color(id)` 返回玩法色；`playstyle_cover(id)` 返回压暗 0.62 的玩法色，作为卡面与 `HeroPickCard`（选英雄覆盖层候选卡）的封面底色。未知 id 回退 `snow-500`。
- 玩法色用于卡面封面、英雄头像块描边与名字首字、英雄小卡描边，不改变布局与结构。
