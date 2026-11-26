extends Node
## 卡顶栏实验台：直接调用共享 `CardTopRow`，覆盖三档卡宽、五种状态、七种时代与棋盘费用位。
##
## 入口：
##   make ui-preview SETUP=res://tests/previews/card_corner_lab.gd
## 一个窗口同时摆 112×146 / 78×102 / 70×91 三档宽：78 与 70 在手机令牌下构建，
## 字号与信息区高度和真机一致。
##
## 卡面、信息区、卡框、框体材质、潜行蒙版与顶栏全部走共享层，实验台只负责摆样，
## 因此这里看到的就是卡面实装后的样子。

const CARD_ID := "adam-smith"
const STATES := ["普通", "金色", "奖励牌 L2", "潜行", "冻结"]

## 每个时代一张真实随从卡：等级 1–6 各一张，用来检查星色与时代节点
## 在七种时代合金色下是否都成立。
const ERA_CARDS := [
	["prehistoric", "amoeba"],
	["primitive", "ant"],
	["classical", "clownfish"],
	["feudal", "black-death"],
	["industrial", "adam-smith"],
	["modern", "airplane"],
	["future", "agi"],
]

var _column: VBoxContainer = null
var _hud: Label = null


static func build(_opts: Dictionary = {}) -> Node:
	var root := Control.new()
	root.name = "CardCornerLab"
	root.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.add_child(UIControls.background())

	var margin := MarginContainer.new()
	margin.name = "LabMargin"
	margin.set_anchors_preset(Control.PRESET_FULL_RECT)
	for side in ["margin_left", "margin_top", "margin_right", "margin_bottom"]:
		margin.add_theme_constant_override(side, 12)
	root.add_child(margin)

	var scroll := ScrollContainer.new()
	scroll.name = "LabScroll"
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	margin.add_child(scroll)

	var column := VBoxContainer.new()
	column.name = "LabColumn"
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column.add_theme_constant_override("separation", 12)
	scroll.add_child(column)

	var lab := new()
	lab.name = "CardCornerLabController"
	root.add_child(lab)
	lab._column = column
	lab._build_section()
	root.tree_entered.connect(func() -> void: lab._start(), CONNECT_ONE_SHOT)
	return root


func _start() -> void:
	_hud = UIControls.label("时代在左 / 等级居中 / 费用在右", UITheme.font_size("font-body-sm"), false)
	_hud.name = "LabHud"
	_hud.add_theme_color_override("font_color", UITheme.color("color-text"))
	_hud.add_theme_constant_override("outline_size", 4)
	_hud.add_theme_color_override("font_outline_color", UITheme.color("color-bg"))
	_hud.set_anchors_preset(Control.PRESET_BOTTOM_LEFT)
	_hud.offset_left = 14
	_hud.offset_top = -30
	_hud.offset_right = 720
	_hud.offset_bottom = -8
	_hud.z_index = 100
	add_child(_hud)


func _build_section() -> void:
	var host_device := ScreenProfile.device_kind()
	var section := VBoxContainer.new()
	section.name = "SectionA"
	section.add_theme_constant_override("separation", 4)
	var head := UIControls.label(
		"卡顶栏 — 时代在左、等级居中、费用在右；只有等级星带胶囊底，星色随时代合金色",
		UITheme.font_size("font-caption"),
		false,
	)
	head.add_theme_color_override("font_color", UITheme.color("color-accent"))
	section.add_child(head)
	for size_spec in _sizes():
		section.add_child(_states_line(int(size_spec[0]), str(size_spec[1]), host_device))
	section.add_child(_sub_head("各时代（等级 1–6）：星色与时代节点随时代合金色变化"))
	for size_spec in _sizes():
		section.add_child(_era_line(int(size_spec[0]), str(size_spec[1]), host_device))
	section.add_child(_sub_head("棋盘费用位：自己棋盘显示发动技能费用，对手棋盘与无主动技能的卡留空"))
	for size_spec in [[112, "112 桌面"], [70, "70 手牌"]]:
		section.add_child(_board_line(int(size_spec[0]), str(size_spec[1]), host_device))
	_column.add_child(section)


func _sub_head(text: String) -> Label:
	var label := UIControls.label(text, UITheme.font_size("font-micro"), false)
	label.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
	return label


## 三档卡宽一次摆齐：桌面商店/棋盘 112×146、手机随从 78×102、手机手牌 70×91。
static func _sizes() -> Array:
	return [[112, "112 桌面"], [78, "78 随从"], [70, "70 手牌"]]


## 78 / 70 两档在手机令牌下构建：信息区高度、名称与数值字号都取手机值，
## 和真机卡面一致；返回是否切过档位，构建完由调用方切回宿主档位。
func _enter_phone(width: int) -> bool:
	if width >= UITheme.metric("card-width"):
		return false
	ScreenProfile.set_device_for_test(ScreenProfile.DEVICE_PHONE)
	return true


