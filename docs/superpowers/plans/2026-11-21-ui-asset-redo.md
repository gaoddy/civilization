# UI 资产重做 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把卡面视觉改为代码绘制的双细线卡框、星级、时代象形与攻防底牌，删除重复的位图底材，只保留一张页面背景纹理。

**Architecture:** 卡框、星级、时代象形、攻防底牌全部用 Godot `Control._draw()` 绘制，随控件尺寸缩放并读取 `UITheme` 令牌；页面背景由纯标准库 Python 生成一张 512×512 纹理，经 `UI.background()` 整屏拉伸；与 `StyleBoxFlat` 重复的 48 张底材位图与稀有度徽章全部删除。

**Tech Stack:** Godot 4 + GDScript、Python 3 标准库（`zlib` + `struct`）、macOS 本地验证。

**Spec:** `docs/superpowers/specs/2026-11-21-ui-asset-redo-design.md`

## Global Constraints

- 客户端平台 Godot 4 + GDScript；不引入新的第三方依赖。
- 只有深色一套主题，不做浅色主题与主题切换。
- UI 代码不写死色值与尺寸：颜色走 `UITheme.color` / `UITheme.alpha` / `UITheme.opacity`，尺寸走 `UITheme.metric` / `stage_metric` / `space` / `radius` / `font_size`。
- 保持既有节点名：`CardAttack`、`CardHealth`、`CardInfo`、`CardName`、`CardType`、`CostBadge`。
- 开发阶段不为旧接口保留兼容分支；`add_corner_badges` 的 `era_text` 直接改名并同步全部调用点。
- 生成器只使用 Python 标准库，不导入 Pillow；固定种子，重跑字节级一致。
- 验证入口统一走 `make`：`make client-check`、`make client-smoke`、`make client-import`。
- 每个提交的说明末尾追加一行：`Co-Authored-By: Claude Code <noreply@anthropic.com>`。

## Review Focus

1. 效果牌与未知身材（文本为空或「—」）不能出现空的黄圆/血滴底牌——Task 3 的 `_draw()` 对空值与「—」直接返回，Task 5 Step 1 断言该文字仍保留，`ui_smoke.gd` 的「效果牌不显示身材」在 Task 5 Step 6 整轮运行中覆盖。
2. 手牌长介绍在加入攻防底牌后仍必须完整落在卡面内——既有 `_check_hand_fit()` 在 Task 5 Step 6 的 `make client-smoke` 中运行，底牌尺寸不超过原 Label 行高。
3. 商店/棋盘的角标位置断言必须随 `EraBadge` → `EraGlyph` 同步，不能留下假通过的空断言——Task 5 Step 1 显式断言 `EraGlyph` 与 `era_id`，Task 8 再全量跑一遍。
4. 背景纹理缺失或导入失败时页面必须回退纯色，不能变透明或变白——Task 6 的 `UI.background()` 在 `ResourceLoader.exists`/`load` 失败时返回 `ColorRect`；缺图环境无法在测试里构造，由 Task 6 代码审查确认两条分支。
5. 卡框在 112×146、84×92、200×260 三种尺寸下都不能越界或压住角标——Task 5 Step 1 断言卡框铺满卡面，Task 8 Step 3 在三种真实尺寸下截图目视确认。

---

### Task 1: 背景生成器重写与位图清理

**Files:**
- Modify: `scripts/gen_ui_assets.py`
- Create: `tests/test_ui_assets.py`
- Delete: `client/assets/cards/frames/`、`client/assets/badges/`、`client/assets/ui/button/`、`client/assets/ui/icon/`、`client/assets/ui/panel/`、`client/assets/ui/pill/`、`client/assets/ui/border/`、`client/assets/ui/bg/bg_panel.png`
- Delete: `DELIVERY_MANIFEST.md`

**Interfaces:**
- Produces: `render_background(size: int = 512) -> bytes`（返回 PNG 字节）、`main() -> int`；输出路径 `client/assets/ui/bg/bg_main.png`。

- [ ] **Step 1: 写失败测试**

创建 `tests/test_ui_assets.py`：

```python
import importlib.util
import struct
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
_spec = importlib.util.spec_from_file_location(
    "gen_ui_assets", ROOT / "scripts" / "gen_ui_assets.py"
)
gen = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(gen)


class BackgroundTextureTest(unittest.TestCase):
    def test_png_signature_and_size(self):
        data = gen.render_background(64)
        self.assertEqual(data[:8], b"\x89PNG\r\n\x1a\n")
        width, height = struct.unpack(">II", data[16:24])
        self.assertEqual((width, height), (64, 64))

    def test_deterministic(self):
        self.assertEqual(gen.render_background(64), gen.render_background(64))

    def test_no_pillow_dependency(self):
        source = (ROOT / "scripts" / "gen_ui_assets.py").read_text(encoding="utf-8")
        self.assertNotIn("PIL", source)
```

