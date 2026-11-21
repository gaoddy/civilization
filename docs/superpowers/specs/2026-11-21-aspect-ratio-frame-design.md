# 内容框宽高比与背景铺满设计

> 状态：已实现（2026-11-21）
> 日期：2026-11-21

## 1. 背景与目标

当前每个页面直接铺在逻辑视口上：`stretch/mode=canvas_items`、`aspect=keep_width`，逻辑宽固定为基准宽（桌面 1280、iPhone 960），逻辑高随窗口比例变化。窗口比基准更宽时，页面横向被拉宽、纵向逻辑空间变少；页面背景用 `STRETCH_SCALE` 拉伸，非基准比例下背景图会变形。

目标是把「背景」和「内容」彻底分层：

- 背景覆盖整个可视区域，等比裁切，永不变形。
- 内容固定 16:9（当前基准比例），完整显示、居中，永不裁切、不变形。
- 视口比 16:9 宽时左右露背景；比 16:9 窄时上下露背景。
- 全端（macOS 任意窗口、iPhone、iPad）只有一套内容比例与一套页面布局尺寸。

这是炉石传说的做法：内容按 16:9 渲染，4:3 加上下边，超宽屏加左右边，官方不支持超宽视野。

## 2. 目标规则

| 层 | 规则 |
| --- | --- |
| 背景层 | 铺满整个逻辑视口（即物理窗口），等比裁切（cover），不参与内容缩放 |
| 内容层 | 固定 16:9，contain 居中；框内永远按 1280×720（iPhone 960×540）布局 |
| 安全区 | 只约束内容框内部，不约束背景；安全区落在背景带内时记 0 |
| 窗口层 | 只决定初始尺寸，不再强制窗口回到比例下限 |

边界：

- 视口比例 > 16:9：内容高 = 视口高，宽 = 高 × 16/9，水平居中。
- 视口比例 = 16:9：内容铺满。
- 视口比例 < 16:9：内容宽 = 视口宽，高 = 宽 × 9/16，垂直居中。
- 内容比例恒为基准比例 `base_canvas().x / base_canvas().y`，当前 1280×720 与 960×540 都是 16:9。

## 3. 各端表现

| 端 | 视口比例 | 表现 |
| --- | --- | --- |
| 桌面 16:9 窗口 | 1.778 | 内容铺满，无背景带 |
| 桌面超宽窗口 | > 1.778 | 左右露背景，纵向逻辑空间不再变少 |
| 桌面窄窗口 | < 1.778 | 上下露背景，不再被弹回比例下限 |
| iPhone 横屏 19.5:9 | 2.167 | 左右各约 9% 背景 |
| iPad 横屏 4:3 | 1.333 | 上下各 12.5% 背景 |
| iPad Pro 11 横屏 | 1.43 | 上下各约 10% 背景 |

## 4. 方案比较

| 方案 | 做法 | 代价 | 结论 |
| --- | --- | --- | --- |
| A. 固定 16:9 + contain | 内容框按 16:9 居中，框内按 1280×720 布局，背景层铺满 | 需要新增内容框与缩放层，安全区换算改成 contain 模型 | **采用** |
| B. 固定 3:2 + contain | 同 A，基准改 3:2 | iPad 留白变小，但全部页面需按 3:2 重排，iPhone 留白变大 | 不采用 |
| C. 可变比例 | 高度恒占满，宽度 ≥ 高 × 16/9，超宽时内容横向铺开 | 页面要响应多种宽度与逻辑高度，现有布局全部要改 | 不采用 |

选择 A：现有页面全部按 1280×720 调好，A 不需要重排页面；16:9 的几何折中比例约 1.70，与 16:9 几乎重合，是「一个比例适配所有端」留白最均衡的取值。

## 5. 实现设计

### 5.1 节点层级

```
Page (Control, full rect = 逻辑视口)
├─ PageBackground (Control/TextureRect, full rect, cover)   ← 在内容框外，铺满
└─ ContentFrame (Control, contain 尺寸, 居中)
   └─ ContentRoot (Control, size = base_canvas, scale = frame高 / base高)
      └─ SafeAreaMargin (full rect)
         ├─ 页面内容（现有直接子节点）
         └─ SafeLayer（动态覆盖层，保持现状）
```

- `UI.safe_area(page)` 负责创建 `ContentFrame` 与 `ContentRoot`，幂等。
- `PageBackground` 仍留在 `ContentFrame` 之外、置底。
- `UI.safe_layer(page)` 的查找路径不变（仍在 `SafeAreaMargin` 下）。

