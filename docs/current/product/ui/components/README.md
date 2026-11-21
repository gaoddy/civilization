# 组件索引

> 状态：已实现
> 入口：`client/ui/ui.gd`

| 组件 | 入口 | 文档 |
| --- | --- | --- |
| 英雄小卡 | `UI.hero_tile` | [hero-tile.md](hero-tile.md) |
| 头像块 | `UI.avatar` | [avatar.md](avatar.md) |
| 数值胶囊 | `UI.pill_label` | [pill.md](pill.md) |
| 按钮 | `UI.styled_button` | [button.md](button.md) |
| 卡片容器 | `UI.card_panel` | [card-panel.md](card-panel.md) |
| 区块标题 | `UI.section_title` | 本文件 |
| 空态提示 | `UI.empty_hint` | 本文件 |
| 卡牌图鉴 | `CardGallery`（`client/scenes/card_gallery.gd`） | [card-gallery.md](card-gallery.md) |
| 图鉴视图切换 | `GalleryModeBar` | [gallery-mode-bar.md](gallery-mode-bar.md) |
| 图鉴顶部筛选栏 | `GalleryFilterBar` | [gallery-filter-bar.md](gallery-filter-bar.md) |
| 图鉴分类目录 | `GalleryToc` | [gallery-toc.md](gallery-toc.md) |
| 图鉴卡面网格 | `GalleryGrid` | [gallery-grid.md](gallery-grid.md) |
| 图鉴卡面单元 | `GalleryCardTile` | [gallery-card-tile.md](gallery-card-tile.md) |
| 图鉴常驻详情 | `GalleryDetail` | [gallery-detail.md](gallery-detail.md) |
| 首局引导 | `FirstGameGuide`（`client/scenes/first_game_guide.gd`） | [first-game-guide.md](first-game-guide.md) |
| 技能瞄准层 | `GameAimLayer`（`client/scenes/game_aim_layer.gd`） | [aim-layer.md](aim-layer.md) |

- `UI` 之外的图鉴组件按 `class_name` 直接实例化，位于 `client/scenes/`，供大厅图鉴页与对局覆盖层共用；它们的卡牌数据仍来自 `CardDB`。
- `FirstGameGuide` 由对局页实例化并挂载，只服务对局页；高亮层忽略鼠标，提示条自身接收点击。
- `GameAimLayer` 由对局页实例化并挂载，承接技能与战吼的选目标手势；全屏但忽略鼠标，只有瞄准期间临时接管按下与抬起。

- `UI.section_title(text)`：区块标题，`font-body` 字号、`color-text-dim` 颜色，单行不换行。
- `UI.empty_hint(text)`：居中占位提示，`font-body-sm` 字号、`color-text-muted` 颜色，用于空棋盘、空手牌、无对手。
- 座位行、商店、河道、英雄行、对手身份、手牌行、选择浮层与随从卡等页面私有结构位于各 `client/scenes/game_*` 视图文件（如 `game_players_rail.gd`、`game_shop_view.gd`、`game_river.gd`、`game_hero_row.gd`、`game_opponent_view.gd`、`game_hand_row.gd`、`game_choice_overlay.gd`），不进入共享层。
- 卡面按场景拆成独立组件：`shop_card_face.gd`、`hand_card_face.gd`、`board_card_face.gd`（棋盘与战斗只读共用）与 `gallery_card_face.gd`；共享绘制与文本在 `card_face_base.gd`、`card_text.gd`。
- 房间候选英雄卡位于 `client/scenes/room_hero_card.gd`（`RoomHeroCard`），悬停详情层位于 `client/scenes/room_hero_detail_layer.gd`（`RoomHeroDetailLayer`）；其他玩家状态行与邀请行仍是 `client/scenes/room.gd` 的页面私有结构。它们只服务房间页，不进入共享层。
