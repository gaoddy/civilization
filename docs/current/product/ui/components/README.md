# 组件索引

> 状态：已实现
> 入口：`client/ui/`（跨页共享）与 `client/pages/<页面>/`（单页）

| 组件 | 入口 | 文档 |
| --- | --- | --- |
| 头像块 | `Avatar.create`（`client/ui/avatar.gd`） | [avatar.md](avatar.md) |
| 数值胶囊 | `UIControls.pill_label`（`client/ui/controls.gd`） | [pill.md](pill.md) |
| 按钮 | `UIControls.styled_button`（`client/ui/controls.gd`） | [button.md](button.md) |
| 卡片容器 | `UIControls.card_panel`（`client/ui/controls.gd`） | [card-panel.md](card-panel.md) |
| 可选项框 | `SelectableBox`（`client/ui/selectable_box.gd`） | [selectable-box.md](selectable-box.md) |
| 弹出层外壳 | `PopupLayer`（`client/ui/popup_layer.gd`） | [popup-layer.md](popup-layer.md) |
| 区块标题 | `UIControls.section_title` | 本文件 |
| 空态提示 | `UIControls.empty_hint` | 本文件 |
| 血量条 | `HpBar.create`（`client/ui/hp_bar.gd`） | 本文件 |
| 名次角标 | `RankBadge.create`（`client/ui/rank_badge.gd`） | 本文件 |
| 技能列表 | `SkillList.build`（`client/ui/skill_list.gd`） | 本文件 |
| 卡牌图鉴 | `CardGallery`（`client/ui/card_gallery.gd`） | [card-gallery.md](card-gallery.md) |
| 图鉴视图切换 | `GalleryModeBar` | [gallery-mode-bar.md](gallery-mode-bar.md) |
| 图鉴顶部筛选栏 | `GalleryFilterBar` | [gallery-filter-bar.md](gallery-filter-bar.md) |
| 图鉴分类目录 | `GalleryToc` | [gallery-toc.md](gallery-toc.md) |
| 图鉴卡面网格 | `GalleryGrid` | [gallery-grid.md](gallery-grid.md) |
| 图鉴卡面单元 | `GalleryCardTile` | [gallery-card-tile.md](gallery-card-tile.md) |
| 图鉴常驻详情 | `GalleryDetail` | [gallery-detail.md](gallery-detail.md) |
| 详情层 | `DetailLayer`（`client/ui/detail_layer.gd`） | 本文件 |
| 首局引导 | `FirstGameGuide`（`client/pages/game/first_game_guide.gd`） | [first-game-guide.md](first-game-guide.md) |
| 技能瞄准层 | `GameAimLayer`（`client/pages/game/game_aim_layer.gd`） | [aim-layer.md](aim-layer.md) |

- 共享层按 `class_name` 直接实例化：跨两个及以上页面复用的组件放 `client/ui/`，只服务一个页面的组件放 `client/pages/<页面>/`；页面只做骨架与装配，不内联展示区块。
- 卡牌图鉴与 `Gallery*` 组件由 `client/ui/` 承载，供大厅图鉴页与对局覆盖层共用；它们的卡牌数据仍来自 `CardDB`。
- `FirstGameGuide` 由对局页实例化并挂载，只服务对局页；高亮层忽略鼠标，提示条自身接收点击。
- `GameAimLayer` 由对局页实例化并挂载，承接技能与战吼的选目标手势；全屏但忽略鼠标，只有瞄准期间临时接管按下与抬起。
- `DetailLayer` 统一承载商店、座位、售货员、英雄与随从详情；选英雄页的 `RoomHeroDetailLayer` 继承它，只覆盖内容与定位。
- 长按判定由 `LongPressTracker`（`client/ui/long_press.gd`）承担，详情层与选英雄页共用同一套 450ms 阈值与 8 画布单位的位移取消距离。
- 可选项框由 `SelectableBox`（`client/ui/selectable_box.gd`）承担内容承载、常态 / 悬停 / 选中三态描边与框内点击；`RoomHeroCard` 与 `GalleryCardTile` 继承它，分别使用外扩与内联两种选中描边。
- 弹出层外壳由 `PopupLayer`（`client/ui/popup_layer.gd`）承担遮罩、居中 / 锚点 / 四向贴边定位与点击遮罩关闭；`InvitePopup`、`FriendsPanel`、`GameChoiceOverlay`、`CombatIntro` 与 `GalleryCategoryPanel` 继承它，内容仍由各自构建。

- `UIControls.section_title(text)`：区块标题，`font-body` 字号、`color-text-dim` 颜色，单行不换行。
- `UIControls.empty_hint(text)`：居中占位提示，`font-body-sm` 字号、`color-text-muted` 颜色，用于空棋盘、空手牌、无对手。
- `HpBar.create(value, cap, width, corner_radius = 0)`：座位与对手身份使用的血量条，按 `value / cap` 比例填充，不显示数字；`corner_radius` 大于 0 时按圆角绘制，座位行用它做叠在头像底部内侧的胶囊。
- `RankBadge.create(rank)`：名次角标，`rank` 为 `null` 时不渲染。
- `SkillList.build(parent, skill_ids)`：英雄技能列表，按 `skills` 顺序逐条展示技能名、触发类型、完整文本与主动技能费用/次数。
- 座位行、商店、河道、英雄行、对手身份、手牌行、选择浮层与随从卡等页面私有结构位于 `client/pages/game/` 的 `game_*` 视图文件（如 `game_players_rail.gd`、`game_shop_view.gd`、`game_river.gd`、`game_hero_row.gd`、`game_opponent_view.gd`、`game_hand_row.gd`、`game_choice_overlay.gd`），不进入共享层。
- 卡面按场景拆成独立组件：`shop_card_face.gd`、`hand_card_face.gd`、`board_card_face.gd`（棋盘与战斗只读共用）与 `gallery_card_face.gd`；共享绘制与文本在 `client/ui/card_face_base.gd`、`client/ui/card_text.gd`。
- 房间候选英雄卡位于 `client/pages/room/room_hero_card.gd`（`RoomHeroCard`），悬停详情层位于 `client/pages/room/room_hero_detail_layer.gd`（`RoomHeroDetailLayer`）；成员行、候选列表、邀请行与底部操作区分别为 `room_player_row.gd`、`room_candidate_list.gd`、`room_invite_row.gd`、`room_action_bar.gd`。它们只服务房间页，不进入共享层。
