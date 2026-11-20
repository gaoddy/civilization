extends Control
## 卡牌图鉴页：大厅入口，浏览全部卡牌与 Token，返回大厅。

func _ready() -> void:
    var gallery := CardGallery.new()
    gallery.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(gallery)
    gallery.closed.connect(_on_closed)
    gallery.set_catalog(
        CardDB.cards.keys(),
        tr("ui.gallery.title"),
        tr("ui.gallery.back_lobby"),
    )
    UI.safe_area(self)


func _on_closed() -> void:
    UI.navigate(self, "res://scenes/lobby.tscn")
