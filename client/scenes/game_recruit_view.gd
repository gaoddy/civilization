class_name GameRecruitView
extends VBoxContainer
## 招募视图：商店与我方棋盘。
##
## 只读 `GameUiState` 并按门控设置控件状态，操作一律以信号交给页面；
## 本视图不构造意图、不判断协议。商店操作按钮在右栏，见 `GameSidePanel`。
## 商店面板同时是棋盘随从的出售落点。

signal buy(slot: int)
signal board_dropped(payload: Dictionary, position: int, target: Dictionary)
signal sell_dropped(payload: Dictionary)
signal sell_hover(hovering: bool, refund: int)

## 卡片详情登记入口，由对局页注入；独立实例化时可为 null。
var detail: GameDetailLayer = null

var _board_row: BoardDrop
var _shop_row: HFlowContainer
var _sell_drop: SellDrop
var _sell_hint: Label


func _init() -> void:
    name = "RecruitPanel"
    size_flags_vertical = Control.SIZE_EXPAND_FILL
    add_theme_constant_override("separation", UITheme.space("space-sm"))

    _build_shop()
    _build_board()


func _build_shop() -> void:
    # 商店：6 个槽位；整块面板同时是棋盘随从的出售落点。面板不带标题，
    # 出售提示以浮层压在右上角，出现时不改变卡片布局。
    var shop_panel := SellDrop.new()
    shop_panel.name = "ShopPanel"
    _sell_drop = shop_panel
    shop_panel.sell_dropped.connect(_on_sell_dropped)
    shop_panel.hover_changed.connect(_on_sell_hover)
    var shop_box := VBoxContainer.new()
    shop_box.add_theme_constant_override("separation", UITheme.space("space-xs"))
    shop_panel.add_child(shop_box)

    _shop_row = _new_flow_row("ShopRow")
    shop_box.add_child(_shop_row)

    _sell_hint = UI.label("", UITheme.font_size("font-caption"), false)
    _sell_hint.name = "SellHint"
    _sell_hint.add_theme_color_override("font_color", UITheme.color("color-accent"))
    _sell_hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
    _sell_hint.mouse_filter = Control.MOUSE_FILTER_IGNORE
    _sell_hint.visible = false
    shop_panel.add_child(_sell_hint)
    add_child(shop_panel)


func _build_board() -> void:
    # 我方棋盘：不带标题，数量与容量由服务端视图决定，不在页面上再报一次。
    var board_panel := UI.card_panel("surface")
    board_panel.name = "BoardPanel"
    var board_scroll := ScrollContainer.new()
    board_scroll.name = "BoardScroll"
    board_scroll.custom_minimum_size = Vector2(0, UITheme.metric("card-height"))
    board_scroll.size_flags_vertical = Control.SIZE_SHRINK_CENTER
    board_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
    board_panel.add_child(board_scroll)
    _board_row = BoardDrop.new()
    _board_row.name = "BoardRow"
    _board_row.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    _board_row.add_theme_constant_override("h_separation", UITheme.space("space-sm"))
    _board_row.add_theme_constant_override("v_separation", UITheme.space("space-sm"))
    _board_row.dropped.connect(_on_board_dropped)
    board_scroll.add_child(_board_row)

    # 棋盘面板按内容高度收缩，顶部留白把棋盘推到面板底部，紧贴下方手牌。
    var top_spacer := Control.new()
    top_spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
    add_child(top_spacer)
    add_child(board_panel)


func _new_flow_row(node_name: String) -> HFlowContainer:
    var row := HFlowContainer.new()
    row.name = node_name
    row.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    row.add_theme_constant_override("h_separation", UITheme.space("space-sm"))
    row.add_theme_constant_override("v_separation", UITheme.space("space-sm"))
    return row


func refresh(state: GameUiState) -> void:
    _refresh_board(state)
    _refresh_shop(state)
    _refresh_sell_gate(state)


## 出售悬停提示由页面回调转入：松手返还金额来自拖拽数据。
func set_sell_hint(hovering: bool, refund: int) -> void:
    _sell_hint.text = tr("ui.game.recruit.sell_hint") % refund
    _sell_hint.visible = hovering


## 商店出售落点与棋盘拖拽共用一套实时门控；不满足时商店不接收棋盘卡。
func _refresh_sell_gate(state: GameUiState) -> void:
    if _sell_drop == null:
        return
    _sell_drop.enabled = state.can_play
    if not _sell_drop.enabled:
        _sell_hint.visible = false


func _refresh_board(state: GameUiState) -> void:
    UI.clear(_board_row)
    var aff: Dictionary = state.self_state.get("affordances", {})
    var board: Array = state.self_state.get("board", [])
    var slots := state.board_slots
    var can_rearrange := not state.stale and not state.expired and not state.target_pending and state.phase == Protocol.PHASE_RECRUIT and not state.recruited
    _board_row.board_full = board.size() >= slots
    for position in board.size():
        var minion: Dictionary = board[position]
        var minion_aff := _find_by_index(aff.get("boardMinions", []), int(minion.get("index", position)))
        _board_row.add_child(_minion_card(minion, minion_aff, can_rearrange, position, state.epoch))
    for _position in range(board.size(), slots):
        _board_row.add_child(_empty_slot())


