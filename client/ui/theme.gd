class_name UITheme
extends RefCounted
## 视觉令牌：原子 → 语义 → 组件与场景三层，页面与组件只使用后两层。
##
## 原子层只放原始取值与固定刻度，语义层按用途引用原子层，组件与场景层引用语义层。
## 引用只能自上而下：页面不直接写色值、字号、间距与圆角。

# ── 原子层 ────────────────────────────────────────────────────────────────

const PRIMITIVE_COLOR := {
	"ink-950": "070a0f",
	"ink-900": "0c1118",
	"ink-800": "121925",
	"ink-700": "1a2331",
	"ink-600": "243044",
	"ink-500": "33435c",
	"snow-100": "e9eef7",
	"snow-300": "b7c3d6",
	"snow-500": "8494ac",
	"gold-400": "f0c66b",
	"gold-600": "b98a2f",
	"green-400": "5fce86",
	"red-400": "e2666b",
	"blue-300": "8fd3ff",
	"blue-400": "62a8e8",
	"cyan-400": "3fd0e0",
	"teal-400": "3fc7ac",
	"olive-400": "9aa83c",
	"amber-400": "e0b04a",
	"orange-400": "e57c4a",
	"magenta-400": "d979c4",
	"purple-400": "b28ce6",
	"slate-400": "93a6bd",
	"lime-400": "a3cf61",
	"rose-400": "d9799b",
	"sand-400": "cdb98c",
	"red-200": "ff8c8c",
	"white": "ffffff",
	"black": "000000",
}

const PRIMITIVE_FONT := {
	"8": 8,
	"11": 11,
	"12": 12,
	"14": 14,
	"16": 16,
	"18": 18,
	"20": 20,
	"22": 22,
	"28": 28,
	"30": 30,
	"72": 72,
}

const PRIMITIVE_SPACE := {
	"0": 0,
	"2": 2,
	"4": 4,
	"6": 6,
	"8": 8,
	"10": 10,
	"12": 12,
	"16": 16,
	"24": 24,
	"32": 32,
}

const PRIMITIVE_RADIUS := {
	"4": 4,
	"8": 8,
	"12": 12,
	"16": 16,
}

# ── 语义层 ────────────────────────────────────────────────────────────────

const SEMANTIC_COLOR := {
	"color-bg": "ink-950",
	"color-surface": "ink-900",
	"color-surface-raised": "ink-800",
	"color-surface-sunken": "ink-700",
	"color-border": "ink-600",
	"color-border-strong": "ink-500",
	"color-text": "snow-100",
	"color-text-dim": "snow-300",
	"color-text-muted": "snow-500",
	"color-accent": "gold-400",
	"color-accent-strong": "gold-600",
	"color-success": "green-400",
	"color-danger": "red-400",
	"color-info": "blue-400",
	"color-highlight": "blue-300",
	"color-combat-tint": "red-200",
	"color-white": "white",
	"color-black": "black",
}

const SEMANTIC_FONT := {
	"font-micro": "8",
	"font-caption": "11",
	"font-body-sm": "12",
	"font-body": "14",
	"font-title": "16",
	"font-title-lg": "18",
	"font-title-xl": "20",
	"font-display": "22",
	"font-hero": "28",
	"font-display-lg": "30",
	"font-watermark": "72",
}

const SEMANTIC_SPACE := {
	"space-none": "0",
	"space-2xs": "2",
	"space-xs": "4",
	"space-sm": "6",
	"space-md": "8",
	"space-md-plus": "10",
	"space-lg": "12",
	"space-xl": "16",
	"space-2xl": "24",
	"space-3xl": "32",
}

const SEMANTIC_RADIUS := {
	"radius-sm": "4",
	"radius-md": "8",
	"radius-lg": "12",
	"radius-pill": "16",
}

const SEMANTIC_OPACITY := {
	"opacity-dim": 0.45,
	"freeze-overlay-alpha": 0.22,
	"preview-dim": 0.7,
	"scrim-alpha": 0.55,
	"scrim-alpha-friends": 0.58,
	"scrim-alpha-strong": 0.6,
	"scrim-alpha-gallery": 0.62,
	"scrim-alpha-choice": 0.72,
}

# ── 组件与场景层 ──────────────────────────────────────────────────────────