### 5.2 尺寸计算

保留 `project.godot` 的 `canvas_items` + `keep_width`，逻辑视口为 `V = (base.x, base.x × H / W)`，每逻辑单位像素数 `u = W / base.x`。

```
r        = base.x / base.y
frame_h  = min(V.y, V.x / r)
frame_w  = frame_h × r
frame    = ((V.x - frame_w) / 2, (V.y - frame_h) / 2, frame_w, frame_h)
root_scale = frame_h / base.y
```

`ContentRoot.scale = Vector2(root_scale, root_scale)`，其内部坐标恒为 `base_canvas` 单位，页面布局不受窗口比例影响。

物理校验：`frame_w × u = min(H, W / r) × r`，即标准 contain；框内 1 单位 = `u × root_scale` 像素，任何比例下都一致。

### 5.3 背景

- `UI.background()` 的 `TextureRect.stretch_mode` 从 `STRETCH_SCALE` 改为 `STRETCH_KEEP_ASPECT_COVERED`。
- 退化的 `ColorRect` 保持不变。
- 背景已是 `PRESET_FULL_RECT` 且不在安全区内，无需改挂载。

### 5.4 窗口

- 启动仍按 16:9 开窗：取可用区域的 90% 作为目标，再按 16:9 contain 收拢（比 16:9 宽则收高，比 16:9 窄则收宽），在可用区域内居中。
- `client/autoload/window_sizer.gd` 删除 `_keep_aspect_floor` 与 `size_changed` 订阅：拖动后不再弹回比例，窄窗口合法，只由内容框加背景带表现。
- 删除 `ScreenProfile.fit_aspect_floor`（加宽语义），替换为启动用的 contain 收拢函数；冒烟用例同步替换。
- 触控端仍不介入窗口尺寸。
- `docs/versions/v0.1/acceptance.md` 中「启动开窗为 16:9」保留；「桌面 16:10 实机窗口上下无留白」改写为「启动为 16:9，拖动为 16:10 时上下各约 6% 背景」。

### 5.5 安全区与键盘

`ScreenProfile.canvas_insets` 与 `canvas_from_pixels` 的换算从 keep_width 模型改为 contain 模型：

```
scale  = min(W / base.x, H / base.y)
content = (base.x × scale, base.y × scale)
origin  = (W - content.x, H - content.y) / 2
insets  = max(0, safe - origin) / scale
```

- 桌面端仍恒为 0。
- iPad 上下留背景后，安全区落在背景带内，记 0；只有安全区侵入内容框时才产生边距。
- 键盘高度先与底部背景带抵消，只有越过内容框底边的部分按同一 `scale` 换算；`SafeAreaMargin` 的底部取值规则（安全区与键盘取大）不变。
- `SafeAreaMargin` 自身逻辑不改，只是数值来源变成 contain 单位。

### 5.6 输入与拖拽

- `ContentRoot.scale` 是标准 `Control` 变换，Godot 的 GUI 命中测试与输入事件会正确反算坐标；按钮、拖拽、长按、滚动不需要单独改坐标。
- 拖拽预览使用全局坐标，与内容框处于同一 canvas 变换下，不需要额外换算。
- 冒烟需要覆盖：内容框内控件仍可按名找到、点击命中。

### 5.7 覆盖层与拖拽预览

- 现有动态覆盖层（`AimLayer`、`MinionDetailLayer`、`ReportLayer`、`ChooserOverlay`、`HeroDetailLayer` 等）保持在 `SafeAreaMargin` 内，随内容框一起缩放与居中。
- 这些层里把全局坐标当本地坐标用的位置要统一换算：
  - 绘制（`game_aim_layer.gd`）用 `to_local()` 或 `get_global_transform_with_canvas().affine_inverse()` 把全局点转成本地坐标后再 `draw_*`。
  - 面板定位（`game_detail_layer.gd`、`room_hero_detail_layer.gd`、`game_players_rail.gd`）改用局部坐标或 `set_global_position()`；夹紧边界改用内容框矩形，不再用 `get_viewport_rect()`。
- `FirstGameGuide` 是 `CanvasLayer`，不继承内容框变换；本次不改，坐标仍按视口。
- 拖拽预览由 `set_drag_preview` 挂在视口 GUI 下，不继承 `ContentRoot.scale`；`UI.drag_preview()` 的 holder 需要设置 `scale = root_scale`，保证预览尺寸与卡片一致、起手点对齐。`root_scale` 由 `ContentRoot` 提供，页面内可通过 `SafeAreaMargin` 的祖先查询。