func _empty_slot() -> PanelContainer:
    var panel := UI.card_panel("empty")
    panel.name = "EmptySlot"
    # PanelContainer 默认 STOP，会截断 Godot 的拖放冒泡（STOP 且不接收即中止），
    # 让棋盘容器收不到落点；空位本身不交互，直接放行。
    panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
    panel.custom_minimum_size = Vector2(UITheme.metric("card-width"), UITheme.metric("card-height"))
    var hint := UI.empty_hint(tr("ui.game.recruit.empty_board_slot"))
    hint.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
    panel.add_child(hint)
    return panel


func _minion_card(
    minion: Dictionary,
    minion_aff: Dictionary,
    can_rearrange: bool,
    position: int,
    epoch: int,
) -> PanelContainer:
    var minion_index := int(minion.get("index", position))
    var panel := BoardCard.new()
    panel.add_theme_stylebox_override("panel", UITheme.panel_style("raised-tight"))
    panel.name = "Board" + ("Minion%d" % minion_index)
    panel.set_meta("minion_id", str(minion.get("minionId", "")))
    panel.set_meta("minion_index", minion_index)
    panel.custom_minimum_size = Vector2(UITheme.metric("card-width"), UITheme.metric("card-height"))

    var card_id := str(minion.get("cardId", ""))
    var entry := CardDB.card(card_id)
    panel.add_child(GameCardFace.face(
        card_id,
        GameCardFace.stat_text(minion.get("attack")),
        GameCardFace.stat_text(minion.get("health")),
        GameCardFace.playstyle_text(entry),
        "★" if bool(minion.get("golden", false)) else "",
        "",
        "",
        false,
        bool(minion.get("stealth", false)),
    ))

    if detail != null:
        detail.track(panel, {"detail_kind": "minion", "minion": minion})

    # 棋盘卡只提供拖拽：拖到棋盘内换位，拖到商店出售；卡内不再放操作按钮。
    panel.payload = {
        "epoch": epoch,
        "minionId": str(minion.get("minionId", "")),
        "fromIndex": minion_index,
        "sellable": bool(minion_aff.get("sellable", false)),
        "golden": bool(minion.get("golden", false)),
    }
    panel.drag_allowed = can_rearrange and Net.is_open()
    if not panel.drag_allowed:
        UI.apply_dim(panel)
    return panel


func _refresh_shop(state: GameUiState) -> void:
    UI.clear(_shop_row)
    var aff: Dictionary = state.self_state.get("affordances", {})
    for slot in state.self_state.get("shop", []):
        var index := int(slot.get("index", 0))
        var card_id := GameCardFace.optional_text(slot.get("cardId"))
        if card_id == "":
            _shop_row.add_child(_empty_shop_slot(index))
            continue
        var slot_aff := _find_by_index(aff.get("shopSlots", []), index)
        var buyable := not state.stale and not state.expired and bool(slot_aff.get("buyable", false))
        var entry := CardDB.card(card_id)
        var panel := UI.card_panel("raised-tight")
        panel.name = "ShopSlot%d" % index
        # PASS：整卡接收点击购买，同时让棋盘卡的出售落点继续冒泡到商店面板。
        panel.mouse_filter = Control.MOUSE_FILTER_PASS
        panel.custom_minimum_size = Vector2(UITheme.metric("card-width"), UITheme.metric("card-height"))
        panel.set_meta("buyable", buyable)
        panel.gui_input.connect(_on_shop_card_input.bind(panel, index))
        panel.add_child(GameCardFace.face(
            card_id,
            GameCardFace.stat_text(entry.get("attack")),
            GameCardFace.stat_text(entry.get("health")),
            GameCardFace.playstyle_text(entry),
            "",
            str(CardDB.display_cost(card_id)),
            UI.enum_text("era", str(entry.get("era", ""))),
            bool(slot.get("frozen", false)),
        ))
        if detail != null:
            detail.track(panel, {
                "detail_kind": "shop",
                "cardId": card_id,
                "disabledReason": slot_aff.get("disabledReason"),
            })
        if not buyable:
            UI.apply_dim(panel)
        _shop_row.add_child(panel)


func _empty_shop_slot(index: int) -> PanelContainer:
    var panel := UI.card_panel("empty")
    panel.name = "ShopSlot%d" % index
    panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
    panel.custom_minimum_size = Vector2(UITheme.metric("card-width"), UITheme.metric("card-height"))
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


func _on_sell_dropped(payload: Dictionary) -> void:
    sell_dropped.emit(payload)


func _on_sell_hover(hovering: bool, refund: int) -> void:
    sell_hover.emit(hovering, refund)


func _on_board_dropped(payload: Dictionary, position: int, target: Dictionary) -> void:
    board_dropped.emit(payload, position, target)
