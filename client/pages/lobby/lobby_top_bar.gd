class_name LobbyTopBar
extends HBoxContainer
## 大厅顶部：标题、登录问候与右侧状态提示。
##
## 只做呈现；身份与状态由页面写入，不订阅全局状态。

signal rank_pressed

var greeting: Label
var status: Label
## 段位与积分入口；账号视图未到达时隐藏。
var rank_button: Button


func _init() -> void:
    name = "LobbyTop"
    add_theme_constant_override("separation", UITheme.space("space-lg"))
    add_child(UIControls.label(tr("ui.lobby.title"), UITheme.font_size("font-hero"), false))
    greeting = UIControls.label("", UITheme.font_size("font-body"), false)
    add_child(greeting)
    rank_button = UIControls.styled_button("", "neutral", 30)
    rank_button.name = "LobbyRankButton"
    rank_button.pressed.connect(func(): rank_pressed.emit())
    rank_button.visible = false
    add_child(rank_button)
    var spacer := Control.new()
    spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    add_child(spacer)
    status = UIControls.label("", UITheme.font_size("font-body-sm"))
    status.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
    add_child(status)


## 登录问候：显示名与玩家码。
func set_identity(display_name: String, player_code: String) -> void:
    greeting.text = tr("ui.lobby.logged_in") % [display_name, player_code]


## 段位与积分：只展示服务端下发的值；缺段位时隐藏入口。
func set_ranked(tier_id: String, rating: int) -> void:
    if tier_id == "":
        rank_button.visible = false
        return
    rank_button.visible = true
    rank_button.text = tr("ui.lobby.rank") % [UIWording.tier_text(tier_id), rating]
