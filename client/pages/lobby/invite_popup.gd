class_name InvitePopup
extends Control
## 好友房间邀请弹窗：盖在宿主页面之上，逐条列出待处理邀请。
##
## 组件自己订阅 `GameStore.social_changed`，按 `live_invites()` 渲染；接受与拒绝
## 直接发 `RespondInvite`，结果仍以服务端回执为准。宿主只需挂进场景，可选连接
## `responded` 更新自己的提示文案。

signal responded(invite_id: String, accept: bool)

var _list: VBoxContainer


func _init() -> void:
    name = "InvitePopup"
    set_anchors_preset(Control.PRESET_FULL_RECT)
    mouse_filter = Control.MOUSE_FILTER_STOP
    visible = false

    var dim := ColorRect.new()
    dim.name = "InviteDim"
    dim.color = Color(0, 0, 0, 0.55)
    dim.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(dim)

    var center := CenterContainer.new()
    center.name = "InviteCenter"
    center.set_anchors_preset(Control.PRESET_FULL_RECT)
    add_child(center)

    var panel := UIControls.card_panel("raised")
    panel.name = "InvitePanel"
    panel.custom_minimum_size = Vector2(360, 0)
    center.add_child(panel)

    var box := VBoxContainer.new()
    box.add_theme_constant_override("separation", 8)
    panel.add_child(box)
    box.add_child(UIControls.section_title(tr("ui.lobby.invite_title")))
    _list = VBoxContainer.new()
    _list.name = "InviteList"
    _list.add_theme_constant_override("separation", 10)
    box.add_child(_list)


func _ready() -> void:
    if not GameStore.social_changed.is_connected(_refresh):
        GameStore.social_changed.connect(_refresh)
    _refresh()


func _exit_tree() -> void:
    # 弹窗随宿主页面一起释放；不主动断开会让全局信号持有已释放的节点。
    if GameStore.social_changed.is_connected(_refresh):
        GameStore.social_changed.disconnect(_refresh)


## 按当前待处理邀请渲染；没有邀请时整体隐藏，不留遮罩挡住宿主页面。
func _refresh() -> void:
    if _list == null:
        return
    UIControls.clear(_list)
    var invites: Array = GameStore.live_invites()
    visible = not invites.is_empty()
    for invite in invites:
        _list.add_child(_invite_card(invite))


func _invite_card(invite: Dictionary) -> VBoxContainer:
    var invite_id := str(invite.get("inviteId", ""))
    var card := VBoxContainer.new()
    card.add_theme_constant_override("separation", 6)
    card.add_child(
        UIControls.label(
            tr("ui.lobby.invite_text") % [
                str(invite.get("fromDisplayName", "")),
                str(invite.get("fromPlayerCode", "")),
                str(invite.get("roomCode", "")),
            ],
            14,
        )
    )
    var row := HBoxContainer.new()
    row.add_theme_constant_override("separation", 8)
    var accept := UIControls.button(tr("ui.lobby.accept_invite"))
    accept.name = "AcceptInvite_%s" % invite_id
    accept.pressed.connect(_on_respond.bind(invite_id, true))
    row.add_child(accept)
    var decline := UIControls.button(tr("ui.lobby.decline_invite"))
    decline.name = "DeclineInvite_%s" % invite_id
    decline.pressed.connect(_on_respond.bind(invite_id, false))
    row.add_child(decline)
    card.add_child(row)
    return card


func _on_respond(invite_id: String, accept: bool) -> void:
    # 回执到达并移除本条之前，禁用本次邀请的按钮，避免重复发送。
    for node_name in ["AcceptInvite_%s" % invite_id, "DeclineInvite_%s" % invite_id]:
        var button := _list.find_child(node_name, true, false)
        if button is Button:
            (button as Button).disabled = true
    Net.send(Protocol.MSG_RESPOND_INVITE, {"inviteId": invite_id, "accept": accept})
    responded.emit(invite_id, accept)