### 5.8 冒烟调整

- `client/tests/ui_smoke_layout.gd`：
  - 根节点子节点数仍为 2：`PageBackground` + `ContentFrame`。
  - 断言 `ContentFrame` 宽高比等于基准比例（容差 1e-3）。
  - 断言 `ContentRoot.size == ScreenProfile.base_canvas()`、`ContentRoot.scale` 在 16:9 下为 1。
  - 现有「安全区内直接子节点数量」与按名查找改用 `ContentFrame → ContentRoot → SafeAreaMargin` 路径。
  - `_check_tall` / `_check_game_tall` 的「贴底」断言改为按 `ContentFrame` 底部判定，不再按视口底部。
  - iPad 4:3 下纵向安全区用例改为 0（安全区落在背景带内）。
- `client/tests/ui_smoke_platform.gd`：
  - 删除 `fit_aspect_floor` 用例，新增启动窗口按 16:9 contain 收拢的用例。
  - 新增 `canvas_insets` 的 contain 用例：超宽、16:9、4:3 三种视口的安全区换算。
  - 新增内容框尺寸的纯函数用例（建议把 5.2 的计算抽成 `ScreenProfile.content_frame(viewport, base)` 静态函数，便于直接断言）。
- 冒烟补一条：非 16:9 视口下瞄准箭头与详情面板的定位换算（纯函数或实例化后断言本地坐标），避免覆盖层错位。

### 5.9 文档回写

- `docs/current/product/ui/mobile-adaptation.md` §2 缩放基准。
- `docs/current/technical/client.md` §9 适配实现。
- `docs/versions/v0.1/acceptance.md` 增加验收项。
- 如 `docs/current/product/ui/interactions.md` 有桌面窗口尺寸规则，同步。

## 6. 非目标

- 不重排页面内部布局、不改控件尺寸与间距。
- 不做移动端竖屏，不改变强制横屏。
- 不改变主题（只有深色一套）。
- 不提供超宽屏视野扩展或玩家可调比例。
- 不改服务端、协议与数据。

## 7. 验证

- `make client-check`：静态检查通过。
- `make client-smoke`：布局冒烟与平台换算冒烟通过。
- `make m1-check`：全量检查通过（生成物、文档、测试、静态检查、确定性）。
- 目标设备验证：桌面 16:9 / 超宽 / 窄窗口三种尺寸各看一页；iOS 模拟器或真机按 iPhone 与 iPad 各看一页。
- 非 16:9 下检查拖拽预览尺寸、瞄准箭头端点、卡牌详情面板位置与背景带，确认无错位、无被裁切。
- 主题范围：单主题（深色），不做浅色检查。
- 文档：相对链接校验通过。

## 8. 实施范围

| 文件 | 改动 |
| --- | --- |
| `client/ui/ui.gd` | `safe_area` 创建 `ContentFrame` / `ContentRoot`；`background` 改 cover；`drag_preview` 应用 `root_scale` |
| `client/ui/safe_area.gd` | 不变（仍由 `ContentRoot` 承载），外部查询 `root_scale` 的入口 |
| `client/autoload/screen_profile.gd` | `canvas_insets` / `canvas_from_pixels` 改 contain 模型；新增内容框与启动收拢的纯函数；删除 `fit_aspect_floor` |
| `client/autoload/window_sizer.gd` | 启动按 16:9 收拢；删除拖动回弹 |
| `client/scenes/game_aim_layer.gd` | 绘制坐标转本地 |
| `client/scenes/game_detail_layer.gd` | 面板定位与夹紧改本地 / 内容框 |
| `client/scenes/room_hero_detail_layer.gd` | 同上 |
| `client/scenes/game_players_rail.gd` | 同上 |
| `client/tests/ui_smoke_layout.gd` | 节点路径、比例断言、贴底断言 |
| `client/tests/ui_smoke_platform.gd` | contain 换算、启动收拢用例 |
| `docs/current/product/ui/mobile-adaptation.md`、`docs/current/technical/client.md`、`docs/versions/v0.1/acceptance.md` | 规则与验收回写 |

## 9. 留给实施的判断

- `ContentFrame` 与 `ContentRoot` 的命名、是否合并为一个节点（用 `size` + `scale` 同时表达）。
- 5.2 的计算抽成 `ScreenProfile` 纯函数的具体签名。
- 冒烟中 16:9 容差的取值。
