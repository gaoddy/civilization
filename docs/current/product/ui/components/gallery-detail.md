# 图鉴常驻详情

> 状态：已实现
> 入口：`GalleryDetail.new()`，`client/scenes/gallery_detail.gd`（`class_name GalleryDetail`）

## 用途

图鉴右侧的常驻详情：选中卡的大卡面、名称与类型、科学分类、玩法、费用、原始数值、技能与效果文本。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `show_card(card_id)` | `void` | 重建为指定卡的详情；空 id 显示占位提示 |

- 固定宽 `gallery-detail-width`（280），纵向撑满，面板样式 `surface`。
- 内部节点名：`GalleryDetailScroll`、`GalleryDetailBody`、`GalleryDetailName`、`GalleryDetailEffect`。

## 变体与状态

- 空 id：「选择一张卡牌查看详情」居中显示，无其它内容。
- 有内容时顺序固定：大卡面（112×146）→ 名称 → 类型 → 科学分类 → 玩法 → 关联玩法 → 时代 → 费用 → 原始 → 技能 → 效果。
- 关联玩法、时代、原始、技能、效果各自在数据为空时省略整行，不留空标签。
- 属性行为「小标题 + 值」的横向两栏：标题固定宽 64、`font-caption`、`color-text-muted`、顶部对齐；值 `font-body-sm`、横向扩展、自动换行。
- 效果区先给区块标题「效果」，正文为 `font-body-sm`、`color-text-dim`。
- 内容超出高度时纵向滚动，不做水平滚动。

## 事件

无。详情只读，不响应点击。

## 禁用场景

- 不显示对局数值（当前攻防、加成层、受击、食物链、擅长玩法），也不显示购买原因。
- 不承担选中逻辑；选中由页面与网格维护。

## 使用者

- [卡牌图鉴](card-gallery.md)。