- [ ] **Step 2: 运行测试确认失败**

Run: `python3 -m unittest tests.test_ui_assets -v`
Expected: FAIL，导入时报 `ModuleNotFoundError: No module named 'PIL'` 或 `AttributeError: module ... has no attribute 'render_background'`。

- [ ] **Step 3: 重写生成器**

`scripts/gen_ui_assets.py` 只保留背景生成：

- `BG_SIZE = 512`
- `OUT = ROOT / "client" / "assets" / "ui" / "bg" / "bg_main.png"`
- `render_background(size: int = 512) -> bytes`：
  - 基色 `(7, 10, 15)`
  - 径向提亮：椭圆归一化坐标 `nx=(x/w-0.5)/0.8`、`ny=(y/h-0.4)/0.8`，`t=clamp(sqrt(nx²+ny²),0,1)`；`t=0` 为 `#2b3a52` alpha 0.88，`t=0.5` 为 `#18212f` alpha 0.44，`t=1` 透明，按 alpha 与基色混合
  - 纸张颗粒：确定性哈希噪声，幅度 ±2 个色阶（≤ 6% 不透明度）
  - 输出 RGB PNG：每行前置 filter byte `0`，`zlib.compress`，手写 IHDR/IDAT/IEND
- `main()` 写出 `OUT` 并打印路径。
- 模块 docstring 改为「页面背景生成器」，只描述 `bg_main.png` 一个产物。
- 删除 Pillow 导入、超采样、卡框/徽章/底材绘制、图集打包与 `PACKER` 常量。

- [ ] **Step 4: 运行测试确认通过**

Run: `python3 -m unittest tests.test_ui_assets -v`
Expected: PASS（3 个用例）。

- [ ] **Step 5: 生成并确认尺寸**

Run:
```bash
python3 scripts/gen_ui_assets.py
python3 -c "import struct;d=open('client/assets/ui/bg/bg_main.png','rb').read();print(struct.unpack('>II', d[16:24]))"
```
Expected: 输出 `(512, 512)`。

- [ ] **Step 6: 删除重复位图与交付清单**

```bash
git rm -r client/assets/cards/frames client/assets/badges client/assets/ui/button client/assets/ui/icon client/assets/ui/panel client/assets/ui/pill client/assets/ui/border
git rm client/assets/ui/bg/bg_panel.png DELIVERY_MANIFEST.md
make client-import
```

- [ ] **Step 7: 提交**

```bash
git add scripts/gen_ui_assets.py tests/test_ui_assets.py client/assets/ui/bg/bg_main.png
git commit -m "refactor(assets): 背景生成器改为纯标准库，删除重复底材位图"
```

---

### Task 2: 卡框与星级

**Files:**
- Create: `client/ui/card_frame.gd`
- Modify: `client/tests/ui_smoke_cards.gd`

**Interfaces:**
- Produces: `class_name CardFrame extends Control`，`static func create(level: int) -> CardFrame`，属性 `level: int`、`star_count: int`。

- [ ] **Step 1: 写失败测试**

在 `client/tests/ui_smoke_cards.gd` 的 `_ready()` 末尾、`_finish()` 之前调用 `await _check_card_frame()`，并新增：

```gdscript
func _check_card_frame() -> void:
    var frame := CardFrame.create(5)
    frame.name = "StandaloneFrame"
    add_child(frame)
    frame.size = Vector2(112, 146)
    await get_tree().process_frame
    _expect(frame.level == 5, "CardFrame 应记录星级")
    _expect(frame.star_count == 5, "CardFrame 星数应等于星级")
    _expect(frame.mouse_filter == Control.MOUSE_FILTER_IGNORE, "卡框不应拦截鼠标")
    frame.queue_free()
```

- [ ] **Step 2: 运行测试确认失败**

Run: `/Applications/Godot.app/Contents/MacOS/Godot --headless --path client res://tests/ui_smoke_cards.tscn`
Expected: 脚本错误，`Identifier "CardFrame" not declared`。

- [ ] **Step 3: 实现 `client/ui/card_frame.gd`**

