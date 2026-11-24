class_name BoardAbilityBadge
extends Button
## 棋盘随从主动技能角标：显示费用与剩余次数，只发出继承的 `pressed` / `button_down`。
##
## 可用性、费用与次数一律由页面按服务端 `abilities` 注入；本控件不判断规则、
## 不构造意图。悬停详情取技能名称与文本，缺省时留空，不泄露内部技能 id。

## 所属随从实例 id，与 `SellMinion` 的 `minionId` 一致。
var minion_id := ""
var skill_id := ""
var cost := 0
var uses_left := 0

## 服务端判定可用且当前阶段允许操作；false 时置灰且不响应点击。
var can_use := true:
    set(value):
        can_use = value
        _sync_enabled()


func _init() -> void:
    name = "MinionAbility"
    focus_mode = Control.FOCUS_NONE
    mouse_filter = Control.MOUSE_FILTER_STOP
    clip_text = true
    custom_minimum_size = Vector2(0, UITheme.hit_small())
    add_theme_font_size_override("font_size", UITheme.font_size("font-caption"))
    add_theme_color_override("font_color", UITheme.color("color-text"))
    add_theme_color_override("font_hover_color", UITheme.color("color-text"))
    add_theme_color_override("font_pressed_color", UITheme.color("color-text"))
    add_theme_color_override("font_focus_color", UITheme.color("color-text"))
    add_theme_color_override("font_disabled_color", UITheme.color("color-text-muted"))
    _apply_styles()


## 按注入字段刷新文本、悬停详情与可用状态；尺寸由调用方按卡片空间设置。
func refresh() -> void:
    # 文案表控制费用字形与剩余次数尾缀；多个角标共用同一模板，靠数值区分。
    var template := TranslationServer.translate("ui.game.minion_ability.badge")
    text = template % [cost, uses_left]
    tooltip_text = _skill_tooltip()
    _sync_enabled()


func _sync_enabled() -> void:
    disabled = not can_use


func _skill_tooltip() -> String:
    var info := CardDB.skill_info(skill_id)
    var parts: Array[String] = []
    var display_name := str(info.get("name", ""))
    var body := str(info.get("text", ""))
    if display_name != "":
        parts.append(display_name)
    if body != "":
        parts.append(body)
    return "\n".join(parts)


## 角标沿用紧凑按钮样式，强调色描边表示可发动的主动技能。
func _apply_styles() -> void:
    for state in ["normal", "hover", "pressed", "focus", "disabled"]:
        var box := UITheme.button_style("slot", state)
        box.set_content_margin_all(UITheme.space("space-2xs"))
        box.content_margin_left = UITheme.space("space-xs")
        box.content_margin_right = UITheme.space("space-xs")
        box.set_corner_radius_all(UITheme.radius("radius-sm"))
        if state == "normal" or state == "hover":
            box.bg_color = UITheme.alpha("color-accent", 0.18)
            box.border_color = UITheme.color("color-accent-strong")
        elif state == "pressed":
            box.bg_color = UITheme.alpha("color-accent", 0.3)
            box.border_color = UITheme.color("color-accent")
        elif state == "focus":
            box.border_color = UITheme.color("color-accent")
        add_theme_stylebox_override(state, box)
