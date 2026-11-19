class_name GameShopView
extends VBoxContainer
## 商店视图：高 216 = 操作行 64 + 间距 6 + 商店 146。
##
## 操作行以售货员为中心，左侧升级时代，右侧刷新与冻结；三个动作都是 40×40 图标按钮。
## 商店面板整块是棋盘随从的出售落点；只发信号，不构造意图。

signal buy(slot: int)
signal reroll_pressed
signal upgrade_pressed
signal freeze_pressed
signal sell_dropped(payload: Dictionary)
signal sell_hover(hovering: bool, refund: int)

## 卡片详情登记入口，由对局页注入；独立实例化时可为 null。
var detail: GameDetailLayer = null

var _upgrade_button: GameIconButton
var _reroll_button: GameIconButton
var _freeze_button: GameIconButton
var _shop_panel: SellDrop
var _shop_row: HBoxContainer
var _sell_hint: Label


## 舞台内小间距：iPhone 基准下再收紧一档。
func _stage_gap() -> int:
    if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE:
        return UITheme.space("space-xs")
    return UITheme.space("space-sm")


func _init() -> void:
    name = "ShopView"
    custom_minimum_size = Vector2(
        0, UITheme.stage_metric("shop-actions-height") + _stage_gap()
            + UITheme.stage_metric("card-height")
    )
    add_theme_constant_override("separation", 0)

    var actions := HBoxContainer.new()
    actions.name = "ShopActions"
    actions.custom_minimum_size = Vector2(0, UITheme.stage_metric("shop-actions-height"))
    actions.alignment = BoxContainer.ALIGNMENT_CENTER
    actions.add_theme_constant_override("separation", UITheme.space("space-lg"))
    add_child(actions)

    _upgrade_button = GameIconButton.new("upgrade", "Upgrade")
    _upgrade_button.name = "UpgradeButton"
    _upgrade_button.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    _upgrade_button.pressed.connect(func() -> void: upgrade_pressed.emit())
    actions.add_child(_upgrade_button)

    var seller := GamePortrait.new()
    seller.name = "SellerTile"
    seller.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    seller.setup(
        tr("ui.game.side_panel.seller"),
        tr("ui.game.side_panel.seller_glyph"),
        UITheme.playstyle_color("neutral"),
        true,
    )
    actions.add_child(seller)

    var right := HBoxContainer.new()
    right.name = "ShopActionRight"
    right.alignment = BoxContainer.ALIGNMENT_CENTER
    right.add_theme_constant_override("separation", UITheme.space("space-sm"))
    actions.add_child(right)

    _reroll_button = GameIconButton.new("reroll", "Reroll")
    _reroll_button.name = "RerollButton"
    _reroll_button.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    _reroll_button.pressed.connect(func() -> void: reroll_pressed.emit())
    right.add_child(_reroll_button)

    _freeze_button = GameIconButton.new("freeze", "Freeze", false)
    _freeze_button.name = "FreezeButton"
    _freeze_button.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    _freeze_button.pressed.connect(func() -> void: freeze_pressed.emit())
    right.add_child(_freeze_button)

    var gap := Control.new()
    gap.name = "ShopGap"
    gap.custom_minimum_size = Vector2(0, _stage_gap())
    gap.mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_child(gap)

    _shop_panel = SellDrop.new()
    _shop_panel.name = "ShopPanel"
    _shop_panel.style_variant = "bare"
    _shop_panel.hover_variant = "accent-bare"
    _shop_panel.custom_minimum_size = Vector2(0, UITheme.stage_metric("card-height"))
    _shop_panel.sell_dropped.connect(func(payload: Dictionary) -> void: sell_dropped.emit(payload))
    _shop_panel.hover_changed.connect(func(hovering: bool, refund: int) -> void: sell_hover.emit(hovering, refund))
    add_child(_shop_panel)

    var stack := Control.new()
    stack.name = "ShopStack"
    stack.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _shop_panel.add_child(stack)

    var center := CenterContainer.new()
    center.name = "ShopCenter"
    center.set_anchors_preset(Control.PRESET_FULL_RECT)
    center.mouse_filter = Control.MOUSE_FILTER_IGNORE
    stack.add_child(center)

    _shop_row = HBoxContainer.new()
    _shop_row.name = "ShopRow"
    _shop_row.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _shop_row.add_theme_constant_override("separation", UITheme.space("space-md"))
    center.add_child(_shop_row)

    _sell_hint = UI.label("", UITheme.font_size("font-caption"), false)
    _sell_hint.name = "SellHint"
    _sell_hint.set_anchors_preset(Control.PRESET_TOP_RIGHT)
    _sell_hint.grow_horizontal = Control.GROW_DIRECTION_BEGIN
    _sell_hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
    _sell_hint.add_theme_color_override("font_color", UITheme.color("color-accent"))
    _sell_hint.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _sell_hint.visible = false
    stack.add_child(_sell_hint)


