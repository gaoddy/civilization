# 所有方形头像统一基于 AvatarFrame

> 状态：已实施
> 需求归属：v0.1
> 产品依据：用户要求「所有的方形头像都要使用 AvatarFrame」；统一方形头像的圆角裁切与描边层次

## 1. 改动边界

- 只改客户端 UI：座位行头像（`Avatar`）与对局方形头像块（`GamePortrait`）改为基于共享头像框；不改页面布局、接口与交互。
- 不改服务端、协议与卡牌数据；不改卡面展示（112:146）与圆形头像。
- 修复 `AvatarFrame` 的裁切模式：`CLIP_CHILDREN_ONLY` 只裁子节点、不画面板自身，导致无头像素材时玩法色封面不显示；改为 `CLIP_CHILDREN_AND_DRAW`。

## 2. 组件接口

### `AvatarFrame`

- 新增 `clipped_content() -> Control`：位于 `AvatarFrameClip` 内、头像之上、描边之下，随头像一起裁到圆角；满宽贴底的名字条等需要保留圆角的内容挂这里。
- 节点层序：`AvatarFrameClip`（`clip_children = CLIP_CHILDREN_AND_DRAW`，内含 `AvatarFrameArt`、`AvatarFrameGlyph`、`AvatarFrameClippedContent`）→ `AvatarFrameBorder` → `AvatarFrameContent`。
- 其余接口（`configure`、`set_art`、`set_glyph`、`content()`）不变。

### `Avatar`

- `class_name Avatar extends AvatarFrame`，保留 `static func create(hero_id, size) -> Avatar`。
- 节点名保持 `Avatar`；`configure(tint.darkened(0.55), tint)`、首字取 `tint.lightened(0.5)`、头像图走 `set_art`。
- 删除自建 `StyleBoxFlat`、`TextureRect` 与 `Initial` 分支；头像图由方块铺满改为 cover 铺满并收进圆角（方形素材下无差异）。

### `GamePortrait`

- `class_name GamePortrait extends AvatarFrame`，保留 `setup(display_name, glyph, tint, show_name, art_path)`。
- 默认封面/描边取 `color-border-strong`；`setup` 时按玩法色 `configure(tint.darkened(0.55), tint)`。
- 名字条 `PortraitName` 挂在 `clipped_content()`；节点名 `PortraitArt` / `PortraitGlyph` 改为 `AvatarFrameArt` / `AvatarFrameGlyph`。
- 删除 `_apply_art`、`_panel_style` 与本地 `_glyph` 成员；首字透明度沿用 `AvatarFrame.GLYPH_ALPHA`。

## 3. 落位与消费者

| 组件 | 使用位置 | 说明 |
| --- | --- | --- |
| `Avatar` | 左栏 8 行座位（`game_players_rail.gd`） | 头像随行高 40–72；血条与名次角标仍是 `SeatAvatar` 容器的兄弟节点，不被裁切 |
| `GamePortrait` | 对手身份（`game_opponent_view.gd`）、售货员（`game_shop_view.gd`）、战斗开场（`combat_intro.gd`） | 64×64；三处均不显示名字条，`show_name = false` |
| `GameHeroCard` | 英雄行 | 上一轮已基于 `AvatarFrame`，本次只随裁切模式修正获得封面绘制 |

## 4. 验证

- `make client-check`：退出码 0。
- `make client-smoke`：17 个场景全部 `UI_SMOKE_OK`，无 `SCRIPT ERROR`。
- 真实渲染目检（临时场景，验证后删除）：并排渲染 100×100 座位头像、64×64 售货员头像与带名字条的 64×64 头像，解码 PNG 比对——三块四角均为 `color-bg`（6×6 角块 1–6 个非背景像素），内部封面为玩法色、描边沿四边连续，名字条文字可见且底部两角仍为圆角。
- 目检发现并修复：初版 `CLIP_CHILDREN_ONLY` 下第三块头像内部只有文字、没有封面；改 `CLIP_CHILDREN_AND_DRAW` 后封面恢复，冒烟断言改为要求 `CLIP_CHILDREN_AND_DRAW` 且封面 alpha 大于 0。

## 5. 实施结果

- `client/ui/avatar.gd`、`client/pages/game/game_portrait.gd` 迁移到 `AvatarFrame`。
- `client/ui/avatar_frame.gd` 新增 `clipped_content()` 并切换到 `CLIP_CHILDREN_AND_DRAW`。
- `ui_smoke_side.gd`、`ui_smoke_seats.gd`、`ui_smoke_cards.gd` 更新节点名与裁切、封面、描边层次断言。
- 文档同步：`avatar.md`、`avatar-frame.md`、组件索引、对局页与技术文档。
