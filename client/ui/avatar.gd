class_name Avatar
extends AvatarFrame
## 英雄头像块：方形，继承 AvatarFrame 的圆角裁切与叠在头像之上的描边。
##
## 有头像纹理时按 cover 铺满并收进圆角，否则用玩法色封面 + 名字首字。


## `border_color` 为 `null` 时描边取玩法色；传入透明色可把外框交给承载方绘制，
## 避免头像描边与承载框描边叠成双线。
static func create(hero_id: String, size := 34, border_color: Variant = null) -> Avatar:
    var node := Avatar.new()
    node.name = "Avatar"
    node.custom_minimum_size = Vector2(size, size)
    var playstyle := CardDB.primary_playstyle(CardDB.hero(hero_id)) if hero_id != "" else ""
    var tint := UITheme.playstyle_color(playstyle) if hero_id != "" else UITheme.color("color-border-strong")
    node.configure(tint.darkened(0.55), tint if border_color == null else border_color)
    node.set_glyph(initial(hero_id), tint.lightened(0.5))
    node.set_art(CardDB.art_path(hero_id, "avatar") if hero_id != "" else "")
    return node


static func initial(hero_id: String) -> String:
    if hero_id == "":
        return "?"
    var display := CardDB.display_name(hero_id)
    if display == "":
        return "?"
    return display.substr(0, 1)
