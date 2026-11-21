# 弹出层公共外壳

> 状态：已实施
> 需求归属：v0.1
> 产品依据：无产品改动，仅组件结构与复用调整

## 1. 改动边界

- 只改客户端弹出层的组织方式，不改视觉、文案、交互时机与协议。
- 不合并业务内容：邀请、好友、选择、战斗开场与图鉴分类仍由各自组件构建。
- 不纳入首局引导的四向挖空遮罩、座位战报的贴行面板与详情层的跟随指针定位。

## 2. 现状与问题

- `InvitePopup`、`FriendsPanel`、`GameChoiceOverlay`、`CombatIntro` 各自搭一份全屏遮罩 + 居中容器。
- 遮罩透明度分散为 0.55 / 0.58 / 0.6 / 0.72，`GalleryCategoryPanel` 另有 0.62，没有共同入口。
- 关闭策略各写各的；层级依赖挂载顺序。

## 3. 方案

新增共享组件 `PopupLayer`（`client/ui/popup_layer.gd`）：

- 全屏根节点，默认隐藏；内部固定 `PopupScrim` 与 `PopupBody` 两个节点。
- `Placement.CENTER`：内容宿主为 `CenterContainer`。
- `Placement.ANCHORED`：内容左上角贴 `open_layer_at` 的锚点，并夹紧在图层内。
- `Placement.EDGE_TOP/BOTTOM/LEFT/RIGHT`：内容贴对应边，另一轴居中。
- 内容通过 `set_content` 注入，旧内容随帧释放；`open_layer` / `close_layer` / `is_layer_open` 控制显隐。
- 遮罩透明度与底色由子类传入，保持既有视觉；`dismiss_on_scrim` 打开后点遮罩关闭并发出 `dismissed`。

继承关系：

| 组件 | 定位 | 遮罩 |
| --- | --- | --- |
| `InvitePopup` | `CENTER` | 黑 0.55 |
| `FriendsPanel` | `CENTER` | 黑 0.58，遮罩放行 |
| `GameChoiceOverlay` | `CENTER` | `color-bg` 0.72 |
| `CombatIntro` | `CENTER` | `color-bg` 0.6，根与遮罩都放行 |
| `GalleryCategoryPanel` | `ANCHORED` | `color-bg` 0.62，点遮罩关闭 |

## 4. 兼容与命名

- 基类方法使用 `open_layer` / `open_layer_at` / `close_layer` / `is_layer_open`，避免与 `FriendsPanel.open`、`GalleryCategoryPanel.open(anchor, ...)` 这类页面既有同名方法冲突。
- 页面与组件原有对外接口不变：大厅仍调用 `FriendsPanel.open/close`，图鉴仍调用 `GalleryCategoryPanel.open/is_open/close`。
- 各组件根节点名与被测试引用的内部节点名保持不变。

## 5. 验证

- `make client-import`、`make client-check`、`make client-smoke` 全部通过。
- `ui_smoke_side` 新增 `PopupLayer` 断言：内容替换、居中显示、锚点夹紧与底边贴边。
- `ui_smoke_social`、`ui_smoke_gallery`、`ui_smoke_combat`、`ui_smoke` 既有浮层场景全部通过。
- `python3 -m unittest tests.test_client_boundary` 与 `python3 scripts/validate_docs.py` 通过。
