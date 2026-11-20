class_name UI
extends RefCounted
## 场景共用的小工具与共享构件；只负责构建 Godot 控件，视觉取值来自 UITheme。


const SAFE_AREA_NAME := "SafeAreaMargin"
const SAFE_LAYER_NAME := "SafeLayer"


static func clear(node: Node) -> void:
    for child in node.get_children():
        node.remove_child(child)
        child.queue_free()


## 拖拽预览：让指针停在起手时的卡片位置，而不是 Godot 默认的预览左上角贴指针。
##
## Viewport 在拖拽中每帧把预览控件的位置设成指针位置，直接偏移预览会被覆盖；
## 这里用包裹层承载被拖卡片，并按起手点 `at_position` 反向偏移。
static func drag_preview(card: Control, at_position: Vector2) -> Control:
    var holder := Control.new()
    holder.name = "DragPreview"
    holder.mouse_filter = Control.MOUSE_FILTER_IGNORE
    card.mouse_filter = Control.MOUSE_FILTER_IGNORE
    card.position = -at_position
    holder.add_child(card)
    return holder


## 页面安全区包裹：把页面现有子节点整体移入 SafeAreaMargin，再挂回页面。
## 幂等：重复调用返回已存在的包裹层，不重复移动节点。
static func safe_area(page: Control) -> SafeAreaMargin:
    if page == null:
        return null
    var existing := _safe_area_of(page)
    if existing != null:
        return existing
    var area := SafeAreaMargin.new()
    area.name = SAFE_AREA_NAME
    page.add_child(area)
    # 包裹层创建时页面可能已有尺寸：只设锚点会把当前零尺寸保留成负偏移。
    area.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    for child in page.get_children():
        if child == area:
            continue
        page.remove_child(child)
        # 重新挂入会触发 _exit_tree：依赖 _ready 重新建立订阅的组件需要再就绪一次。
        child.request_ready()
        area.add_child(child)
    return area


## 动态全屏覆盖层的挂载点：位于安全区内、内容之上，自身不拦截输入。
static func safe_layer(page: Control) -> Control:
    var area := safe_area(page)
    if area == null:
        return null
    var layer := area.get_node_or_null(SAFE_LAYER_NAME) as Control
    if layer == null:
        layer = Control.new()
        layer.name = SAFE_LAYER_NAME
        layer.mouse_filter = Control.MOUSE_FILTER_IGNORE
        area.add_child(layer)
        layer.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
    return layer


static func _safe_area_of(page: Control) -> SafeAreaMargin:
    for child in page.get_children():
        if child is SafeAreaMargin:
            return child
    return null


## 页面导航入口。change_scene_to_file 会立即把当前场景摘下，摘下后的页面仍连着
## GameStore 等 autoload 信号，回调再取场景树会拿到 null；这里以「是否仍在树上」
## 作为唯一判定，离树页面的导航直接丢弃。
static func navigate(page: Node, path: String) -> bool:
    if page == null or not is_instance_valid(page) or not page.is_inside_tree():
        return false
    var tree := page.get_tree()
    if tree == null:
        return false
    return tree.change_scene_to_file(path) == OK


static func label(text: String, size := 16, wrap := true) -> Label:
    var node := Label.new()
    node.text = text
    node.add_theme_font_size_override("font_size", size)
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


static func panel() -> PanelContainer:
    return PanelContainer.new()


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


