# 跨设备一致性设计

> 状态：已实现
> 关联：[移动端适配](../product/ui/mobile-adaptation.md)、[客户端技术现状](client.md) §9、[v0.1 验收](../../versions/v0.1/acceptance.md) §3

## 设计目标

保证桌面端（macOS）、iPhone 与 iPad 使用同一套设计，只按设备类型调整紧凑度，不因屏幕比例、尺寸或方向改变布局结构：

- **单一布局基准**：所有设备按 16:9 比例设计，只有缩放差异
- **自动紧凑适配**：iPhone 自动使用 `-compact` 视觉令牌变体
- **比例容差**：非 16:9 设备（iPhone 19.5:9、iPad 4:3）内容框居中、背景填充
- **安全区透明**：刘海、灵动岛与 Home 指示条不影响内容框布局
- **触控命中自动换算**：44pt / 32pt 按设备缩放换算为画布单位

## 四层保证机制

### 1. 固定 16:9 内容框 + 等比缩放

所有设备的游戏内容在固定 16:9 比例的内容框内渲染，永不裁切、永不变形。

#### 逻辑基准

- **桌面 / iPad**：1280×720
- **iPhone**：960×540

基准只是逻辑坐标，最终按视口尺寸等比缩放。

#### 内容框计算

内容框按基准比例 contain 居中：宽或高一侧贴满视口，另一侧留背景带。

公式（`client/autoload/screen_profile.gd:113-119`）：

```gdscript
static func content_frame(viewport: Vector2, base: Vector2i) -> Rect2:
    var ratio := float(base.x) / float(base.y)
    var height := minf(viewport.y, viewport.x / ratio)
    var width := height * ratio
    return Rect2(
        Vector2((viewport.x - width) * 0.5, (viewport.y - height) * 0.5),
        Vector2(width, height)
    )
```

#### 三层布局结构

页面调用 `UILayout.safe_area(self)` 后自动创建：

```
Page (视口全尺寸)
├─ PageBackground (铺满,cover)
└─ ContentFrame (16:9 contain 居中)
   └─ ContentRoot (固定基准尺寸,整体缩放)
      └─ SafeAreaMargin (安全区)
         └─ 页面内容
```

- **PageBackground**：铺满整个视口，按 `STRETCH_KEEP_ASPECT_COVERED` 裁切，露出的部分即背景带
- **ContentFrame**：16:9 内容框，contain 居中，逻辑坐标原点
- **ContentRoot**：固定基准尺寸（1280×720 或 960×540），整体缩放到 ContentFrame 大小
- **SafeAreaMargin**：安全区边距，只在刘海等侵入内容框时生效

#### 画布缩放

逻辑画布到屏幕像素的缩放比例：

```gdscript
static func canvas_scale(window_size: Vector2, base: Vector2i) -> float:
    return minf(window_size.x / float(base.x), window_size.y / float(base.y))
```

宽高两个方向取最小比例，保证内容框完整可见。

#### 背景层

`PageBackground` 留在 `ContentFrame` 外，按 `STRETCH_KEEP_ASPECT_COVERED` 铺满整个视口：

- **16:9 设备**：背景完全被内容框遮挡，无背景带
- **超宽设备**（16:10、21:9）：左右各露出背景带
- **更高设备**（4:3、iPad）：上下各露出背景带
- **iPhone 19.5:9**：左右各约 9% 背景带

背景纹理不参与内容缩放，只负责视觉填充。

### 2. 三层视觉令牌系统

通过 `client/ui/theme.gd` 的三层令牌保证所有控件尺寸统一，同时按设备自动选择紧凑变体。

#### 令牌层级

```
原子层 (PRIMITIVE_*) → 语义层 (SEMANTIC_*) → 组件 / 场景层 (SCENARIO)
```

- **原子层**：固定数值，如 `{"2": 2, "4": 4, "8": 8, ...}`
- **语义层**：按用途引用，如 `{"space-xs": "4", "space-md": "8", ...}`
- **组件层**：按场景命名，如 `{"card-width": 112, "card-height": 146, ...}`

#### 设备自动切换

`stage_metric()` 按设备类型自动选择紧凑变体（`client/ui/theme.gd:258-263`）：

```gdscript
static func stage_metric(name: String) -> int:
    if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE:
        var compact := name + "-compact"
        if SCENARIO.has(compact):
            return int(SCENARIO[compact])  # iPhone 用紧凑值
    return metric(name)  # 其他设备用标准值
```

示例：

| 令牌 | 桌面 / iPad | iPhone |
| --- | --- | --- |
| `card-width` | 112 | 84 |
| `card-height` | 146 | 92 |
| `hero-row-height` | 84 | 76 |

