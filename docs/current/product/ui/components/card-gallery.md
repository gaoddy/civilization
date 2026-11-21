# 卡牌图鉴

> 状态：已实现
> 入口：`CardGallery.new()`，`client/ui/card_gallery.gd`（`class_name CardGallery`）

## 用途

只读的卡牌图鉴主体：标题、视图切换、当前视图的顶部筛选栏与返回按钮同处头部一行。桌面与 iPad 主体由左栏分类目录、滚动网格与常驻详情装配；手机主体由分类按钮与二级筛选工具栏、滚动网格与常驻详情装配，分类目录收进覆盖式面板。大厅页整页使用，对局页作为全屏覆盖层使用。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `closed` | `signal` | 点返回按钮或按 `Esc` 时发出，由页面决定跳转或收起 |
| `set_catalog(card_ids, title, close_text, hero_ids = null)` | `void` | 载入普通卡集合、页面文案与英雄集合；`hero_ids` 为 `null` 时取全部英雄，传数组（含空数组）时按传入集合展示；重置为随从视图与全部筛选，并选中筛选后的首张 |

- `card_ids` 为普通卡（含非池卡）的 id 数组，`hero_ids` 为英雄卡 id 数组，均由页面给出；组件不读取 `GameStore`。
- `title`、`close_text` 为已翻译文案。
- 内部节点名：`GalleryBackground`、`GalleryHeader`、`GalleryTitle`、`GalleryModeBar`、`ModeToggle`、`GalleryClose`、`GalleryChapters`、`GalleryHeroCategories`、`GalleryBody`、`GalleryGridColumn`、`GalleryToolbar`、`GalleryCategoryButton`、`GalleryKinds`、`GalleryCategoryPanel`、`GalleryToc`（仅桌面与 iPad）、`GalleryGrid`、`GalleryDetail`。
- `GalleryHeader` 内依次为标题、`GalleryModeBar`、当前视图的顶部筛选栏与返回按钮；`GalleryChapters` 与 `GalleryHeroCategories` 同时只显示一个。

## 变体与状态

- 视图分随从与英雄两套，默认随从；标题右侧的 `ModeToggle` 只显示当前视图，点击切到另一视图；英雄集合为空时按钮禁用。
- 随从视图筛选同时存在时代（`0` 表示全部）、顶层分类（空串表示全部）与二级「随从 / 效果」（空串表示全部），三维交叉生效；每个维度的计数都按另外两维筛选换算。卡牌按全部 `play:` 归属归入分类，「全部」按唯一卡牌 id 计数。
- 随从视图顶层分类列「全部」与 13 个玩法，不含英雄，也不再有「中立」；桌面与 iPad 选中项下方展开二级项，手机点「分类」按钮打开两列面板，二级项常驻工具栏；切换顶层分类时二级回到全部。
- 英雄视图隐藏时代栏与左侧目录，顶部筛选栏为「全部」+ 13 个玩法；英雄按全部 `play:` 归属计数与筛选，带多条归属的英雄在多个分类中同时出现，零计数项禁用，切换分类回到首批。
- 切换视图保留各视图筛选，选中改为新视图结果首张。
- 手机分类面板打开时覆盖网格与详情，点分类项、点面板外、再点按钮或按 `Esc` 收起；
- 详情预览区显示卡面美术大图（手机 128×167），无美术时显示玩法色 + 首字占位；英雄视图走 `GalleryDetail.show_hero`，显示介绍、玩法、关联分类与技能，不显示类型「英雄」。
- 网格按批加载，滚动到底部追加下一批，底部显示已加载张数与提示；改筛选或视图后回到首批与顶部。
- 选中卡不在筛选结果内时自动落到结果首张；结果为空时选中清空，网格与详情分别显示空态。
- `Esc` 只在可见时响应，并标记输入已处理。

## 事件

- `closed`：返回按钮与 `ui_cancel` 共用。
- 内部转发：视图切换 `mode_selected`、顶部筛选栏 `item_selected`、目录 `taxon_selected` / `kind_selected`、网格 `card_selected` 都只更新自身筛选与选中。

## 禁用场景

- 不承载购买、出牌等对局操作，也不显示当前数值、手牌或商店状态。
- 不读取 `GameStore` 或网络状态，不自行决定返回目标。

## 使用者

- [卡牌图鉴页](../pages/gallery.md)（大厅整页）。
- 对局页右上角工具组「图鉴」入口的全屏覆盖层。
