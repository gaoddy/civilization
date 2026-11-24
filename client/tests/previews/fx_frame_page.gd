extends Node
## 框体材质整页对比：在对局页真实布局上铺 A / B 两套框体材质。
##
## 入口：`make ui-preview SETUP=res://tests/previews/fx_frame_page.gd FIXTURE=game`
## 1 = A 黄铜浮雕，2 = B 蚀刻铜锈，0 = 关闭材质。
## 只加框体一层，布局与信息层级不动，便于判断材质在真实密度下是否成立。

const PAGE_SCENE := "res://pages/game/game.tscn"
const STYLE_NAMES := ["关闭", "A 黄铜浮雕", "B 蚀刻铜锈"]

## 方向差异的氛围层：A 加暖色顶光与轻暗角，B 整体做旧（褐色罩层 + 颗粒 + 重暗角）。
const ATMOSPHERE_SHADER := """
shader_type canvas_item;
uniform int mode = 0;

void fragment() {
	float vig = smoothstep(0.30, 0.92, length((UV - vec2(0.5, 0.45)) * vec2(1.10, 1.0)));
	if (mode == 0) {
		float glow = smoothstep(0.62, 0.0, length((UV - vec2(0.5, 0.02)) * vec2(0.85, 1.35)));
		vec3 warm = vec3(1.0, 0.74, 0.40);
		float ga = glow * 0.13;
		float va = vig * 0.14;
		vec3 col = mix(warm, vec3(0.0), va / max(ga + va, 0.0001));
		COLOR = vec4(col, ga + va);
	} else {
		float grain = fract(sin(dot(UV * 731.0, vec2(12.9898, 78.233))) * 43758.5453);
		vec3 tint = vec3(0.15, 0.12, 0.07);
		float ga = 0.06 + grain * 0.10;
		float va = vig * 0.40;
		vec3 col = mix(tint, vec3(0.0), va / max(ga + va, 0.0001));
		COLOR = vec4(col, ga + va);
	}
}
"""

var _root: Control
var _page: Control
var _hud: Label
var _style := 0


static func build(opts: Dictionary = {}) -> Node:
	var root := Control.new()
	root.name = "FramePageLab"
	root.set_anchors_preset(Control.PRESET_FULL_RECT)

	var packed := load(PAGE_SCENE) as PackedScene
	if packed == null:
		return root
	var page := packed.instantiate() as Control
	page.name = "PreviewPage"
	root.add_child(page)

	var lab := new()
	lab.name = "FramePageLabController"
	root.add_child(lab)
	lab._root = root
	lab._page = page
	lab._style = int(OS.get_environment("FX_LAB_STYLE")) if OS.get_environment("FX_LAB_STYLE") != "" else 1
	root.tree_entered.connect(func() -> void: lab._start(), CONNECT_ONE_SHOT)
	return root


func _start() -> void:
	await _root.get_tree().create_timer(0.5).timeout
	_apply(_style)
	_hud = UIControls.label("", 13, false)
	_hud.add_theme_color_override("font_color", UITheme.color("color-accent"))
	_hud.position = Vector2(12, 8)
	_hud.z_index = 100
	_root.add_child(_hud)
	_update_hud()
	_hook_keys()


func _hook_keys() -> void:
	var catcher := KeyCatcher.new()
	catcher.on_key = func(key: int) -> void:
		if key == KEY_0:
			_apply(0)
		elif key == KEY_1:
			_apply(1)
		elif key == KEY_2:
			_apply(2)
	_root.add_child(catcher)


func _apply(style: int) -> void:
	_style = style
	FrameMaterial.clear_tree(_page)
	var count := 0
	if style > 0:
		count = FrameMaterial.apply_to_tree(_page, style - 1)
	_apply_atmosphere(style)
	print("FX_FRAME_PAGE style=%d surfaces=%d" % [style, count])
	_update_hud()


## 氛围层：整页一张叠图，不参与布局、不拦截鼠标。
func _apply_atmosphere(style: int) -> void:
	var existing := _root.get_node_or_null("FrameAtmosphere")
	if existing != null:
		existing.queue_free()
	if style <= 0:
		return
	var shader := Shader.new()
	shader.code = ATMOSPHERE_SHADER
	var material := ShaderMaterial.new()
	material.shader = shader
	material.set_shader_parameter("mode", style - 1)
	var overlay := ColorRect.new()
	overlay.name = "FrameAtmosphere"
	overlay.color = Color(1, 1, 1, 1)
	overlay.material = material
	overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
	# 垫在页面之上、HUD 之下：方向差异只作用于页面本身。
	_root.add_child(overlay)
	_root.move_child(overlay, _root.get_child_count() - 1)
	overlay.z_index = 50


func _update_hud() -> void:
	if _hud == null:
		return
	_hud.text = "框体材质：%s — 按 1 / 2 / 0 切换" % STYLE_NAMES[_style]


class KeyCatcher extends Node:
	var on_key: Callable

	func _unhandled_key_input(event: InputEvent) -> void:
		if event is InputEventKey and event.pressed and not event.echo:
			on_key.call((event as InputEventKey).keycode)
