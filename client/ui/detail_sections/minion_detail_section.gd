class_name MinionDetailSection
extends RefCounted
## 棋盘随从详情：名称、科学分类、玩法、关联分类与攻击/生命的构成分层。

## 身材分层的展示顺序。
const DETAIL_ROWS := ["initial", "permanent", "turn", "aura", "damage"]


static func render(box: VBoxContainer, minion: Dictionary, kit: DetailKit) -> void:
    var card_id := str(minion.get("cardId", ""))
    box.add_child(kit.name_label(CardDB.display_name(card_id)))
    var entry := CardDB.card(card_id)
    kit.append_classification(box, entry)
    kit.append_association(box, entry)
    var skill_text := CardDB.display_text(card_id, bool(minion.get("golden", false)))
    if skill_text != "":
        box.add_child(kit.text_block(skill_text))
    kit.append_gained_keywords(box, minion)
    var stats: Dictionary = minion.get("stats", {})
    if typeof(stats) != TYPE_DICTIONARY or stats.is_empty():
        box.add_child(kit.row(TranslationServer.translate("ui.game.detail.current"), "%d / %d" % [
            int(minion.get("attack", 0)),
            int(minion.get("health", 0)),
        ]))
        return
    var attack: Dictionary = CardText.stat_layers(stats.get("attack"))
    var health: Dictionary = CardText.stat_layers(stats.get("health"))
    for layer in DETAIL_ROWS:
        var a := int(attack.get(layer, 0))
        var h := int(health.get(layer, 0))
        if layer == "damage" and a == 0 and h == 0:
            continue
        var label := UIWording.enum_text("stat_layer", layer)
        if layer == "initial":
            box.add_child(kit.row(label, "%d / %d" % [a, h]))
        elif layer == "damage":
            box.add_child(kit.row(label, "%s / %s" % [
                CardText.damage_text(a),
                CardText.damage_text(h),
            ]))
        else:
            box.add_child(kit.row(label, "%s / %s" % [
                CardText.signed(a),
                CardText.signed(h),
            ]))
    box.add_child(kit.row(TranslationServer.translate("ui.game.detail.current"), "%d / %d" % [
        int(attack.get("current", 0)),
        int(health.get("current", 0)),
    ]))
