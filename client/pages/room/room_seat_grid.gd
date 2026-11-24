class_name RoomSeatGrid
extends GridContainer
## 房间等待区座位矩阵：固定 8 格、4 列 2 行，按 `seat` 字段填充，缺位即空位。
##
## 网格只负责构建与布局；移除按钮回调由页面在 `render` 后写入各座位格。

const SEAT_COUNT := 8


func _init() -> void:
    name = "RoomSeatGrid"
    columns = 4
    add_theme_constant_override("h_separation", UITheme.space("space-md"))
    add_theme_constant_override("v_separation", UITheme.space("space-md"))
    size_flags_horizontal = Control.SIZE_EXPAND_FILL
    size_flags_vertical = Control.SIZE_EXPAND_FILL


func render(players: Array, self_id: String) -> void:
    UIControls.clear(self)
    var by_seat := {}
    for player in players:
        if not (player is Dictionary):
            continue
        var value = (player as Dictionary).get("seat")
        if value == null:
            continue
        var seat := int(value)
        if seat < 0 or seat >= SEAT_COUNT:
            continue
        by_seat[seat] = player

    for index in SEAT_COUNT:
        var cell := RoomSeatCell.new()
        cell.name = "SeatCell%d" % index
        cell.size_flags_horizontal = Control.SIZE_EXPAND_FILL
        cell.size_flags_vertical = Control.SIZE_EXPAND_FILL
        var player: Dictionary = by_seat.get(index, {})
        var is_self := not player.is_empty() and str(player.get("playerId", "")) == self_id
        cell.setup(index, player, is_self)
        add_child(cell)