```gdscript
class_name CardFrame
extends Control

const OUTER_INSET := 1.2
const MID_INSET := 2.4
const INNER_INSET := 4.6
const STAR_RADIUS := 2.15
const STAR_GAP := 5.0
const STAR_Y := 7.4

var level := 1
var star_count: int:
	get:
		return clampi(level, 0, 7)

static func create(level_value: int) -> CardFrame:
	var node := CardFrame.new()
	node.name = "CardFrame"
	node.level = level_value
	node.set_anchors_preset(Control.PRESET_FULL_RECT)
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	node.resized.connect(node.queue_redraw)
	return node

func _draw() -> void:
	var base := Vector2(UITheme.metric("card-width"), UITheme.metric("card-height"))
	var s := minf(size.x / base.x, size.y / base.y)
	_draw_rule(OUTER_INSET * s, 8.0 * s, UITheme.alpha("color-accent-strong", 0.60))
	_draw_rule(MID_INSET * s, 6.8 * s, UITheme.color("color-bg"))
	_draw_rule(INNER_INSET * s, 6.0 * s, UITheme.alpha("color-border-strong", 0.55))
	_draw_stars(s)

func _draw_rule(inset: float, radius: float, color: Color) -> void:
	var box := StyleBoxFlat.new()
	box.bg_color = Color(0, 0, 0, 0)
	box.draw_center = false
	box.border_color = color
	box.set_border_width_all(1)
	box.set_corner_radius_all(maxi(1, roundi(radius)))
	draw_style_box(box, Rect2(Vector2(inset, inset), size - Vector2(inset, inset) * 2.0))

func _draw_stars(s: float) -> void:
	var count := star_count
	if count <= 0:
		return
	var total := (count - 1) * STAR_GAP * s
	var center_y := STAR_Y * s
	for i in count:
		var center := Vector2(size.x * 0.5 - total * 0.5 + i * STAR_GAP * s, center_y)
		_draw_star(center, STAR_RADIUS * s, UITheme.alpha("color-accent", 0.95))

func _draw_star(center: Vector2, radius: float, color: Color) -> void:
	var points: Array[Vector2] = []
	for i in 8:
		var angle := -PI * 0.5 + i * PI * 0.25
		var r := radius if i % 2 == 0 else radius * 0.32
		points.append(center + Vector2(cos(angle), sin(angle)) * r)
	for i in 8:
		draw_colored_polygon(
			PackedVector2Array([center, points[i], points[(i + 1) % 8]]), color
		)
```

三条线由外到内：黄铜 `color-accent-strong`（60%）、分隔用的 `color-bg` 实线、`color-border-strong`（55%），与已确认预览的 `frameA()` 一致；不画四角花饰与中轴刻度，也不使用稀有度颜色。

- [ ] **Step 4: 运行测试确认通过**

Run: `/Applications/Godot.app/Contents/MacOS/Godot --headless --path client res://tests/ui_smoke_cards.tscn`
Expected: 输出 `UI_SMOKE_OK`，退出码 0。

- [ ] **Step 5: 提交**

```bash
git add client/ui/card_frame.gd client/tests/ui_smoke_cards.gd
git commit -m "feat(client): 代码绘制卡框与顶部星级"
```

---

### Task 3: 攻防底牌

**Files:**
- Create: `client/ui/stat_plate.gd`
- Modify: `client/tests/ui_smoke_cards.gd`

**Interfaces:**
- Produces: `class_name StatPlate extends Control`，`static func create(kind: String, text: String, font_size: int, label_name := "StatLabel") -> StatPlate`，属性 `kind: String`。

- [ ] **Step 1: 写失败测试**

在 `_ready()` 末尾调用 `await _check_stat_plate()`，新增：

```gdscript
func _check_stat_plate() -> void:
    for kind in ["attack", "health"]:
        var plate := StatPlate.create(kind, "4", 14, "Card" + kind.capitalize())
        add_child(plate)
        await get_tree().process_frame
        _expect(plate.kind == kind, "底牌应记录类型")
        _expect(plate.custom_minimum_size == Vector2(18, 18), "14 号字底牌应为 18×18")
        var label := plate.find_child("Card" + kind.capitalize(), true, false)
        _expect(label is Label and (label as Label).text == "4", "底牌应包含数字 Label")
        _expect(
            (label as Label).get_theme_constant("outline_size") >= 1,
            "数字应有描边",
        )
        plate.queue_free()
```

- [ ] **Step 2: 运行测试确认失败**

Run: `/Applications/Godot.app/Contents/MacOS/Godot --headless --path client res://tests/ui_smoke_cards.tscn`
Expected: `Identifier "StatPlate" not declared`。

- [ ] **Step 3: 实现 `client/ui/stat_plate.gd`**