const SCENARIO := {
	"rail-width": 88,
	"pill-button-height": 26,
	"status-dialog-width": 360,
	"river-height": 44,
	"river-gold-width": 72,
	"river-gold-height": 24,
	"hero-row-height": 84,
	"hero-meta-row-height": 24,
	"hero-card-size": 84,
	"shop-icon-size": 40,
	"recruit-shop-gap": 6,
	"opponent-hp-width": 84,
	"avatar-sm": 22,
	"avatar-md": 30,
	"avatar-lg": 38,
	"portrait-size": 64,
	"hero-detail-width": 160,
	"hero-detail-width-compact": 120,
	"room-hero-detail-width": 280,
	"room-hero-detail-width-compact": 200,
	"card-width": 112,
	"card-height": 146,
	"card-cover-inset": 3,
	"card-info-height": 90,
	"card-info-compact-height": 56,
	# 随从卡（棋盘与商店）尺寸：手机端比手牌大一档，英雄卡不放大。
	"minion-card-width": 112,
	"minion-card-height": 146,
	"minion-card-width-compact": 78,
	"minion-card-height-compact": 102,
	"hand-card-overlap": 32,
	"hand-hover-scale": 1.12,
	"detail-panel-width": 200,
	# 棋盘随从详情：默认 280；来源超过 8 行时桌面加宽到 360，手机两档同宽保持单列。
	"detail-minion-width": 280,
	"detail-minion-width-compact": 200,
	"detail-minion-width-dense": 360,
	"detail-minion-width-dense-compact": 200,
	# 图鉴：详情改为浮层抽屉，网格占满主体宽度；筛选面板挂在触发器下沿。
	"gallery-drawer-width": 420,
	"gallery-filter-panel-width": 240,
	"gallery-preview-width": 200,
	"gallery-batch-size": 27,
	"gallery-grid-columns": 9,
	"gallery-preview-width-compact": 128,
	"gallery-grid-columns-compact": 6,
	"gallery-batch-size-compact": 12,
	# 座位悬停详情比通用详情宽一档，容纳一行战报文案。
	"seat-detail-width": 236,
	# 详情里的生命条要容纳居中的当前/上限数字，比座位行上的血条高一档。
	"seat-detail-hp-bar-height": 16,
	"avatar-rail": 44,
	"seat-row-gap": 4,
	"seat-hp-bar-height": 6,
	"rank-badge-size": 18,
	"rank-badge-size-compact": 14,
	"freeze-crystal-gap": 14,
	# iPhone 基准（960×540）的对局舞台紧凑值。
	"card-width-compact": 70,
	"card-height-compact": 91,
	"card-info-height-compact": 60,
	"hand-card-overlap-compact": 24,
	"hero-row-height-compact": 64,
	"hero-card-size-compact": 64,
	"shop-icon-size-compact": 36,
	"river-height-compact": 30,
	"river-gold-width-compact": 64,
	"river-gold-height-compact": 20,
	"recruit-shop-gap-compact": 4,
	"opponent-hp-width-compact": 64,
	"pill-button-height-compact": 24,
	"status-dialog-width-compact": 280,
	"portrait-size-compact": 48,
	# 左栏 8 席需在 960×540 下一屏放下，座位行按紧凑值收缩。
	"avatar-rail-compact": 40,
	"seat-hp-bar-height-compact": 4,
	"seat-row-gap-compact": 2,
	# 技能瞄准：弧线、流动光点、候选角标与命中描边。小数令牌经 `factor` 读取。
	"aim-arc-width": 2.5,
	"aim-dash": 10.0,
	"aim-gap": 6.0,
	"aim-dot-radius": 2.5,
	"aim-flow-speed": 0.55,
	"aim-source-radius": 12.0,
	"aim-mark-length": 8.0,
	"aim-mark-inset": 3.0,
	"aim-outline-width": 2.0,
	"aim-pulse": 1.2,
	"aim-cross": 6.0,
	"era-glyph-size": 14,
}

const PLAYSTYLE_COLOR := {
	"mammal": "blue-400",
	"bird": "cyan-400",
	"reptile": "green-400",
	"fish": "teal-400",
	"insect": "lime-400",
	"cephalopod": "purple-400",
	"cnidarian": "rose-400",
	"plant": "olive-400",
	"fungus": "amber-400",
	"bacteria": "orange-400",
	"protozoa": "magenta-400",
	"virus": "red-400",
	"machine": "slate-400",
	"neutral": "sand-400",
}

# ── 取值入口 ──────────────────────────────────────────────────────────────

