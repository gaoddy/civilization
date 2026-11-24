class_name LobbyHallImage
extends PanelContainer
## 大厅主视觉：左栏铺满剩余高度的大厅图。
##
## 图未生成时保留凹陷面板底色，不占位报错；纹理按 cover 等比裁切，不留信箱边。

const HALL_PATH := "res://assets/ui/lobby/lobby_hall.png"


func _init() -> void:
    name = "LobbyHallImage"
    clip_contents = true
    var panel := UITheme.panel_style("sunken")
    # 主视觉铺满整块左栏，面板只留凹陷底色与圆角，不留内容边距。
    panel.set_content_margin_all(0)
    add_theme_stylebox_override("panel", panel)
    if not ResourceLoader.exists(HALL_PATH):
        return
    var texture := ResourceLoader.load(HALL_PATH) as Texture2D
    if texture == null:
        return
    var rect := TextureRect.new()
    rect.name = "HallTexture"
    rect.texture = texture
    rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
    rect.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
    rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_child(rect)
