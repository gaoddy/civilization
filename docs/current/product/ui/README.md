# 客户端 UI

> 状态：已实现
> 代码：`client/ui/theme.gd`（令牌）、`client/ui/ui.gd`（共享构件）、`client/scenes/`
> 验证：`make client-check`、`make client-smoke`

本文记录客户端已实现的 UI 现状：视觉语言、令牌入口、页面与共享组件索引。设计流程与评审规则见项目 runbook（仓库外）。

## 1. 视觉语言

- 深色酒馆基调：近黑的背景、分层卡片、金色强调，状态色只表达结果（成功、危险、信息）。
- 资源约束：仓库没有图片、字体与主题资源文件，界面全部由 Godot 内置控件与 `StyleBoxFlat` 构建；英雄「头像」是玩法色块加名字首字的排版块，不使用也不伪造位图素材。
- 深浅色：工程当前只有深色一套，没有主题切换入口。

## 2. 令牌与共享构件

| 入口 | 内容 |
| --- | --- |
| `client/ui/theme.gd`（`UITheme`） | 原子 → 语义 → 组件与场景三层令牌；取值见 [tokens.md](tokens.md) |
| `client/ui/ui.gd`（`UI`） | 场景共用控件构造、统一导航入口 `UI.navigate`、文本取用（`UI.enum_text` / `UI.error_text` / `UI.error_tooltip`） |
| `client/text/zh_CN.csv` | 唯一的运行时中文文本表：卡牌/技能/Token 名称与效果、枚举展示、错误与页面文案；来源与取用规则见 [client.md](../../technical/client.md)「国际化与文本表」 |

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
| 图鉴章节栏 | `GalleryChapterBar` | [components/gallery-chapter-bar.md](components/gallery-chapter-bar.md) |
| 图鉴分类目录 | `GalleryToc` | [components/gallery-toc.md](components/gallery-toc.md) |
| 图鉴卡面网格 | `GalleryGrid` | [components/gallery-grid.md](components/gallery-grid.md) |
| 图鉴卡面单元 | `GalleryCardTile` | [components/gallery-card-tile.md](components/gallery-card-tile.md) |
| 图鉴常驻详情 | `GalleryDetail` | [components/gallery-detail.md](components/gallery-detail.md) |
| 首局引导 | `FirstGameGuide` | [components/first-game-guide.md](components/first-game-guide.md) |

## 5. 交互约定

通用交互、状态与无障碍规则见 [interactions.md](interactions.md)；页面特有交互写在页面文档内，同一规则只写一处。