## 英雄头像块：没有图片资源，用玩法色块 + 名字首字表示。
static func avatar(hero_id: String, size := 34) -> PanelContainer:
    var node := PanelContainer.new()
    node.name = "Avatar"
    node.custom_minimum_size = Vector2(size, size)
    var playstyle := str(CardDB.card(hero_id).get("playstyle", ""))
    var tint := UITheme.playstyle_color(playstyle) if hero_id != "" else UITheme.color("color-border-strong")
    var box := StyleBoxFlat.new()
    box.bg_color = tint.darkened(0.55)
    box.border_color = tint
    box.set_border_width_all(2)
    box.set_corner_radius_all(int(size * 0.3))
    node.add_theme_stylebox_override("panel", box)
    var avatar_path := CardDB.art_path(hero_id, "avatar")
    if avatar_path != "" and ResourceLoader.exists(avatar_path):
        var texture := ResourceLoader.load(avatar_path) as Texture2D
        if texture != null:
            var art := TextureRect.new()
            art.name = "AvatarArt"
            art.texture = texture
            art.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
            art.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
            art.set_anchors_preset(Control.PRESET_FULL_RECT)
            art.mouse_filter = Control.MOUSE_FILTER_IGNORE
            node.add_child(art)
            return node
    var text := label(_avatar_initial(hero_id), int(size * 0.42), false)
    text.name = "Initial"
    text.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    text.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    text.add_theme_color_override("font_color", tint.lightened(0.5))
    node.add_child(text)
    return node


## 名次角标：头像左上角的金色小方块，已淘汰也继续显示数字。
static func rank_badge(rank: int) -> PanelContainer:
    var size := UITheme.stage_metric("rank-badge-size")
    var node := PanelContainer.new()
    node.name = "RankBadge"
    node.custom_minimum_size = Vector2(size, size)
    node.size = Vector2(size, size)
    node.mouse_filter = Control.MOUSE_FILTER_IGNORE

    var box := StyleBoxFlat.new()
    box.bg_color = UITheme.color("color-accent-strong")
    box.border_color = UITheme.color("color-accent")
    box.set_border_width_all(1)
    box.set_corner_radius_all(UITheme.radius("radius-sm"))
    node.add_theme_stylebox_override("panel", box)

    var text := label(str(rank) if rank > 0 else "—", UITheme.font_size("font-caption"), false)
    text.name = "RankText"
    text.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    text.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    text.mouse_filter = Control.MOUSE_FILTER_IGNORE
    text.add_theme_color_override("font_color", UITheme.color("color-bg"))
    node.add_child(text)
    return node


## 血量条：固定宽度、不显示数字；`value` 超出 [0, cap] 时填充夹到两端。
static func hp_bar(value: int, cap: int, width: int) -> Control:
    var height := UITheme.stage_metric("seat-hp-bar-height")
    var node := Control.new()
    node.name = "HpBar"
    node.custom_minimum_size = Vector2(width, height)
    node.size = Vector2(width, height)
    node.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
    node.mouse_filter = Control.MOUSE_FILTER_IGNORE

    var track := ColorRect.new()
    track.name = "HpTrack"
    track.color = UITheme.color("color-surface-sunken")
    track.position = Vector2.ZERO
    track.size = Vector2(width, height)
    track.mouse_filter = Control.MOUSE_FILTER_IGNORE
    node.add_child(track)

    var safe_cap := maxi(cap, 1)
    var clamped := clampi(value, 0, safe_cap)
    var fill_width := clampi(int(round(float(width) * float(clamped) / float(safe_cap))), 0, width)
    var fill := ColorRect.new()
    fill.name = "HpFill"
    fill.color = UITheme.color("color-success")
    fill.position = Vector2.ZERO
    fill.size = Vector2(fill_width, height)
    fill.mouse_filter = Control.MOUSE_FILTER_IGNORE
    node.add_child(fill)
    return node


static func _avatar_initial(hero_id: String) -> String:
    if hero_id == "":
        return "?"
    var display := CardDB.display_name(hero_id)
    if display == "":
        return "?"
    return display.substr(0, 1)


