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
## 引导流光偏好：按钮默认可操作表面都带流光，页面可在创建后给次要按钮写 `false`。
## 页面级 `dress_page` 还会整页再扫一遍，偏好写进 meta 才能在重扫后继续生效。
const GUIDE_META := "frame_guide"
## 材质层挂在 Node2D 下：容器只排列 Control 子节点，这样 PanelContainer 的
## 内容内边距不会把材质层挤进内容区；Node2D 仍是 CanvasItem，祖先缩放照常生效。
const HOLDER_NAME := "FrameMaterialHolder"
## 数字底板层：铺在内容之下，给段位、倒计时与数字这类没有整块底色的信息表面。
const PLATE_HOLDER_NAME := "FramePlateHolder"
const PLATE_OVERLAY_NAME := "FramePlate"
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
## `opts` 支持 `tint: Color`、`golden: bool`、`guide: bool`、`surface: bool`。
## `surface` 打开时额外铺一层数字底板：底板画在内容之下，边框仍压在内容之上。
static func apply(target: Control, style: int, opts: Dictionary = {}) -> ColorRect:
	if target == null or shader() == null:
		return null
	var holder := target.get_node_or_null(HOLDER_NAME) as Node
	var overlay := holder.get_node_or_null(OVERLAY_NAME) as ColorRect if holder != null else null
	if overlay == null:
		holder = Node2D.new()
		holder.name = HOLDER_NAME
		target.add_child(holder)
		var material := ShaderMaterial.new()
		material.shader = shader()
		overlay = ColorRect.new()
		overlay.name = OVERLAY_NAME
		overlay.color = UITheme.color("color-white")
		overlay.material = material
		overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
		overlay.position = Vector2.ZERO
		overlay.size = target.size
		holder.add_child(overlay)
		overlay.ready.connect(func() -> void: _sync(overlay, target), CONNECT_ONE_SHOT)
		target.resized.connect(func() -> void: _sync(overlay, target))
		target.mouse_entered.connect(func() -> void: _hover(overlay, 1.0))
		target.mouse_exited.connect(func() -> void: _hover(overlay, 0.0))
		# 材质是叠在内容之上的装饰：始终保持在最后一个子节点，内容仍从索引 0 开始。
		target.child_entered_tree.connect(func(_child: Node) -> void: _raise(holder))
	overlay.set_meta("tint", opts.get("tint", default_tint()))
	overlay.set_meta("golden", bool(opts.get("golden", false)))
	overlay.set_meta("guide", bool(opts.get("guide", false)))
	overlay.set_meta("surface", false)
	var material := overlay.material as ShaderMaterial
	if material == null:
		return overlay
	material.set_shader_parameter("style", style)
	_sync(overlay, target)
	if bool(opts.get("surface", false)):
		_ensure_plate(target, style, opts)
	return overlay


## 数字底板：铺在内容之下的一整块表面。
## `Label` 的文字由控件自身绘制，子节点默认压在文字上，底板用 `show_behind_parent`
## 落到文字后面；`PanelContainer` 的底色先于子节点绘制，底板插到第一个子节点即可。
static func _ensure_plate(target: Control, style: int, opts: Dictionary) -> void:
	var holder := target.get_node_or_null(PLATE_HOLDER_NAME) as Node2D
	var plate := holder.get_node_or_null(PLATE_OVERLAY_NAME) as ColorRect if holder != null else null
	if plate == null:
		holder = Node2D.new()
		holder.name = PLATE_HOLDER_NAME
		var material := ShaderMaterial.new()
		material.shader = shader()
		plate = ColorRect.new()
		plate.name = PLATE_OVERLAY_NAME
		plate.color = UITheme.color("color-white")
		plate.material = material
		plate.mouse_filter = Control.MOUSE_FILTER_IGNORE
		plate.position = Vector2.ZERO
		plate.size = target.size
		holder.add_child(plate)
		if target is Label:
			holder.show_behind_parent = true
			target.add_child(holder)
		else:
			target.add_child(holder)
			target.move_child(holder, 0)
		plate.ready.connect(func() -> void: _sync(plate, target), CONNECT_ONE_SHOT)
		target.resized.connect(func() -> void: _sync(plate, target))
	plate.set_meta("surface", true)
	plate.set_meta("tint", opts.get("tint", default_tint()))
	plate.set_meta("golden", bool(opts.get("golden", false)))
	plate.set_meta("guide", false)
	var material := plate.material as ShaderMaterial
	if material == null:
		return
	material.set_shader_parameter("style", style)
	_sync(plate, target)


