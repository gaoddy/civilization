class_name HeroDetailSection
extends RefCounted
## 我方英雄详情：介绍、玩法、关联分类与英雄技能；英雄卡不含科学分类与时代。


static func render(box: VBoxContainer, hero_id: Variant, kit: DetailKit) -> void:
    var id := "" if hero_id == null else str(hero_id)
    # 详情顶部叠卡面美术：按卡面比例铺满，无图时只显示文字内容。
    var art := UIArt.layer(
        CardDB.art_path(id, "card_face"),
        "HeroDetailArt",
        TextureRect.STRETCH_KEEP_ASPECT_COVERED,
    )
    if art != null:
        var width := kit.text_width
        art.custom_minimum_size = Vector2(width, UITheme.card_face_height(width))
        box.add_child(art)
    box.add_child(kit.name_label(CardDB.display_name(id)))
    var introduction := CardDB.hero_introduction(id)
    if introduction != "":
        box.add_child(kit.text_block(introduction))
    var entry := CardDB.hero(id)
    box.add_child(kit.row(tr("ui.game.detail.playstyle"), CardText.playstyle_text(entry)))
    kit.append_association(box, entry)
    var skill_text := CardDB.hero_text(id)
    if skill_text == "":
        return
    var heading := UIControls.label(tr("ui.game.detail.hero_power"), UITheme.font_size("font-caption"), false)
    heading.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    box.add_child(heading)
    box.add_child(kit.text_block(skill_text))
