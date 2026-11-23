# 图鉴详情

> 状态：已实现
> 入口：`GalleryDetail.new()`，`client/ui/gallery_detail.gd`（`class_name GalleryDetail`）

## 用途

图鉴详情的内容：首行左侧为卡面美术图，右侧为名称与元信息（类型、时代、费用、攻防）、玩法、关联分类与科学分类；卡片下方用分隔线与「效果」标题逐行展示效果文本。英雄视图只显示介绍、玩法、关联分类与技能。外层遮罩、抽屉尺寸与关闭方式由 [图鉴详情浮层](gallery-detail-layer.md) 提供。

## 属性与方法

| 成员 | 类型 | 说明 |
| --- | --- | --- |
| `show_card(card_id)` | `void` | 重建为指定普通卡的详情；空 id 显示占位提示 |
| `show_hero(card_id)` | `void` | 重建为指定英雄卡的详情；空 id 显示占位提示 |

- 本身是 `ScrollContainer`，纵向撑满抽屉；内容在 `GalleryDetailMargin → GalleryDetailBody` 中纵向排列。
- 内边距桌面 `space-xl`（16）、手机 `space-md`（8），纵向间距 `space-sm`。
- 内部节点名：`GalleryDetailMargin`、`GalleryDetailBody`、`GalleryDetailHeader`、`GalleryDetailArtFrame`、`GalleryDetailCover`、`GalleryDetailArt`、`GalleryDetailArtPlaceholder`、`GalleryDetailInfo`、`GalleryDetailName`、`GalleryDetailIntroduction`、`GalleryDetailMeta`、`GalleryDetailDivider`、`GalleryDetailEffectLine<N>`。

## 变体与状态

- 空 id：「选择一张卡牌查看详情」居中显示，无其它内容。
- 普通卡首行：左封面 160×209（手机 112×146，按卡面 112:146 等比）＋ 右侧信息列，顺序为名称 → 元信息一行 → 玩法 → 关联分类 → 科学分类；卡片下方为分隔线 +「效果」标题，`display_text` 按换行拆成一行一个效果。
- 元信息一行用「·」串联：类型 · 时代 · 费用 · 攻/血。攻防取「攻 %s / 血 %s」文案，任一数值为空时整项省略；时代为空时省略该项。四项全部写成文字，不使用图标或角标。
- 英雄首行：左封面 ＋ 右侧名称 → 介绍（有才显示）→ 玩法 → 关联分类；卡片下方为分隔线与技能列表（技能块自带名称与触发标签，不再另加标题）。不显示类型「英雄」、科学分类、时代、费用与攻防，也不再单独显示「技能」名称行。
- 有 `card_face` 美术时显示等比放大的美术图；无美术时显示玩法色底与卡名首字占位。预览不再拼装卡面组件、费用/时代角标或卡面信息区。
- 封面底色取 `UITheme.playstyle_cover`，美术向内收 `card-cover-inset`（3），外框尺寸固定为令牌宽 × `card_face_height`；卡框复用公共 `CardFrame`（`client/ui/card_frame.gd`），普通卡按卡表 `level` 画星级，英雄不计星级。
- 关联分类只显示 `prey:` / `host:`，格式为「捕食：…；寄生：…」；关联分类与效果在数据为空时省略整块，不留空标签。
- 玩法行展示全部 `play:` 归属，按固定玩法顺序用「、」连接；玩法为空时显示「未定」。
- 英雄技能列表按 `skills` 顺序逐条展示：技能名（没有名称时省略）、触发类型标签、完整技能文本；主动技能追加「费用 N · 每回合 N 次」，无技能时显示空态。
- 属性行为「小标题 + 值」的横向两栏：标题桌面固定宽 64（手机 52）、`font-caption`、`color-text-muted`、顶部对齐；值 `font-body-sm`、横向扩展、自动换行。
- 分隔线为 1px `color-border`，上下各留 `space-sm`。
- 内容超出高度时纵向滚动，不做水平滚动。

## 事件

无。详情只读，不响应点击。

## 禁用场景

- 不显示对局数值（当前攻防、加成层、受击），也不显示购买原因。
- 不承担选中逻辑与浮层显隐；选中由页面与网格维护，抽屉由外层负责。

## 使用者

- [图鉴详情浮层](gallery-detail-layer.md)。
