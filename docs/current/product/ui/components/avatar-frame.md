# 头像框

> 状态：已实现
> 入口：`AvatarFrame`（`client/ui/avatar_frame.gd`）

## 用途

带圆角描边的方形头像框：头像图按 cover 铺满整框并裁到框的圆角，描边叠在头像之上，四角不会露出方形头像。底色与描边色由调用方按场景给出，组件不读取业务状态。

## 属性

| 属性 / 方法 | 类型 | 说明 |
| --- | --- | --- |
| `configure(cover_color, border_color)` | `void` | 设置封面底色与描边色；圆角与描边宽度固定取组件规则 |
| `set_art(path)` | `void` | 换头像图；有素材时按 cover 铺满并隐藏首字，路径为空或缺图时回退首字 |
| `set_glyph(text, color)` | `void` | 设置无头像素材时显示的首字与颜色 |
| `content()` | `Control` | 业务内容叠层；位于描边之上，不参与头像裁切 |

- 尺寸由调用方 `custom_minimum_size` 决定，组件内部子节点全部用满幅锚点，不参与容器布局。
- `BORDER_WIDTH = 2`、圆角取 `radius-md`，不开放参数。

## 层序与裁切

| 层 | 节点名 | 说明 |
| --- | --- | --- |
| 头像 / 首字 | `AvatarFrameClip` 内的 `AvatarFrameArt`、`AvatarFrameGlyph` | 面板绘制形状（圆角矩形）作为裁切遮罩，`clip_children = CLIP_CHILDREN_ONLY`；有头像图时首字隐藏 |
| 描边 | `AvatarFrameBorder` | 透明底、只画 2px 圆角描边的覆盖层，叠在头像之上，四角压住头像 |
| 业务内容 | `AvatarFrameContent` | 调用方内容叠层（如生命牌），在描边之上且不被裁切 |

- 头像图使用 `STRETCH_KEEP_ASPECT_COVERED`：方形头像按框铺满，超出的部分被圆角裁掉。
- 描边层与内容层都忽略鼠标，点击与悬停由调用方框本身决定。

## 禁用场景

- 不承载业务数据、状态与点击语义，不接受任意圆角、描边宽度或字号参数。
- 不用于直角方形头像位：[头像块](avatar.md)（座位行）与页面内直角头像块仍用各自实现。

## 使用者

- [对局页](../pages/game.md) 的英雄卡：`GameHeroCard` 继承本组件，头像取英雄 `avatar`，描边取玩法色，生命牌挂在 `content()`。
