# 头像框组件与英雄卡圆角描边

> 状态：已实施
> 需求归属：v0.1
> 产品依据：对局页英雄卡「边框圆角没盖住头像」缺陷修复；无产品规则改动

## 1. 改动边界

- 只改客户端 UI：新增共享头像框组件，对局英雄卡改为继承它；不改服务端、协议与卡牌数据。
- 视觉目标不变：84×84（手机 64×64）方形头像铺满、玩法色 2px 描边、右下角只显示当前生命。
- 不改 `Avatar`（座位行，直角）、`GamePortrait`（头像块）、`RoomHeroCard`（封面内收规避圆角）；本次只迁移对局英雄卡。

## 2. 根因

原 `GameHeroCard` 是 `PanelContainer`，面板 `StyleBoxFlat` 带 `radius-md` 圆角描边；头像 `TextureRect` 是面板内容的普通子节点。`StyleBoxFlat` 只裁面板自身绘制，不裁子节点，方形头像四角因此压过圆角弧线。

## 3. 组件接口

- 文件：`client/ui/avatar_frame.gd`，`class_name AvatarFrame extends Control`。
- `configure(cover_color: Color, border_color: Color)`：设置封面底色与描边色；`BORDER_WIDTH = 2`、圆角取 `radius-md`，由组件固定。
- `set_art(path: String)`：头像图按 `STRETCH_KEEP_ASPECT_COVERED` 铺满；路径为空或素材缺失时移除图并回退首字。
- `set_glyph(text: String, color: Color)`：无头像素材时的首字与颜色（`GLYPH_ALPHA = 0.55` 由调用方给出颜色）。
- `content() -> Control`：业务内容叠层，位于描边之上，不参与头像裁切。
- 节点层序：`AvatarFrameClip`（`Panel`，`clip_children = CLIP_CHILDREN_ONLY`，面板圆角形状即裁切遮罩；内含 `AvatarFrameGlyph` 与索引 0 的 `AvatarFrameArt`）→ `AvatarFrameBorder`（透明底、只画 2px 圆角描边的 `Panel` 覆盖层）→ `AvatarFrameContent`。后续在 `2026-11-22-square-avatar-frame` 中改为 `CLIP_CHILDREN_AND_DRAW`，以同时绘制封面。
- 尺寸由调用方 `custom_minimum_size` 决定；子节点全部满幅锚点，不参与容器布局。

## 4. 落位与消费者

| 组件 | 继承方式 | 说明 |
| --- | --- | --- |
| `GameHeroCard` | `extends AvatarFrame` | 删除自建 `_body`、`_cover_mark`、`_cover_art`、`_panel_style()`；头像取 `CardDB.art_path(hero_id, "avatar")`，描边取玩法色，`HeroHpMargin` 生命牌挂到 `content()`；节点名 `HeroCard`、`HeroHpPlate`、`HeroHp` 保持 |

- 组件不读 `CardDB`、不碰业务状态；显示名、玩法色与生命值仍由调用方计算。
- 生命牌位于内容叠层，因此右下的 `space-2xs` 内缩在圆角以内且不被头像裁切。

## 5. 验证

- `make client-check`：退出码 0。
- `make client-smoke`：全部 `UI_SMOKE_OK`，无 `SCRIPT ERROR`；断言头像图、首字回退、裁切开启、描边层位于头像之后、生命牌位置与 tooltip。
- 真实渲染目检（临时场景，验证后删除，500×250 逻辑分辨率、480×240 渲染窗口）：并排渲染裁切开启与关闭两张 `GameHeroCard`，解码 PNG 像素比对——开启裁切卡四角 6×6 角块仅 1–3 个非背景像素、极端角像素为 `color-bg`；关闭裁切卡同角块 11–36/36 为头像像素；描边色沿四边连续命中约 113/123 个采样点，生命牌区域存在近白文字像素。

## 6. 实施结果

- 新增 `client/ui/avatar_frame.gd`。
- `client/pages/game/game_hero_card.gd` 改为继承 `AvatarFrame`，保留 `refresh()`、生命牌与 tooltip 行为。
- `client/tests/ui_smoke_side.gd`、`ui_smoke_layout.gd` 更新节点名与裁切、层次断言；`ui_smoke_cards.gd` 增加 `AvatarFrame` 直测。
- 文档新增 `docs/current/product/ui/components/avatar-frame.md`，并同步组件索引、对局页与技术文档。
