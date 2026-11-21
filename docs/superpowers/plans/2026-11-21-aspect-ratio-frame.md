# 内容框宽高比与背景铺满 Implementation Plan

> **历史计划（实施前快照）：** 本文保留当时的公式与示例代码，未随实现回改；实际结论以对应提交与 `docs/current/technical/client.md` 为准。

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让每个页面的内容固定为 16:9 居中显示，背景铺满整个视口且不变形，全端共用一套布局尺寸。

**Architecture:** 保留 Godot 的 `canvas_items` + `keep_width`。在页面根与 `SafeAreaMargin` 之间插入 `ContentFrame`（按 contain 计算的 16:9 区域）与 `ContentRoot`（固定 `base_canvas` 尺寸、按比例缩放的逻辑画布）。背景留在 `ContentFrame` 之外铺满视口。窗口启动仍按 16:9 开，拖动后不再回弹。

**Tech Stack:** Godot 4、GDScript；冒烟用 `client/tests/ui_smoke*.tscn` 场景；入口统一走 `make`。

**Spec:** `docs/superpowers/specs/2026-11-21-aspect-ratio-frame-design.md`

## Global Constraints

- 内容比例取 `ScreenProfile.base_canvas().x / base_canvas().y`（当前桌面 1280×720、iPhone 960×540，均为 16:9），不硬编码 16:9。
- 只有深色主题，不做浅色检查。
- 不改变协议、数据、服务端；不重排页面内部布局。
- 构建、检查、冒烟统一走 `make client-check`、`make client-smoke`、`make m1-check`。
- 提交按任务拆分，一个任务一个提交。

## Review Focus

- 非 16:9 视口下瞄准箭头、卡牌详情、战报浮层的定位与命中是否仍与卡片对齐。
- 安全区落在背景带内时应记 0，只有侵入内容框才产生边距。
- 键盘弹出时底部边距是否按新的 contain 比例换算。
- 拖拽预览在缩放内容框下是否与指针和卡片尺寸一致。
- 触控长按与拖拽在 `ContentRoot.scale` 下是否仍能命中正确的卡片。

---

### Task 1: ScreenProfile 的 contain 换算

**Files:**
- Modify: `client/autoload/screen_profile.gd`
- Test: `client/tests/ui_smoke_platform.gd`

**Interfaces:**
- Consumes: `ScreenProfile.BASE_CANVAS_TABLET`、`BASE_CANVAS_PHONE`、`base_canvas()`。
- Produces:
  - `static func content_frame(viewport: Vector2, base: Vector2i) -> Rect2`：返回逻辑视口内 16:9 内容框的位置与尺寸。
  - `static func launch_window_size(usable: Vector2i, base: Vector2i) -> Vector2i`：可用区域的 90% 再收拢成 16:9。
  - `canvas_insets` / `canvas_from_pixels` 改为 contain 模型。
  - 删除 `fit_aspect_floor`。

- [ ] **Step 1: 写失败用例**

在 `client/tests/ui_smoke_platform.gd` 的 `_ready()` 中把 `_check_window_floor()` 换成 `_check_content_frame()` 与 `_check_launch_window()`，并新增：

