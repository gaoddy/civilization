class_name GameRecruitView
extends VBoxContainer
## 招募组合：高 418 = 商店 216 + 间距 18 + 河道 32 + 间距 6 + 棋盘 146。
##
## 只读 `GameUiState` 并按门控设置控件状态，操作一律以信号交给页面；
## 本视图不构造意图、不判断协议。商店面板同时是棋盘随从的出售落点。

signal buy(slot: int)
signal reroll_pressed
signal upgrade_pressed
signal freeze_pressed
signal board_dropped(payload: Dictionary, position: int, target: Dictionary)
signal sell_dropped(payload: Dictionary)
signal sell_hover(hovering: bool, refund: int)
signal end_recruit_pressed

## 卡片详情登记入口，由对局页注入；同步给商店视图。
var detail: GameDetailLayer = null

var _shop: GameShopView
var _river: GameRiver
var _board_row: BoardDrop


func _init() -> void:
    name = "RecruitPanel"
    size_flags_vertical = Control.SIZE_EXPAND_FILL
    add_theme_constant_override("separation", 0)

    _shop = GameShopView.new()
    _shop.buy.connect(func(slot: int) -> void: buy.emit(slot))
    _shop.reroll_pressed.connect(func() -> void: reroll_pressed.emit())
    _shop.upgrade_pressed.connect(func() -> void: upgrade_pressed.emit())
    _shop.freeze_pressed.connect(func() -> void: freeze_pressed.emit())
    _shop.sell_dropped.connect(func(payload: Dictionary) -> void: sell_dropped.emit(payload))
    _shop.sell_hover.connect(
        func(hovering: bool, refund: int) -> void: sell_hover.emit(hovering, refund)
    )
    add_child(_shop)

    add_child(_gap("RecruitShopGap", UITheme.stage_metric("recruit-shop-gap")))

    _river = GameRiver.new(GameRiver.Mode.RECRUIT)
    _river.end_recruit_pressed.connect(func() -> void: end_recruit_pressed.emit())
    add_child(_river)

    add_child(_gap("RecruitRiverGap", _stage_gap()))

    add_child(_build_board())


## 舞台内小间距：iPhone 基准下再收紧一档。
func _stage_gap() -> int:
    if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE:
        return UITheme.space("space-xs")
    return UITheme.space("space-sm")


func _gap(node_name: String, height: int) -> Control:
    var gap := Control.new()
    gap.name = node_name
    gap.custom_minimum_size = Vector2(0, height)
    gap.mouse_filter = Control.MOUSE_FILTER_IGNORE
    return gap


func _build_board() -> PanelContainer:
    var board_panel := UI.card_panel("bare")
    board_panel.name = "BoardPanel"
    board_panel.custom_minimum_size = Vector2(0, UITheme.stage_metric("card-height"))

    # 棋盘行整体居中；棋盘槽宽固定、最大 7 张，不会超出舞台宽度。
    var board_scroll := Control.new()
    board_scroll.name = "BoardScroll"
    board_scroll.custom_minimum_size = Vector2(0, UITheme.stage_metric("card-height"))
    board_scroll.mouse_filter = Control.MOUSE_FILTER_IGNORE
    board_panel.add_child(board_scroll)

    var center := CenterContainer.new()
    center.name = "BoardCenter"
    center.set_anchors_preset(Control.PRESET_FULL_RECT)
    center.mouse_filter = Control.MOUSE_FILTER_IGNORE
    board_scroll.add_child(center)

    _board_row = BoardDrop.new()
    _board_row.name = "BoardRow"
    _board_row.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    _board_row.add_theme_constant_override("h_separation", UITheme.space("space-md"))
    _board_row.add_theme_constant_override("v_separation", UITheme.space("space-md"))
    _board_row.dropped.connect(_on_board_dropped)
    center.add_child(_board_row)
    return board_panel


## 注入悬停详情层；商店视图同享同一层。
func set_detail(value: GameDetailLayer) -> void:
    detail = value
    _shop.detail = value


func refresh(state: GameUiState) -> void:
    _refresh_board(state)
    _shop.refresh(state)
    _river.refresh(state)


## 只刷新河道计时：战斗阶段玩家棋盘与商店保持不变。
func refresh_clock(state: GameUiState) -> void:
    _river.refresh(state)


## 出售悬停提示由页面回调转入：松手返还金额来自拖拽数据。
func set_sell_hint(hovering: bool, refund: int) -> void:
    _shop.set_sell_hint(hovering, refund)


func has_deadline() -> bool:
    return _river.has_deadline()


func seconds_left() -> int:
    return _river.seconds_left()


func _refresh_board(state: GameUiState) -> void:
    UI.clear(_board_row)
    var aff: Dictionary = state.self_state.get("affordances", {})
    var board: Array = state.self_state.get("board", [])
    var slots := state.board_slots
    var can_rearrange := (
        not state.stale
        and not state.expired
        and not state.target_pending
        and state.phase == Protocol.PHASE_RECRUIT
        and not state.recruited
    )
    _board_row.board_full = board.size() >= slots
    for position in board.size():
        var minion: Dictionary = board[position]
        var minion_aff := _find_by_index(aff.get("boardMinions", []), int(minion.get("index", position)))
        _board_row.add_child(_minion_card(minion, minion_aff, can_rearrange, position, state.epoch))
    for _position in range(board.size(), slots):
        _board_row.add_child(_empty_slot())
    # HFlowContainer 的最小宽度只按单张卡计算；显式给出整行宽度，
    # 才能保持 7 槽单行并被 CenterContainer 精确居中。
    _board_row.custom_minimum_size.x = (
        slots * UITheme.stage_metric("card-width") + maxi(slots - 1, 0) * UITheme.space("space-md")
    )


func _empty_slot() -> PanelContainer:
    var panel := UI.card_panel("empty")
    panel.name = "EmptySlot"
    # PanelContainer 默认 STOP，会截断 Godot 的拖放冒泡（STOP 且不接收即中止），
    # 让棋盘容器收不到落点；空位本身不交互，直接放行。
    panel.mouse_filter = Control.MOUSE_FILTER_IGNORE
    panel.custom_minimum_size = Vector2(UITheme.stage_metric("card-width"), UITheme.stage_metric("card-height"))
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
    var panel := BoardCardFace.build_card(minion, minion_index)

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


func _find_by_index(items: Array, index: int) -> Dictionary:
    for item in items:
        if int(item.get("index", -1)) == index:
            return item
    return {}


func _on_board_dropped(payload: Dictionary, position: int, target: Dictionary) -> void:
    board_dropped.emit(payload, position, target)
