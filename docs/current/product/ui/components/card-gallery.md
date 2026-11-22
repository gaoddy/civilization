# 卡牌图鉴

> 状态：已实现
> 入口：`CardGallery.new()`，`client/ui/card_gallery.gd`（`class_name CardGallery`）

## 用途

只读的卡牌图鉴主体：头部一行放视图分段、当前视图的筛选下拉、结果计数与返回按钮；头部以下是占满宽度的卡面网格。详情改为按需打开的浮层抽屉，不再常驻。大厅页整页使用，对局页作为全屏覆盖层使用。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `closed` | `signal` | 返回按钮或 `Esc` 关到最外层时发出，由页面决定跳转或收起 |
| `set_catalog(card_ids, close_text, hero_ids = null)` | `void` | 载入普通卡集合、返回文案与英雄集合；`hero_ids` 为 `null` 时取全部英雄，传数组（含空数组）时按传入集合展示；重置为随从视图与全部筛选，并预选筛选后的首张 |

- `card_ids` 为普通卡（含非池卡）的 id 数组，`hero_ids` 为英雄卡 id 数组，均由页面给出；组件不读取 `GameStore`。
- `close_text` 为已翻译文案。
- 内部节点名：`GalleryBackground`、`GalleryHeader`、`GalleryModeBar`、`ModeMinion`、`ModeHero`、`GalleryEraSelect`、`GalleryPlaySelect`、`GalleryKindSelect`、`GalleryCount`、`GalleryClose`、`GalleryGrid`、`GalleryFilterPanel`、`GalleryDetailLayer`。
- `GalleryHeader` 内依次为视图分段、三个筛选下拉、弹性空隙、计数与返回按钮；英雄视图隐藏 `GalleryEraSelect` 与 `GalleryKindSelect`。
- 头部高度跟随工具行次级按钮：桌面取 `toolbar-button-height`（22）；触控端控件高度抬到次要操作命中高度，背景与边框仍按 22 绘制，视觉与桌面一致；视图分段、筛选下拉与返回按钮与头部同高；切换视图或筛选不改变头部高度。
- `GalleryFilterPanel` 与 `GalleryDetailLayer` 挂在组件根节点上，位于外边距之后，覆盖网格但不改变头部布局。

## 变体与状态

- 视图分随从与英雄两套，默认随从；`GalleryModeBar` 同时显示「随从」「英雄」两个分段，当前视图为金色；英雄集合为空时英雄分段禁用。
- 随从视图筛选同时存在时代（`0` 表示全部）、玩法（空串表示全部）与类型（空串表示全部），三维交叉生效；每个维度的计数都按另外两维筛选换算。卡牌按全部 `play:` 归属归入玩法，「全部」按唯一卡牌 id 计数。
- 玩法含「全部」与 13 个玩法，不含英雄，也不再有「中立」；效果牌通过「类型」下拉筛选。改玩法时类型回到「全部」。
- 英雄视图隐藏时代与类型下拉，玩法下拉改为英雄玩法；英雄按全部 `play:` 归属计数与筛选，带多条归属的英雄在多个玩法中同时出现，零计数项禁用。
- 筛选下拉的文字为「维度 · 当前值 ▾」，选中非「全部」时转为金色；点按钮打开 `GalleryFilterPanel`，再次点同一按钮收起。
- 切换视图保留各视图筛选，选中改为新视图结果首张；每次重建网格都回到首批与顶部。
- 进入页面只预选结果首张，不打开详情；点击卡面时选中并打开 `GalleryDetailLayer`。改筛选、切换视图或重新载入集合都会关闭详情浮层。
- 空结果时计数为「0 张」，网格显示空态，详情浮层不可打开。
- `Esc` 逐层关闭：筛选面板 → 详情浮层 → `closed`；只在可见时响应，并标记输入已处理。

## 事件

- `closed`：返回按钮与最外层 `ui_cancel` 共用。
- 内部转发：视图分段 `mode_selected`、筛选面板 `item_selected` 与网格 `card_selected` 都只更新自身筛选、选中与浮层状态。

## 禁用场景

- 不承载购买、出牌等对局操作，也不显示当前数值、手牌或商店状态。
- 不读取 `GameStore` 或网络状态，不自行决定返回目标。

## 使用者

- [卡牌图鉴页](../pages/gallery.md)（大厅整页）。
- 对局页右上角工具组「图鉴」入口的全屏覆盖层。
