class_name BoardCardFace
extends RefCounted
## 棋盘随从卡面：★、潜行与当前身材；无费用、时代与介绍。
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
        UITheme.stage_metric("card-width"), UITheme.stage_metric("card-height")
    )
    panel.add_child(build(minion))
    return panel


static func build(minion: Dictionary) -> Control:
    var card_id := str(minion.get("cardId", ""))
    var entry := CardDB.card(card_id)
    var face_node := CardFaceBase.create_face(card_id, false)
    CardFaceBase.add_info(
        face_node,
        card_id,
        CardText.stat_text(minion.get("attack")),
        CardText.stat_text(minion.get("health")),
        CardText.playstyle_text(entry),
        false,
    )
    CardFaceBase.add_frame(face_node, card_id)
    # 卡框压在信息区之上、角标之下：角标最后添加。
    CardFaceBase.add_corner_badges(
        face_node,
        "",
        "",
        "★" if bool(minion.get("golden", false)) else "",
        bool(minion.get("stealth", false)),
    )
    return face_node


## 战斗演示的就地身材更新，按卡面子节点名改文本。
static func set_stats(panel: Control, attack_text: String, health_text: String) -> void:
    CardFaceBase.set_stats(panel, attack_text, health_text)
