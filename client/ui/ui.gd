class_name UI
extends RefCounted
## 场景共用的小工具与共享构件；只负责构建 Godot 控件，视觉取值来自 UITheme。


static func clear(node: Node) -> void:
    for child in node.get_children():
        node.remove_child(child)
        child.queue_free()


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
    node.custom_minimum_size = Vector2(0, 34)
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
static func styled_button(text: String, variant := "default", min_height := 30) -> Button:
    var node := Button.new()
    node.text = text
    node.custom_minimum_size = Vector2(0, min_height)
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


## 英雄头像块：没有图片资源，用部落色块 + 名字首字表示。
static func avatar(hero_id: String, size := 34) -> PanelContainer:
    var node := PanelContainer.new()
    node.name = "Avatar"
    node.custom_minimum_size = Vector2(size, size)
    var tribe := str(CardDB.card(hero_id).get("tribe", ""))
    var tint := UITheme.tribe_color(tribe) if hero_id != "" else UITheme.color("color-border-strong")
    var box := StyleBoxFlat.new()
    box.bg_color = tint.darkened(0.55)
    box.border_color = tint
    box.set_border_width_all(2)
    box.set_corner_radius_all(int(size * 0.3))
    node.add_theme_stylebox_override("panel", box)
    var text := label(_avatar_initial(hero_id), int(size * 0.42), false)
    text.name = "Initial"
    text.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    text.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    text.add_theme_color_override("font_color", tint.lightened(0.5))
    node.add_child(text)
    return node


## 名次角标：头像左上角的金色小方块，已淘汰也继续显示数字。
static func rank_badge(rank: int) -> PanelContainer:
    var size := UITheme.metric("rank-badge-size")
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
    var height := UITheme.metric("seat-hp-bar-height")
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


## 英雄小卡：头像、名字、部落与时代、技能摘要。hero_id 为空表示未选英雄。
static func hero_tile(hero_id: String, width := 150) -> PanelContainer:
    var node := PanelContainer.new()
    node.name = "HeroTile"
    node.custom_minimum_size = Vector2(width, 0)
    var box := UITheme.panel_style("raised")
    if hero_id != "":
        box.border_color = UITheme.tribe_color(str(CardDB.card(hero_id).get("tribe", "")))
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
        CardDB.display_name(hero_id) if hero_id != "" else "未选英雄",
        UITheme.font_size("font-title"),
        false,
    )
    name_label.name = "Name"
    lines.add_child(name_label)

    var entry := CardDB.card(hero_id)
    var meta_parts := PackedStringArray()
    var tribe := str(entry.get("tribe", ""))
    var era := str(entry.get("era", ""))
    if tribe != "":
        meta_parts.append(tribe)
    if era != "":
        meta_parts.append(era)
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


const ERROR_TEXT := {
    "ALREADY_MAX_ERA": "时代已满",
    "AUTH_FAILED": "登录失败",
    "AUTH_RATE_LIMITED": "登录过于频繁，请稍后再试",
    "BAD_PAYLOAD": "请求格式不正确",
    "CARD_NOT_IN_HAND": "手牌中没有这张牌",
    "HERO_NOT_SELECTED": "还没有选择英雄",
    "HERO_POWER_LIMIT": "英雄技能次数已用完",
    "INTERNAL_ERROR": "服务端内部错误",
    "INVALID_CHOICE": "选择无效",
    "INVALID_STAGE": "当前阶段不能执行该操作",
    "NOT_ENOUGH_GOLD": "金币不足",
    "NOT_HOST": "只有房主可以操作",
    "NOT_IN_ROOM": "已不在房间中",
    "NO_SPACE": "棋盘或手牌已满",
    "NO_SUCH_MINION": "随从不存在",
    "NO_VALID_TARGET": "需要选择目标",
    "RATE_LIMITED": "操作过于频繁，请稍后再试",
    "ROOM_FULL": "房间已满",
    "ROOM_NOT_FOUND": "房间不存在",
    "ROOM_NOT_READY": "房间尚未满足开局条件",
    "SEQUENCE_STALE": "消息序号过期",
    "SESSION_REPLACED": "账号已在其他位置登录",
    "SHOP_SLOT_EMPTY": "商店槽位为空",
    "TARGET_NOT_ALLOWED": "该操作不能指定目标",
    "TOKEN_EXPIRED": "登录已过期，请重新登录",
    "UNKNOWN_TYPE": "未知消息类型",
}


static func error_text(code: String) -> String:
    return ERROR_TEXT.get(code, code)


## 错误码对应的页面去向：login、lobby 或空串（留在当前页面）。
static func error_route(code: String) -> String:
    if code in ["AUTH_FAILED", "TOKEN_EXPIRED", "SESSION_REPLACED"]:
        return "login"
    if code in ["NOT_IN_ROOM", "ROOM_NOT_FOUND"]:
        return "lobby"
    return ""
