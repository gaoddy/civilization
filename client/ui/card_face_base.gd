class_name CardFaceBase
extends RefCounted
## 卡面共享绘制：封面、首字水印、角标行、信息区、冻结蒙层与就地改身材。
##
## 只构造通用部件，不决定场景显示哪些元素；商店、手牌、棋盘与图鉴的卡面
## 组件各自组合本文件的构造入口。

## 角标行在胶囊高度之外留的余量，避免文字贴边被裁切。
const CORNER_BADGE_EXTRA_HEIGHT := 2
## 冰晶细线的不透明度与条数；角度固定 45°，间距取主题令牌。
const FREEZE_CRYSTAL_ALPHA := 0.55
const FREEZE_CRYSTAL_LINES := 10
## 封面美术与卡边之间保留的距离：方形美术的角必须落在卡片圆角以内。
## 手牌介绍的排版范围取 `font-caption` 到 `font-micro`，行距收紧到 0。
const DESC_LINE_SPACING := 0

static var _line_height_cache: Dictionary = {}


## 卡面根与封面：玩法色铺满整卡；有美术资源时叠卡面纹理，无图或加载失败时
## 保留纯色封面与首字水印。所有子节点忽略鼠标，点击与悬停由外层卡片决定。
static func create_face(card_id: String, show_desc: bool, hero := false) -> Control:
    var face_node := Control.new()
    face_node.name = "CardFace"
    # 卡面记下自己的卡与三连标记：框体材质按它取时代色与金框，不需要逐处传参。
    face_node.set_meta("card_id", card_id)
    face_node.mouse_filter = Control.MOUSE_FILTER_IGNORE
    face_node.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    face_node.size_flags_vertical = Control.SIZE_EXPAND_FILL

    var entry := CardDB.hero(card_id) if hero else CardDB.card(card_id)
    # 封面用带圆角的面板而不是色块：卡面容器只留 1px 描边，方形色块会盖住
    # 卡片四角的圆弧。
    var cover := Panel.new()
    cover.name = "Cover"
    var cover_style := StyleBoxFlat.new()
    cover_style.bg_color = UITheme.playstyle_cover(CardDB.primary_playstyle(entry))
    cover_style.set_corner_radius_all(UITheme.radius("radius-md"))
    cover_style.set_content_margin_all(0)
    cover.add_theme_stylebox_override("panel", cover_style)
    cover.set_anchors_preset(Control.PRESET_FULL_RECT)
    cover.mouse_filter = Control.MOUSE_FILTER_IGNORE
    face_node.add_child(cover)

    # 有美术资源时在其上叠卡面纹理；无图或加载失败保留上面的纯色封面回退。
    # 美术本身是方图，向内收几像素后四角落在卡片圆角内，不需要额外的圆角裁剪。
    var cover_path := CardDB.art_path(card_id, "card_face")
    if cover_path != "" and ResourceLoader.exists(cover_path):
        var art := ResourceLoader.load(cover_path) as Texture2D
        if art != null:
            var cover_art := TextureRect.new()
            cover_art.name = "CoverArt"
            cover_art.texture = art
            cover_art.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
            cover_art.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
            cover_art.set_anchors_preset(Control.PRESET_FULL_RECT)
            cover_art.offset_left = UITheme.stage_metric("card-cover-inset")
            cover_art.offset_top = UITheme.stage_metric("card-cover-inset")
            cover_art.offset_right = -UITheme.stage_metric("card-cover-inset")
            cover_art.offset_bottom = -UITheme.stage_metric("card-cover-inset")
            cover_art.mouse_filter = Control.MOUSE_FILTER_IGNORE
            face_node.add_child(cover_art)

    # 有卡面纹理时不叠首字水印，避免与立绘重叠。
    var has_cover_art := face_node.get_node_or_null("CoverArt") != null
    if not has_cover_art:
        var desc_text := _desc_text(card_id, hero) if show_desc else ""
        var info_height := UITheme.stage_metric(
            "card-info-height" if desc_text != "" else "card-info-compact-height"
        )
        var watermark := UIControls.label(CardText.initial(card_id), UITheme.font_size("font-hero"), false)
        watermark.name = "CoverMark"
        watermark.set_anchors_preset(Control.PRESET_FULL_RECT)
        watermark.offset_bottom = -info_height
        watermark.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
        watermark.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
        watermark.add_theme_color_override("font_color", UITheme.alpha("color-text", 0.16))
        watermark.mouse_filter = Control.MOUSE_FILTER_IGNORE
        face_node.add_child(watermark)
    return face_node


