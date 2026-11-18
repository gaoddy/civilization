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
}

const PRIMITIVE_FONT := {
	"11": 11,
	"12": 12,
	"14": 14,
	"16": 16,
	"18": 18,
	"22": 22,
	"28": 28,
}

const PRIMITIVE_SPACE := {
	"2": 2,
	"4": 4,
	"6": 6,
	"8": 8,
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
}

const SEMANTIC_FONT := {
	"font-caption": "11",
	"font-body-sm": "12",
	"font-body": "14",
	"font-title": "16",
	"font-title-lg": "18",
	"font-display": "22",
	"font-hero": "28",
}

const SEMANTIC_SPACE := {
	"space-2xs": "2",
	"space-xs": "4",
	"space-sm": "6",
	"space-md": "8",
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
}

# ── 组件与场景层 ──────────────────────────────────────────────────────────

const SCENARIO := {
	"rail-width": 88,
	"side-width": 272,
	"avatar-sm": 22,
	"avatar-md": 30,
	"avatar-lg": 38,
	"hero-tile-width": 150,
	"portrait-size": 200,
	"portrait-height": 150,
	"card-width": 112,
	"card-height": 146,
	"combat-banner-height": 30,
	"card-info-height": 90,
	"card-info-compact-height": 56,
	"hand-card-overlap": 32,
	"hand-hover-scale": 1.12,
	"detail-panel-width": 200,
	"gallery-toc-width": 168,
	"gallery-detail-width": 280,
	"gallery-page-size": 40,
	"gallery-grid-columns": 5,
	"report-width": 236,
	"avatar-rail": 44,
	"seat-hp-bar-height": 6,
	"rank-badge-size": 18,
	"freeze-crystal-gap": 14,
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
		"raised":
			box.bg_color = color("color-surface-raised")
		"raised-tight":
			box.bg_color = color("color-surface-raised")
			box.set_content_margin_all(space("space-md"))
		"empty":
			box.bg_color = alpha("color-surface", 0.4)
			box.border_color = alpha("color-border", 0.7)
			box.set_content_margin_all(space("space-md"))
		"sunken":
			box.bg_color = color("color-surface-sunken")
		"accent":
			box.bg_color = alpha("color-accent", 0.12)
			box.border_color = color("color-accent")
		"accent-row":
			box.set_content_margin_all(space("space-md"))
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
