# 可选项框组件

> 状态：已实施
> 需求归属：v0.1
> 产品依据：无产品改动，仅组件提取与文档落位调整

## 1. 改动边界

- 只改客户端 UI 的组件组织：新增共享组件，替换两处重复实现；不改视觉、布局与交互行为。
- 不改服务端、协议、卡牌数据与页面业务规则。
- 不引入 `.tscn` 组件，沿用 GDScript `class_name` 继承；文件名沿用 snake_case。

## 2. 命中点判定

| 现有实现 | 选中表现 | 是否替换 | 原因 |
| --- | --- | --- | --- |
| `RoomHeroCard` | 卡片外扩 2px `color-accent` 描边（`SelectedFrame`） | 是 | 框承载卡面内容，具备持久选中态 |
| `GalleryCardTile` | `raised-flush` 面板 2px `color-accent` 描边 | 是 | 框承载卡面内容，具备持久选中态 |
| `GalleryFilterBar` 条目 | `primary` 填充按下态 | 否 | `Button` + `ButtonGroup` 的文本按钮，不承载内容框 |
| `GalleryToc` 条目 | `accent-row` 填充按下态 | 否 | 同上 |
| `GameChoiceOverlay` 选项块 | 悬停 / 按下描边，点击即发意图 | 否 | 一次性动作，无持久选中态 |

## 3. 组件接口

- 文件：`client/ui/selectable_box.gd`，`class_name SelectableBox extends PanelContainer`。
- `configure(base_style: StyleBoxFlat, outline := Outline.INLINE, hover_enabled := true)`：设置底板、描边变体与是否接悬停回调，每个实例只设置一次。
- `content() -> Control`：内容容器，调用方把展示内容挂到这里。
- `set_selected(value: bool)` / `set_hovered(value: bool)`：切换选中与悬停态。
- `signal activated`：左键在框内按下并在 `TAP_SLOP = 12` 画布单位内松开时发出，不携带参数。
- 变体：`Outline.INLINE` 在面板描边上表达选中；`Outline.OUTER` 在内容区外扩 `space-2xs` 显示 `SelectedFrame`。
- 状态：常态沿用底板描边；悬停改 `color-border-strong`；选中改 2px `color-accent`，`INLINE` 下选中优先于悬停。

## 4. 落位与消费者

| 组件 | 继承方式 | 底板 | 说明 |
| --- | --- | --- | --- |
| `RoomHeroCard` | `Outline.OUTER` | `raised`，内容边距置 0 | 保留 `SelectedFrame` 节点名与外扩语义；触控端不接悬停回调，鼠标指针保持默认 |
| `GalleryCardTile` | `Outline.INLINE` | `raised-flush` | 保持 `mouse_filter = PASS` 与手型指针，点击经 `activated` 转 `card_selected` |

- 组件只做呈现与交互，不读取 `CardDB`、页面状态或网络；选中值与业务标识由调用方提供。

## 5. 验证

- `make client-check`：退出码 0。
- `ui_smoke_cards.tscn`、`ui_smoke_gallery.tscn`、`ui_smoke_room_ops.tscn`、`ui_smoke_touch.tscn`：均输出 `UI_SMOKE_OK`，无 `SCRIPT ERROR`。

## 6. 实施结果

- 新增 `client/ui/selectable_box.gd`。
- `client/pages/room/room_hero_card.gd` 改为继承 `SelectableBox`，删除自建底板样式、悬停回调与外扩选中框。
- `client/ui/gallery_card_tile.gd` 改为继承 `SelectableBox`，删除本地描边计算与点击阈值。
- `ui_smoke_cards.gd` 增加内联与外扩两变体的状态断言。
