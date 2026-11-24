class_name FrameMaterial
extends RefCounted
## 框体材质（方向 A：黄铜浮雕）：给任意 Control 叠一层由 shader 绘制的边框与内侧阴影。
##
## 只改框体一层，不参与布局、不拦截鼠标；中间完全透明，卡面与文字照常显示。
## 三种变体共用同一套光照与倒角：
## - 时代：`tint` 取 `ERA_COLOR` 的令牌，每个时代换一种合金基色；
## - 三连：`golden` 打开足金、内圈金线与更强外发光；
## - 引导：`guide` 才走流光，只有需要玩家操作的元素开启，其余保持静态。

enum Style {
	BEVEL = 0,
	ETCH = 1,
}

const OVERLAY_NAME := "FrameMaterial"
const SHADER_PATH := "res://ui/frame_material.gdshader"
## 线宽按短边比例取，夹在区间内：小按钮不掉到 1px，大面板不粗成色块。
const BAND_RATIO := 0.022
const BAND_MIN := 1.6
const BAND_MAX := 4.2
## 三连金框比同尺寸普通框再厚一档，保证隔一张卡也能认出来。
const GOLDEN_BAND_BONUS := 0.8

## 时代合金色：同一条黄铜骨架，按时代换金属基色，不换形状与光照。
const ERA_COLOR := {
	"prehistoric": "color-era-prehistoric",
	"primitive": "color-era-primitive",
	"classical": "color-era-classical",
	"feudal": "color-era-feudal",
	"industrial": "color-era-industrial",
	"modern": "color-era-modern",
	"future": "color-era-future",
}

static var _shader: Shader = null


static func shader() -> Shader:
	if _shader == null and ResourceLoader.exists(SHADER_PATH):
		_shader = ResourceLoader.load(SHADER_PATH) as Shader
	return _shader


static func default_tint() -> Color:
	return UITheme.color("color-accent")


static func tint_for_era(era: String) -> Color:
	return UITheme.color(str(ERA_COLOR.get(era, "color-accent")))


static func golden_tint() -> Color:
	return UITheme.color("color-golden")


## 给单个控件叠材质；重复调用只更新参数，不叠加多份。
## `opts` 支持 `tint: Color`、`golden: bool`、`guide: bool`。
static func apply(target: Control, style: int, opts: Dictionary = {}) -> ColorRect:
	if target == null or shader() == null:
		return null
	var overlay := target.get_node_or_null(OVERLAY_NAME) as ColorRect
	if overlay == null:
		var material := ShaderMaterial.new()
		material.shader = shader()
		overlay = ColorRect.new()
		overlay.name = OVERLAY_NAME
		overlay.color = UITheme.color("color-white")
		overlay.material = material
		overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
		overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
		target.add_child(overlay)
		overlay.ready.connect(func() -> void: _sync(overlay, target), CONNECT_ONE_SHOT)
		target.resized.connect(func() -> void: _sync(overlay, target))
		target.mouse_entered.connect(func() -> void: _hover(overlay, 1.0))
		target.mouse_exited.connect(func() -> void: _hover(overlay, 0.0))
		# 材质是叠在内容之上的装饰：始终保持在最后一个子节点，内容仍从索引 0 开始。
		target.child_entered_tree.connect(func(_child: Node) -> void: _raise(overlay))
	overlay.set_meta("tint", opts.get("tint", default_tint()))
	overlay.set_meta("golden", bool(opts.get("golden", false)))
	overlay.set_meta("guide", bool(opts.get("guide", false)))
	var material := overlay.material as ShaderMaterial
	if material == null:
		return overlay
	material.set_shader_parameter("style", style)
	_sync(overlay, target)
	return overlay


## 切换引导流光：只为需要玩家操作的元素打开。`root` 自身与全部后代一起处理，
## 卡片的流光挂在卡面（材质所在节点）上，外层卡容器不需要逐层找。
static func set_guide(target: Control, guide: bool) -> void:
	var value := 1.0 if guide else 0.0
	for node in _overlays(target):
		node.set_meta("guide", guide)
		var material := node.material as ShaderMaterial
		if material != null:
			material.set_shader_parameter("guide", value)


## 新内容加入后把材质挪回最后一个子节点：装饰层永远压在内容上面，
## 容器内容的索引与顺序保持不变。
static func _raise(overlay: ColorRect) -> void:
	if not is_instance_valid(overlay):
		return
	var parent := overlay.get_parent()
	if parent == null or overlay.get_index() == parent.get_child_count() - 1:
		return
	parent.move_child.call_deferred(overlay, parent.get_child_count() - 1)


## 目标自身与其子树里的全部材质节点；清空内容后重挂、切拖拽态都走同一份清单。
static func _overlays(root: Node) -> Array[ColorRect]:
	var found: Array[ColorRect] = []
	var own := root.get_node_or_null(OVERLAY_NAME) as ColorRect
	if own != null:
		found.append(own)
	for node in root.find_children(OVERLAY_NAME, "ColorRect", true, false):
		found.append(node as ColorRect)
	return found


static func clear(target: Control) -> void:
	if target == null:
		return
	var overlay := target.get_node_or_null(OVERLAY_NAME)
	if overlay != null:
		overlay.queue_free()


