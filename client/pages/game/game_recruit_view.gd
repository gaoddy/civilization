class_name GameRecruitView
extends VBoxContainer
## 招募组合：高 438 = 商店 236 + 间距 6 + 河道 44 + 间距 6 + 棋盘 146。
##
## 只读 `GameUiState` 并按门控设置控件状态，操作一律以信号交给页面；
## 本视图不构造意图、不判断协议。商店面板同时是棋盘随从的出售落点。

signal reroll_pressed
signal upgrade_pressed
signal freeze_pressed
signal board_dropped(payload: Dictionary, position: int, target: Dictionary)
signal sell_dropped(payload: Dictionary)
signal sell_hover(hovering: bool, refund: int)
signal end_recruit_pressed
## 棋盘随从技能角标被点击：页面据此直接提交 `UseMinionAbility`。
signal minion_ability_pressed(minion_id: String, skill_id: String)
## 技能角标按下：需要目标的技能由页面进入瞄准，抬起由瞄准层接管。
signal minion_ability_down(minion_id: String, skill_id: String)

## 卡片详情登记入口，由对局页注入；同步给商店视图。
var detail: DetailLayer = null

var _shop: GameShopView
var _river: GameRiver
var _board_row: BoardDrop
## 角标登记：`minion_id|skill_id` → 控件，供瞄准弧线取全局矩形。
var _ability_badges: Dictionary = {}


func _init() -> void:
    name = "RecruitPanel"
    size_flags_vertical = Control.SIZE_EXPAND_FILL
    add_theme_constant_override("separation", UITheme.space("space-none"))

    _shop = GameShopView.new()
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
    var board_panel := UIControls.card_panel("bare")
    board_panel.name = "BoardPanel"
    board_panel.custom_minimum_size = Vector2(0, UITheme.stage_metric("minion-card-height"))

    # 棋盘行整体居中；棋盘槽宽固定、最大 7 张，不会超出舞台宽度。
    var board_scroll := Control.new()
    board_scroll.name = "BoardScroll"
    board_scroll.custom_minimum_size = Vector2(0, UITheme.stage_metric("minion-card-height"))
    board_scroll.mouse_filter = Control.MOUSE_FILTER_IGNORE
    board_panel.add_child(board_scroll)

    var center := CenterContainer.new()
    center.name = "BoardCenter"
    center.set_anchors_preset(Control.PRESET_FULL_RECT)
    center.mouse_filter = Control.MOUSE_FILTER_IGNORE
    board_scroll.add_child(center)

    _board_row = BoardDrop.new()
    _board_row.name = "BoardRow"
    _board_row.alignment = FlowContainer.ALIGNMENT_CENTER
    _board_row.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
    _board_row.add_theme_constant_override("h_separation", UITheme.space("space-md"))
    _board_row.add_theme_constant_override("v_separation", UITheme.space("space-md"))
    _board_row.dropped.connect(_on_board_dropped)
    center.add_child(_board_row)
    return board_panel


## 注入悬停详情层；商店视图同享同一层。
func set_detail(value: DetailLayer) -> void:
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
    UIControls.clear(_board_row)
    _ability_badges.clear()
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
    # HFlowContainer 的最小尺寸只按子卡计算：棋盘为空时高度会收缩为 0，
    # 拖放区随之消失。显式给出整行宽高，既保留整行拖放区，也让不足满槽的卡片居中。
    _board_row.custom_minimum_size = Vector2(
        slots * UITheme.stage_metric("minion-card-width") + maxi(slots - 1, 0) * UITheme.space("space-md"),
        UITheme.stage_metric("minion-card-height"),
    )


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
        UIControls.apply_dim(panel)
    _add_ability_badges(panel, minion, can_rearrange, position)
    return panel


## 技能角标：作为卡片外层叠加，锚定右上并纵向排列，不进入卡面内部。
## 只渲染服务端下发的 `abilities`；对手棋盘本就无该字段，不会出现角标。
func _add_ability_badges(
    panel: Control,
    minion: Dictionary,
    can_rearrange: bool,
    position: int,
) -> void:
    var abilities: Array = minion.get("abilities", [])
    if abilities.is_empty():
        return
    var minion_id := str(minion.get("minionId", ""))
    var overlay := Control.new()
    overlay.name = "AbilityBadges"
    overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
    overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
    panel.add_child(overlay)

    # 角标从卡面顶部角标行下方开始：不遮挡 ★/潜行角标，底部攻血数值也保持可见。
    var width := UITheme.space("space-3xl") + UITheme.space("space-lg")
    var gap := UITheme.space("space-2xs")
    var top := (
        UITheme.space("space-xs")
        + UITheme.font_size("font-caption")
        + UITheme.space("space-md")
    )
    var count := abilities.size()
    # 底部为攻血数值留出固定高度：角标纵向排列到底也不会盖住数值行。
    var available := (
        UITheme.stage_metric("minion-card-height")
        - top
        - UITheme.space("space-3xl")
        - gap * maxi(count - 1, 0)
    )
    var height := mini(UITheme.hit_small(), maxi(1, available / count))
    for index in count:
        var skill = abilities[index]
        if not skill is Dictionary:
            continue
        var skill_id := str(skill.get("skillId", ""))
        var badge := BoardAbilityBadge.new()
        badge.name = "MinionAbility%d_%d" % [position, index]
        badge.minion_id = minion_id
        badge.skill_id = skill_id
        badge.cost = int(skill.get("cost", 0))
        badge.uses_left = int(skill.get("usesLeft", 0))
        badge.can_use = bool(skill.get("canUse", false)) and can_rearrange
        badge.refresh()
        badge.custom_minimum_size = Vector2(width, height)
        badge.size = Vector2(width, height)
        badge.set_anchors_preset(Control.PRESET_TOP_RIGHT)
        badge.offset_left = -UITheme.space("space-xs") - width
        badge.offset_right = -UITheme.space("space-xs")
        badge.offset_top = top + index * (height + gap)
        badge.offset_bottom = badge.offset_top + height
        badge.pressed.connect(
            func() -> void: minion_ability_pressed.emit(minion_id, skill_id)
        )
        badge.button_down.connect(
            func() -> void: minion_ability_down.emit(minion_id, skill_id)
        )
        overlay.add_child(badge)
        _ability_badges[_ability_key(minion_id, skill_id)] = badge


## 技能角标的全局矩形：瞄准弧线的起点；未知技能返回空矩形。
func ability_rect(minion_id: String, skill_id: String) -> Rect2:
    var badge: Control = _ability_badges.get(_ability_key(minion_id, skill_id))
    return badge.get_global_rect() if badge != null else Rect2()


func _ability_key(minion_id: String, skill_id: String) -> String:
    return minion_id + "|" + skill_id


func _find_by_index(items: Array, index: int) -> Dictionary:
    for item in items:
        if int(item.get("index", -1)) == index:
            return item
    return {}


func _on_board_dropped(payload: Dictionary, position: int, target: Dictionary) -> void:
    board_dropped.emit(payload, position, target)
