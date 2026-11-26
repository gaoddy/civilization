class_name UIControls
extends RefCounted
## 通用控件工厂：标签、按钮、面板、背景与区块标题。
##
## 只构建 Godot 控件，视觉取值来自 UITheme；业务状态由调用方设置。

const PAGE_SURFACE_NAME := "PageSurface"


static func clear(node: Node) -> void:
    for child in node.get_children():
        # 材质层是挂在控件上的表面装饰，不是内容：清空内容时保留，避免边框闪没。
        if child.name == FrameMaterial.OVERLAY_NAME:
            continue
        node.remove_child(child)
        child.queue_free()


## 标签：省略字号时落到正文字号令牌，调用方只在需要其他档位时传令牌值。
static func label(text: String, size := -1, wrap := true) -> Label:
    var node := Label.new()
    node.text = text
    var resolved := UITheme.font_size("font-body") if size < 0 else size
    node.add_theme_font_size_override("font_size", resolved)
    node.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART if wrap else TextServer.AUTOWRAP_OFF
    return node


static func button(text: String) -> Button:
    var node := Button.new()
    node.text = text
    var height := 34
    if ScreenProfile.is_touch():
        height = UITheme.hit_primary()
    node.custom_minimum_size = Vector2(0, height)
    return node


## 输入框：桌面与按钮同高，触控端按主操作命中尺寸抬升；宽度由调用方按场景给出。
static func line_edit(placeholder := "", min_width := 0) -> LineEdit:
    var node := LineEdit.new()
    node.placeholder_text = placeholder
    var height := 34
    if ScreenProfile.is_touch():
        height = UITheme.hit_primary()
    node.custom_minimum_size = Vector2(min_width, height)
    return node


static func panel() -> PanelContainer:
    var node := PanelContainer.new()
    FrameMaterial.apply(node, FrameMaterial.Style.BEVEL)
    return node


## 软件背景：铺满整个窗口的深底纹理；纹理缺失时退回纯色，不留空白页。
## 页面传入自己的氛围图（`PageBackgrounds.ambient(page)`），不传时用共用底图。
static func background(path := "res://assets/ui/bg/bg_main.png") -> Control:
    if ResourceLoader.exists(path):
        var texture := ResourceLoader.load(path) as Texture2D
        if texture != null:
            var rect := TextureRect.new()
            rect.name = "PageBackground"
            rect.texture = texture
            rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
            rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
            rect.set_anchors_preset(Control.PRESET_FULL_RECT)
            rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
            return rect
    var fallback := ColorRect.new()
    fallback.name = "PageBackground"
    fallback.color = UITheme.color("color-bg")
    fallback.set_anchors_preset(Control.PRESET_FULL_RECT)
    fallback.mouse_filter = Control.MOUSE_FILTER_IGNORE
    return fallback


## 页面容器背景：设计画布内的一层，覆盖 `PageBody` 整块。
## 各页面传入自己的 16:9 结构框架图（`PageBackgrounds.surface(page)`），按同尺寸拉伸，
## 分区边界与控件矩形对齐；不传或纹理缺失时退回统一语义面板，不留空白页。
static func page_surface(texture_path := "") -> Control:
    if texture_path != "" and ResourceLoader.exists(texture_path):
        var texture := ResourceLoader.load(texture_path) as Texture2D
        if texture != null:
            var rect := TextureRect.new()
            rect.name = PAGE_SURFACE_NAME
            rect.texture = texture
            rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
            rect.stretch_mode = TextureRect.STRETCH_SCALE
            rect.set_anchors_preset(Control.PRESET_FULL_RECT)
            rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
            return rect
    var panel := Panel.new()
    panel.name = PAGE_SURFACE_NAME
    panel.add_theme_stylebox_override("panel", UITheme.panel_style("page"))
    panel.set_anchors_preset(Control.PRESET_FULL_RECT)
    panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
    return panel


## 带视觉令牌的卡片容器；variant 见 UITheme.panel_style。
static func card_panel(variant := "surface") -> PanelContainer:
    var node := PanelContainer.new()
    node.add_theme_stylebox_override("panel", UITheme.panel_style(variant))
    # 卡面信息条（cover-info）贴在卡框里面，由卡面统一描边，不单独上材质。
    if variant != "cover-info":
        FrameMaterial.apply(node, FrameMaterial.Style.BEVEL)
    return node


## 胶囊标签：数值、状态与计时使用。variant：neutral / accent / success / danger / info / plain。
## 胶囊是数字与计时的承载面：带底色的变体统一铺一层数字底板；
## `plain` 只留文字，不参与底板，避免在已有面板上再叠一层框。
static func pill_label(text: String, variant := "neutral", font_token := "font-body") -> Label:
    var node := label(text, UITheme.font_size(font_token), false)
    var box := UITheme.pill_style(variant)
    # 底板铺在胶囊底色之下：不透明变体把底色压半透，底板才透得出来。
    box.bg_color.a = minf(box.bg_color.a, 0.55)
    node.add_theme_stylebox_override("normal", box)
    node.add_theme_color_override("font_color", _pill_text_color(variant))
    node.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    if variant != "plain":
        FrameMaterial.apply(node, FrameMaterial.Style.BEVEL, {
            "tint": _pill_tint(variant),
            "surface": true,
        })
    return node


