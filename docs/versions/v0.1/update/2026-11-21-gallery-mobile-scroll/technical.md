# 图鉴手机布局与滚动加载

> 状态：已实施
> 需求归属：v0.1
> 产品依据：[product.md](product.md)

## 1. 改动边界

- 只改客户端图鉴：`client/scenes/card_gallery.gd`、`gallery_toc.gd`、`gallery_grid.gd`、`gallery_detail.gd`，新增 `gallery_category_panel.gd`，以及 `client/ui/theme.gd` 的图鉴令牌。
- 分类数据、计数与筛选逻辑沿用 `CardGallery` 现有实现，不改 `CardDB` 与服务端。

## 2. 令牌

- `gallery-grid-columns-compact`：4 → 6。
- `gallery-detail-width-compact`：200 → 176。
- `gallery-preview-width-compact`：160 → 128。
- `gallery-page-size` / `gallery-page-size-compact` 更名为 `gallery-batch-size`（18）/ `gallery-batch-size-compact`（12），语义改为每批加载张数。
- 删除 `gallery-toc-width-compact`；新增 `gallery-category-panel-width`（360）。
- 手机网格列间距取 `space-md`，桌面与 iPad 保持 `space-lg`。

## 3. 网格与滚动加载

- `GalleryGrid` 结构改为 `ScrollContainer`（`GalleryScroll`）→ `VBoxContainer`（`GalleryScrollBody`）→ `GridContainer`（`GalleryGridRows`）+ 状态标签（`GalleryLoadStatus`）。
- `set_cards` 重建首批卡面并把滚动值归零；`_loaded` 记录已构建张数，`_load_next_batch()` 追加下一批。
- 监听 `ScrollContainer.get_v_scroll_bar().value_changed`，当 `value >= max_value - page - card-height` 时加载下一批；同一批只发起一次。
- 首批构建后按内容高度与可视高度比较，不足一屏时继续加载，避免内容不满却无法触发滚动。
- 触屏拖拽：卡面 `mouse_filter` 改为 `PASS`，按下与拖动冒泡到 `GalleryScroll`，由 `ScrollContainer` 内建的触摸拖拽处理（移动端由触摸模拟鼠标事件）滚动；卡面不再消费按下事件，改为松手且位移不超过 12 像素时发出 `card_selected`，拖动滚动不误切详情。位移用 `get_global_transform_with_canvas()` 换算成画布坐标后比较，滚动造成的卡面移动不计入手指位移。
- 删除 `GalleryPager`、`GalleryPrev`、`GalleryPageLabel`、`GalleryNext` 与相关 `_page` 状态。

## 4. 手机分类面板

- `GalleryToc` 增加 compact 模式：内部改用两列 `GridContainer`，只构建顶层分类；节点名沿用 `TaxonAll` / `TaxonX`，选中与禁用样式不变。
- 新增 `GalleryCategoryPanel`（`client/scenes/gallery_category_panel.gd`）：全屏遮罩 + 面板，面板内放 compact `GalleryToc`；发出 `taxon_selected`，不持有筛选状态。
- `CardGallery` 在手机随从视图把 `GalleryToc` 从主体左栏移入面板；主体上方新增工具栏：分类按钮 + `GalleryFilterBar`（二级筛选）。桌面与 iPad 保持左栏目录与无工具栏。
- 分类按钮文案取 `ui.gallery.category_filter`（「分类：%s」），面板标题取 `ui.gallery.category_panel_title`。
- 面板位置按工具栏下沿定位，尺寸 `gallery-category-panel-width` × 内容高度；点遮罩、按钮或 `Esc` 收起。

## 5. 详情

- `GalleryDetail` 在手机上宽度 176、内边距 `space-md`、属性行标题列 52；预览 128×167。
- 详情内部仍用 `ScrollContainer`，尺寸完全落在主体内，底部可滚动到达。

## 6. 文案

- 新增 `ui.gallery.category_filter`、`ui.gallery.category_panel_title`、`ui.gallery.loaded_stats`、`ui.gallery.load_more_hint`、`ui.gallery.load_all_done`；删除 `ui.gallery.page`、`ui.gallery.prev`、`ui.gallery.next`。
- 人工 key 直接写入 `client/text/zh_CN.csv`，`scripts/gen_text.py` 保留人工部分。

## 7. 测试

- `ui_smoke_gallery.gd`：改为断言 `GalleryScroll` 存在、无分页节点、滚动后已加载张数增加；卡面按下冒泡到滚动区、按下后拖动不切换详情；新增手机 960×540 用例：6 列、分类面板 14 项、页面与详情不越界。
- `ui_smoke_layout.gd` 的英雄视图越界检查改为 `GalleryScroll`，保持桌面与手机两种基准。
- 验证：`make client-check`、`make client-smoke`。

## 8. 文档

- `docs/current/product/ui/pages/gallery.md`、`components/gallery-grid.md`、`gallery-toc.md`、`gallery-detail.md`、`mobile-adaptation.md`、`tokens.md`、`docs/current/technical/client.md`。
