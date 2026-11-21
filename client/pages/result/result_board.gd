class_name ResultBoard
extends VBoxContainer
## 结算阵容列表：逐条展示随从名称与最终攻防，无阵容时显示空态。


func _init(items: Array) -> void:
    name = "BoardList"
    add_theme_constant_override("separation", UITheme.space("space-xs"))
    if items.is_empty():
        add_child(UIControls.label(tr("ui.result.board_empty"), UITheme.font_size("font-title")))
        return
    for item in items:
        if not (item is Dictionary):
            continue
        var card_id := str(item.get("cardId", ""))
        var card_name := CardDB.display_name(card_id)
        if bool(item.get("golden", false)):
            card_name += " ★"
        add_child(UIControls.label(tr("ui.result.board_item") % [
            card_name,
            _int_of(item, "attack"),
            _int_of(item, "health"),
        ], UITheme.font_size("font-title")))


## 结算字段一律直接取服务端下发值；缺失、空值按 0 处理。
static func _int_of(source: Dictionary, key: String) -> int:
    var value = source.get(key, 0)
    if value == null:
        return 0
    return int(value)
