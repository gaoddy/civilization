extends Control
## 结算：展示服务端下发的排名与对局种子。

func _ready() -> void:
    _build()


func _build() -> void:
    var background := ColorRect.new()
    background.color = Color("10141c")
    background.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(background)

    var center := CenterContainer.new()
    center.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(center)

    var panel := UI.panel()
    panel.custom_minimum_size = Vector2(460, 0)
    center.add_child(panel)

    var margin := MarginContainer.new()
    for side in ["left", "right", "top", "bottom"]:
        margin.add_theme_constant_override("margin_" + side, 24)
    panel.add_child(margin)

    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", 10)
    margin.add_child(box)

    box.add_child(UI.label("对局结束", 28))
    box.add_child(UI.label(_my_place_text(), 20))
    box.add_child(HSeparator.new())

    var ranking := VBoxContainer.new()
    ranking.name = "RankingList"
    ranking.add_theme_constant_override("separation", 6)
    var entries: Array = GameStore.ranking.duplicate()
    entries.sort_custom(func(a, b): return int(a.get("place", 99)) < int(b.get("place", 99)))
    for entry in entries:
        var player_id := str(entry.get("playerId", ""))
        var row := UI.label("第 %d 名   %s" % [
            int(entry.get("place", 0)),
            GameStore.player_name(player_id),
        ], 18)
        if player_id == GameStore.player_id:
            row.modulate = Color("8fd3ff")
        ranking.add_child(row)
    box.add_child(ranking)

    box.add_child(UI.label("对局种子：%s" % GameStore.game_seed, 14))
    var back := UI.button("返回大厅")
    back.pressed.connect(_on_back_pressed)
    box.add_child(back)


func _my_place_text() -> String:
    for entry in GameStore.ranking:
        if str(entry.get("playerId", "")) == GameStore.player_id:
            return "你的名次：第 %d 名" % int(entry.get("place", 0))
    return "你的名次：未记录"


func _on_back_pressed() -> void:
    GameStore.reset_room()
    Net.relogin()
    UI.navigate(self, "res://scenes/lobby.tscn")
