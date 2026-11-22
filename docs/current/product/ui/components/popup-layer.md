# 弹出层外壳

> 状态：已实现
> 入口：`PopupLayer`，`client/ui/popup_layer.gd`
> 使用：页面或页面内组件继承它，只提供内容块与遮罩参数

## 1. 用途

统一全屏弹出层的「壳」：遮罩、内容宿主、定位、显隐与点击遮罩关闭。内容、按钮与业务状态仍由各业务组件自己构建，外壳不读取 `GameStore`，不发请求。

覆盖的场景：邀请弹窗、好友浮层、选择浮层、战斗开场与手机图鉴分类面板。

## 2. 结构

```text
PopupLayer（Control，全屏，默认隐藏）
├─ PopupScrim：ColorRect，全屏遮罩，透明度与底色由子类传入
└─ PopupBody：内容宿主
   └─ 子类通过 set_content 放入的面板 / 内容块
```

- `CENTER` 时 `PopupBody` 是 `CenterContainer`，内容块居中。
- 其余定位方式下 `PopupBody` 是整屏 `Control`，内容块由外壳设置 `position`。
- 遮罩与内容宿主的鼠标过滤由子类按场景调整；内容块自身是否接收点击由业务组件决定。

## 3. 接口

| 成员 | 说明 |
| --- | --- |
| `_init(placement, scrim_alpha, scrim_tint)` | 子类在 `_init` 里调用 `super(...)`；底色默认黑色 |
| `set_content(node)` | 替换内容块，旧内容随帧释放；贴边与锚点定位会监听内容尺寸变化重新摆放 |
| `clear_content()` | 释放当前内容块 |
| `open_layer()` | 显示；非居中定位按内容自然尺寸重新摆放 |
| `open_layer_at(anchor)` | 以锚点为内容块左上角显示，仅 `ANCHORED` 使用 |
| `close_layer()` / `is_layer_open()` | 收起 / 查询显隐 |
| `dismiss_on_scrim` | 点击遮罩是否关闭，默认关闭需要业务组件显式打开 |
| `dismissed` | 点击遮罩关闭后发出 |
| `edge_margin` | 贴边与锚点定位的间距，取 `space-md` |

## 4. 定位方式

| 枚举 | 行为 |
| --- | --- |
| `CENTER` | 整屏居中 |
| `ANCHORED` | 内容块左上角贴 `open_layer_at` 传入的锚点，并夹紧在图层内 |
| `EDGE_TOP` / `EDGE_BOTTOM` | 内容块贴住上 / 下边，水平居中 |
| `EDGE_LEFT` / `EDGE_RIGHT` | 内容块贴住左 / 右边，垂直居中 |

## 5. 使用方

| 组件 | 定位 | 遮罩 | 备注 |
| --- | --- | --- | --- |
| `InvitePopup` | `CENTER` | 黑 0.55 | 无邀请时整体隐藏 |
| `FriendsPanel` | `CENTER` | 黑 0.58，遮罩放行 | 根节点拦截点击；Esc 与关闭按钮走 `closed` |
| `GameChoiceOverlay` | `CENTER` | `color-bg` 0.72 | 拦截点击，待选状态由页面控制 |
| `CombatIntro` | `CENTER` | `color-bg` 0.6 | 根节点与遮罩都放行，不抢详情与输入 |
| `GameStatusModal` | `CENTER` | 黑 0.55，遮罩放行 | 面板与内部容器放行输入，只有「知道了」按钮接收点击，状态提示不锁对局 |
| `GalleryCategoryPanel` | `ANCHORED` | `color-bg` 0.62 | 点遮罩关闭 |

## 6. 不进外壳的浮层

- `FirstGameGuide`：四向挖空遮罩与底部提示条属于教练标记，不是通用遮罩。
- 座位详情：没有遮罩，悬停时由详情层贴在座位行右侧，不在 `PopupLayer` 上挂载。
- `DetailLayer` 与 `RoomHeroDetailLayer`：跟随指针、默认不拦截输入，定位规则与弹出层不同。
- 图鉴覆盖层与房间、结算页：属于整页切换，不是覆盖式弹层。

## 7. 验证

- `make client-check`、`make client-smoke`：`ui_smoke_side` 覆盖居中、锚点夹紧、底边贴边与内容替换。
- 邀请、好友、选择、战斗开场与手机图鉴分类面板的既有冒烟场景全部通过。