static func color(name: String) -> Color:
	if name == "color-transparent":
		return Color(0, 0, 0, 0)
	var primitive := str(SEMANTIC_COLOR.get(name, name))
	return Color(str(PRIMITIVE_COLOR.get(primitive, "ffffff")))


static func alpha(name: String, value: float) -> Color:
	var value_color := color(name)
	value_color.a = value
	return value_color


static func font_size(name: String) -> int:
	return int(PRIMITIVE_FONT.get(str(SEMANTIC_FONT.get(name, name)), 14))


static func space(name: String) -> int:
	return int(PRIMITIVE_SPACE.get(str(SEMANTIC_SPACE.get(name, name)), 8))


static func radius(name: String) -> int:
	return int(PRIMITIVE_RADIUS.get(str(SEMANTIC_RADIUS.get(name, name)), 8))


static func metric(name: String) -> int:
	return int(SCENARIO.get(name, 0))


## 对局舞台令牌：iPhone 基准取 `<name>-compact`，其余设备与桌面取原值。
static func stage_metric(name: String) -> int:
	if ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE:
		var compact := name + "-compact"
		if SCENARIO.has(compact):
			return int(SCENARIO[compact])
	return metric(name)


## 卡面展示框高度：按 base 卡面令牌的 112:146 由宽度等比计算并取整。
## 比例只取 `card-width` / `card-height`，不读 compact 值，避免手机端紧凑尺寸改变比例。
static func card_face_height(width: int) -> int:
	return roundi(float(width) * float(metric("card-height")) / float(metric("card-width")))


## 触控端主操作命中高度：44pt 换算为画布单位；桌面端保持 44。
static func hit_primary() -> int:
	return ScreenProfile.hit_min(44)


## 触控端次要操作命中高度：32pt 换算为画布单位；桌面端保持 32。
static func hit_small() -> int:
	return ScreenProfile.hit_min(32)


## 小数场景令牌（如缩放倍数）；整数令牌请用 [`UITheme.metric`]。
static func factor(name: String, fallback: float = 1.0) -> float:
	return float(SCENARIO.get(name, fallback))


static func opacity(name: String) -> float:
	return float(SEMANTIC_OPACITY.get(name, 1.0))


static func playstyle_color(playstyle: String) -> Color:
	return color(str(PLAYSTYLE_COLOR.get(playstyle, "snow-500")))


## 卡面封面：玩法色压暗作底，文字与信息区始终压在它上面。
static func playstyle_cover(playstyle: String) -> Color:
	return playstyle_color(playstyle).darkened(0.62)

# ── StyleBox 工厂 ─────────────────────────────────────────────────────────

static func panel_style(variant: String) -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.set_corner_radius_all(radius("radius-md"))
	box.set_content_margin_all(space("space-lg"))
	box.bg_color = color("color-surface")
	box.border_color = color("color-border")
	box.set_border_width_all(1)
	match variant:
		"row":
			box.set_content_margin_all(space("space-md"))
		"row-compact":
			box.set_content_margin_all(space("space-2xs"))
		"raised":
			box.bg_color = color("color-surface-raised")
		"raised-tight":
			box.bg_color = color("color-surface-raised")
			box.set_content_margin_all(space("space-md"))
		"raised-flush":
			# 卡面容器：内容只让出 1px 描边，封面与美术铺满整张卡。
			box.bg_color = color("color-surface-raised")
			box.set_content_margin_all(1)
		"sunken":
			box.bg_color = color("color-surface-sunken")
		"accent":
			box.bg_color = alpha("color-accent", 0.12)
			box.border_color = color("color-accent")
		"accent-row":
			box.set_content_margin_all(space("space-md"))
			box.bg_color = alpha("color-accent", 0.12)
			box.border_color = color("color-accent")
		"accent-row-compact":
			box.set_content_margin_all(space("space-2xs"))
			box.bg_color = alpha("color-accent", 0.12)
			box.border_color = color("color-accent")
		"success":
			box.bg_color = alpha("color-success", 0.12)
			box.border_color = color("color-success")
		"danger":
			box.bg_color = alpha("color-danger", 0.12)
			box.border_color = color("color-danger")
		"transparent":
			box.bg_color = Color(0, 0, 0, 0)
			box.border_color = Color(0, 0, 0, 0)
		"bare":
			box.set_content_margin_all(0)
			box.bg_color = Color(0, 0, 0, 0)
			box.border_color = Color(0, 0, 0, 0)
		"accent-bare":
			box.set_content_margin_all(0)
			box.bg_color = alpha("color-accent", 0.12)
			box.border_color = color("color-accent")
			box.set_border_width_all(2)
		"cover-info":
			box.set_corner_radius_all(0)
			box.corner_radius_bottom_left = radius("radius-md")
			box.corner_radius_bottom_right = radius("radius-md")
			box.set_content_margin_all(space("space-sm"))
			box.bg_color = alpha("color-surface-sunken", 0.92)
			box.border_color = Color(0, 0, 0, 0)
		_:
			pass
	return box


