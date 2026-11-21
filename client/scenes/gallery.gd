extends Control
## 卡牌图鉴页：大厅入口，浏览全部卡牌，返回大厅。

func _ready() -> void:
    # 页面自带整屏背景，图鉴组件不再叠一层；安全区只约束内容。
    add_child(UI.background())
    var gallery := CardGallery.new()
    gallery.set_anchors_preset(Control.PRESET_FULL_RECT)
    gallery.set_background_visible(false)
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
