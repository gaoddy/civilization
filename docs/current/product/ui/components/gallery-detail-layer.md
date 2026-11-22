# 图鉴详情浮层

> 状态：已实现
> 入口：`GalleryDetailLayer.new()`，`client/ui/gallery_detail_layer.gd`（`class_name GalleryDetailLayer`）

## 用途

图鉴详情的外壳：头部以下的遮罩与右侧详情抽屉，承载 [图鉴详情](gallery-detail.md)。点卡面时打开，点遮罩、点关闭按钮或由页面按 `Esc` 收起。手机档位改为贴底的整宽抽屉。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `closed` | `signal` | 收起时发出，由页面同步状态 |
| `open_card(card_id, hero)` | `void` | 载入普通卡或英雄详情并显示；0.12 秒淡入 |
| `close()` / `is_open()` | `void` / `bool` | 收起 / 查询显隐 |

- 根节点全屏、默认隐藏、忽略鼠标；遮罩与抽屉分别接收输入。
- 内部节点名：`GalleryDetailScrim`、`GalleryDrawer`、`GalleryDrawerBody`、`GalleryDrawerBar`、`GalleryDetailClose`、`GalleryDetail`。
- 遮罩从头部下沿开始（外边距 `space-2xl` + 头部高 32 = 56），底色 `color-bg`、透明度 `scrim-alpha-gallery`（0.62）；点遮罩关闭，抽屉内点击不关闭。
- 抽屉顶部右侧为 `×` 关闭按钮（`ghost`、24×24）。

## 定位

| 档位 | 位置与尺寸 |
| --- | --- |
| 桌面与 iPad | 贴右：宽 `gallery-drawer-width`（420）+ 外边距 24，上 68（遮罩顶 + 12），右与下各 24；`surface` 样式，圆角 `radius-lg` |
| 手机 | 贴底整宽：从页面高度 30% 处开始到底边，左右与底部各 16 |

- 抽屉高度随内容与滚动；内容超出时由 [图鉴详情](gallery-detail.md) 纵向滚动。
- 遮罩只覆盖头部以下，打开详情后头部仍可切换视图、改筛选或返回。

## 禁用场景

- 不持有筛选、选中或卡牌集合，不计算计数。
- `Esc` 的层级顺序由 [卡牌图鉴](card-gallery.md) 统一处理，本组件不监听键盘。

## 使用者

- [卡牌图鉴](card-gallery.md)。
