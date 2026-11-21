class_name ShopDetailSection
extends RefCounted
## 商店卡详情：类型、科学分类、玩法、关联分类、费用、原始数值、时代、
## 技能文本与不可购买原因。


static func render(box: VBoxContainer, data: Dictionary, kit: DetailKit) -> void:
    var card_id := str(data.get("cardId", ""))
    var entry := CardDB.card(card_id)
    box.add_child(kit.name_label(CardDB.display_name(card_id)))
    box.add_child(kit.row(TranslationServer.translate("ui.game.detail.type"), CardText.type_text(entry)))
    kit.append_classification(box, entry)
    kit.append_association(box, entry)
    box.add_child(kit.row(TranslationServer.translate("ui.game.detail.cost"), TranslationServer.translate("ui.game.detail.cost_value") % CardDB.display_cost(card_id)))
    box.add_child(kit.row(TranslationServer.translate("ui.game.detail.initial_stats"), "%s / %s" % [
        CardText.stat_text(entry.get("attack")),
        CardText.stat_text(entry.get("health")),
    ]))
    var era := str(entry.get("era", ""))
    if era != "":
        box.add_child(kit.row(TranslationServer.translate("ui.game.detail.era"), UIWording.enum_text("era", era)))
    var skill_text := CardDB.display_text(card_id)
    if skill_text != "":
        box.add_child(kit.text_block(skill_text))
    if data.get("disabledReason") != null:
        var reason := UIControls.label(UIWording.error_text(str(data.get("disabledReason"))), UITheme.font_size("font-body-sm"))
        reason.name = "DetailBlocked"
        reason.add_theme_color_override("font_color", UITheme.color("color-danger"))
        box.add_child(reason)
