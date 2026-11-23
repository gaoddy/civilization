extends Control
## 个人主页：当前段位与积分、历史最高段位、最近 10 局名次与前 4 名比例。
##
## 数据全部来自服务端 `PlayerProfile`；最近战绩是否可见由服务端 `recentVisible`
## 决定，客户端不做额外判断，也不在本地算分。

var _content: VBoxContainer
var _status: Label
var _return_page := ""


func _ready() -> void:
    UILayout.safe_area(self)
    _return_page = GameStore.profile_return_page
    if _return_page == "":
        _return_page = UINavigation.PAGE_LOBBY
    _build()
    GameStore.profile_changed.connect(_render)
    _render()
    GameStore.request_profile(GameStore.profile_target)


func _build() -> void:
    add_child(UIControls.background())

    var margin := MarginContainer.new()
    margin.set_anchors_preset(Control.PRESET_FULL_RECT)
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override("margin_" + side, UITheme.space("space-2xl"))
    add_child(margin)

    var layout := VBoxContainer.new()
    layout.add_theme_constant_override("separation", UITheme.space("space-lg"))
    margin.add_child(layout)

    var header := HBoxContainer.new()
    header.add_theme_constant_override("separation", UITheme.space("space-lg"))
    layout.add_child(header)
    header.add_child(UIControls.label(tr("ui.profile.title"), UITheme.font_size("font-hero"), false))
    var spacer := Control.new()
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    header.add_child(spacer)
    var back := UIControls.button(tr("ui.profile.back"))
    back.name = "BackButton"
    back.pressed.connect(_on_back_pressed)
    header.add_child(back)

    var scroll := ScrollContainer.new()
    scroll.name = "ProfileScroll"
    scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
    layout.add_child(scroll)

    _status = UIControls.label(tr("ui.profile.loading"), UITheme.font_size("font-body"))
    _status.name = "ProfileStatus"
    scroll.add_child(_status)

    _content = VBoxContainer.new()
    _content.name = "ProfileContent"
    _content.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _content.add_theme_constant_override("separation", UITheme.space("space-md"))
    scroll.add_child(_content)


func _render() -> void:
    UIControls.clear(_content)
    var profile := _current_profile()
    _status.visible = profile.is_empty()
    _content.visible = not profile.is_empty()
    if profile.is_empty():
        _status.text = tr("ui.profile.loading")
        return

    _content.add_child(UIControls.label(
        str(profile.get("displayName", "")),
        UITheme.font_size("font-title-xl"),
    ))
    _content.add_child(_field_row(
        "ui.profile.tier",
        UIWording.tier_text(str(profile.get("tier", ""))),
        "ProfileTier",
    ))
    _content.add_child(_field_row(
        "ui.profile.rating",
        str(_int_of(profile, "rating")),
        "ProfileRating",
    ))
    _content.add_child(_field_row(
        "ui.profile.highest_tier",
        UIWording.tier_text(str(profile.get("highestTier", ""))),
        "ProfileHighestTier",
    ))

    var visible := bool(profile.get("recentVisible", false))
    var ratio := 0.0
    var raw_ratio = profile.get("top4Ratio", 0.0)
    if raw_ratio != null:
        ratio = float(raw_ratio)
    _content.add_child(_field_row(
        "ui.profile.top4",
        (
            tr("ui.profile.top4_value") % roundi(ratio * 100.0)
            if visible
            else tr("ui.profile.hidden")
        ),
        "ProfileTop4",
    ))

    _content.add_child(UIControls.label(
        tr("ui.profile.recent_title"),
        UITheme.font_size("font-title-xl"),
    ))
    var recent = profile.get("recent", [])
    var recent_list: Array = recent if recent is Array else []
    if not visible:
        _content.add_child(UIControls.label(
            tr("ui.profile.recent_hidden"),
            UITheme.font_size("font-body"),
        ))
    elif recent_list.is_empty():
        _content.add_child(UIControls.label(
            tr("ui.profile.empty_recent"),
            UITheme.font_size("font-body"),
        ))
    else:
        var recent_box := VBoxContainer.new()
        recent_box.name = "ProfileRecent"
        recent_box.add_theme_constant_override("separation", UITheme.space("space-sm"))
        _content.add_child(recent_box)
        for entry in recent_list:
            if not (entry is Dictionary):
                continue
            var delta := _int_of(entry, "ratingDelta")
            var signed_delta := ("+" if delta >= 0 else "") + str(delta)
            recent_box.add_child(UIControls.label(
                tr("ui.profile.recent_row") % [
                    _int_of(entry, "placement"),
                    signed_delta,
                    _int_of(entry, "ratingAfter"),
                ],
                UITheme.font_size("font-title"),
            ))


## 当前页面目标玩家的资料；收到其他玩家的旧结果时保持加载态，避免串号。
func _current_profile() -> Dictionary:
    var value = GameStore.profile
    if not (value is Dictionary) or value.is_empty():
        return {}
    var target := GameStore.profile_target
    if target != "" and str(value.get("playerId", "")) != target:
        return {}
    return value


func _field_row(label_key: String, value: String, node_name: String) -> Label:
    var row := UIControls.label(
        tr("ui.profile.field_row") % [tr(label_key), value],
        UITheme.font_size("font-title-lg"),
    )
    row.name = node_name
    return row


func _on_back_pressed() -> void:
    UINavigation.navigate(self, _return_page)


## Android 系统返回：回到进入个人主页前的页面。
func system_back() -> bool:
    _on_back_pressed()
    return true


## 服务端字段缺失或为空时按 0 处理。
static func _int_of(source: Dictionary, key: String) -> int:
    var value = source.get(key, 0)
    return 0 if value == null else int(value)