static func clear_tree(root: Node) -> void:
	for node in root.find_children("*", "ColorRect", true, false):
		if node.name == OVERLAY_NAME:
			node.get_parent().remove_child(node)
			node.queue_free()


## 整棵子树统一接入：只给最外层承担表面的控件加材质，嵌套面板不重复描边。
## 卡牌表面按卡面 `card_id` 取时代色与三连标记；按钮与可拖拽卡自动开启引导流光。
static func apply_to_tree(root: Node, style: int, opts: Dictionary = {}) -> int:
	var targets: Array[Control] = []
	_collect(root, targets)
	for target in targets:
		apply(target, style, _options_for(target, opts))
	return targets.size()


static func _options_for(target: Control, base: Dictionary) -> Dictionary:
	var opts := base.duplicate()
	var face := _card_face(target)
	if face != null:
		var era := str(CardDB.card(str(face.get_meta("card_id", ""))).get("era", ""))
		opts["tint"] = tint_for_era(era)
		opts["golden"] = bool(face.get_meta("golden", false))
	else:
		opts["tint"] = opts.get("tint", default_tint())
	if not opts.has("guide"):
		opts["guide"] = _wants_guide(target)
	return opts


## 引导流光只给「现在就该操作」的表面：按钮，以及当前允许拖拽的卡。
static func _wants_guide(control: Control) -> bool:
	if control is Button:
		return true
	if "drag_allowed" in control:
		return bool(control.get("drag_allowed"))
	return false


static func _card_face(control: Control) -> Control:
	for node in control.find_children("*", "Control", true, false):
		if node.has_meta("card_id"):
			return node as Control
	return null


static func _collect(node: Node, out: Array[Control]) -> void:
	for child in node.get_children():
		if child is Control:
			var control := child as Control
			# 卡面自带材质：整棵卡容器（含卡内信息条）不再重复描边。
			if _card_face(control) != null:
				continue
			if _is_surface(control) and not _nested_in(control, out):
				out.append(control)
		_collect(child, out)


## 页面级接入：等首帧布局落定后整页套一遍，页面里的静态面板与按钮一次性接入；
## 之后新建的卡与按钮由各自组件自挂，不需要页面再扫。
static func dress_page(page: Control, style: int = Style.BEVEL) -> void:
	if page == null or not page.is_inside_tree():
		return
	var tree := page.get_tree()
	await tree.process_frame
	await tree.process_frame
	if not is_instance_valid(page):
		return
	apply_to_tree(page, style)


## 承担表面的控件：按钮、面板与面板容器；装饰性容器（无底色、无描边）不参与。
static func _is_surface(control: Control) -> bool:
	if not (control is Button or control is PanelContainer or control is Panel):
		return false
	# 整屏底图不是「表面」：加边框会把画面圈成一整块。
	if str(control.name) in [UIControls.PAGE_SURFACE_NAME, "PageBackground"]:
		return false
	if not control.is_visible_in_tree():
		return false
	if control.size.x < 18.0 or control.size.y < 14.0:
		return false
	var box := _style_box(control)
	if box == null:
		return false
	return box.bg_color.a > 0.05 or _has_border(box)


static func _nested_in(control: Control, applied: Array[Control]) -> bool:
	for parent in applied:
		if parent.is_ancestor_of(control):
			return true
	return false


static func _style_box(control: Control) -> StyleBoxFlat:
	var names := ["panel", "normal"]
	if control is Button:
		names = ["normal", "panel"]
	for name in names:
		var box := control.get_theme_stylebox(name)
		if box is StyleBoxFlat:
			return box
	return null


static func _has_border(box: StyleBoxFlat) -> bool:
	return (
		box.border_width_left + box.border_width_top
		+ box.border_width_right + box.border_width_bottom
	) > 0


static func _sync(overlay: ColorRect, target: Control) -> void:
	# 面板重排时可能只剩信号：节点已释放就直接跳过，不让旧的连接报错。
	if not is_instance_valid(overlay) or not is_instance_valid(target):
		return
	var material := overlay.material as ShaderMaterial
	if material == null:
		return
	var size := target.size
	var band := clampf(minf(size.x, size.y) * BAND_RATIO, BAND_MIN, BAND_MAX)
	if bool(overlay.get_meta("golden", false)):
		band += GOLDEN_BAND_BONUS
	material.set_shader_parameter("box_size", size)
	material.set_shader_parameter("band", band)
	material.set_shader_parameter("corner", _corner(target))
	var tint: Color = overlay.get_meta("tint", default_tint())
	material.set_shader_parameter("tint", Vector3(tint.r, tint.g, tint.b))
	var gold := golden_tint()
	material.set_shader_parameter("golden_tint", Vector3(gold.r, gold.g, gold.b))
	material.set_shader_parameter("golden", 1.0 if bool(overlay.get_meta("golden", false)) else 0.0)
	material.set_shader_parameter("guide", 1.0 if bool(overlay.get_meta("guide", false)) else 0.0)


static func _corner(target: Control) -> float:
	var box := _style_box(target)
	if box == null:
		return 8.0
	return float(maxi(box.corner_radius_top_left, box.corner_radius_bottom_right))


static func _hover(overlay: ColorRect, value: float) -> void:
	if not is_instance_valid(overlay):
		return
	var material := overlay.material as ShaderMaterial
	if material != null:
		material.set_shader_parameter("hover", value)
