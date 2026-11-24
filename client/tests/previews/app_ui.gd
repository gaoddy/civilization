extends Node
## App UI 预览：一个窗口逐屏检查真实页面、共享组件与对局演出。
##
## 入口：`make ui-preview SETUP=res://tests/previews/app_ui.gd`
## 按键：1 登录 / 2 大厅 / 3 排队 / 4 房间 / 5 对局 / 6 对局·战斗 /
##       7 组件总览 / 8 攻击掉血 / 9 战吼选目标 / 0 框体材质开关
##
## 页面屏直接实例化 `client/pages/` 下的真实场景，并按 `PreviewFixture` 注入最小状态；
## 组件屏用同一批共享组件现场拼装，方便逐项检查卡框、星级、图标与按钮表面。

const PreviewFixture = preload("res://tests/preview_fixture.gd")

const SCREENS := [
	{"id": "login", "key": KEY_1, "label": "登录", "page": "login", "fixture": "none"},
	{"id": "lobby", "key": KEY_2, "label": "大厅", "page": "lobby", "fixture": "lobby"},
	{"id": "lobby_queue", "key": KEY_3, "label": "排队", "page": "lobby", "fixture": "lobby_queue"},
	{"id": "room", "key": KEY_4, "label": "房间", "page": "room", "fixture": "room"},
	{"id": "game", "key": KEY_5, "label": "对局", "page": "game", "fixture": "game"},
	{"id": "game_combat", "key": KEY_6, "label": "对局·战斗", "page": "game", "fixture": "game_combat"},
	{"id": "components", "key": KEY_7, "label": "组件总览", "builder": "_build_components"},
	{"id": "combat_fx", "key": KEY_8, "label": "攻击掉血", "builder": "_build_combat"},
	{"id": "targeting", "key": KEY_9, "label": "战吼选目标", "builder": "_build_targeting"},
]

const ERA_SAMPLES := [
	{"era": "prehistoric", "id": "ammonite"},
	{"era": "primitive", "id": "ant"},
	{"era": "classical", "id": "argentavis"},
	{"era": "feudal", "id": "bisheng"},
	{"era": "industrial", "id": "adam-smith"},
	{"era": "modern", "id": "airplane"},
	{"era": "future", "id": "agi"},
]
const ERA_LABELS := {
	"prehistoric": "史前",
	"primitive": "原始",
	"classical": "古典",
	"feudal": "封建",
	"industrial": "工业",
	"modern": "现代",
	"future": "未来",
}
const ERA_CARD_SIZE := Vector2(138, 190)
const COMBAT_CARD_SIZE := Vector2(150, 205)
const TARGET_CARD_SIZE := Vector2(140, 190)
const ATTACK_INTERVAL := 1.9

const TARGET_MINIONS := [
	{"minionId": "demo-source", "cardId": "adam-smith", "golden": false},
	{"minionId": "demo-left", "cardId": "airplane", "golden": true},
	{"minionId": "demo-right", "cardId": "ant", "golden": false},
]

var _root: Control
var _hud: Label
var _start_index := 0
var _content: Control = null
var _index := 0
var _material_on := true
var _fx: CombatFx
var _fx_left: Control
var _fx_right: Control
var _aim: GameAimLayer
var _clock := 0.0
var _flip := false


static func build(opts: Dictionary = {}) -> Node:
	var root := Control.new()
	root.name = "AppUI"
	root.set_anchors_preset(Control.PRESET_FULL_RECT)
	var controller := new()
	controller.name = "AppUIController"
	controller._root = root
	controller._start_index = screen_index(str(opts.get("page", "")))
	root.add_child(controller)
	root.tree_entered.connect(func() -> void: controller._start(), CONNECT_ONE_SHOT)
	return root


## PAGE=<屏幕 id> 可指定起始屏，便于 `make ui-preview-check SETUP=... PAGE=lobby` 定点自检。
static func screen_index(id: String) -> int:
	for index in SCREENS.size():
		if str(SCREENS[index]["id"]) == id:
			return index
	return 0