## 切换引导流光：只为需要玩家操作的元素打开。`root` 自身与全部后代一起处理，
## 卡片的流光挂在卡面（材质所在节点）上，外层卡容器不需要逐层找。
static func set_guide(target: Control, guide: bool) -> void:
	var value := 1.0 if guide else 0.0
	for node in _overlays(target):
		node.set_meta("guide", guide)
		var material := node.material as ShaderMaterial
		if material != null:
			material.set_shader_parameter("guide", value)


## 设置控件是否跟随引导流光：同时写入 meta，页面级 `dress_page` 重扫时按偏好生效。
static func set_guide_preference(target: Control, guide: bool) -> void:
	if target == null:
		return
	target.set_meta(GUIDE_META, guide)
	set_guide(target, guide)


## 新内容加入后把材质挪回最后一个子节点：装饰层永远压在内容上面，
## 容器内容的索引与顺序保持不变。
static func _raise(holder: Node) -> void:
	if not is_instance_valid(holder):
		return
	var parent := holder.get_parent()
	if parent == null or holder.get_index() == parent.get_child_count() - 1:
		return
	parent.move_child.call_deferred(holder, parent.get_child_count() - 1)


## 目标自身与其子树里的全部材质节点；清空内容后重挂、切拖拽态都走同一份清单。
static func _overlays(root: Node) -> Array[ColorRect]:
	var found: Array[ColorRect] = []
	var holder := root.get_node_or_null(HOLDER_NAME) as Node
	var own := holder.get_node_or_null(OVERLAY_NAME) as ColorRect if holder != null else null
	if own != null:
		found.append(own)
	for node in root.find_children(OVERLAY_NAME, "ColorRect", true, false):
		found.append(node as ColorRect)
	return found


static func clear(target: Control) -> void:
	if target == null:
		return
	for holder_name in [HOLDER_NAME, PLATE_HOLDER_NAME]:
		var holder := target.get_node_or_null(holder_name)
		if holder != null:
			target.remove_child(holder)
			holder.queue_free()


static func clear_tree(root: Node) -> void:
	for node in root.find_children("*", "ColorRect", true, false):
		if node.name == OVERLAY_NAME or node.name == PLATE_OVERLAY_NAME:
			var holder := node.get_parent()
			var victim: Node = (
				holder
				if holder != null and str(holder.name) in [HOLDER_NAME, PLATE_HOLDER_NAME]
				else node
			)
			var parent := victim.get_parent()
			if parent != null:
				parent.remove_child(victim)
			victim.queue_free()


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


## 引导流光只给「现在就该操作」的表面：按钮，以及当前允许拖拽的卡；
## 页面声明过偏好的控件按声明走。
static func _wants_guide(control: Control) -> bool:
	if control.has_meta(GUIDE_META):
		return bool(control.get_meta(GUIDE_META))
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
## 隐藏的表面同样接入：排队卡、座位列表等按状态才出现的区块在页面装载时不可见，
## 只在可见时扫描会让它们显示后一直没有边框。
static func _is_surface(control: Control) -> bool:
	if not (control is Button or control is PanelContainer or control is Panel):
		return false
	# 整屏底图不是「表面」：加边框会把画面圈成一整块。
	if str(control.name) in [UIControls.PAGE_SURFACE_NAME, "PageBackground"]:
		return false
	# 未布局的隐藏控件尺寸为 0，尺寸门槛只对已显示的控件生效；显示后由 resized 补齐矩形。
	if control.is_visible_in_tree() and (control.size.x < 18.0 or control.size.y < 14.0):
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
	overlay.position = Vector2.ZERO
	overlay.size = target.size
	var material := overlay.material as ShaderMaterial
	if material == null:
		return
	var size := target.size
	var band := clampf(minf(size.x, size.y) * BAND_RATIO, BAND_MIN, BAND_MAX)
	if bool(overlay.get_meta("golden", false)):
		band += GOLDEN_BAND_BONUS
	material.set_shader_parameter("box_size", size)
	material.set_shader_parameter("band", band)
	# 圆角不能超过短边的一半：胶囊与小角标按自身圆角绘制，避免倒角在矩形内溢出。
	material.set_shader_parameter("corner", minf(_corner(target), minf(size.x, size.y) * 0.5))
	material.set_shader_parameter("surface", 1.0 if bool(overlay.get_meta("surface", false)) else 0.0)
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
