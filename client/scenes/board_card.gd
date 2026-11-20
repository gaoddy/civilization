class_name BoardCard
extends PanelContainer
## 棋盘随从卡：拖拽是换位与出售的唯一入口，卡片本身不放操作按钮。
##
## 页面把「现在能不能拖」汇总成 [`BoardCard.drag_allowed`]（阶段、视图新鲜度、
## 倒计时、连接、待选与是否已结束招募），本控件只负责在被允许时交出拖拽数据。
## 拖拽数据带视图版本 `epoch`：落点方按它判断这次拖拽是否已过期。

## {epoch, minionId, fromIndex, sellable, golden}
var payload: Dictionary = {}

## 由页面按阶段、视图新鲜度、连接、待选与重排门槛汇总；`false` 时拖拽不开始。
var drag_allowed := false


func _ready() -> void:
    # PanelContainer 默认 STOP：拖放冒泡遇到 STOP 且本控件不接收数据就会中止，
    # 父级棋盘容器再也收不到落点。PASS 既保留本卡的拖拽起手与悬停详情，
    # 又让落点继续交给父级判定。
    mouse_filter = Control.MOUSE_FILTER_PASS


func _get_drag_data(at_position: Vector2) -> Variant:
    if not drag_allowed or payload.is_empty():
        return null
    var preview := duplicate() as Control
    preview.modulate = Color(1, 1, 1, 0.7)
    preview.custom_minimum_size = custom_minimum_size
    set_drag_preview(UI.drag_preview(preview, at_position))
    return payload.duplicate()