## 左上费用、金色/奖励角标与右上时代的角标行；窄卡上左右贴边、互不重叠。
## 潜行状态标识放在右侧时代角标之前，与金色角标分列两端。全部为空时不建行。
static func add_corner_badges(
    face_node: Control,
    cost_text: String,
    era_id: String,
    badge_text: String,
    stealth := false,
) -> void:
    if cost_text == "" and era_id == "" and badge_text == "" and not stealth:
        return
    var margin := UITheme.space("space-xs")
    var row := HBoxContainer.new()
    row.name = "CardCorners"
    row.mouse_filter = Control.MOUSE_FILTER_IGNORE
    row.set_anchors_preset(Control.PRESET_TOP_WIDE)
    row.offset_left = margin
    row.offset_right = -margin
    row.offset_top = margin
    row.offset_bottom = margin + _badge_height()
    face_node.add_child(row)

    if cost_text != "":
        var cost := _corner_badge(cost_text, "accent")
        cost.name = "CostBadge"
        row.add_child(cost)

    if badge_text != "":
        var badge := _corner_badge(badge_text, "accent")
        badge.name = "CardBadge"
        row.add_child(badge)

    var spacer := Control.new()
    spacer.mouse_filter = Control.MOUSE_FILTER_IGNORE
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    row.add_child(spacer)

    if stealth:
        var stealth_badge := _corner_badge(
            TranslationServer.translate("ui.game.card.stealth_glyph"), "info"
        )
        stealth_badge.name = "StealthBadge"
        row.add_child(stealth_badge)

    if era_id != "":
        var era := EraGlyph.create(era_id, UITheme.stage_metric("era-glyph-size"))
        era.name = "EraGlyph"
        row.add_child(era)