```gdscript
## 内容框：视口更宽时左右留白，更窄时上下留白，比例恒为基准比例。
func _check_content_frame() -> void:
    var base := ScreenProfile.BASE_CANVAS_TABLET
    var full := ScreenProfile.content_frame(Vector2(1280, 720), base)
    _check(full.is_equal_approx(Rect2(0, 0, 1280, 720)), "16:9 视口内容框应铺满")
    var wide := ScreenProfile.content_frame(Vector2(1280, 480), base)
    _check(is_equal_approx(wide.size.y, 480), "超宽视口内容框高应等于视口高")
    _check(is_equal_approx(wide.size.x, 480.0 * 16.0 / 9.0), "超宽视口内容框宽按比例")
    _check(is_equal_approx(wide.position.x, (1280.0 - wide.size.x) * 0.5), "超宽视口内容框应水平居中")
    var tall := ScreenProfile.content_frame(Vector2(1280, 960), base)
    _check(is_equal_approx(tall.size.x, 1280), "窄视口内容框宽应等于视口宽")
    _check(is_equal_approx(tall.size.y, 720), "窄视口内容框高按比例")
    _check(is_equal_approx(tall.position.y, 120), "窄视口内容框应垂直居中")


## 启动窗口：可用区域的 90% 收拢成 16:9。
func _check_launch_window() -> void:
    var base := ScreenProfile.BASE_CANVAS_TABLET
    _check(ScreenProfile.launch_window_size(Vector2i(1920, 1080), base) == Vector2i(1728, 972),
        "16:9 屏幕启动窗口保持 16:9")
    _check(ScreenProfile.launch_window_size(Vector2i(2560, 1600), base) == Vector2i(2304, 1296),
        "16:10 屏幕启动窗口按宽收拢")
    _check(ScreenProfile.launch_window_size(Vector2i(3440, 1440), base) == Vector2i(2304, 1296),
        "21:9 屏幕启动窗口按高收拢")
    _check(ScreenProfile.launch_window_size(Vector2i(1512, 982), base) == Vector2i(1360, 765),
        "真实 16:10 屏幕启动窗口收拢为 16:9")
    _check(ScreenProfile.launch_window_size(Vector2i.ZERO, base) == base,
        "可用区域未知时回退基准尺寸")
```

同时把 `_check_phone()` 中的导入与 `_check_tablet()` 的纵向断言按新模型改：

```gdscript
    # iPhone 超宽视口：左右留白吸收刘海，只有底部 Home 指示条生效。
    var insets := ScreenProfile.canvas_insets(
        window, base, Rect2i(177, 0, 2202, 1116), Vector2.ZERO
    )
    _check(insets == Vector4i(0, 0, 0, 29), "iPhone 安全区只在底部生效")
```

```gdscript
    # iPad 4:3：内容上下各留 256px 背景，安全区整体落在背景带内。
    var insets := ScreenProfile.canvas_insets(
        window, base, Rect2i(12, 24, 2708, 2000), Vector2.ZERO
    )
    _check(insets == Vector4i(6, 0, 6, 0), "iPad 上下安全区被背景带吸收")
```

- [ ] **Step 2: 运行确认失败**

Run: `make client-smoke`
Expected: FAIL，`PLATFORM_SMOKE_FAIL` 报 `content_frame` 等方法不存在或断言不符。

- [ ] **Step 3: 实现纯函数与 contain 换算**

在 `client/autoload/screen_profile.gd` 新增：

```gdscript
## 逻辑视口内的 16:9 内容框：宽或高按比例收拢后居中。
static func content_frame(viewport: Vector2, base: Vector2i) -> Rect2:
    if viewport.x <= 0.0 or viewport.y <= 0.0 or base.x <= 0 or base.y <= 0:
        return Rect2(Vector2.ZERO, viewport)
    var ratio := float(base.x) / float(base.y)
    var height := minf(viewport.y, viewport.x / ratio)
    var width := height * ratio
    return Rect2(Vector2((viewport.x - width) * 0.5, (viewport.y - height) * 0.5), Vector2(width, height))


## 启动窗口：可用区域的 90%，再收拢成基准比例；可用区域未知时回退基准尺寸。
static func launch_window_size(usable: Vector2i, base: Vector2i) -> Vector2i:
    if usable.x <= 0 or usable.y <= 0:
        return base
    var target := Vector2i(int(float(usable.x) * 0.9), int(float(usable.y) * 0.9))
    var ratio := float(base.x) / float(base.y)
    if float(target.x) / float(target.y) > ratio:
        target.x = int(round(float(target.y) * ratio))
    else:
        target.y = int(round(float(target.x) / ratio))
    return target
```

把 `canvas_insets` 的内容矩形改成 contain：

