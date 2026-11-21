# 发现与进化待选改为卡片行：技术方案

> 状态：已实施
> 需求归属：v0.1
> 产品规则：[product.md](product.md)（已确认）

## 1. 改动边界

- 包含：`client/scenes/` 下的选择浮层组件与对局页接线、`client/text/zh_CN.csv`、客户端冒烟与 UI 文档。
- 不包含：服务端、协议、规则引擎、卡牌数据、目标待选瞄准层与其它弹层。

## 2. 客户端

### 2.1 选择浮层组件

- 新增 `client/scenes/game_choice_overlay.gd`（`GameChoiceOverlay extends Control`），节点名仍为 `ChooserOverlay`，保持现有测试与页面查找方式不变。
- 对外接口：
  - `signal chosen(index: int)`：点击第 `index` 个卡面或选项块时发出。
  - `show_choice(choice: Dictionary)`：按待选构建并显示；同一份待选（`kind` 与候选/选项数组序列化一致）重复调用不重建，避免每次视图刷新丢失焦点与悬停。
  - `hide_choice()`：隐藏并清空子节点。
- 结构：全屏 `ColorRect` 压暗（`color-bg`，透明度沿用现有 0.72）→ `CenterContainer` → `VBoxContainer`（间距 `space-md`）→ 提示标签 + `HBoxContainer` 卡片行（间距 `space-md`）。提示文字沿用 `ui.game.overlay.discover_hint` 与 `ui.game.overlay.evolve_hint`。
- 卡面 / 选项块统一为 `Button` 根节点：尺寸取 `UITheme.stage_metric("card-width")` × `card-height`（桌面 112×146，iPhone 紧凑 70×91，保持 112:146 卡形），`focus_mode` 关闭，鼠标指针为手型。`normal` / `hover` / `pressed` / `focus` / `disabled` 五个状态使用 `UITheme.panel_style("raised-flush")`：`hover` / `pressed` 把描边换成 `color-accent`，描边宽度取组件常量 2。
- 发现卡面：`HandCardFace.build({"cardId": card_id, "attack": 基础攻击, "health": 基础生命}, self)`，与手牌同源，介绍优先压缩字号，空间不足时沿用同一省略规则；卡面节点铺满按钮并让出 1px 描边，鼠标事件由按钮承接。
- 进化选项块：同尺寸按钮内放居中标签，文字取 `UI.enum_text("evolve", option)`，不构造卡面。
- 只支持点击：按下即选中，不做拖拽、长按或双击手势。

### 2.2 对局页接线

- `client/scenes/game.gd`：
  - `_overlay` 类型改为 `GameChoiceOverlay`，实例化时连接 `chosen` 到现有的 `_on_choose`；名称、锚点与层级保持在瞄准层之上、首局引导之下。
  - `_refresh_overlay` 保留原有门槛判断（阶段、视图过期、倒计时、已结束招募、目标待选）；满足时调用 `show_choice(choices[0])`，否则 `hide_choice()`；浮层可见性仍在函数内统一收口，`_detail.blocked` 与关闭图鉴的行为不变。
  - 删除 `_overlay_frame` 与 `_build_choice_overlay`，纵向文字按钮不再存在。
- `_on_choose` 的门控与意图构造不变：`_click_allowed()` 通过后发 `ChooseDiscover { choice }`。

### 2.3 文本

- 复用 `ui.game.overlay.discover_hint`、`ui.game.overlay.evolve_hint`。
- 删除不再使用的 `ui.game.overlay.title`（「待选择」）。

## 3. 测试与验证

- 客户端：`make client-check`、`make client-smoke`。
- 冒烟新增：
  - 发现待选：浮层可见，卡片行含与候选数量一致的按钮，按钮内卡面名称与候选 cardId 一致；点击第 2 个按钮发出 `ChooseDiscover { choice: 1 }`。
  - 进化待选：浮层可见，卡片行含与选项数量一致的按钮，标签文字为选项名称；点击第 3 个按钮发出 `ChooseDiscover { choice: 2 }`。
- 既有门槛测试保持：战斗阶段、已结束招募与视图过期时 `ChooserOverlay` 不显示，`_on_choose` 不发意图。
- 文档：`docs/current/product/ui/pages/game.md`、`docs/current/product/ui/interactions.md`；相对链接与索引核对。

## 4. 风险

- 发现卡面使用基础身材；服务端只下发 cardId，若后续发现候选带实时身材需改为读取候选字段。
- 三个候选在 iPhone 紧凑尺寸下总宽 84×3 + 间距，远小于舞台宽度；候选项更多时仍保持单行，不换行、不滚动，超出宽度由容器裁切，与产品规则一致。
- 全屏压暗沿用现有浮层，不改层级；瞄准层在浮层之下，目标待选时不显示浮层，二者不会叠加。
