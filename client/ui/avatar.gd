class_name Avatar
extends RefCounted
## 英雄头像块：方形；有头像纹理时叠图，否则用玩法色块 + 名字首字表示。


static func create(hero_id: String, size := 34) -> PanelContainer:
    var node := PanelContainer.new()
    node.name = "Avatar"
    node.custom_minimum_size = Vector2(size, size)
    var playstyle := CardDB.primary_playstyle(CardDB.hero(hero_id)) if hero_id != "" else ""
    var tint := UITheme.playstyle_color(playstyle) if hero_id != "" else UITheme.color("color-border-strong")
    var box := StyleBoxFlat.new()
    box.bg_color = tint.darkened(0.55)
    box.border_color = tint
    box.set_border_width_all(2)
    box.set_corner_radius_all(0)
    node.add_theme_stylebox_override("panel", box)
    var texture := UIArt.texture(CardDB.art_path(hero_id, "avatar"))
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
    var text := UIControls.label(initial(hero_id), int(size * 0.42), false)
    text.name = "Initial"
    text.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    text.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    text.add_theme_color_override("font_color", tint.lightened(0.5))
    node.add_child(text)
    return node


static func initial(hero_id: String) -> String:
    if hero_id == "":
        return "?"
    var display := CardDB.display_name(hero_id)
    if display == "":
        return "?"
    return display.substr(0, 1)