```gdscript
    var content := Vector2(base.x * scale, base.y * scale)
    var origin := (window_size - content) * 0.5
```

`canvas_from_pixels` 与 `canvas_hit_min` 已使用 `canvas_scale`（`minf`），保持不变；删除 `fit_aspect_floor` 与 `_ceil_div`。

- [ ] **Step 4: 运行确认通过**

Run: `make client-smoke`
Expected: `UI_SMOKE_OK`，退出码 0。

- [ ] **Step 5: 提交**

```bash
git add client/autoload/screen_profile.gd client/tests/ui_smoke_platform.gd
git commit -m "feat(client): 内容框按 contain 计算并统一安全区换算"
```

---

### Task 2: 内容框与背景分层

**Files:**
- Modify: `client/ui/ui.gd`
- Modify: `client/tests/ui_smoke_layout.gd`

**Interfaces:**
- Consumes: `ScreenProfile.content_frame`、`ScreenProfile.base_canvas`（Task 1）。
- Produces:
  - `UI.CONTENT_FRAME_NAME := "ContentFrame"`、`UI.CONTENT_ROOT_NAME := "ContentRoot"`。
  - `UI.safe_area(page)` 返回 `SafeAreaMargin`，其父为 `ContentRoot`，`ContentRoot` 父为 `ContentFrame`。
  - `UI.root_scale(node: Node) -> float`：向上查找 `ContentRoot` 的 scale，找不到返回 1.0。
  - `UI.background()` 的贴图改为 `STRETCH_KEEP_ASPECT_COVERED`。
  - `UI.drag_preview(card, at_position)` 的 holder 按 `root_scale` 缩放。

- [ ] **Step 1: 写失败用例**

在 `client/tests/ui_smoke_layout.gd` 的 `_check_desktop()` 中，把 `page.get_child_count() == 2` 的断言改为：

```gdscript
            var frame := page.get_node_or_null(UI.CONTENT_FRAME_NAME) as Control
            _check(frame != null and frame.get_parent() == page, "%s 应有 ContentFrame" % page_case["path"])
            var root: Control = null
            if frame != null:
                root = frame.get_node_or_null(UI.CONTENT_ROOT_NAME) as Control
            _check(root != null, "%s 的 ContentFrame 应包含 ContentRoot" % page_case["path"])
            if root != null:
                _check(root.size.is_equal_approx(Vector2(ScreenProfile.base_canvas())),
                    "%s 的 ContentRoot 应为基准尺寸" % page_case["path"])
                var ratio := root.size.x / root.size.y
                _check(is_equal_approx(ratio, 16.0 / 9.0), "%s 的 ContentRoot 应保持基准比例" % page_case["path"])
            _check(page.get_child_count() == 2, "%s 根节点应为背景 + ContentFrame" % page_case["path"])
```

把 `_safe_area` 与 `_count_safe_areas` 的查找改为递归：

```gdscript
func _safe_area(page: Node) -> SafeAreaMargin:
    return page.find_child(UI.SAFE_AREA_NAME, true, false) as SafeAreaMargin


func _count_safe_areas(page: Node) -> int:
    return _count_named(page, UI.SAFE_AREA_NAME)


func _count_named(node: Node, node_name: String) -> int:
    var count := 0
    for child in node.get_children():
        if str(child.name) == node_name:
            count += 1
        count += _count_named(child, node_name)
    return count
```

新增拖拽预览用例（放在 `_check_desktop()` 后）：

```gdscript
## 拖拽预览在缩放内容框下仍与卡片同尺寸、起手点对齐。
func _check_drag_preview() -> void:
    var page := Control.new()
    page.size = Vector2(1280, 480)
    add_child(page)
    var area := UI.safe_area(page)
    var card := Control.new()
    card.size = Vector2(84, 92)
    area.add_child(card)
    var scale := UI.root_scale(card)
    var preview := UI.drag_preview(card, Vector2(10, 20))
    _check(is_equal_approx(preview.scale.x, scale), "拖拽预览应按内容框缩放")
    _check(card.position.is_equal_approx(Vector2(-10, -20) / scale), "拖拽预览起手点应对齐")
    page.queue_free()
    await get_tree().process_frame
```

