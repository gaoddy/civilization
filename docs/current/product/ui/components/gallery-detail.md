# 图鉴常驻详情

> 状态：已实现
> 入口：`GalleryDetail.new()`，`client/scenes/gallery_detail.gd`（`class_name GalleryDetail`）

## 用途

图鉴右侧的常驻详情：选中卡的卡面美术图、名称与类型、科学分类、玩法、费用、原始数值、技能与效果文本；英雄视图只显示身份与技能。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `show_card(card_id)` | `void` | 重建为指定普通卡的详情；空 id 显示占位提示 |
| `show_hero(card_id)` | `void` | 重建为指定英雄卡的详情；空 id 显示占位提示 |

- 固定宽 `gallery-detail-width`（280），纵向撑满，面板样式 `surface`。
- 内部节点名：`GalleryDetailScroll`、`GalleryDetailBody`、`GalleryDetailPreview`、`GalleryDetailArt`、`GalleryDetailArtPlaceholder`、`GalleryDetailNameRow`、`GalleryDetailName`、`GalleryDetailEraGlyph`、`GalleryDetailEffect`。

## 变体与状态

- 空 id：「选择一张卡牌查看详情」居中显示，无其它内容。
- 普通卡顺序固定：卡面美术图（200×260，手机 160×208）→ 名称（右侧带 24×24 时代象形 `GalleryDetailEraGlyph`）→ 类型 → 科学分类 → 玩法 → 关联分类 → 时代 → 费用 → 原始 → 技能 → 效果。
- 英雄顺序固定：卡面美术图 → 名称 → 类型「英雄」→ 玩法 → 关联分类 → 技能列表；不显示科学分类、时代、费用与原始数值，也不再单独显示「技能」名称行。
- 有 `card_face` 美术时显示等比放大的美术图；无美术时显示玩法色底与卡名首字占位。预览不再拼装卡面组件、费用/时代角标或卡面信息区。
- 关联分类只显示 `prey:` / `host:`，格式为「捕食：…；寄生：…」；关联分类、时代、原始、技能、效果各自在数据为空时省略整行，不留空标签。
- 英雄技能列表按 `skills` 顺序逐条展示：技能名（没有名称时省略）、触发类型标签、完整技能文本；主动技能追加「费用 N · 每回合 N 次」，无技能时显示空态。
- 属性行为「小标题 + 值」的横向两栏：标题固定宽 64、`font-caption`、`color-text-muted`、顶部对齐；值 `font-body-sm`、横向扩展、自动换行。
- 效果区先给区块标题「效果」，正文为 `font-body-sm`、`color-text-dim`。
- 内容超出高度时纵向滚动，不做水平滚动。

## 事件

无。详情只读，不响应点击。

## 禁用场景

- 不显示对局数值（当前攻防、加成层、受击），也不显示购买原因。
- 不承担选中逻辑；选中由页面与网格维护。

## 使用者

- [卡牌图鉴](card-gallery.md)。