## 底部信息区：介绍、居中名称与「攻击 · 类型 · 生命」。
## `show_desc` 为假时不渲染介绍，信息区退回紧凑高度（商店与棋盘卡在手机上
## 放不下介绍）。`fit_desc` 为真时介绍不再截断：信息区向上扩展并按可用空间
## 逐档缩小字号与行距，直到整段介绍放下；角标由调用方在信息区之后添加，
## 保证信息区扩大后角标仍绘制在最上层。
static func add_info(
    face_node: Control,
    card_id: String,
    attack_text: String,
    health_text: String,
    type_text: String,
    show_desc: bool,
    fit_desc := false,
    measure_host: Node = null,
    show_stats := true,
    hero := false,
    golden := false,
) -> void:
    var desc_text := _desc_text(card_id, hero, golden) if show_desc else ""
    var padding := UITheme.space("space-sm")
    var compact := ScreenProfile.device_kind() == ScreenProfile.DEVICE_PHONE
    # 手机卡面更窄：名称与数值各收一档，避免「攻击 · 类型 · 生命」挤成省略号。
    var name_token := "font-caption" if compact else "font-body-sm"
    var stats_token := "font-body" if compact else "font-title"
    var desc_font := UITheme.font_size("font-caption")
    var desc_max_lines := 2
    var desc_overrun := TextServer.OVERRUN_TRIM_ELLIPSIS
    var info_height := UITheme.stage_metric(
        "card-info-height" if desc_text != "" else "card-info-compact-height"
    )
    var fitting := desc_text != "" and fit_desc and _can_measure(measure_host)
    if fitting:
        # 手牌卡不显示费用/时代之外的封面信息，名称与数值各收一档，把高度让给介绍。
        name_token = "font-caption"
        stats_token = "font-caption" if compact else "font-body"
        var face_width := UITheme.stage_metric("card-width") - 2
        var face_height := UITheme.stage_metric("card-height") - 2
        var desc_width := face_width - padding * 2
        # 角标行贴顶：上边距 + 胶囊高度。信息区顶边越过角标底边时由绘制顺序兜底，
        # 介绍文字从信息区内边距之后开始，正好落在角标下方。
        var badge_bottom := (
            1
            + UITheme.space("space-xs")
            + _line_height(UITheme.font_size("font-caption"), measure_host)
            + UITheme.space("space-2xs") * 2
        )
        var max_info := face_height - (badge_bottom - padding)
        var non_desc := (
            padding * 2
            + _line_height(UITheme.font_size(name_token), measure_host)
            + _line_height(UITheme.font_size(stats_token), measure_host)
            + UITheme.space("space-2xs") * 2
        )
        var fit := _fit_desc(desc_text, desc_width, max_info - non_desc, measure_host)
        desc_font = int(fit["font_size"])
        desc_max_lines = int(fit["lines"])
        if desc_max_lines < 0:
            desc_overrun = TextServer.OVERRUN_NO_TRIMMING
        info_height = clampi(non_desc + int(fit["height"]), info_height, max_info)
    var info := UIControls.card_panel("cover-info")
    info.name = "CardInfo"
    info.mouse_filter = Control.MOUSE_FILTER_IGNORE
    info.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
    info.offset_top = -info_height
    info.offset_bottom = 0
    var info_box := VBoxContainer.new()
    info_box.name = "CardInfoBox"
    info_box.mouse_filter = Control.MOUSE_FILTER_IGNORE
    info_box.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    info_box.alignment = BoxContainer.ALIGNMENT_END
    info.add_child(info_box)

    if desc_text != "":
        # 固定高度的承载框隔离 Label 最小尺寸，保证信息区不被长介绍顶高；
        # 非手牌卡介绍最多两行，超长省略；手牌由上面的 fit 决定字号与行数。
        var desc_box := Control.new()
        desc_box.name = "CardDescBox"
        desc_box.mouse_filter = Control.MOUSE_FILTER_IGNORE
        desc_box.size_flags_vertical = Control.SIZE_EXPAND_FILL
        var desc_label := UIControls.label(desc_text, desc_font)
        desc_label.name = "CardDesc"
        desc_label.set_anchors_preset(Control.PRESET_FULL_RECT)
        desc_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
        desc_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
        desc_label.max_lines_visible = desc_max_lines
        desc_label.text_overrun_behavior = desc_overrun
        if fit_desc:
            desc_label.add_theme_constant_override("line_spacing", DESC_LINE_SPACING)
        desc_label.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
        desc_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
        desc_box.add_child(desc_label)
        info_box.add_child(desc_box)

    var name_label := UIControls.label(CardDB.display_name(card_id), UITheme.font_size(name_token), false)
    name_label.name = "CardName"
    name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    name_label.max_lines_visible = 1
    name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    info_box.add_child(name_label)

    var stats_row := HBoxContainer.new()
    stats_row.name = "CardStats"
    stats_row.mouse_filter = Control.MOUSE_FILTER_IGNORE
    stats_row.add_theme_constant_override("separation", UITheme.space("space-2xs"))
    info_box.add_child(stats_row)

    # 底牌始终创建：文本可能为空或稍后由 `set_stats` 写入，空值时不画装饰。
    if show_stats:
        var attack := StatPlate.create(
            StatPlate.ATTACK, attack_text, UITheme.font_size(stats_token), "CardAttack"
        )
        attack.name = "AttackPlate"
        stats_row.add_child(attack)

    var type_label := UIControls.label(type_text, UITheme.font_size("font-caption"), false)
    type_label.name = "CardType"
    type_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    type_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    type_label.max_lines_visible = 1
    type_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
    type_label.add_theme_color_override("font_color", UITheme.color("color-text-muted"))
    type_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
    stats_row.add_child(type_label)

    if show_stats:
        var health := StatPlate.create(
            StatPlate.HEALTH, health_text, UITheme.font_size(stats_token), "CardHealth"
        )
        health.name = "HealthPlate"
        stats_row.add_child(health)
    face_node.add_child(info)