func refresh(state: GameUiState) -> void:
    if detail != null and is_instance_valid(detail):
        var seller := find_child("SellerTile", true, false)
        if seller != null:
            detail.track(seller, {"detail_kind": "seller"})
    _refresh_actions(state)
    _refresh_shop(state)
    _shop_panel.enabled = state.can_play
    if not _shop_panel.enabled:
        _sell_hint.visible = false


## 出售悬停提示由页面回调转入：松手返还金额来自拖拽数据。
func set_sell_hint(hovering: bool, refund: int) -> void:
    _sell_hint.text = tr("ui.game.recruit.sell_hint") % refund
    _sell_hint.visible = hovering


func _refresh_actions(state: GameUiState) -> void:
    var aff: Dictionary = state.self_state.get("affordances", {})
    var blocked := state.stale or state.expired or state.target_pending

    var upgrade_cost = state.self_state.get("upgradeCost")
    _upgrade_button.set_cost(-1 if upgrade_cost == null else int(upgrade_cost))
    _upgrade_button.disabled = blocked or not bool(aff.get("canUpgradeEra", false))
    var upgrade_text := (
        tr("ui.game.side_panel.era_max")
        if upgrade_cost == null
        else tr("ui.game.side_panel.upgrade_cost") % int(upgrade_cost)
    )
    var upgrade_reason = aff.get("upgradeDisabledReason")
    if upgrade_reason != null:
        upgrade_text += "\n" + UI.error_text(str(upgrade_reason))
    _upgrade_button.tooltip_text = upgrade_text

    _reroll_button.set_cost(int(state.self_state.get("rerollCost", 0)))
    _reroll_button.disabled = blocked or not bool(aff.get("canReroll", false))
    _reroll_button.tooltip_text = tr("ui.game.side_panel.reroll_cost") % int(
        state.self_state.get("rerollCost", 0)
    )

    _freeze_button.disabled = (
        blocked or state.phase != Protocol.PHASE_RECRUIT or state.recruited
    )
    var frozen := false
    for slot in state.self_state.get("shop", []):
        if bool(slot.get("frozen", false)):
            frozen = true
            break
    _freeze_button.set_active(frozen)
    _freeze_button.tooltip_text = tr("ui.game.side_panel.freeze")


func _refresh_shop(state: GameUiState) -> void:
    UI.clear(_shop_row)
    var aff: Dictionary = state.self_state.get("affordances", {})
    for slot in state.self_state.get("shop", []):
        var index := int(slot.get("index", 0))
        var card_id := CardText.optional_text(slot.get("cardId"))
        if card_id == "":
            _shop_row.add_child(_empty_slot(index))
            continue
        var slot_aff := _find_by_index(aff.get("shopSlots", []), index)
        var buyable := not state.stale and not state.expired and bool(slot_aff.get("buyable", false))
        var entry := CardDB.card(card_id)
        var panel := UI.card_panel("raised-tight")
        panel.name = "ShopSlot%d" % index
        # PASS：整卡接收点击购买，同时让棋盘卡的出售落点继续冒泡到商店面板。
        panel.mouse_filter = Control.MOUSE_FILTER_PASS
        panel.custom_minimum_size = Vector2(UITheme.stage_metric("card-width"), UITheme.stage_metric("card-height"))
        panel.set_meta("buyable", buyable)
        panel.gui_input.connect(_on_shop_card_input.bind(panel, index))
        panel.add_child(ShopCardFace.build(card_id, entry, bool(slot.get("frozen", false))))
        if detail != null and is_instance_valid(detail):
            detail.track(panel, {
                "detail_kind": "shop",
                "cardId": card_id,
                "disabledReason": slot_aff.get("disabledReason"),
            })
        if not buyable:
            UI.apply_dim(panel)
        _shop_row.add_child(panel)


func _empty_slot(index: int) -> PanelContainer:
    var panel := UI.card_panel("empty")
    panel.name = "ShopSlot%d" % index
    panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
    panel.custom_minimum_size = Vector2(UITheme.stage_metric("card-width"), UITheme.stage_metric("card-height"))
    var hint := UI.empty_hint(tr("ui.game.recruit.empty_shop_slot"))
    hint.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    panel.add_child(hint)
    return panel


## 商店卡点击购买；不可购买时静默忽略，原因在详情面板里给出。
func _on_shop_card_input(event: InputEvent, panel: Control, index: int) -> void:
    if not (event is InputEventMouseButton):
        return
    var mouse := event as InputEventMouseButton
    if mouse.button_index != MOUSE_BUTTON_LEFT or not mouse.pressed:
        return
    if bool(panel.get_meta("buyable", false)):
        buy.emit(index)


func _find_by_index(items: Array, index: int) -> Dictionary:
    for item in items:
        if int(item.get("index", -1)) == index:
            return item
    return {}