开发者只调用 `UITheme.stage_metric("card-width")`，iPhone 自动返回 84，其他设备返回 112。

#### 令牌约束

- **所有间距**：只使用 `space-*` 令牌，不写魔法数字
- **所有颜色**：只使用 `color-*` 令牌，不写字面量
- **所有字体大小**：只使用 `font-*` 令牌
- **紧凑变体命名**：标准令牌名 + `-compact` 后缀，如 `card-width-compact`

### 3. 统一控件工厂

`client/ui/controls.gd` 提供统一控件构造，自动应用视觉令牌和触控适配。

#### 触控命中自动换算

所有按钮在触控设备上自动应用 44pt 最小命中尺寸（`client/ui/controls.gd`）：

```gdscript
static func button(text: String) -> Button:
    var height := 34
    if ScreenProfile.is_touch():
        height = UITheme.hit_primary()  # 44pt 换算为画布单位
    node.custom_minimum_size = Vector2(0, height)
```

#### 命中尺寸换算公式

逻辑点（pt）换算为画布单位（`client/autoload/screen_profile.gd:105-109`）：

```gdscript
static func canvas_hit_min(pt: float, px_per_pt: float, base: Vector2i, window_size: Vector2) -> int:
    var scale := canvas_scale(window_size, base)
    if scale <= 0.0:
        return int(round(pt))
    return maxi(1, int(round(pt * px_per_pt / scale)))
```

- **逻辑点**：iOS 设计规范的 44pt / 32pt
- **每点像素数**：`DisplayServer.screen_get_scale()` 返回的设备像素比
- **每画布单位像素数**：`canvas_scale()` 返回的缩放比例

示例：

| 设备 | 逻辑点 | 设备像素比 | 画布缩放 | 画布单位 |
| --- | --- | --- | --- | --- |
| 桌面 | 44pt | 1.0 | 1.0 | 44 |
| iPhone (960×540) | 44pt | 3.0 | 2.2 | 60 |
| iPad (1280×720) | 44pt | 2.0 | 1.4 | 41 |

#### 控件工厂约束

- **所有按钮**：通过 `UIControls.button()` / `UIControls.styled_button()` 创建，不直接 `new()`
- **所有标签**：通过 `UIControls.label()` 创建，自动应用字体令牌
- **所有输入框**：通过 `UIControls.line_edit()` 创建，自动应用高度与边距
- **工厂默认值**：`UIControls.label()` 省略字号时落到 `font-body` 令牌，`UIControls.margin()` 省略数值时落到 `space-lg` 令牌

### 4. 页面统一入口

所有页面在 `_ready()` 时调用同一个入口创建布局结构。

#### 入口调用

```gdscript
func _ready() -> void:
    UILayout.safe_area(self)  # 自动创建内容框与安全区
    # 后续构建页面内容；新增子节点自动移入安全区
```

#### 入口行为

`UILayout.safe_area(page)` 会：

1. 创建 `ContentFrame` 与 `ContentRoot`
2. 把页面原有子节点移入 `SafeAreaMargin`
3. 订阅后续新增子节点，自动移入安全区并保持创建顺序
4. 把 `PageBackground` 留在框外置底
5. 订阅视口变化自动重新布局

#### 页面约束

- **所有页面**：`_ready()` 第一句必须是 `UILayout.safe_area(self)`；之后创建的内容节点自动移入安全区
- **背景节点**：命名为 `PageBackground`，放在页面最外层
- **内容节点**：直接作为页面子节点，会被自动移入安全区
- **覆盖层**：详情层、战报层、瞄准层等覆盖层按 `ContentRoot` 缩放定位

### 5. 自动化验收保证

`client/tests/ui_smoke_platform.gd` 验证设备基准与换算，`client/tests/ui_smoke_layout.gd` 验证页面布局与覆盖层：

#### 验收场景

- **设备基准**：桌面 / iPhone（960×540）/ iPad（1280×720）三档基准正确（`ui_smoke_platform`）
- **命中换算**：44pt→60/41、32pt→44/30 按设备换算正确（`ui_smoke_platform`）
- **输入框工厂**：桌面高 34、触控端不低于主操作命中高度（`ui_smoke_layout`）
- **小屏布局**：960×540 下六个页面关键节点不越界
- **内容框居中**：1280×800（16:10）与 1280×960（4:3）下内容框 contain 居中
- **非 16:9 覆盖层**：超宽视口下瞄准层源矩形落在内容框局部坐标
- **安全区背景**：六个页面 `PageBackground` 留在安全区外

