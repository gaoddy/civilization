class_name DetailKit
extends RefCounted
## 详情块的公共排版：名称、键值行、文本块与分类/关联/关键词追加。
##
## 文本宽度由详情层按面板令牌传入；子类覆写宽度后仍经同一入口生效。

## 科学分类完整路径的连接符。
const TAXON_PATH_SEPARATOR := " › "

var text_width := 0


func _init(width_value: int) -> void:
    text_width = width_value


func name_label(text: String) -> Label:
    var node := UIControls.label(text, UITheme.font_size("font-title"), false)
    node.name = "DetailName"
    node.max_lines_visible = 1
    node.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    node.mouse_filter = Control.MOUSE_FILTER_IGNORE
    return node


## 详情里的介绍/技能文本块：按面板宽度换行，颜色弱于正文。
func text_block(text: String) -> Label:
    var node := UIControls.label(text, UITheme.font_size("font-body-sm"))
    node.name = "DetailText"
    node.custom_minimum_size = Vector2(text_width, 0)
    node.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
    node.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    return node


func row(prefix: String, value: String) -> HBoxContainer:
    var node := HBoxContainer.new()
    node.mouse_filter = Control.MOUSE_FILTER_IGNORE
    node.add_theme_constant_override("separation", UITheme.space("space-md"))
    var key := UIControls.label(prefix, UITheme.font_size("font-body-sm"), false)
    key.mouse_filter = Control.MOUSE_FILTER_IGNORE
    key.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    node.add_child(key)
    var spacer := Control.new()
    spacer.mouse_filter = Control.MOUSE_FILTER_IGNORE
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    node.add_child(spacer)
    var text := UIControls.label(value, UITheme.font_size("font-body-sm"), false)
    text.mouse_filter = Control.MOUSE_FILTER_IGNORE
    text.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
    node.add_child(text)
    return node


## 科学分类与玩法：分类路径缺失显示「未定」，卡表无该实体时整组跳过。
func append_classification(box: VBoxContainer, entry: Dictionary) -> void:
    if entry.is_empty():
        return
    var path: Array = CardDB.taxon_path(str(entry.get("taxon", "")))
    var path_text := TAXON_PATH_SEPARATOR.join(PackedStringArray(path)) if not path.is_empty() else tr("ui.game.detail.undetermined")
    box.add_child(row(tr("ui.game.detail.taxonomy"), path_text))
    box.add_child(row(tr("ui.game.detail.playstyle"), CardText.playstyle_text(entry)))


## 关联分类：只展示 prey / host，空列表省略整行。
func append_association(box: VBoxContainer, entry: Dictionary) -> void:
    if entry.is_empty():
        return
    var associations := CardText.association_text(entry)
    if associations == "":
        return
    box.add_child(row(tr("ui.game.detail.association"), associations))


## 对局中获得的关键词：摘要行后逐条给出规则说明；没有获得项时不显示。
## 说明缺失的未知关键词只保留摘要，不暴露内部 id。
func append_gained_keywords(box: VBoxContainer, minion: Dictionary) -> void:
    var gained := CardText.gained_keywords(
        str(minion.get("cardId", "")),
        minion.get("keywords", []),
    )
    if gained.is_empty():
        return
    var names := PackedStringArray()
    for raw in gained:
        names.append(CardText.keyword_name(str(raw)))
    var summary := row(tr("ui.game.detail.gained"), "、".join(names))
    summary.name = "DetailGainedKeywords"
    box.add_child(summary)
    for raw in gained:
        var description := CardText.keyword_description(str(raw))
        if description == "":
            continue
        box.add_child(text_block("%s：%s" % [
            CardText.keyword_name(str(raw)),
            description,
        ]))