func _start() -> void:
	_hud = UIControls.label("", 13, false)
	_hud.add_theme_color_override("font_color", UITheme.color("color-accent"))
	_hud.position = Vector2(14, 10)
	_hud.z_index = 100
	_root.add_child(_hud)

	var catcher := KeyCatcher.new()
	catcher.on_key = func(key: int) -> void: _on_key(key)
	_root.add_child(catcher)
	_show(_start_index)


func _on_key(key: int) -> void:
	if key == KEY_0:
		_material_on = not _material_on
		_set_overlays_visible(_material_on)
		_update_hud()
		return
	for index in SCREENS.size():
		if int(SCREENS[index]["key"]) == key:
			_show(index)
			return


func _show(index: int) -> void:
	_index = index
	if is_instance_valid(_content):
		_root.remove_child(_content)
		_content.queue_free()
	_content = null
	_fx = null
	_fx_left = null
	_fx_right = null
	_aim = null
	_clock = 0.0

	var spec: Dictionary = SCREENS[index]
	var built: Node = null
	if spec.has("page"):
		PreviewFixture.apply(str(spec["fixture"]))
		var page := str(spec["page"])
		var packed = load("res://pages/%s/%s.tscn" % [page, page])
		if packed is PackedScene:
			built = (packed as PackedScene).instantiate()
	else:
		built = call(str(spec["builder"]))
	if not (built is Control):
		return
	_content = built as Control
	_root.add_child(_content)
	_update_hud()

	# 等页面自建完布局再套材质：材质按可见表面收集，需要真实尺寸。
	await _root.get_tree().process_frame
	await _root.get_tree().process_frame
	if _aim != null:
		_start_targeting()
	_apply_material()


func _update_hud() -> void:
	if _hud == null:
		return
	var keys := PackedStringArray()
	for index in SCREENS.size():
		keys.append("%d %s" % [index + 1, SCREENS[index]["label"]])
	_hud.text = "App UI · %s　—　%s　/　0 %s" % [
		SCREENS[_index]["label"],
		" / ".join(keys),
		"关闭框体材质" if _material_on else "打开框体材质",
	]


func _apply_material() -> void:
	if _content == null:
		return
	FrameMaterial.apply_to_tree(_content, FrameMaterial.Style.BEVEL)
	_set_overlays_visible(_material_on)


func _set_overlays_visible(on: bool) -> void:
	for node in _root.find_children(FrameMaterial.OVERLAY_NAME, "ColorRect", true, false):
		(node as ColorRect).visible = on


# ── 组件总览 ──────────────────────────────────────────────────────────────

func _build_components() -> Control:
	var view := _page()
	var center := CenterContainer.new()
	center.set_anchors_preset(Control.PRESET_FULL_RECT)
	view.add_child(center)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 22)
	center.add_child(column)

	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 14)
	column.add_child(row)
	for sample in ERA_SAMPLES:
		row.add_child(_era_column(str(sample["era"]), str(sample["id"]), false))
	row.add_child(_era_column("feudal", "adam-smith", true))
	column.add_child(_controls_row())
	return view


func _era_column(era: String, card_id: String, golden: bool) -> Control:
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 8)
	column.add_child(_card(card_id, golden, ERA_CARD_SIZE))
	var label := UIControls.label("三连金框" if golden else str(ERA_LABELS.get(era, era)), 13, false)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.add_theme_color_override(
		"font_color",
		Color("ffe089") if golden else FrameMaterial.tint_for_era(era).lightened(0.2),
	)
	column.add_child(label)
	return column


