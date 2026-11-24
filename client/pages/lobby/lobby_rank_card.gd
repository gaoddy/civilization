class_name LobbyRankCard
extends PanelContainer
## 右栏中部段位卡：大字段位与积分，背景画一圈淡金环。
##
## 未收到账号视图时显示「暂无段位 / 0 分」，不隐藏卡片，避免右栏高度跳动。

var _tier: Label
var _score: Label


func _init() -> void:
    name = "LobbyRankCard"
    add_theme_stylebox_override("panel", UITheme.panel_style("raised"))

    var box := VBoxContainer.new()
    box.alignment = BoxContainer.ALIGNMENT_CENTER
    box.add_theme_constant_override("separation", UITheme.space("space-sm"))
    add_child(box)

    _tier = UIControls.label("", UITheme.stage_metric("lobby-rank-tier-size"), false)
    _tier.name = "RankTier"
    _tier.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _tier.add_theme_color_override("font_color", UITheme.color("color-accent"))
    box.add_child(_tier)

    _score = UIControls.label("", UITheme.stage_metric("lobby-rank-score-size"), false)
    _score.name = "RankScore"
    _score.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
    _score.add_theme_color_override("font_color", UITheme.color("color-text-dim"))
    box.add_child(_score)


func set_ranked(tier_id: String, rating: int) -> void:
    _tier.text = UIWording.tier_text(tier_id) if tier_id != "" else tr("ui.lobby.rank_none")
    _score.text = tr("ui.lobby.rank_score") % rating
    queue_redraw()


func _draw() -> void:
    var radius := UITheme.stage_metric("lobby-rank-ring-size") * 0.5
    if radius <= 0.0:
        return
    draw_arc(
        size * 0.5,
        radius,
        0.0,
        TAU,
        64,
        UITheme.alpha("color-accent", UITheme.opacity("lobby-rank-ring-alpha")),
        UITheme.metric("lobby-rank-ring-width"),
        true,
    )
