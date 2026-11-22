class_name PillButton
extends Button
## 胶囊按钮：全局共用的圆角药丸按钮，单行文本，视觉取值来自 `UITheme.pill_button_style`。
##
## 只发出继承的 `pressed` 信号，不构造业务意图；变体决定底色与文字色。

const VARIANTS := ["neutral", "primary", "danger"]

var variant := "neutral"


## `height` 为 0 时取全局胶囊高度（触控端抬到次要操作命中下限）；
## 传正值则固定为该高度，用于需要三端一致的场景，例如图鉴头部工具行。
func _init(text_value := "", variant_value := "neutral", height_value := 0) -> void:
	variant = variant_value if VARIANTS.has(variant_value) else "neutral"
	text = text_value
	focus_mode = Control.FOCUS_ALL
	var height := UITheme.stage_metric("pill-button-height") if height_value <= 0 else height_value
	if height_value <= 0 and ScreenProfile.is_touch():
		height = maxi(height, UITheme.hit_small())
	custom_minimum_size = Vector2(0, height)
	add_theme_font_size_override("font_size", UITheme.font_size("font-body-sm"))
	_apply_styles()


## 切换变体；未知变体回落到 neutral。
func set_variant(value: String) -> void:
	variant = value if VARIANTS.has(value) else "neutral"
	_apply_styles()


func _apply_styles() -> void:
	for state in ["normal", "hover", "pressed", "hover_pressed", "focus", "disabled"]:
		add_theme_stylebox_override(state, UITheme.pill_button_style(variant, state))
	var text_color := (
		UITheme.color("color-bg")
		if variant == "primary"
		else UITheme.color("color-text")
	)
	for state in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
		add_theme_color_override(state, text_color)
	add_theme_color_override("font_disabled_color", UITheme.color("color-text-muted"))