func _line_head(tag_text: String) -> HBoxContainer:
	var line := HBoxContainer.new()
	line.add_theme_constant_override("separation", 8)
	var tag := UIControls.label(tag_text, UITheme.font_size("font-micro"), false)
	tag.custom_minimum_size = Vector2(34, 0)
	tag.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
	line.add_child(tag)
	return line


func _states_line(width: int, tag_text: String, host_device: String) -> Control:
	var phone := _enter_phone(width)
	var line := _line_head(tag_text)
	for state in STATES.size():
		# 潜行只出现在棋盘卡上：这一行用空费用位展示棋盘卡面。
		line.add_child(_specimen(state, width, CARD_ID, "", state == 3))
	if phone:
		ScreenProfile.set_device_for_test(host_device)
	return line


## 七种时代一条：同一档宽度下把七张真实卡并排，等级 1–6 覆盖不同星数。
func _era_line(width: int, tag_text: String, host_device: String) -> Control:
	var phone := _enter_phone(width)
	var line := _line_head(tag_text)
	for pair in ERA_CARDS:
		var era_id := str(pair[0])
		var caption := TranslationServer.translate("enum.era.%s" % era_id)
		line.add_child(_specimen(0, width, str(pair[1]), caption))
	if phone:
		ScreenProfile.set_device_for_test(host_device)
	return line


## 棋盘费用位的三种情形：自己棋盘显示主动技能的发动费用；对手棋盘与无主动技能的卡留空。
func _board_line(width: int, tag_text: String, host_device: String) -> Control:
	var phone := _enter_phone(width)
	var line := _line_head(tag_text)
	var own_cost := _active_skill_cost("beethoven")
	var cases := [
		["beethoven", "自己棋盘 · 技能费用 %s" % own_cost, own_cost],
		["beethoven", "对手棋盘", ""],
		["amoeba", "自己棋盘 · 无主动技能", _active_skill_cost("amoeba")],
	]
	for item in cases:
		line.add_child(_specimen(0, width, str(item[0]), str(item[1]), true, str(item[2])))
	if phone:
		ScreenProfile.set_device_for_test(host_device)
	return line


## 随从主动技能的发动费用；没有主动技能时返回空串，费用位留空。
static func _active_skill_cost(card_id: String) -> String:
	var entry := CardDB.card(card_id)
	for skill_id in entry.get("skills", []):
		var cost := CardDB.skill_cost(str(skill_id))
		if cost >= 0:
			return str(cost)
	return ""


## 单张样卡：卡面、信息区、卡框、框体材质、潜行蒙版与顶栏全部走共享层。
static func _specimen(
	state: int,
	width: int,
	card_id := CARD_ID,
	caption_text := "",
	board := false,
	skill_cost := "",
) -> Control:
	var height := UITheme.card_face_height(width)
	var golden := state == 1
	var reward := "2" if state == 2 else ""
	var stealth := state == 3
	var frozen := state == 4
	var entry := CardDB.card(card_id)
	# 棋盘卡与商店卡一样显示时代；潜行只出现在棋盘卡上，所以也不带费用。
	var era := str(entry.get("era", ""))
	var cost := ""
	if board:
		# 自己棋盘的费用位改显示发动技能的费用；对手棋盘与无主动技能的卡留空。
		cost = skill_cost
	elif not stealth:
		cost = str(CardDB.display_cost(card_id))

	var cell := VBoxContainer.new()
	cell.name = "Cell_%d_%s" % [state, card_id]
	cell.add_theme_constant_override("separation", 3)

	var holder := UIControls.card_panel("raised-flush")
	holder.custom_minimum_size = Vector2(width, height)
	cell.add_child(holder)

	var face := CardFaceBase.create_face(card_id, false)
	face.set_meta("golden", golden)
	face.custom_minimum_size = Vector2(width, height)
	holder.add_child(face)
	# 潜行不占顶栏：封面压一层蒙版，信息区与顶栏都不受影响。
	if stealth:
		CardFaceBase.add_stealth_veil(face)

	CardFaceBase.add_info(
		face,
		card_id,
		CardText.stat_text(entry.get("attack")),
		CardText.stat_text(entry.get("health")),
		CardText.playstyle_text(entry),
		false,
		false,
		null,
		true,
		false,
		golden,
	)

	CardFaceBase.add_frame(face)
	CardFaceBase.add_corner_badges(face, cost, era, reward)

	if frozen:
		CardFaceBase.add_freeze(face)

	var caption := UIControls.label(
		caption_text if caption_text != "" else STATES[state],
		UITheme.font_size("font-micro"),
		false,
	)
	caption.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	caption.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
	cell.add_child(caption)
	return cell