## 卡框：按卡表等级画三条嵌套描边与顶部星级。英雄卡没有等级，不建卡框。
## 在 `add_info` 之后、冻结蒙层与角标之前调用：卡框要压在信息区之上，
## 又要在冻结蒙层之下。
static func add_frame(face_node: Control, card_id: String, hero := false) -> void:
    if hero:
        return
    var entry := CardDB.card(card_id)
    var level := int(entry.get("level", 1))
    face_node.add_child(CardFrame.create(
        level, str(entry.get("era", "")), bool(face_node.get_meta("golden", false))
    ))
    # 框体材质叠在卡面最外层：按时代取合金色，三连换成足金；卡内信息条等子面板
    # 由 `_collect` 跳过，保证一张卡只有一圈边框。
    FrameMaterial.apply(face_node, FrameMaterial.Style.BEVEL, {
        "tint": FrameMaterial.tint_for_era(str(entry.get("era", ""))),
        "golden": bool(face_node.get_meta("golden", false)),
    })


## 冻结表现：整卡半透明蒙层 + 固定角度与间距的冰晶细线 + 右下角「冻」字。
static func add_freeze(face_node: Control) -> void:
    # 蒙层同样用圆角面板，避免铺满卡面后把卡片的圆角盖成直角。
    var overlay := Panel.new()
    overlay.name = "FreezeOverlay"
    var overlay_style := StyleBoxFlat.new()
    overlay_style.bg_color = UITheme.color("color-info")
    overlay_style.set_corner_radius_all(UITheme.radius("radius-md"))
    overlay_style.set_content_margin_all(0)
    overlay.add_theme_stylebox_override("panel", overlay_style)
    # self_modulate 只压暗蒙层自身，不向下继承到冰晶与「冻」字。
    overlay.self_modulate.a = UITheme.opacity("freeze-overlay-alpha")
    overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
    overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
    face_node.add_child(overlay)

    var crystals := Control.new()
    crystals.name = "FreezeCrystals"
    crystals.mouse_filter = Control.MOUSE_FILTER_IGNORE
    crystals.clip_contents = true
    crystals.set_anchors_preset(Control.PRESET_FULL_RECT)
    overlay.add_child(crystals)

    # 斜向细线：角度与间距由令牌固定，不随卡片尺寸变化。
    var gap := UITheme.stage_metric("freeze-crystal-gap")
    var line_color := UITheme.alpha("color-info", FREEZE_CRYSTAL_ALPHA)
    for index in FREEZE_CRYSTAL_LINES:
        var line := ColorRect.new()
        line.color = line_color
        line.size = Vector2(gap * 2, 1)
        line.position = Vector2(
            -gap + (index % 2) * gap / 2.0,
            -gap + index * gap,
        )
        line.rotation = deg_to_rad(45.0)
        line.mouse_filter = Control.MOUSE_FILTER_IGNORE
        crystals.add_child(line)

    var glyph := UIControls.label(
        TranslationServer.translate("ui.game.card.freeze_glyph"),
        UITheme.font_size("font-caption"),
        false,
    )
    glyph.name = "FreezeGlyph"
    glyph.add_theme_color_override("font_color", UITheme.color("color-info"))
    glyph.set_anchors_preset(Control.PRESET_BOTTOM_RIGHT)
    glyph.grow_horizontal = Control.GROW_DIRECTION_BEGIN
    glyph.grow_vertical = Control.GROW_DIRECTION_BEGIN
    var edge := UITheme.space("space-xs")
    glyph.offset_left = -edge
    glyph.offset_top = -edge
    glyph.offset_right = -edge
    glyph.offset_bottom = -edge
    glyph.mouse_filter = Control.MOUSE_FILTER_IGNORE
    overlay.add_child(glyph)