在 `_ready()` 中 `await _check_drag_preview()`。

- [ ] **Step 2: 运行确认失败**

Run: `make client-smoke`
Expected: FAIL，`LAYOUT_SMOKE_FAIL` 报缺少 `ContentFrame` 或 `root_scale`。

- [ ] **Step 3: 实现层级、背景与预览**

在 `client/ui/ui.gd`：

```gdscript
const CONTENT_FRAME_NAME := "ContentFrame"
const CONTENT_ROOT_NAME := "ContentRoot"
```

`background()` 的 `rect.stretch_mode` 改为 `TextureRect.STRETCH_KEEP_ASPECT_COVERED`。

`safe_area` 改为创建 `ContentFrame → ContentRoot → SafeAreaMargin`，并把非背景子节点移入安全区；末尾调用 `_layout_content(page, frame, root)`，并连接 `page.resized` 重算（用 `Callable` 去重）。

```gdscript
static func safe_area(page: Control) -> SafeAreaMargin:
    ...
    var frame := Control.new()
    frame.name = CONTENT_FRAME_NAME
    frame.mouse_filter = Control.MOUSE_FILTER_IGNORE
    page.add_child(frame)
    var root := Control.new()
    root.name = CONTENT_ROOT_NAME
    root.size = Vector2(ScreenProfile.base_canvas())
    root.mouse_filter = Control.MOUSE_FILTER_IGNORE
    frame.add_child(root)
    var area := SafeAreaMargin.new()
    area.name = SAFE_AREA_NAME
    root.add_child(area)
    area.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    ...（移动既有子节点，背景仍置底）
    _layout_content(page, frame, root)
    if not page.has_meta("content_layout_connected"):
        page.set_meta("content_layout_connected", true)
        page.resized.connect(_layout_content.bind(page, frame, root))
    return area


static func _layout_content(page: Control, frame: Control, root: Control) -> void:
    var base := ScreenProfile.base_canvas()
    var rect := ScreenProfile.content_frame(page.size, base)
    if rect.size.x <= 0.0 or rect.size.y <= 0.0:
        return
    frame.position = rect.position
    frame.size = rect.size
    root.scale = Vector2(rect.size.y / float(base.y), rect.size.y / float(base.y))
```

新增：

```gdscript
## 向上查找内容框逻辑画布的缩放；不在内容框内时返回 1。
static func root_scale(node: Node) -> float:
    var current := node
    while current != null:
        if current is Control and str(current.name) == CONTENT_ROOT_NAME:
            return (current as Control).scale.x
        current = current.get_parent()
    return 1.0
```

`drag_preview`：

```gdscript
static func drag_preview(card: Control, at_position: Vector2) -> Control:
    var holder := Control.new()
    holder.name = "DragPreview"
    holder.mouse_filter = Control.MOUSE_FILTER_IGNORE
    var scale := root_scale(card)
    holder.scale = Vector2(scale, scale)
    card.mouse_filter = Control.MOUSE_FILTER_IGNORE
    card.position = -at_position / scale
    holder.add_child(card)
    return holder
```

- [ ] **Step 4: 运行确认通过**

Run: `make client-smoke`
Expected: `UI_SMOKE_OK`，退出码 0。

- [ ] **Step 5: 提交**

```bash
git add client/ui/ui.gd client/tests/ui_smoke_layout.gd
git commit -m "feat(client): 页面内容套 16:9 内容框并对齐拖拽预览"
```

---

### Task 3: 启动窗口保持 16:9、拖动不再回弹

**Files:**
- Modify: `client/autoload/window_sizer.gd`
- Test: `client/tests/ui_smoke_platform.gd`（Task 1 的 `launch_window_size` 用例已覆盖计算）

