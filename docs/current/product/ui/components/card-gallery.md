# 卡牌图鉴

> 状态：已实现
> 入口：`CardGallery.new()`，`client/scenes/card_gallery.gd`（`class_name CardGallery`）

## 用途

只读的卡牌图鉴主体：头部、时代章节栏、分类目录、卡面网格、常驻详情与返回按钮的整体装配。大厅页整页使用，对局页作为全屏覆盖层使用。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `closed` | `signal` | 点返回按钮或按 `Esc` 时发出，由页面决定跳转或收起 |
| `set_catalog(card_ids, title, close_text, subtitle)` | `void` | 载入卡牌 id 集合与页面文案；重置筛选与选中，并选中筛选后的首张 |

- `card_ids` 为卡牌与 Token 的 id 数组，由页面给出；组件不读取 `GameStore`。
- `title`、`close_text`、`subtitle` 为已翻译文案。
- 内部节点名：`GalleryBackground`、`GalleryHeader`、`GalleryTitle`、`GallerySubtitle`、`GalleryCount`、`GalleryClose`、`GalleryChapters`、`GalleryBody`、`GalleryToc`、`GalleryGrid`、`GalleryDetail`。

## 变体与状态

- 无视觉变体；大厅与对局只差文案与目录内容。
- 筛选同时存在时代（`0` 表示全部）与界级分类（空串表示全部），两维交叉生效。
- 章节计数按分类筛选换算，目录计数按时代筛选换算。
- 选中卡不在筛选结果内时自动落到结果首张；结果为空时选中清空，网格与详情分别显示空态。
- `Esc` 只在可见时响应，并标记输入已处理。

## 事件

- `closed`：返回按钮与 `ui_cancel` 共用。
- 内部转发：章节栏 `chapter_selected`、目录 `taxon_selected`、网格 `card_selected` 都只更新自身筛选与选中。

## 禁用场景

- 不承载购买、出牌等对局操作，也不显示当前数值、手牌或商店状态。
- 不读取 `GameStore` 或网络状态，不自行决定返回目标。

## 使用者

- [卡牌图鉴页](../pages/gallery.md)（大厅整页）。
- 对局页右栏「图鉴」入口的全屏覆盖层。