#### 真机验收

- **iPhone 真机**：左横与右横均可操作，刘海与 Home 指示条不遮挡按钮
- **iPad 真机**：1280×720 基准，图鉴按可用宽度分列（基准 9 列）
- **全触控完成一局**：8 人房间招募、战斗、结算走完，不依赖鼠标

验收记录见 [v0.1 验收](../../versions/v0.1/acceptance.md) §3。

## 窗口策略

### 桌面端

启动时按屏幕可用区域的 90% 收拢成基准比例并居中（`client/autoload/window_sizer.gd`）：

```gdscript
func _ready() -> void:
    var usable := DisplayServer.screen_get_usable_rect(...)
    var target := ScreenProfile.launch_window_size(usable.size, ScreenProfile.base_canvas())
    get_window().size = target
    get_window().position = usable.position + (usable.size - target) / 2
```

拖动缩放后不再调整窗口尺寸；全屏、最大化不介入。

`make client-run` 的 `WIDTH` 与 `HEIGHT` 指定窗口像素尺寸，`CLIENT_DEVICE` 指定设备档位（`phone` / `tablet` / `desktop`）。只传尺寸不改档位；模拟移动端布局需同时指定档位：

```bash
make client-run WIDTH=960 HEIGHT=540 CLIENT_DEVICE=phone   # 模拟 iPhone 档位
make client-run WIDTH=1280 HEIGHT=800                      # 只模拟 16:10 窗口尺寸
```

### 移动端

移动端不调整窗口尺寸，直接按逻辑视口全屏横屏：

- **iPhone**：逻辑视口 ~1920×878（19.5:9），内容框 16:9 居中，左右各约 9% 背景
- **iPad**：逻辑视口 ~2048×1536（4:3），内容框 16:9 居中，上下各 12.5% 背景

只做横屏，竖屏移出范围（见 [移动端只做横屏](../../../.claude/projects/-Users-gaoddy-Projects-Civilization/memory/mobile-landscape-only.md)）。

## 安全区处理

安全区只在刘海、灵动岛或 Home 指示条侵入内容框时生效，不重复扣减背景带。

### 安全区换算

公式（`client/autoload/screen_profile.gd:154-167`）：

```gdscript
static func canvas_insets(window_size: Vector2, base: Vector2i, safe: Rect2i, window_position: Vector2) -> Vector4i:
    var scale := canvas_scale(window_size, base)
    var content := Vector2(base.x * scale, base.y * scale)
    var origin := (window_size - content) * 0.5
    var safe_position := Vector2(safe.position) - window_position
    var safe_end := Vector2(safe.end) - window_position
    return Vector4i(
        maxi(0, int(round(maxf(0.0, safe_position.x - origin.x) / scale))),
        maxi(0, int(round(maxf(0.0, safe_position.y - origin.y) / scale))),
        maxi(0, int(round(maxf(0.0, origin.x + content.x - safe_end.x) / scale))),
        maxi(0, int(round(maxf(0.0, origin.y + content.y - safe_end.y) / scale)))
    )
```

- **内容框按基准比例 contain 居中**，与 `content_frame()` 同一模型
- **安全矩形与内容框的差**，换算为画布单位
- **落在背景带内的边距记零**，不重复扣减

### 安全区应用

`SafeAreaMargin` 节点按安全区边距调整内边距：

```gdscript
var insets := ScreenProfile.safe_insets()
safe_margin.add_theme_constant_override("margin_left", insets.x)
safe_margin.add_theme_constant_override("margin_top", insets.y)
safe_margin.add_theme_constant_override("margin_right", insets.z)
safe_margin.add_theme_constant_override("margin_bottom", insets.w)
```

页面内容自动避开刘海与 Home 指示条。

## 键盘避让

虚拟键盘侵入内容框的高度，换算为画布单位（`client/autoload/screen_profile.gd:88-93`）：

```gdscript
static func keyboard_from_pixels(pixels: float, base: Vector2i, window_size: Vector2) -> int:
    var scale := canvas_scale(window_size, base)
    if scale <= 0.0 or pixels <= 0.0:
        return 0
    var band := maxf(0.0, (window_size.y - float(base.y) * scale) * 0.5)
    return maxi(0, int(round((pixels - band) / scale)))
```

- **窄于基准比例时内容框上下留背景带**，键盘先从背景带算起
- **只有越过内容框底边的部分才让位**

页面监听键盘高度变化，动态调整底部内容位置（如聊天输入框）。

## 实现文件

