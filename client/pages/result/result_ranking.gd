class_name ResultRanking
extends VBoxContainer
## 结算排名列表：按名次排序，本人行高亮。
##
## 名字由页面注入的取值函数解析，组件不直接读全局状态。


func _init(entries: Array, my_player_id: String, name_of: Callable) -> void:
    name = "RankingList"
    add_theme_constant_override("separation", UITheme.space("space-sm"))
    var sorted := entries.duplicate()
    sorted.sort_custom(func(a, b): return int(a.get("place", 99)) < int(b.get("place", 99)))
    for entry in sorted:
        var player_id := str(entry.get("playerId", ""))
        var row := UIControls.label(tr("ui.result.ranking_row") % [
            _int_of(entry, "place"),
            name_of.call(player_id),
            _int_of(entry, "survivedTurns"),
            _int_of(entry, "damageDealt"),
        ], UITheme.font_size("font-title-lg"))
        if player_id == my_player_id:
            row.modulate = Color("8fd3ff")
        add_child(row)


## 结算字段一律直接取服务端下发值；缺失、空值按 0 处理。
static func _int_of(source: Dictionary, key: String) -> int:
    var value = source.get(key, 0)
    if value == null:
        return 0
    return int(value)