## 战斗演示的就地身材更新：按节点名改文本，不重建整张卡。
static func set_stats(face_node: Control, attack_text: String, health_text: String) -> void:
    var attack_label := face_node.find_child("CardAttack", true, false) as Label
    if attack_label != null:
        attack_label.text = attack_text
    var health_label := face_node.find_child("CardHealth", true, false) as Label
    if health_label != null:
        health_label.text = health_text


## 在给定宽度内选择能让介绍完整放下的最大字号；都放不下时退回最小字号，
## 并按可用高度给出最多行数（`lines` 为 -1 表示不限制行数）。
## 卡面介绍：英雄只读英雄池，避免与同名随从的技能文本串用；
## 金色随从读取金色文案，缺键回落普通文案。
static func _desc_text(card_id: String, hero: bool, golden := false) -> String:
    return CardDB.hero_text(card_id) if hero else CardDB.display_text(card_id, golden)


static func _fit_desc(text: String, width: int, available: int, host: Node) -> Dictionary:
    var min_size := UITheme.font_size("font-micro")
    for font_size in range(UITheme.font_size("font-caption"), min_size - 1, -1):
        var height := _measure_text_height(text, width, font_size, host)
        if height > 0 and height <= available:
            return {"font_size": font_size, "height": height, "lines": -1}
    var line_height := _line_height(min_size, host)
    var lines := maxi(1, floori(float(available) / maxf(float(line_height), 1.0)))
    return {"font_size": min_size, "height": line_height * lines, "lines": lines}


## 测量节点必须挂在一个已在场景树里的普通节点上：容器会改写测量节点的宽度，
## 所以调用方传入手牌行或测试夹具这类非容器节点。
static func _can_measure(host: Node) -> bool:
    return host != null and is_instance_valid(host) and host.is_inside_tree()


## 用与卡面介绍相同的换行与行距测量文本高度；测完立即释放，不进入绘制。
static func _measure_text_height(text: String, width: int, font_size: int, host: Node) -> int:
    var probe := UIControls.label(text, font_size)
    probe.max_lines_visible = -1
    probe.text_overrun_behavior = TextServer.OVERRUN_NO_TRIMMING
    probe.add_theme_constant_override("line_spacing", DESC_LINE_SPACING)
    probe.custom_minimum_size = Vector2(width, 0)
    probe.size = Vector2(width, 0)
    probe.mouse_filter = Control.MOUSE_FILTER_IGNORE
    host.add_child(probe)
    probe.size = Vector2(width, 0)
    var height := int(probe.get_minimum_size().y)
    probe.free()
    return height


## 单行行高：与项目字体一致的字体度量，供信息区高度计算使用。
static func _line_height(font_size: int, host: Node) -> int:
    if _line_height_cache.has(font_size):
        return int(_line_height_cache[font_size])
    if not _can_measure(host):
        return roundi(font_size * 1.45)
    var probe := UIControls.label("A", font_size, false)
    probe.mouse_filter = Control.MOUSE_FILTER_IGNORE
    host.add_child(probe)
    var height := int(probe.get_minimum_size().y)
    probe.free()
    _line_height_cache[font_size] = height
    return height


## 窄卡角标：比胶囊更紧凑，避免两角标在 112 宽的商店卡上相撞。
static func _corner_badge(text: String, variant: String) -> Control:
    # 角标统一走 CornerBadge：深底 + 铜边 + 顶部高光，和卡框同一套厚度语言。
    return CornerBadge.create(text, variant)


static func _badge_height() -> int:
    return UITheme.font_size("font-caption") + UITheme.space("space-2xs") * 2 + CORNER_BADGE_EXTRA_HEIGHT
