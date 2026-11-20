# 客户端 UI

> 状态：已实现
> 代码：`client/ui/theme.gd`（令牌）、`client/ui/ui.gd`（共享构件）、`client/scenes/`
> 验证：`make client-check`、`make client-smoke`

本文记录客户端已实现的 UI 现状：视觉语言、令牌入口、页面与共享组件索引。设计流程与评审规则见项目 runbook（仓库外）。

## 1. 视觉语言

- 深色酒馆基调：近黑的背景、分层卡片、金色强调，状态色只表达结果（成功、危险、信息）。
- 资源约束：工程没有字体与主题资源文件，界面由 Godot 内置控件与 `StyleBoxFlat` 构建；位图资源只有卡牌立绘、头像、卡面与页面背景 `assets/ui/bg/bg_main.png`，清单与回退规则见 [client.md](../../technical/client.md#10-美术素材层)。英雄「头像」无素材时仍是玩法色块加名字首字的排版块。
- 代码绘制：卡框（`CardFrame`）、顶部星级、时代象形（`EraGlyph`）与攻防底牌（`StatPlate`）由代码绘制，不占位图；入口见第 4 节组件表。稀有度不是展示维度，卡面以 `level` 的 1–7 星表达成长档位。
- 深浅色：只有深色一套，不实现浅色主题与主题切换，UI 评审不做浅色检查。

## 2. 令牌与共享构件

| 入口 | 内容 |
| --- | --- |
| `client/ui/theme.gd`（`UITheme`） | 原子 → 语义 → 组件与场景三层令牌；取值见 [tokens.md](tokens.md) |
| `client/ui/ui.gd`（`UI`） | 场景共用控件构造、统一导航入口 `UI.navigate`、文本取用（`UI.enum_text` / `UI.error_text` / `UI.error_tooltip`） |
| `client/ui/icons.gd`（`Icons`） | 代码绘制的矢量图标；清单与绘制约定见 [icons.md](icons.md) |
| `client/text/zh_CN.csv` | 唯一的运行时中文文本表：卡牌与技能名称、效果、枚举展示、错误与页面文案；来源与取用规则见 [client.md](../../technical/client.md)「国际化与文本表」 |

- 页面通过 `UI` 的构件与 `UITheme.metric` 取值，不直接写颜色、字号、间距与圆角。
- `UITheme` 的原子层与语义层按刻度扩展，不因单个页面新增。

## 3. 页面

| 页面 | 场景 | 文档 |
| --- | --- | --- |
| 登录 | `scenes/login.tscn` | 未单独成文 |
| 大厅 | `scenes/lobby.tscn` | 未单独成文 |
| 房间（开局前） | `scenes/room.tscn` | [pages/room.md](pages/room.md) |
| 对局 | `scenes/game.tscn` | [pages/game.md](pages/game.md) |
| 结算 | `scenes/result.tscn` | [pages/result.md](pages/result.md) |
| 卡牌图鉴 | `scenes/gallery.tscn` | [pages/gallery.md](pages/gallery.md) |

## 4. 组件

| 组件 | 入口 | 文档 |
| --- | --- | --- |
| 英雄小卡 | `UI.hero_tile` | [components/hero-tile.md](components/hero-tile.md) |
| 头像块 | `UI.avatar` | [components/avatar.md](components/avatar.md) |
| 数值胶囊 | `UI.pill_label` | [components/pill.md](components/pill.md) |
| 按钮变体 | `UI.styled_button` | [components/button.md](components/button.md) |
| 卡片容器 | `UI.card_panel` | [components/card-panel.md](components/card-panel.md) |
| 卡牌图鉴 | `CardGallery`（`client/scenes/card_gallery.gd`） | [components/card-gallery.md](components/card-gallery.md) |
| 图鉴视图切换 | `GalleryModeBar` | [components/gallery-mode-bar.md](components/gallery-mode-bar.md) |
| 图鉴顶部筛选栏 | `GalleryFilterBar` | [components/gallery-filter-bar.md](components/gallery-filter-bar.md) |
| 图鉴分类目录 | `GalleryToc` | [components/gallery-toc.md](components/gallery-toc.md) |
| 图鉴卡面网格 | `GalleryGrid` | [components/gallery-grid.md](components/gallery-grid.md) |
| 图鉴卡面单元 | `GalleryCardTile` | [components/gallery-card-tile.md](components/gallery-card-tile.md) |
| 图鉴常驻详情 | `GalleryDetail` | [components/gallery-detail.md](components/gallery-detail.md) |
| 首局引导 | `FirstGameGuide` | [components/first-game-guide.md](components/first-game-guide.md) |
| 技能瞄准层 | `GameAimLayer`（`client/scenes/game_aim_layer.gd`） | [components/aim-layer.md](components/aim-layer.md) |

## 5. 交互约定

通用交互、状态与无障碍规则见 [interactions.md](interactions.md)；iOS 端的横屏、缩放与触控规则见 [mobile-adaptation.md](mobile-adaptation.md)；页面特有交互写在页面文档内，同一规则只写一处。