```gdscript
class_name StatPlate
extends Control

const ATTACK := "attack"
const HEALTH := "health"

var kind := ATTACK

static func create(kind_value: String, text: String, font_size: int, label_name := "StatLabel") -> StatPlate:
	var node := StatPlate.new()
	node.name = "StatPlate"
	node.kind = kind_value
	var side := font_size + 4
	node.custom_minimum_size = Vector2(side, side)
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var label := UI.label(text, font_size, false)
	label.name = label_name
	label.set_anchors_preset(Control.PRESET_FULL_RECT)
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	label.add_theme_color_override("font_color", Color.WHITE)
	label.add_theme_color_override("font_outline_color", Color.BLACK)
	label.add_theme_constant_override("outline_size", 2)
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	node.add_child(label)
	node.resized.connect(node.queue_redraw)
	return node

func set_text(text: String) -> void:
	var label := get_child(0) as Label
	if label != null:
		label.text = text

func _draw() -> void:
	# 空值与「—」不画底牌，只留描边文字：效果牌、未知身材保持原排版。
	var label := get_child(0) as Label
	if label == null:
		return
	var value := label.text.strip_edges()
	if value == "" or value == "—":
		return
	if kind == HEALTH:
		_draw_drop()
	else:
		_draw_circle()
```

绘制细节：

- 圆形：先 `draw_circle(center, 0.48w, UITheme.color("color-bg"))` 作深色剪影，再 `draw_circle(center, 0.44w, UITheme.color("color-accent-strong"))`，最后 `draw_circle(center, 0.32w, UITheme.color("color-accent"))` 叠出明暗；外圈再 `draw_arc(center, 0.44w, 0, TAU, 48, UITheme.color("color-bg"), 1.0)` 收边。
- 血滴：`UITheme.color("color-danger").darkened(0.45)` 画略大一圈的剪影，再 `UITheme.color("color-danger")` 画圆底（圆心 `(0.5w, 0.62h)`，半径 `0.36w`）与三角尖（顶点 `(0.5w, 0.10h)`，两底角 `(0.14w, 0.62h)`、`(0.86w, 0.62h)`）。
- 尺寸全部取 `size` 的比例，不写死像素。

- [ ] **Step 4: 运行测试确认通过**

Run: `/Applications/Godot.app/Contents/MacOS/Godot --headless --path client res://tests/ui_smoke_cards.tscn`
Expected: `UI_SMOKE_OK`。

- [ ] **Step 5: 提交**

```bash
git add client/ui/stat_plate.gd client/tests/ui_smoke_cards.gd
git commit -m "feat(client): 攻击黄圆与血量血滴底牌"
```

---

### Task 4: 时代象形与图标控件

**Files:**
- Modify: `client/ui/icons.gd`
- Modify: `client/ui/theme.gd`（`SCENARIO` 增加 `"era-glyph-size": 14`、`"era-glyph-size-detail": 24`）
- Create: `client/ui/era_glyph.gd`
- Modify: `client/tests/ui_smoke_cards.gd`

**Interfaces:**
- Produces: `Icons.draw_era(canvas: CanvasItem, era_id: String, rect: Rect2, color: Color, width: float = 1.4) -> void`；`class_name EraGlyph extends Control`，`static func create(era_id: String, size: int) -> EraGlyph`，属性 `era_id: String`。

- [ ] **Step 1: 写失败测试**

在 `_ready()` 末尾调用 `await _check_era_glyph()`，新增：

```gdscript
func _check_era_glyph() -> void:
    for era in ["prehistoric", "primitive", "classical", "feudal", "industrial", "modern", "future"]:
        var glyph := EraGlyph.create(era, 14)
        add_child(glyph)
        await get_tree().process_frame
        _expect(glyph.era_id == era, "时代象形应记录 era id")
        _expect(glyph.custom_minimum_size == Vector2(14, 14), "卡面象形应为 14×14")
        _expect(glyph.mouse_filter == Control.MOUSE_FILTER_IGNORE, "象形不应拦截鼠标")
        glyph.queue_free()
```

- [ ] **Step 2: 运行测试确认失败**

Run: `/Applications/Godot.app/Contents/MacOS/Godot --headless --path client res://tests/ui_smoke_cards.tscn`
Expected: `Identifier "EraGlyph" not declared`。

- [ ] **Step 3: 实现时代象形**

在 `client/ui/icons.gd` 新增 `draw_era`。路径数据取自 `output/ui_final_preview.html` 的 `eraPaths()`（源坐标 16×16 网格），绘制时按 `s = min(rect.w, rect.h) / 16.0` 映射到实际矩形，描边宽度 `maxf(1.0, width * s)`；不复用 `Icons` 里 24 网格的 `_scale`，避免两台网格混算。三次贝塞尔用 8 段采样折线。

| era | 象形 |
| --- | --- |
| `prehistoric` | 木柄 + 燧石斧头 + 绑扎线 |
| `primitive` | 外焰 + 内焰 + 交叉木柴 |
| `classical` | 山花 + 额枋 + 三柱 + 基座 |
| `feudal` | 城垛 + 拱门 |
| `industrial` | 六齿轮 + 轮轴 |
| `modern` | 原子核 + 双轨道 |
| `future` | 火箭身 + 舷窗 + 尾翼 + 焰 |

