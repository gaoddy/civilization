class_name HeroDetailSection
extends RefCounted
## 我方英雄详情：介绍、玩法、关联分类与英雄技能；英雄卡不含科学分类与时代。


static func render(box: VBoxContainer, hero_id: Variant, kit: DetailKit) -> void:
    var id := "" if hero_id == null else str(hero_id)
    box.add_child(kit.name_label(CardDB.display_name(id)))
    var introduction := CardDB.hero_introduction(id)
    if introduction != "":
        box.add_child(kit.text_block(introduction))
    var entry := CardDB.hero(id)
    box.add_child(kit.row(TranslationServer.translate("ui.game.detail.playstyle"), CardText.playstyle_text(entry)))
    kit.append_association(box, entry)
    var skill_text := CardDB.hero_text(id)
    if skill_text == "":
        return
    var heading := UIControls.label(TranslationServer.translate("ui.game.detail.hero_power"), UITheme.font_size("font-caption"), false)
    heading.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    box.add_child(heading)
    box.add_child(kit.text_block(skill_text))
