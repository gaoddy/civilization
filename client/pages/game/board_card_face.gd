class_name BoardCardFace
extends RefCounted
## 棋盘随从卡面：等级星、时代、潜行蒙版与当前身材；无购买费用与介绍。
##
## 招募棋盘与战斗只读共用同一卡面与卡容器；战斗演示通过 `set_stats`
## 就地更新身材，不重建整张卡。

## 带外层的棋盘卡：尺寸、节点名与随从元数据由本入口统一设置。
static func build_card(minion: Dictionary, minion_index: int) -> BoardCard:
    var panel := BoardCard.new()
    panel.add_theme_stylebox_override("panel", UITheme.panel_style("raised-flush"))
    panel.name = "Board" + ("Minion%d" % minion_index)
    panel.set_meta("minion_id", str(minion.get("minionId", "")))
    panel.set_meta("minion_index", minion_index)
    panel.set_meta("target_zone", "board")
    panel.set_meta("target_id", str(minion.get("minionId", "")))
    panel.custom_minimum_size = Vector2(
        UITheme.stage_metric("minion-card-width"), UITheme.stage_metric("minion-card-height")
    )
    panel.add_child(build(minion))
    return panel


static func build(minion: Dictionary) -> Control:
    var card_id := str(minion.get("cardId", ""))
    var entry := CardDB.card(card_id)
    var golden := bool(minion.get("golden", false))
    var face_node := CardFaceBase.create_face(card_id, false)
    face_node.set_meta("golden", golden)
    # 潜行只给封面压蒙版：先于信息区添加，数值与名称保持在蒙版之上。
    if bool(minion.get("stealth", false)):
        CardFaceBase.add_stealth_veil(face_node)
    CardFaceBase.add_info(
        face_node,
        card_id,
        CardText.stat_text(minion.get("attack")),
        CardText.stat_text(minion.get("health")),
        CardText.playstyle_text(entry),
        false,
    )
    CardFaceBase.add_frame(face_node)
    # 卡框压在信息区之上、顶栏之下。时代取卡表，与其他卡面一致；费用位显示主动
    # 技能的发动费用：对手棋盘不下发 `abilities`，无主动技能的随从也没有该字段，
    # 两种情况都留空。
    CardFaceBase.add_corner_badges(
        face_node,
        _skill_cost(minion),
        str(entry.get("era", "")),
        "",
    )
    return face_node


## 第一个主动技能的发动费用；没有主动技能时返回空串，费用位留空。
static func _skill_cost(minion: Dictionary) -> String:
    for skill in minion.get("abilities", []):
        if not skill is Dictionary:
            continue
        var value: Variant = skill.get("cost")
        if value != null:
            return str(int(value))
    return ""


## 战斗演示的就地身材更新，按卡面子节点名改文本。
static func set_stats(panel: Control, attack_text: String, health_text: String) -> void:
    CardFaceBase.set_stats(panel, attack_text, health_text)