- 描边宽度用 `_stroke(rect, width)`，主形填充用 `color` 且 alpha 0.24。
- 未知 era 不绘制、不报错。

在 `client/ui/theme.gd` 的 `SCENARIO` 常量中点号后追加：

```gdscript
	"era-glyph-size": 14,
	"era-glyph-size-detail": 24,
```

创建 `client/ui/era_glyph.gd`：

```gdscript
class_name EraGlyph
extends Control

var era_id := ""

static func create(era_value: String, side: int) -> EraGlyph:
	var node := EraGlyph.new()
	node.name = "EraGlyph"
	node.era_id = era_value
	node.custom_minimum_size = Vector2(side, side)
	node.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	node.mouse_filter = Control.MOUSE_FILTER_IGNORE
	node.resized.connect(node.queue_redraw)
	return node

func _draw() -> void:
	Icons.draw_era(self, era_id, Rect2(Vector2.ZERO, size), UITheme.alpha("color-accent", 0.62), 1.4)
```

- [ ] **Step 4: 运行测试确认通过**

Run: `/Applications/Godot.app/Contents/MacOS/Godot --headless --path client res://tests/ui_smoke_cards.tscn`
Expected: `UI_SMOKE_OK`。

- [ ] **Step 5: 提交**

```bash
git add client/ui/icons.gd client/ui/era_glyph.gd client/tests/ui_smoke_cards.gd
git commit -m "feat(client): 七个时代的代码绘制象形"
```

---

### Task 5: 接入卡面（卡框、星级、时代、攻防）

**Files:**
- Modify: `client/scenes/card_face_base.gd`
- Modify: `client/scenes/shop_card_face.gd`
- Modify: `client/scenes/hand_card_face.gd`
- Modify: `client/scenes/board_card_face.gd`
- Modify: `client/scenes/gallery_card_face.gd`
- Modify: `client/autoload/card_db.gd`（删除已无引用的 `RARITY` 与 `rarity_tier()`）
- Modify: `client/tests/ui_smoke_cards.gd`

**Interfaces:**
- Consumes: `CardFrame.create(level)`、`StatPlate.create(...)`、`EraGlyph.create(era_id, size)`。
- Produces: `CardFaceBase.add_frame(face_node: Control, card_id: String, hero := false) -> void`；`CardFaceBase.add_corner_badges(face_node, cost_text: String, era_id: String, badge_text: String, stealth := false) -> void`。

- [ ] **Step 1: 更新失败测试**

修改 `client/tests/ui_smoke_cards.gd`（以下断言都放进 `_ready()` 里已有的 `if open_card != null:` 块内，紧跟该块现有断言之后；`face_node` 已在该块定义）：

- 把 `EraBadge` 文本断言改为：

```gdscript
var era := open_card.find_child("EraGlyph", true, false)
_expect(era is EraGlyph, "商店卡右上角应有 EraGlyph")
if era is EraGlyph:
    _expect(
        (era as EraGlyph).era_id == str(CardDB.card("aolinpike-jidian").get("era", "")),
        "时代象形应取卡表 era id",
    )
```

- 位置断言里的 `era_rect` 改取 `EraGlyph` 的 `get_global_rect()`。
- 新增卡框与底牌断言：

```gdscript
var frame := open_card.find_child("CardFrame", true, false)
_expect(frame is CardFrame, "卡面应有 CardFrame")
if frame is CardFrame:
    var level := int(CardDB.card("aolinpike-jidian").get("level", 1))
    _expect((frame as CardFrame).level == level, "卡框星级应取卡表 level")
    _expect((frame as CardFrame).star_count == level, "卡框星数应等于 level")
    _expect(
        (frame as Control).size == (face_node as Control).size,
        "卡框应铺满卡面",
    )
_expect(open_card.find_child("AttackPlate", true, false) is StatPlate, "卡面应有攻击底牌")
_expect(open_card.find_child("HealthPlate", true, false) is StatPlate, "卡面应有血量底牌")
```

- 英雄卡与无时代卡（不依赖本测试的商店 fixture，直接组装）：

