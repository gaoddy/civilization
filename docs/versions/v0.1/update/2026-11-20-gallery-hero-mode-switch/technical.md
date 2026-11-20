# 图鉴随从 / 英雄视图切换：技术方案

> 状态：已实施
> 需求归属：v0.1
> 产品规则：[product.md](product.md)（已确认）

## 1. 改动边界

- 包含：`client/scenes/` 下的图鉴组件、图鉴文本、图鉴冒烟与 UI 文档。
- 不包含：服务端、协议、卡牌数据、规则与详情内容。

## 2. 客户端

### 2.1 顶部筛选栏

- 删除 `client/scenes/gallery_chapter_bar.gd`，新增 `client/scenes/gallery_filter_bar.gd`（`GalleryFilterBar`）。
- 组件为顶部居中的横向单选栏：`set_state(items, counts, selected)`，`items` 为 `{id, name}`，计数以 id 为键，空串表示「全部」，`item_selected(id)` 发出选择。
- 按钮沿用原章节栏样式：`default` 未选中、`primary` 选中，名称在上、计数在下，高 44，零计数且未选中时禁用。
- 节点名由构造参数给出：时代栏为 `GalleryChapters`、按钮前缀 `Chapter`；英雄分类栏为 `GalleryHeroCategories`、按钮前缀 `HeroCategory`。

### 2.2 视图切换

- 新增 `client/scenes/gallery_mode_bar.gd`（`GalleryModeBar`）：`set_state(mode, hero_available)`，`mode_selected(mode)` 发出 `minion` / `hero`。
- 组件内只有一个按钮 `ModeToggle`，文字为当前视图名称，点击 emit 另一视图；样式与章节栏一致，英雄集合为空时禁用。

### 2.3 CardGallery

- 状态拆为 `_mode`（`minion` / `hero`）与两套筛选：随从视图沿用 `_era`、`_taxon`、`_kind`；英雄视图使用 `_hero_taxon`。
- 头部顺序为标题、`GalleryModeBar`、当前视图的顶部筛选栏、返回按钮；两个筛选栏同时只显示一个。
- 随从视图：时代栏 + 左侧目录 + 随从 / 效果二级筛选，左侧目录不再含「英雄」。
- 英雄视图：隐藏时代栏与左侧目录，显示 `GalleryHeroCategories`；按英雄自身 `playstyle` 计数与筛选，「全部」为空串，末项为「中立」。
- `set_catalog` 重置为随从视图与全部筛选；切换视图保留各视图筛选，但重置选中到结果首张。
- 英雄视图的网格与详情沿用 `GalleryGrid.set_cards(ids, selected, true)` 与 `GalleryDetail.show_hero`。

### 2.4 文本

- `client/text/zh_CN.csv` 增加 `ui.gallery.mode_minion`（随从）与 `ui.gallery.mode_hero`（英雄）。
- 删除不再使用的 `ui.gallery.category_hero`。

## 3. 测试与验证

- 客户端：`make client-check`、`make client-smoke`。
- 冒烟更新：默认随从、切换英雄、左侧目录隐藏、英雄分类栏计数与零计数禁用、切回随从恢复 432 项与时代栏、空英雄池禁用切换。
- 文档：相对链接与索引核对。

## 4. 风险

- 英雄视图隐藏左侧目录后网格变宽，手机 4 列与桌面 6 列仍需保持每页张数不变。
- 英雄分类栏含 15 个按钮，窄屏会换行；顶部高度增加时需确认网格与分页不越界。
- 随从视图筛选在切换视图后保留，清理时代或分类时要避免英雄视图读取随从分类状态。
