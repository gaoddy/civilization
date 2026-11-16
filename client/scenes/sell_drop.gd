class_name SellDrop
extends PanelContainer
## 商店面板兼作出售落点：拖入我方棋盘随从时金色描边，松手发出出售数据。
##
## 只接受棋盘卡拖拽数据；手牌拖到商店不产生任何动作。是否可卖由拖拽数据里的
## `sellable` 决定，页面再用视图版本 `epoch` 与操作门控做落点复查。

## 松手时发出完整拖拽数据，页面据此校验 `epoch` 并发送 `SellMinion`。
signal sell_dropped(payload: Dictionary)

## 悬停状态变化；`refund` 为本次可返还金币（普通 1、金色 3）。
signal hover_changed(hovering: bool, refund: int)

## 由页面按阶段、视图新鲜度、连接与待选汇总；`false` 时整块商店不接收拖拽。
var enabled := false

var _hovering := false


func _ready() -> void:
    mouse_filter = Control.MOUSE_FILTER_STOP
    _restore_style()


func _notification(what: int) -> void:
    if what == NOTIFICATION_DRAG_END:
        _clear_hover()


func _process(_delta: float) -> void:
    if _hovering and not get_global_rect().has_point(get_global_mouse_position()):
        _clear_hover()


func _can_drop_data(_at_position: Vector2, data: Variant) -> bool:
    if not _accepts(data):
        return false
    if not _hovering:
        _hovering = true
        add_theme_stylebox_override("panel", UITheme.panel_style("accent"))
        hover_changed.emit(true, _refund(data))
    return true


func _drop_data(_at_position: Vector2, data: Variant) -> void:
    if not _accepts(data):
        return
    var payload: Dictionary = (data as Dictionary).duplicate()
    _clear_hover()
    sell_dropped.emit(payload)


func _accepts(data: Variant) -> bool:
    if not enabled or not (data is Dictionary):
        return false
    var payload: Dictionary = data
    if not payload.has("minionId"):
        return false
    return bool(payload.get("sellable", false))


func _refund(data: Variant) -> int:
    var payload: Dictionary = data
    return 3 if bool(payload.get("golden", false)) else 1


func _clear_hover() -> void:
    if not _hovering:
        return
    _hovering = false
    _restore_style()
    hover_changed.emit(false, 0)


func _restore_style() -> void:
    add_theme_stylebox_override("panel", UITheme.panel_style("surface"))