```gdscript
# 英雄卡：无星级、无攻防底牌、无时代象形。
var hero_face := GalleryCardFace.build_hero("adam-smith", CardDB.hero("adam-smith"))
add_child(hero_face)
await get_tree().process_frame
_expect(hero_face.find_child("CardFrame", true, false) == null, "英雄卡不应有卡框星级")
_expect(hero_face.find_child("AttackPlate", true, false) == null, "英雄卡不应有攻击底牌")
_expect(hero_face.find_child("EraGlyph", true, false) == null, "英雄卡不应有时代象形")
hero_face.queue_free()

# 棋盘卡：有卡框，无时代象形；空身材不出现底牌装饰（文字仍为「—」）。
var board_face := BoardCardFace.build({
    "cardId": "aolinpike-jidian", "attack": 2, "health": 3,
    "golden": false, "stealth": false,
})
add_child(board_face)
await get_tree().process_frame
_expect(board_face.find_child("CardFrame", true, false) is CardFrame, "棋盘卡应有卡框")
_expect(board_face.find_child("EraGlyph", true, false) == null, "棋盘卡不应有时代象形")
var effect_plate := StatPlate.create(StatPlate.ATTACK, "—", UITheme.font_size("font-title"), "CardAttack")
_expect(effect_plate.get_child(0).text == "—", "效果牌身材文字应保持「—」")
effect_plate.free()
board_face.queue_free()
```

- [ ] **Step 2: 运行测试确认失败**

Run: `/Applications/Godot.app/Contents/MacOS/Godot --headless --path client res://tests/ui_smoke_cards.tscn`
Expected: FAIL，找不到 `CardFrame` / `EraGlyph` / `AttackPlate`。

- [ ] **Step 3: 修改 `card_face_base.gd`**

- `add_corner_badges` 第三参数改名 `era_id`；当 `era_id != ""` 时改为：

```gdscript
var era := EraGlyph.create(era_id, UITheme.metric("era-glyph-size"))
era.name = "EraGlyph"
row.add_child(era)
```

- `add_info` 中把攻防 Label 替换为底牌。仍按 `if show_stats:` 无条件创建（文本可能为空或稍后由 `set_stats` 写入），只把 Label 换成底牌内的同名 Label：

```gdscript
var stat_size := UITheme.font_size(stats_token)
if show_stats:
    var attack := StatPlate.create(StatPlate.ATTACK, attack_text, stat_size, "CardAttack")
    attack.name = "AttackPlate"
    stats_row.add_child(attack)
...
if show_stats:
    var health := StatPlate.create(StatPlate.HEALTH, health_text, stat_size, "CardHealth")
    health.name = "HealthPlate"
    stats_row.add_child(health)
```

`set_stats` 用 `find_child("CardAttack")` 取到的是底牌内的 Label，保持原实现不动。

- 新增：

```gdscript
static func add_frame(face_node: Control, card_id: String, hero := false) -> void:
    if hero:
        return
    var level := int(CardDB.card(card_id).get("level", 1))
    face_node.add_child(CardFrame.create(level))
```

- 同时删除 `client/autoload/card_db.gd` 中已无引用的稀有度映射（`const RARITY := {...}` 与 `func rarity_tier(id: String) -> String`）。稀有度不再是展示维度，开发阶段不留兼容代码。

- [ ] **Step 4: 更新四个卡面组装器**

`add_frame` 紧跟在每个 `add_info(...)` 调用之后（不是放到函数末尾）：卡框要在信息区之上，但必须在冻结蒙层与最后添加的角标之下。商店卡的 `add_freeze` 与手牌卡的角标仍保持在卡框之后。

- `client/scenes/shop_card_face.gd`：`add_info(...)` 之后、`if frozen:` 之前加 `CardFaceBase.add_frame(face_node, card_id)`；`add_corner_badges` 第三参数改为 `str(entry.get("era", ""))`。
- `client/scenes/hand_card_face.gd`：`add_info(...)` 之后、注释「角标最后添加」之前加 `CardFaceBase.add_frame(face_node, card_id)`；角标第三参数改为 `str(entry.get("era", ""))`。
- `client/scenes/board_card_face.gd`：`add_info(...)` 之后加 `CardFaceBase.add_frame(face_node, card_id)`；角标第三参数保持 `""`。
- `client/scenes/gallery_card_face.gd`：`build` 的 `add_info(...)` 之后加 `CardFaceBase.add_frame(face_node, card_id)`，角标第三参数改为 `era_id`；`build_hero` 加 `CardFaceBase.add_frame(face_node, card_id, true)`（内部直接返回，不建节点）。

- [ ] **Step 5: 运行测试确认通过**

Run: `/Applications/Godot.app/Contents/MacOS/Godot --headless --path client res://tests/ui_smoke_cards.tscn`
Expected: `UI_SMOKE_OK`。

- [ ] **Step 6: 跑全量客户端检查**

Run: `make client-check && make client-smoke`
Expected: 两条命令退出码均为 0。

- [ ] **Step 7: 提交**

