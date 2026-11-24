extends Control
## 卡牌图鉴页：大厅入口，浏览全部卡牌，返回大厅。
##
## 卡牌数据由 `CardDB` 从服务端下载；未就绪时展示加载或失败提示，
## 失败时提供重试，数据到达后重建目录。

var _gallery: CardGallery
var _status_root: CenterContainer
var _status_label: Label
var _retry: Button
## 已渲染的内容版本：版本未变时不重建目录，避免重连刷新重置筛选。
var _rendered_version := ""


func _ready() -> void:
    UILayout.safe_area(self)
    # 页面自带整屏背景，图鉴组件不再叠一层；安全区只约束内容。
    add_child(UIControls.background())
    _gallery = CardGallery.new()
    _gallery.set_anchors_preset(Control.PRESET_FULL_RECT)
    _gallery.set_background_visible(false)
    add_child(_gallery)
    _gallery.closed.connect(_on_closed)
    _build_status()
    CardDB.content_changed.connect(_refresh)
    CardDB.content_failed.connect(_on_content_failed)
    _refresh()


func _build_status() -> void:
    var center := CenterContainer.new()
    center.name = "GalleryContentStatus"
    center.set_anchors_preset(Control.PRESET_FULL_RECT)
    center.mouse_filter = Control.MOUSE_FILTER_IGNORE
    add_child(center)
    _status_root = center

    var status := VBoxContainer.new()
    status.name = "GalleryContentColumn"
    status.alignment = BoxContainer.ALIGNMENT_CENTER
    status.add_theme_constant_override("separation", UITheme.space("space-md"))
    center.add_child(status)

    _status_label = UIControls.label("", UITheme.font_size("font-body"), false)
    _status_label.name = "GalleryContentLabel"
    _status_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _status_label.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    status.add_child(_status_label)

    _retry = UIControls.styled_button(tr("ui.content.retry"), "primary", 30)
    _retry.name = "GalleryContentRetry"
    _retry.pressed.connect(CardDB.fetch)
    status.add_child(_retry)


func _refresh() -> void:
    _gallery.visible = CardDB.is_loaded()
    if CardDB.is_loaded():
        var version := CardDB.content_version()
        if version != _rendered_version:
            _gallery.set_catalog(CardDB.cards.keys(), tr("ui.gallery.back_lobby"))
            _rendered_version = version
    _sync_status()


func _on_content_failed(reason: String) -> void:
    push_warning("图鉴内容不可用：%s" % reason)
    _sync_status()


func _sync_status() -> void:
    var loaded := CardDB.is_loaded()
    _status_root.visible = not loaded
    _gallery.visible = loaded
    if loaded:
        return
    var failed := CardDB.has_failed()
    _status_label.text = tr("ui.content.failed") if failed else tr("ui.content.loading")
    _retry.visible = failed


func _on_closed() -> void:
    UINavigation.navigate(self, UINavigation.PAGE_LOBBY)


## Android 系统返回：图鉴的筛选面板、详情抽屉由 SystemBack 先收起；
## 没有打开的子浮层时执行与页面返回按钮相同的动作，回大厅。
func system_back() -> bool:
    _on_closed()
    return true