static func pill_style(variant: String) -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.set_corner_radius_all(radius("radius-pill"))
	box.content_margin_left = space("space-lg")
	box.content_margin_right = space("space-lg")
	box.content_margin_top = space("space-2xs")
	box.content_margin_bottom = space("space-2xs")
	box.bg_color = color("color-surface-raised")
	box.border_color = color("color-border")
	box.set_border_width_all(1)
	match variant:
		"accent":
			box.bg_color = alpha("color-accent", 0.16)
			box.border_color = alpha("color-accent", 0.6)
		"success":
			box.bg_color = alpha("color-success", 0.16)
			box.border_color = alpha("color-success", 0.6)
		"danger":
			box.bg_color = alpha("color-danger", 0.16)
			box.border_color = alpha("color-danger", 0.6)
		"info":
			box.bg_color = alpha("color-info", 0.16)
			box.border_color = alpha("color-info", 0.6)
		"plain":
			box.bg_color = Color(0, 0, 0, 0)
			box.border_color = Color(0, 0, 0, 0)
		_:
			pass
	return box


static func pill_button_style(variant: String, state: String) -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.set_corner_radius_all(radius("radius-pill"))
	box.content_margin_left = space("space-lg")
	box.content_margin_right = space("space-lg")
	box.content_margin_top = space("space-2xs")
	box.content_margin_bottom = space("space-2xs")
	var base := "color-surface-raised"
	var border := "color-border"
	match variant:
		"primary":
			base = "color-accent-strong"
			border = "color-accent"
		"danger":
			base = "color-danger"
			border = "color-danger"
	box.bg_color = color(base)
	box.border_color = color(border)
	box.set_border_width_all(1)
	match state:
		"hover":
			box.bg_color = box.bg_color.lightened(0.12)
			if variant == "neutral":
				box.border_color = color("color-border-strong")
		"pressed":
			box.bg_color = box.bg_color.darkened(0.18)
		"focus":
			box.border_color = color("color-accent")
		"disabled":
			box.bg_color = color("color-surface")
			box.border_color = color("color-border")
	return box


static func button_style(variant: String, state: String) -> StyleBoxFlat:
	var box := StyleBoxFlat.new()
	box.set_corner_radius_all(radius("radius-sm"))
	box.content_margin_left = space("space-lg")
	box.content_margin_right = space("space-lg")
	box.content_margin_top = space("space-xs")
	box.content_margin_bottom = space("space-xs")
	var base := "color-surface-raised"
	var border := "color-border"
	match variant:
		"primary":
			base = "color-accent-strong"
			border = "color-accent"
		"danger":
			base = "color-danger"
			border = "color-danger"
		"ghost":
			base = "color-surface"
			border = "color-surface"
		"slot":
			base = "color-surface-raised"
			border = "color-border-strong"
		"micro":
			base = "color-surface"
			border = "color-surface"
		_:
			pass
	box.bg_color = color(base)
	box.border_color = color(border)
	box.set_border_width_all(1)
	if variant == "ghost" or variant == "micro":
		box.bg_color = Color(0, 0, 0, 0)
		box.border_color = Color(0, 0, 0, 0)
	if variant == "micro":
		box.content_margin_left = space("space-xs")
		box.content_margin_right = space("space-xs")
		box.content_margin_top = space("space-2xs")
		box.content_margin_bottom = space("space-2xs")
	match state:
		"hover":
			if variant == "ghost" or variant == "micro":
				box.bg_color = color("color-surface-raised")
			else:
				box.bg_color = box.bg_color.lightened(0.12)
			box.border_color = color("color-border-strong")
		"pressed":
			box.bg_color = box.bg_color.darkened(0.18)
		"focus":
			box.border_color = color("color-accent")
		"disabled":
			box.bg_color = color("color-surface")
			box.border_color = color("color-border")
		_:
			pass
	return box
