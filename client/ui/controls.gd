class_name UIControls
extends RefCounted
## 通用控件工厂：标签、按钮、面板、背景与区块标题。
##
## 只构建 Godot 控件，视觉取值来自 UITheme；业务状态由调用方设置。

const PAGE_SURFACE_NAME := "PageSurface"


static func clear(node: Node) -> void:
    for child in node.get_children():
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
    return PanelContainer.new()


## 软件背景：铺满整个窗口的深底纹理；纹理缺失时退回纯色，不留空白页。
## 所有页面统一使用共用底图，页面自己的背景交给 `page_surface`。
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
## 传入对局分区图时按同尺寸拉伸（内容框与图同为 16:9，不产生裁切）；
## 不传或纹理缺失时退回统一语义面板，不留空白页。
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
    return node


## 胶囊标签：数值、状态与计时使用。variant：neutral / accent / success / danger / info / plain。
static func pill_label(text: String, variant := "neutral", font_token := "font-body") -> Label:
    var node := label(text, UITheme.font_size(font_token), false)
    node.add_theme_stylebox_override("normal", UITheme.pill_style(variant))
    node.add_theme_color_override("font_color", _pill_text_color(variant))
    node.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    return node


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
## 触控端按命中尺寸抬升最小高度，micro 使用较小档；桌面端保留原值。
static func styled_button(text: String, variant := "default", min_height := 30) -> Button:
    var node := Button.new()
    node.text = text
    var height := min_height
    if ScreenProfile.is_touch():
        var target := UITheme.hit_small() if variant == "micro" else UITheme.hit_primary()
        height = maxi(min_height, target)
    node.custom_minimum_size = Vector2(0, height)
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
    return node


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