**Interfaces:**
- Consumes: `ScreenProfile.launch_window_size`（Task 1）。
- Produces: 启动窗口按可用区域 90% 收拢成 16:9；不再订阅 `size_changed`。

- [ ] **Step 1: 确认基线**

启动尺寸的计算与用例已在 Task 1 完成。运行：

Run: `make client-smoke`
Expected: 通过，`UI_SMOKE_OK`。

- [ ] **Step 2: 实现启动收拢并删除回弹**

`client/autoload/window_sizer.gd`：

```gdscript
func _ready() -> void:
    if DisplayServer.get_name() == "headless":
        get_window().size = ScreenProfile.base_canvas()
        return
    if ScreenProfile.is_touch():
        return
    var usable := _usable_rect()
    if usable.size.x <= 0 or usable.size.y <= 0:
        return
    var target := ScreenProfile.launch_window_size(usable.size, ScreenProfile.base_canvas())
    var window := get_window()
    window.size = target
    window.position = usable.position + (usable.size - target) / 2
```

删除 `_adjusting`、`_last_valid`、`_keep_aspect_floor` 与 `window.size_changed.connect`。

- [ ] **Step 3: 运行确认通过**

Run: `make client-smoke` 与 `make client-check`
Expected: 均通过。

- [ ] **Step 4: 目标设备验证启动窗口**

Run: `make client-run`
Expected: 16:10 的屏幕上启动窗口为 16:9，上下无背景带；拖成 4:3 后上下出现背景带且窗口不再弹回。

- [ ] **Step 5: 提交**

```bash
git add client/autoload/window_sizer.gd
git commit -m "feat(client): 启动按 16:9 开窗，拖动不再回弹比例"
```

---

### Task 4: 覆盖层改用内容框局部坐标

**Files:**
- Modify: `client/scenes/game_aim_layer.gd`
- Modify: `client/scenes/game_detail_layer.gd`
- Modify: `client/scenes/room_hero_detail_layer.gd`
- Modify: `client/scenes/game_players_rail.gd`
- Test: `client/tests/ui_smoke_layout.gd`

**Interfaces:**
- Consumes: `ContentRoot.scale`（Task 2）、Godot `CanvasItem.to_local(global_point)`。
- Produces: 各层内部统一使用层局部坐标；对外入口（`begin_skill`、`begin_pending`、`show_for`、`track`）仍接收全局 `Rect2` / `Control`，进入时立即转换。

- [ ] **Step 1: 写失败用例**

在 `client/tests/ui_smoke_layout.gd` 新增：

```gdscript
## 超宽视口下瞄准层把全局矩形换算到内容框局部坐标。
func _check_aim_local() -> void:
    var host := Control.new()
    host.name = "AimHost"
    host.size = Vector2(1280, 480)
    add_child(host)
    var page := await _instantiate_into("res://scenes/game.tscn", host)
    if page != null:
        var aim := page.find_child("AimLayer", true, false) as Control
        _check(aim != null, "对局页缺少 AimLayer")
        if aim != null:
            var global := Rect2(400, 100, 80, 80)
            aim.begin_skill(global, [])
            var local := aim.source_rect_local()
            _check(local.position.is_equal_approx(aim.to_local(global.position)),
                "瞄准层源矩形应转为局部坐标")
            _check(is_equal_approx(local.size.x, global.size.x / aim.get_global_transform().get_scale().x),
                "瞄准层源矩形尺寸应除以内容框缩放")
            _check(Rect2(Vector2.ZERO, aim.size).encloses(local), "瞄准层源矩形应落在层内")
            aim.clear()
    host.queue_free()
    await get_tree().process_frame
```

在 `_ready()` 中 `await _check_aim_local()`。

- [ ] **Step 2: 运行确认失败**

Run: `make client-smoke`
Expected: FAIL，`source_rect_local` 不存在或坐标不符。

- [ ] **Step 3: 实现局部坐标**

