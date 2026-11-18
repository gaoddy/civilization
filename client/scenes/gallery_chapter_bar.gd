class_name GalleryChapterBar
extends HFlowContainer
## 图鉴章节栏：全部时代与 1–7 时代的卡牌计数，单选切换。
##
## 计数由页面按当前分类筛选换算后传入；本组件只呈现与发出选择。

signal chapter_selected(era: int)

const ERA_IDS := [
    "prehistoric",
    "primitive",
    "classical",
    "feudal",
    "industrial",
    "modern",
    "future",
]


func _init() -> void:
    name = "GalleryChapters"
    add_theme_constant_override("h_separation", UITheme.space("space-xs"))
    add_theme_constant_override("v_separation", UITheme.space("space-xs"))


## `counts` 以时代序号为键（0 表示全部），`selected` 为当前时代序号。
func set_state(counts: Dictionary, selected: int) -> void:
    UI.clear(self)
    var group := ButtonGroup.new()
    group.allow_unpress = false
    _add_chapter(
        "%s %d" % [tr("ui.gallery.chapter_all"), int(counts.get(0, 0))],
        int(counts.get(0, 0)),
        selected == 0,
        0,
        group,
    )
    for era in range(1, ERA_IDS.size() + 1):
        var label := "%s %d" % [UI.enum_text("era", ERA_IDS[era - 1]), int(counts.get(era, 0))]
        _add_chapter(label, int(counts.get(era, 0)), selected == era, era, group)


func _add_chapter(text: String, count: int, selected: bool, era: int, group: ButtonGroup) -> void:
    var button := Button.new()
    button.name = "ChapterAll" if era == 0 else "Chapter%d" % era
    button.text = text
    button.toggle_mode = true
    button.button_group = group
    button.button_pressed = selected
    button.disabled = count == 0 and not selected
    button.focus_mode = Control.FOCUS_ALL
    button.custom_minimum_size = Vector2(0, 30)
    button.add_theme_font_size_override("font_size", UITheme.font_size("font-body-sm"))
    for state in ["normal", "hover", "pressed", "focus", "disabled"]:
        button.add_theme_stylebox_override(state, UITheme.button_style("default", state))
    button.add_theme_stylebox_override("pressed", UITheme.button_style("primary", "normal"))
    button.add_theme_stylebox_override("hover_pressed", UITheme.button_style("primary", "hover"))
    button.add_theme_color_override("font_color", UITheme.color("color-text"))
    button.add_theme_color_override("font_hover_color", UITheme.color("color-text"))
    button.add_theme_color_override("font_pressed_color", UITheme.color("color-bg"))
    button.add_theme_color_override("font_hover_pressed_color", UITheme.color("color-bg"))
    button.add_theme_color_override("font_focus_color", UITheme.color("color-text"))
    button.add_theme_color_override("font_disabled_color", UITheme.color("color-text-muted"))
    button.pressed.connect(func() -> void: chapter_selected.emit(era))
    add_child(button)
