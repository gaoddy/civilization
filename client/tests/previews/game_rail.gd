extends RefCounted
## 左栏积分榜单视图预览：用八人对局夹具构造 GameUiState，再交给视图的静态 preview。

const FixtureScript = preload("res://tests/fixture.gd")
const RailScript = preload("res://pages/game/game_players_rail.gd")


static func build(_opts: Dictionary = {}) -> Node:
    var view := FixtureScript.game_view()
    var state := GameUiState.new()
    state.view = view
    state.phase = Protocol.PHASE_RECRUIT
    state.self_state = view.get("self", {})
    state.board_slots = maxi(int(view.get("boardSlots", 7)), 1)
    state.hp_cap = maxi(int(view.get("hpCap", 40)), 1)
    state.connected = true
    var root := Control.new()
    root.name = "RailPreview"
    root.set_anchors_preset(Control.PRESET_FULL_RECT)
    root.add_child(UIControls.background())
    var center := CenterContainer.new()
    center.set_anchors_preset(Control.PRESET_FULL_RECT)
    root.add_child(center)
    var frame := Control.new()
    frame.name = "RailPreviewFrame"
    frame.custom_minimum_size = Vector2(
        UITheme.stage_metric("rail-width"), float(ScreenProfile.base_canvas().y)
    )
    center.add_child(frame)
    var rail = RailScript.preview({"state": state})
    rail.set_anchors_preset(Control.PRESET_FULL_RECT)
    frame.add_child(rail)
    return root