## 按钮与面板：同一套材质下检查主次按钮、悬停流光与面板厚度。
func _controls_row() -> Control:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 18)
	row.alignment = BoxContainer.ALIGNMENT_CENTER
	var primary := UIControls.styled_button("确认出战", "primary")
	primary.custom_minimum_size.x = 180
	row.add_child(primary)
	var secondary := UIControls.styled_button("取消")
	secondary.custom_minimum_size.x = 140
	row.add_child(secondary)
	var panel := UIControls.card_panel()
	panel.custom_minimum_size = Vector2(280, 0)
	var pad := MarginContainer.new()
	for side in ["left", "top", "right", "bottom"]:
		pad.add_theme_constant_override("margin_" + str(side), 14)
	pad.add_child(UIControls.label("面板表面 · 材质与圆角", 13, false))
	panel.add_child(pad)
	row.add_child(panel)
	return row


# ── 攻击掉血 ──────────────────────────────────────────────────────────────

func _build_combat() -> Control:
	var view := _page()
	var center := CenterContainer.new()
	center.set_anchors_preset(Control.PRESET_FULL_RECT)
	view.add_child(center)
	var pair := HBoxContainer.new()
	pair.add_theme_constant_override("separation", 220)
	center.add_child(pair)
	_fx_left = _card("adam-smith", false, COMBAT_CARD_SIZE)
	_fx_right = _card("airplane", false, COMBAT_CARD_SIZE)
	pair.add_child(_fx_left)
	pair.add_child(_fx_right)

	_fx = CombatFx.new()
	_fx.set_anchors_preset(Control.PRESET_FULL_RECT)
	view.add_child(_fx)
	return view


# ── 战吼选目标 ────────────────────────────────────────────────────────────

func _build_targeting() -> Control:
	var view := _page()
	var center := CenterContainer.new()
	center.set_anchors_preset(Control.PRESET_FULL_RECT)
	view.add_child(center)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 80)
	center.add_child(row)
	for minion in TARGET_MINIONS:
		var holder := _card(
			str(minion["cardId"]), bool(minion["golden"]), TARGET_CARD_SIZE
		)
		holder.set_meta("target_zone", "board")
		holder.set_meta("target_id", str(minion["minionId"]))
		row.add_child(holder)

	_aim = GameAimLayer.new()
	_aim.set_anchors_preset(Control.PRESET_FULL_RECT)
	view.add_child(_aim)
	return view


func _start_targeting() -> void:
	_aim.begin_pending("demo-source", [
		{"zone": "board", "id": "demo-left"},
		{"zone": "board", "id": "demo-right"},
	])


# ── 公共 ──────────────────────────────────────────────────────────────────

func _page() -> Control:
	var view := Control.new()
	view.set_anchors_preset(Control.PRESET_FULL_RECT)
	var bg := ColorRect.new()
	bg.color = Color("080b10")
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	view.add_child(bg)
	return view


func _card(card_id: String, golden: bool, size: Vector2) -> Control:
	# 内边距交给 MarginContainer：PanelContainer 留着零边距，材质 overlay 才能贴住卡边。
	var holder := PanelContainer.new()
	holder.custom_minimum_size = size
	var base := StyleBoxFlat.new()
	base.bg_color = UITheme.color("color-surface")
	base.set_corner_radius_all(int(UITheme.radius("radius-md")))
	holder.add_theme_stylebox_override("panel", base)
	var pad := MarginContainer.new()
	for side in ["left", "top", "right", "bottom"]:
		pad.add_theme_constant_override("margin_" + str(side), 5)
	var entry := CardDB.card(card_id)
	pad.add_child(HandCardFace.build({
		"cardId": card_id,
		"attack": entry.get("attack"),
		"health": entry.get("health"),
		"golden": golden,
	}))
	holder.add_child(pad)
	return holder


func _process(delta: float) -> void:
	if _fx == null:
		return
	_fx.advance(delta)
	if _fx_left == null or _fx_right == null:
		return
	_clock += delta
	if _clock >= ATTACK_INTERVAL:
		_clock = 0.0
		_flip = not _flip
		_fx.play_attack(_fx_left if _flip else _fx_right, _fx_right if _flip else _fx_left, 7)


class KeyCatcher extends Node:
	var on_key: Callable

	func _unhandled_key_input(event: InputEvent) -> void:
		if event is InputEventKey and event.pressed and not event.echo:
			on_key.call((event as InputEventKey).keycode)
