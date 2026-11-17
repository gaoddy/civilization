class_name GameSidePanel
extends VBoxContainer
## 右栏：我的英雄、英雄技能、结束招募与战报。
##
## 只发按钮信号，不发送意图；是否禁用由 `refresh(state)` 按页面门控决定。
## 战报文本由页面写入 `log_text`。

signal hero_power_pressed
signal end_recruit_pressed

var log_text: RichTextLabel

var _self_hero_box: VBoxContainer
var _hero_power_button: Button
var _end_button: Button


func _init() -> void:
    name = "SidePanel"
    custom_minimum_size = Vector2(UITheme.metric("side-width"), 0)
    add_theme_constant_override("separation", UITheme.space("space-sm"))

    var hero_panel := UI.card_panel("surface")
    hero_panel.name = "SelfHeroPanel"
    _self_hero_box = VBoxContainer.new()
    _self_hero_box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    hero_panel.add_child(_self_hero_box)
    add_child(hero_panel)

    _hero_power_button = UI.styled_button("英雄技能", "default", 40)
    _hero_power_button.name = "HeroPowerButton"
    _hero_power_button.pressed.connect(func() -> void: hero_power_pressed.emit())
    add_child(_hero_power_button)

    _end_button = UI.styled_button("结束招募", "primary", 46)
    _end_button.name = "EndButton"
    _end_button.pressed.connect(func() -> void: end_recruit_pressed.emit())
    add_child(_end_button)

    var log_panel := UI.card_panel("surface")
    log_panel.name = "LogPanel"
    log_panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
    var log_box := VBoxContainer.new()
    log_box.add_theme_constant_override("separation", UITheme.space("space-xs"))
    log_panel.add_child(log_box)
    log_box.add_child(UI.section_title("战报与事件"))
    log_text = RichTextLabel.new()
    log_text.name = "LogText"
    log_text.fit_content = false
    log_text.scroll_active = true
    log_text.size_flags_vertical = Control.SIZE_EXPAND_FILL
    log_text.add_theme_font_size_override("normal_font_size", UITheme.font_size("font-body-sm"))
    log_text.add_theme_color_override("default_color", UITheme.color("color-text-dim"))
    log_box.add_child(log_text)
    add_child(log_panel)


func refresh(state: GameUiState) -> void:
    _refresh_hero(state.self_state)
    _refresh_actions(state)


func set_combat_mode(in_combat: bool) -> void:
    _hero_power_button.visible = not in_combat
    _end_button.visible = not in_combat


func _refresh_hero(self_state: Dictionary) -> void:
    UI.clear(_self_hero_box)
    var hero_id := GameCardFace.optional_text(self_state.get("heroCardId"))
    _self_hero_box.add_child(UI.hero_tile(hero_id, UITheme.metric("side-width") - UITheme.space("space-2xl")))

    var stat_line := UI.label("生命 %d · %s · 金币 %d" % [
        int(self_state.get("hp", 0)),
        GameCardFace.era_text(int(self_state.get("era", 1))),
        int(self_state.get("gold", 0)),
    ], UITheme.font_size("font-body-sm"), false)
    stat_line.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    _self_hero_box.add_child(stat_line)

    var power: Dictionary = self_state.get("heroPower", {})
    var skill_id := GameCardFace.optional_text(power.get("skillId"))
    var skill_text := CardDB.display_text(skill_id) if skill_id != "" else ""
    if skill_text != "":
        var skill := UI.label(skill_text, UITheme.font_size("font-caption"))
        skill.max_lines_visible = 2
        skill.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
        skill.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
        _self_hero_box.add_child(skill)


func _refresh_actions(state: GameUiState) -> void:
    var aff: Dictionary = state.self_state.get("affordances", {})
    var blocked := state.stale or state.expired or state.target_pending

    var power: Dictionary = state.self_state.get("heroPower", {})
    _hero_power_button.text = "英雄技能（剩 %d）" % int(power.get("usesLeft", 0))
    _hero_power_button.disabled = blocked or not bool(aff.get("canUseHeroPower", false))
    _hero_power_button.tooltip_text = "" if aff.get("heroPowerDisabledReason") == null else UI.error_text(str(aff.get("heroPowerDisabledReason")))

    _end_button.text = "已结束招募" if state.recruited else "结束招募"
    _end_button.disabled = blocked or not bool(aff.get("canEndRecruit", false))