`game_aim_layer.gd`：
- `_canvas_point` 末尾追加 `to_local(...)`。
- `_source_rect`、`_board_rect()`、`_candidate_rect()` 返回的全局 `Rect2` 统一经 `_local_rect()` 转换：

```gdscript
func _local_rect(global: Rect2) -> Rect2:
    var scale := get_global_transform().get_scale().x
    return Rect2(to_local(global.position), global.size / scale)


func _local_point(global: Vector2) -> Vector2:
    return to_local(global)
```

- 新增只读方法 `func source_rect_local() -> Rect2: return _source_rect`。
- `begin_skill` / `begin_pending` 接收的 `source_rect` 与候选 `Rect2` 在存储前经 `_local_rect`。

`game_detail_layer.gd`：
- `_canvas_point` 追加 `to_local`。
- 所有 `card.get_global_rect()` 经 `_local_rect()`；`_place_detail` 的夹紧边界改用 `Rect2(Vector2.ZERO, size)`；`_panel.position = pos`（不再用全局坐标加减）。
- `_on_card_gui_input` 中 `card.get_global_rect().position + event.position` 改为 `_local_point(card.get_global_rect().position) + event.position`。

`room_hero_detail_layer.gd`：
- `_layout_hero_detail` 的 `layer_rect` 改为 `Rect2(Vector2.ZERO, size)`；传入的 `card_rect` 由调用点 `_local_rect(_hover_card.get_global_rect())` 转换；末尾 `_panel.position = pos`。

`game_players_rail.gd`：
- `_position_report` 的 `layer_rect` 改为 `Rect2(Vector2.ZERO, size)`；`row_rect` 经 `to_local` 与尺寸换算；`_report_panel.position = position`。

- [ ] **Step 4: 运行确认通过**

Run: `make client-smoke` 与 `make client-check`
Expected: 均通过。

- [ ] **Step 5: 目标设备验证**

Run: `make client-run`，把窗口拖成超宽与窄两种尺寸，打开对局页；验证瞄准箭头端点、卡牌详情面板、左栏战报位置正确且不越出内容框。iOS 用 `make client-run-ios` 按 iPhone 检查长按详情与拖拽。

Expected: 覆盖层与卡片对齐，无错位、无越界。

- [ ] **Step 6: 提交**

```bash
git add client/scenes/game_aim_layer.gd client/scenes/game_detail_layer.gd client/scenes/room_hero_detail_layer.gd client/scenes/game_players_rail.gd client/tests/ui_smoke_layout.gd
git commit -m "fix(client): 覆盖层按内容框局部坐标定位与绘制"
```

---

### Task 5: 文档回写与全量验证

**Files:**
- Modify: `docs/current/product/ui/mobile-adaptation.md`
- Modify: `docs/current/technical/client.md`
- Modify: `docs/versions/v0.1/acceptance.md`
- Modify: `docs/current/product/ui/interactions.md`（如含桌面窗口尺寸规则）

**Interfaces:**
- Consumes: Task 1–4 的最终实现。
- Produces: 文档描述与代码一致。

- [ ] **Step 1: 回写文档**

- `mobile-adaptation.md` §2：内容固定 16:9、contain 居中；iPhone 左右背景、iPad 上下背景；安全区落在背景带内记 0。
- `technical/client.md` §9：`ContentFrame` / `ContentRoot` 结构、`content_frame` 与 `launch_window_size`、窗口不再回弹。
- `acceptance.md`：更新「桌面 16:10 实机窗口上下无留白」为「启动 16:9，拖动为 16:10 时上下约 6% 背景」；新增非 16:9 下覆盖层对齐检查项。
- `interactions.md`：同步桌面窗口可自由缩放。

- [ ] **Step 2: 全量验证**

Run: `make m1-check`
Expected: 退出码 0。

Run: `make client-smoke`
Expected: `UI_SMOKE_OK`。

Run: `python3 scripts/validate_docs.py`
Expected: 通过。

- [ ] **Step 3: 提交**

```bash
git add docs/
git commit -m "docs: 回写内容框比例与背景铺满规则"
```
