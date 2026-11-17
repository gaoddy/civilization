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

- `UI.section_title(text)`：区块标题，`font-body` 字号、`color-text-dim` 颜色，单行不换行。
- `UI.empty_hint(text)`：居中占位提示，`font-body-sm` 字号、`color-text-muted` 颜色，用于空棋盘、空手牌、无对手。
- 座位行、右栏身份、随从卡等页面私有结构位于各 `client/scenes/game_*` 视图文件（如 `game_players_rail.gd`、`game_side_panel.gd`、`game_card_face.gd`），不进入共享层。
- 候选英雄卡、其他玩家状态行与技能浮层是页面私有结构，位于 `client/scenes/room.gd`，不进入共享层。