```bash
git add client/scenes/card_face_base.gd client/scenes/shop_card_face.gd client/scenes/hand_card_face.gd client/scenes/board_card_face.gd client/scenes/gallery_card_face.gd client/autoload/card_db.gd client/tests/ui_smoke_cards.gd
git commit -m "feat(client): 卡面接入程序化卡框、星级、时代象形与攻防底牌"
```

---

### Task 6: 页面背景

**Files:**
- Modify: `client/ui/ui.gd`
- Modify: `client/scenes/login.gd`
- Modify: `client/scenes/lobby.gd`
- Modify: `client/scenes/room.gd`
- Modify: `client/scenes/result.gd`
- Modify: `client/scenes/game.gd`
- Modify: `client/scenes/card_gallery.gd`
- Modify: `client/tests/ui_smoke_cards.gd`

**Interfaces:**
- Produces: `UI.background() -> Control`，节点名 `PageBackground`。

- [ ] **Step 1: 写失败测试**

在 `_check_stat_plate()` 之后新增：

```gdscript
func _check_page_background() -> void:
    _expect(
        ResourceLoader.exists("res://assets/ui/bg/bg_main.png"),
        "页面背景纹理应已导入",
    )
    var background := UI.background()
    _expect(background.name == "PageBackground", "背景节点名应为 PageBackground")
    _expect(background.mouse_filter == Control.MOUSE_FILTER_IGNORE, "背景不应拦截鼠标")
    _expect(background is TextureRect, "有纹理时应使用 TextureRect")
    background.free()
```

- [ ] **Step 2: 运行测试确认失败**

Run: `/Applications/Godot.app/Contents/MacOS/Godot --headless --path client res://tests/ui_smoke_cards.tscn`
Expected: `Invalid call. Nonexistent function 'background'`。

- [ ] **Step 3: 实现 `UI.background()`**

```gdscript
static func background() -> Control:
    var path := "res://assets/ui/bg/bg_main.png"
    if ResourceLoader.exists(path):
        var texture := ResourceLoader.load(path) as Texture2D
        if texture != null:
            var rect := TextureRect.new()
            rect.name = "PageBackground"
            rect.texture = texture
            rect.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
            rect.stretch_mode = TextureRect.STRETCH_SCALE
            rect.set_anchors_preset(Control.PRESET_FULL_RECT)
            rect.mouse_filter = Control.MOUSE_FILTER_IGNORE
            return rect
    var fallback := ColorRect.new()
    fallback.name = "PageBackground"
    fallback.color = UITheme.color("color-bg")
    fallback.set_anchors_preset(Control.PRESET_FULL_RECT)
    fallback.mouse_filter = Control.MOUSE_FILTER_IGNORE
    return fallback
```

- [ ] **Step 4: 替换六个页面的背景**

把每个页面里构造 `ColorRect` 背景的整段（`var background := ColorRect.new()` 到 `add_child(background)`，含 `color` / `name` / `mouse_filter` 行）替换为：

```gdscript
add_child(UI.background())
```

- `login.gd`、`result.gd` 原来写死 `Color("10141c")`，一并删除写死色值。
- `card_gallery.gd` 的节点名 `GalleryBackground` 有文档引用，改为：

```gdscript
var background := UI.background()
background.name = "GalleryBackground"
add_child(background)
```

- [ ] **Step 5: 运行测试确认通过**

Run: `/Applications/Godot.app/Contents/MacOS/Godot --headless --path client res://tests/ui_smoke_cards.tscn`
Expected: `UI_SMOKE_OK`。

- [ ] **Step 6: 提交**

```bash
git add client/ui/ui.gd client/scenes/login.gd client/scenes/lobby.gd client/scenes/room.gd client/scenes/result.gd client/scenes/game.gd client/scenes/card_gallery.gd client/tests/ui_smoke_cards.gd
git commit -m "feat(client): 六个页面铺纯质感深底背景"
```

---

### Task 7: 图鉴详情时代象形

**Files:**
- Modify: `client/scenes/gallery_detail.gd`
- Modify: `client/tests/ui_smoke_gallery.gd`

**Interfaces:**
- Consumes: `EraGlyph.create(era_id, size)`。

- [ ] **Step 1: 写失败测试**

在 `ui_smoke_gallery.gd` 的 `_check_detail()` 中，`GalleryDetailName` 断言之后、`detail_text` 断言之前插入：

```gdscript
    var glyph = instance.find_child("GalleryDetailEraGlyph", true, false)
    _expect(glyph is EraGlyph, "图鉴详情卡名旁应有时代象形")
    if glyph is EraGlyph:
        _expect(
            (glyph as EraGlyph).era_id == str(CardDB.card(card_id).get("era", "")),
            "详情时代象形应取卡表 era id",
        )
        _expect(
            (glyph as Control).custom_minimum_size == Vector2(24, 24),
            "详情时代象形应为 24×24",
        )
```

