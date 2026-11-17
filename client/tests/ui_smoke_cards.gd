extends "res://tests/smoke_case.gd"
## 商店卡角标与冻结表现：费用在左上、时代在右上，冻结用整卡蒙层 + 冰晶 + 右下「冻」字。


func _ready() -> void:
    var view: Dictionary = FixtureScript.view()
    view["self"]["shop"] = [
        {"index": 0, "cardId": "aolinpike-jidian", "frozen": false},
        {"index": 1, "cardId": "agi", "frozen": true},
    ]
    view["self"]["board"] = []
    view["self"]["affordances"]["shopSlots"] = [
        {"index": 0, "buyable": true, "disabledReason": null},
        {"index": 1, "buyable": false, "disabledReason": "NOT_ENOUGH_GOLD"},
    ]

    var recruit := GameRecruitView.new()
    add_child(recruit)
    var state := GameUiState.new()
    state.view = view
    state.self_state = view["self"]
    state.phase = Protocol.PHASE_RECRUIT
    state.board_slots = int(view.get("boardSlots", 7))
    recruit.refresh(state)
    await get_tree().process_frame
    await get_tree().process_frame

    # 未冻结卡：左上费用、右上时代，没有冻结蒙层。
    var open_card := recruit.find_child("ShopSlot0", true, false)
    _expect(open_card != null, "商店应存在 ShopSlot0")
    if open_card != null:
        var cost := open_card.find_child("CostBadge", true, false)
        _expect(cost != null, "商店卡左上角应有 CostBadge")
        if cost != null:
            _expect(str(cost.text) == str(CardDB.display_cost("aolinpike-jidian")), "CostBadge 文本应为展示费用")
            _expect(str(cost.text) == "2", "CostBadge 应显示该卡展示费用 2，实际：%s" % str(cost.text))
        var era := open_card.find_child("EraBadge", true, false)
        _expect(era != null, "商店卡右上角应有 EraBadge")
        if era != null:
            _expect(str(era.text) == str(CardDB.card("aolinpike-jidian").get("era", "")), "EraBadge 文本应为卡表时代")
            _expect(str(era.text) == "古典", "EraBadge 应显示「古典」，实际：%s" % str(era.text))
        _expect(open_card.find_child("FreezeOverlay", true, false) == null, "未冻结卡不应有冻结蒙层")
        _expect(open_card.find_child("FreezeGlyph", true, false) == null, "未冻结卡不应有「冻」字")
        if cost != null and era != null:
            var card_rect: Rect2 = (open_card as Control).get_global_rect()
            var cost_rect: Rect2 = (cost as Control).get_global_rect()
            var era_rect: Rect2 = (era as Control).get_global_rect()
            _expect(cost_rect.position.x < card_rect.get_center().x, "费用角标应在卡片左上")
            _expect(cost_rect.position.y < card_rect.get_center().y, "费用角标应在卡片左上")
            _expect(era_rect.position.x > card_rect.get_center().x, "时代角标应在卡片右上")
            _expect(era_rect.position.y < card_rect.get_center().y, "时代角标应在卡片右上")

    # 冻结卡：整卡蒙层 + 冰晶 + 右下「冻」字，且不再使用「冻」角标。
    var frozen_card := recruit.find_child("ShopSlot1", true, false)
    _expect(frozen_card != null, "商店应存在 ShopSlot1")
    if frozen_card != null:
        var overlay := frozen_card.find_child("FreezeOverlay", true, false)
        _expect(overlay != null, "冻结卡应有 FreezeOverlay 蒙层")
        if overlay is Control:
            _expect((overlay as Control).mouse_filter == Control.MOUSE_FILTER_IGNORE, "冻结蒙层不应拦截鼠标")
        var glyph := frozen_card.find_child("FreezeGlyph", true, false)
        _expect(glyph != null, "冻结卡应有 FreezeGlyph")
        if glyph != null:
            _expect(str(glyph.text) == "冻", "FreezeGlyph 文本应为「冻」，实际：%s" % str(glyph.text))
        var old_badge := false
        for badge in frozen_card.find_children("CardBadge", "Label", true, false):
            if str(badge.text) == "冻":
                old_badge = true
        _expect(not old_badge, "冻结卡不应再用 text 为「冻」的 CardBadge")

    recruit.queue_free()
    _finish()