## 给单个标签补数字底板：透明面板 + 内边距 + 底板材质。
## 用于没有整块底色的倒计时与数字；调用方仍持有标签本身，面板只承担表面。
static func plate_wrap(label: Label, tint = null) -> PanelContainer:
    var panel := PanelContainer.new()
    panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
    # 底板贴着数字收口，不随容器拉成整条横带。
    panel.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    var style := StyleBoxFlat.new()
    style.bg_color = UITheme.color("color-transparent")
    style.set_corner_radius_all(UITheme.radius("radius-md"))
    panel.add_theme_stylebox_override("panel", style)
    var pad := MarginContainer.new()
    for side in ["left", "right"]:
        pad.add_theme_constant_override("margin_" + side, UITheme.space("space-lg"))
    for side in ["top", "bottom"]:
        pad.add_theme_constant_override("margin_" + side, UITheme.space("space-sm"))
    pad.add_child(label)
    panel.add_child(pad)
    FrameMaterial.apply(panel, FrameMaterial.Style.BEVEL, {
        "tint": tint if tint != null else UITheme.color("color-accent"),
        "surface": true,
    })
    return panel


static func _pill_tint(variant: String) -> Color:
    match variant:
        "accent":
            return UITheme.color("color-accent")
        "success":
            return UITheme.color("color-success")
        "danger":
            return UITheme.color("color-danger")
        "info":
            return UITheme.color("color-info")
        _:
            return UITheme.color("color-border")


static func _pill_text_color(variant: String) -> Color:
    match variant:
        "accent":
            return UITheme.color("color-accent")
        "success":
            return UITheme.color("color-success")
        "danger":
            return UITheme.color("color-danger")
        "info":
            return UITheme.color("color-info")
        _:
            return UITheme.color("color-text")


## 统一按钮外观。variant：default / primary / danger / ghost / slot / micro。
## `design_height` 是视觉高度：传入即按该值固定，三端一致；
## 省略时取 `action-button-height`，避免漏传参数时被触控命中撑大。
static func styled_button(text: String, variant := "default", design_height := -1) -> Button:
    var node := Button.new()
    node.text = text
    var height := design_height if design_height >= 0 else UITheme.stage_metric("action-button-height")
    node.custom_minimum_size = Vector2(0, height)
    apply_button_style(node, variant)
    return node


## 给已有 Button 套用统一按钮样式；Button 子类复用它，避免重复样式代码。
static func apply_button_style(node: Button, variant := "default") -> void:
    node.focus_mode = Control.FOCUS_ALL
    for state in ["normal", "hover", "pressed", "focus", "disabled"]:
        node.add_theme_stylebox_override(state, UITheme.button_style(variant, state))
    var text_color := UITheme.color("color-text")
    if variant == "primary":
        text_color = UITheme.color("color-bg")
    node.add_theme_color_override("font_color", text_color)
    node.add_theme_color_override("font_hover_color", text_color)
    node.add_theme_color_override("font_pressed_color", text_color)
    node.add_theme_color_override("font_focus_color", text_color)
    node.add_theme_color_override("font_disabled_color", UITheme.color("color-text-muted"))
    node.add_theme_font_size_override("font_size", UITheme.font_size("font-body"))
    # 框体材质：按钮是可操作表面，默认带引导流光；重复调用只更新参数。
    FrameMaterial.apply(node, FrameMaterial.Style.BEVEL, {"guide": true})


## 淘汰或失效的行：统一降低透明度，不改变布局。
static func apply_dim(node: CanvasItem) -> void:
    node.modulate = UITheme.alpha("color-white", UITheme.opacity("opacity-dim"))


## 空态/占位提示，居中且不抢排版。
static func empty_hint(text: String) -> Label:
    var node := label(text, UITheme.font_size("font-body-sm"), false)
    node.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    node.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    node.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    return node


## 小标题：区块名与计数。
static func section_title(text: String) -> Label:
    var node := label(text, UITheme.font_size("font-body"), false)
    node.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    return node


## 统一边距容器：省略数值时四边落到大号间距令牌。
static func margin(container: Control, value := -1) -> MarginContainer:
    var resolved := UITheme.space("space-lg") if value < 0 else value
    var node := MarginContainer.new()
    node.add_theme_constant_override("margin_left", resolved)
    node.add_theme_constant_override("margin_right", resolved)
    node.add_theme_constant_override("margin_top", resolved)
    node.add_theme_constant_override("margin_bottom", resolved)
    node.add_child(container)
    return node