- [ ] **Step 2: 运行测试确认失败**

Run: `/Applications/Godot.app/Contents/MacOS/Godot --headless --path client res://tests/ui_smoke_gallery.tscn`
Expected: FAIL，找不到 `GalleryDetailEraGlyph`。

- [ ] **Step 3: 修改 `gallery_detail.gd`**

`_show` 中把 `_box.add_child(name_label)` 改为名称行：

```gdscript
var name_row := HBoxContainer.new()
name_row.name = "GalleryDetailNameRow"
name_row.add_theme_constant_override("separation", UITheme.space("space-xs"))
name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
name_row.add_child(name_label)
var name_era := str(entry.get("era", ""))
if name_era != "":
    var glyph := EraGlyph.create(name_era, UITheme.metric("era-glyph-size-detail"))
    glyph.name = "GalleryDetailEraGlyph"
    name_row.add_child(glyph)
_box.add_child(name_row)
```

变量名用 `name_era` 而不是 `era_id`：`_show` 在下文已有 `var era_id := str(entry.get("era", ""))`，同名会在同一作用域重复声明。英雄详情（`hero == true`）的 `entry` 没有 era，自然不建象形。

- [ ] **Step 4: 运行测试确认通过**

Run: `/Applications/Godot.app/Contents/MacOS/Godot --headless --path client res://tests/ui_smoke_gallery.tscn`
Expected: `UI_SMOKE_OK`。

- [ ] **Step 5: 提交**

```bash
git add client/scenes/gallery_detail.gd client/tests/ui_smoke_gallery.gd
git commit -m "feat(client): 图鉴详情卡名旁显示时代象形"
```

---

### Task 8: 文档回写与整分支验证

**Files:**
- Modify: `docs/current/product/ui/README.md`
- Modify: `docs/current/product/ui/tokens.md`
- Modify: `docs/current/technical/client.md`（§10 美术素材层）

- [ ] **Step 1: 回写 UI 文档**

`docs/current/product/ui/README.md` §1 改为：

- 位图资源只有卡牌立绘、头像、卡面与页面背景 `assets/ui/bg/bg_main.png`。
- 卡框、星级、时代象形、攻防底牌由 `CardFrame` / `EraGlyph` / `StatPlate` 代码绘制，入口写在组件表。
- 稀有度不再是展示维度，卡面展示 `level` 星级。

`docs/current/product/ui/tokens.md` §5 增加：

| 令牌 | 取值 | 用途 |
| --- | --- | --- |
| `era-glyph-size` | 14 | 卡面右上角时代象形边长 |
| `era-glyph-size-detail` | 24 | 图鉴详情卡名旁时代象形边长 |

攻防底牌边长不是独立令牌，按内嵌 Label 的字号推导：`边长 = 字号 + 4`（`font-body` 14 → 18，`font-caption` 11 → 15）。

`docs/current/technical/client.md` §10：

- 第 326 行的素材清单补一句「`assets/ui/bg/bg_main.png` 为页面背景纹理（512×512，整屏拉伸，不平铺）」。
- 删除第 329 行「稀有度」整条（`CardDB.rarity_tier` 与 `RARITY` 已删除）。
- 第 328 行回退说明补一句「页面背景缺图或加载失败时 `UI.background()` 回退 `color-bg` 纯色」。

- [ ] **Step 2: 全量验证**

```bash
make client-check
make client-smoke
make test
```
Expected: 全部退出码 0。

- [ ] **Step 3: 视觉截图核验**

创建一个**不提交**的临时场景 `output/ui_preview.tscn` 或临时脚本（放在 `output/`，不入库），windowed Godot 渲染四档卡面与图鉴详情，`screencapture` 截图到 `output/`，确认：双细线不糊、星级数量正确、时代象形在 14px 可辨、攻防底牌白字黑描边、背景不发灰；并覆盖 112×146、84×92、200×260 三种尺寸。核验后删除临时文件。

- [ ] **Step 4: 提交文档**

```bash
git add docs/current/product/ui/README.md docs/current/product/ui/tokens.md docs/current/technical/client.md
git commit -m "docs(ui): 回写程序化卡框、时代象形与背景资产"
```

---

## 交付说明

- 已执行：逐条列出命令与退出码（`make client-check`、`make client-smoke`、`make test`、生成器复现哈希）。
- 未验证：真机 iPhone 84×92 的实机观感（无设备时的说明与影响）。
- 阻塞：无；若 windowed Godot 截图无法进行，说明原因并给出替代核验方式。