| 文件 | 职责 |
| --- | --- |
| `client/autoload/screen_profile.gd` | 设备检测、逻辑基准、画布缩放、安全区与键盘换算 |
| `client/autoload/window_sizer.gd` | 桌面端启动窗口按基准比例收拢居中 |
| `client/ui/layout.gd` | 创建三层布局结构（ContentFrame + ContentRoot + SafeAreaMargin） |
| `client/ui/theme.gd` | 三层视觉令牌与设备自动切换 |
| `client/ui/controls.gd` | 统一控件工厂与触控命中自动换算 |
| `client/tests/ui_smoke_layout.gd` | 跨设备一致性自动化验收 |

## 开发约束

### 布局规则

1. **所有页面**：`_ready()` 第一句必须是 `UILayout.safe_area(self)`
2. **所有坐标**：只在 `ContentRoot` 局部坐标系内计算，不直接使用视口坐标
3. **所有尺寸**：按逻辑基准设计（1280×720 或 960×540），运行时自动缩放
4. **背景节点**：命名为 `PageBackground`，放在页面最外层，铺满视口

### 令牌规则

1. **所有间距**：只使用 `UITheme.space_*()` 或 `stage_metric("space-*")`
2. **所有颜色**：只使用 `UITheme.color_*()` 或 `stage_metric("color-*")`
3. **所有字体大小**：只使用 `UITheme.font_size_*()` 或 `stage_metric("font-*")`
4. **紧凑变体**：标准令牌名 + `-compact` 后缀，开发时不需要手动判断设备
5. **舞台令牌**：布局尺寸统一经 `UITheme.stage_metric("token")` 读取，iPhone 自动取 `-compact`

### 控件规则

1. **所有按钮**：通过 `UIControls.button()` / `UIControls.styled_button()` 创建，不直接 `new()`
2. **所有标签**：通过 `UIControls.label()` 创建
3. **所有输入框**：通过 `UIControls.line_edit()` 创建
4. **触控命中**：控件工厂已自动换算，不需要手动调整高度

### 验收规则

1. **每次改动页面布局**：运行 `make client-smoke` 验证六个页面不越界
2. **每次改动令牌**：检查 iPhone 紧凑变体是否需要同步调整
3. **每次改动控件工厂**：确认触控命中换算公式不受影响
4. **移动端真机验收**：刘海、灵动岛与 Home 指示条不遮挡按钮与文本

## 常见问题

### Q: 为什么不直接用 Godot 的 `content_scale_mode`？

A: Godot 的 `canvas_items` + `expand` 只能保证画布铺满视口，无法保证 16:9 内容框居中与背景带填充。我们需要三层结构：

- **PageBackground**：铺满视口，cover 裁切
- **ContentFrame**：16:9 contain 居中
- **ContentRoot**：固定基准尺寸，整体缩放

这是炉石传说的做法，不是 Godot 内置功能。

### Q: 为什么 iPhone 用 960×540 而不是 1280×720？

A: 小屏设备用 960×540 基准，可以在同样物理尺寸下显示更多内容（因为缩放比例更小）。同时通过 `-compact` 令牌调整控件紧凑度，避免按钮过大。

### Q: 安全区为什么不直接扣减视口尺寸？

A: 安全区可能落在背景带内（如 iPhone 左右背景带内的刘海），此时不应该扣减内容框。只有安全区侵入内容框时才生效，避免重复扣减。

### Q: 为什么不用 `@export` 暴露基准尺寸？

A: 基准尺寸是架构约束，不是可配置参数。所有设备只有两档基准（960×540 / 1280×720），由设备类型决定，不允许运行时修改。

### Q: 如何在桌面端测试不同设备？

A: 用 `make client-run` 的 `WIDTH`、`HEIGHT` 指定窗口像素尺寸，`CLIENT_DEVICE` 指定设备档位：

```bash
# 模拟 iPhone 档位（960×540 逻辑基准、紧凑卡面、触控输入）
make client-run-on-iphone

# 模拟 iPad 档位
make client-run-on-ipad

# 只模拟 16:10 窗口尺寸，设备档位仍按桌面
make client-run WIDTH=1280 HEIGHT=800
```

设备档位也可直接传 `CLIENT_DEVICE`；`ScreenProfile.set_device_for_test()` 只用于测试代码。

## 相关文档

- [移动端适配](../product/ui/mobile-adaptation.md)：产品层面的适配规则
- [客户端技术现状](client.md) §9：移动端适配实现
- [v0.1 验收](../../versions/v0.1/acceptance.md) §3：iOS 适配验收记录
- [16:9 内容框设计](../../superpowers/specs/2026-11-21-aspect-ratio-frame-design.md)：设计决策记录