## 英雄小卡：头像、名字、玩法与时代、技能摘要。hero_id 为空表示未选英雄。
static func hero_tile(hero_id: String, width := 150) -> PanelContainer:
    var node := PanelContainer.new()
    node.name = "HeroTile"
    node.custom_minimum_size = Vector2(width, 0)
    var box := UITheme.panel_style("raised")
    if hero_id != "":
        box.border_color = UITheme.playstyle_color(str(CardDB.card(hero_id).get("playstyle", "")))
    node.add_theme_stylebox_override("panel", box)

    var row := HBoxContainer.new()
    row.add_theme_constant_override("separation", UITheme.space("space-md"))
    node.add_child(row)
    row.add_child(avatar(hero_id, UITheme.metric("avatar-lg")))

    var lines := VBoxContainer.new()
    lines.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    lines.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    row.add_child(lines)

    var name_label := label(
        CardDB.display_name(hero_id) if hero_id != "" else TranslationServer.translate("ui.common.no_hero"),
        UITheme.font_size("font-title"),
        false,
    )
    name_label.name = "Name"
    lines.add_child(name_label)

    var entry := CardDB.card(hero_id)
    var meta_parts := PackedStringArray()
    var playstyle := str(entry.get("playstyle", ""))
    var era := str(entry.get("era", ""))
    if playstyle != "":
        meta_parts.append(enum_text("playstyle", playstyle))
    if era != "":
        meta_parts.append(enum_text("era", era))
    var meta := label(" · ".join(meta_parts) if not meta_parts.is_empty() else "—", UITheme.font_size("font-caption"), false)
    meta.name = "Meta"
    meta.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    lines.add_child(meta)

    var skill_text := CardDB.display_text(hero_id)
    if skill_text != "":
        var skill := label(skill_text, UITheme.font_size("font-caption"))
        skill.name = "Skill"
        skill.max_lines_visible = 2
        skill.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
        skill.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
        lines.add_child(skill)
    return node


## 淘汰或失效的行：统一降低透明度，不改变布局。
static func apply_dim(node: CanvasItem) -> void:
    node.modulate = Color(1, 1, 1, UITheme.opacity("opacity-dim"))


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


static func margin(container: Control, value := 12) -> MarginContainer:
    var node := MarginContainer.new()
    node.add_theme_constant_override("margin_left", value)
    node.add_theme_constant_override("margin_right", value)
    node.add_theme_constant_override("margin_top", value)
    node.add_theme_constant_override("margin_bottom", value)
    node.add_child(container)
    return node


## 按枚举组与英文 id 取展示文案；缺键回退为 id。
static func enum_text(group: String, id: String) -> String:
    var key := "enum.%s.%s" % [group, id]
    var text := TranslationServer.translate(key)
    return id if text == key else text


## 错误码对应的展示文案；缺键回退为错误码。
static func error_text(code: String) -> String:
    var key := "error.%s" % code
    var text := TranslationServer.translate(key)
    return code if text == key else text


## 按完整文本 id 取错误详情；空 id 或缺键返回空串，params 非空时按 % 格式化。
static func error_text_by_id(id: String, params: Array = []) -> String:
    if id == "":
        return ""
    var text := TranslationServer.translate(id)
    if text == id:
        return ""
    if not params.is_empty():
        text = text % params
    return text


## 错误提示的悬停详情：优先用服务端给出的 messageId 与参数，缺失时回退错误码文案。
static func error_tooltip(code: String) -> String:
    var raw_params = GameStore.last_error.get("messageParams", [])
    var params: Array = raw_params if raw_params is Array else []
    var detail := error_text_by_id(
        str(GameStore.last_error.get("messageId", "")),
        params,
    )
    return detail if detail != "" else error_text(code)


## 错误码对应的页面去向：login、lobby 或空串（留在当前页面）。
static func error_route(code: String) -> String:
    if code in ["AUTH_FAILED", "TOKEN_EXPIRED", "SESSION_REPLACED"]:
        return "login"
    if code in ["NOT_IN_ROOM", "ROOM_NOT_FOUND"]:
        return "lobby"
    return ""


## RFC3339 时间转 Unix 秒；共享给需要按 `phaseDeadline` 反推进度的视图。
static func unix_from_rfc3339(value: String) -> float:
    var text := value
    if text.ends_with("Z"):
        text = text.substr(0, text.length() - 1)
    var dot := text.find(".")
    if dot >= 0:
        text = text.substr(0, dot)
    return Time.get_unix_time_from_datetime_string(text)
