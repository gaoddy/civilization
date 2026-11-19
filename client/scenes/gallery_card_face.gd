class_name GalleryCardFace
extends RefCounted
## 图鉴卡面：英雄标识、费用/时代与两行介绍。
##
## 图鉴网格与图鉴详情共用同一条目；尺寸由调用方的卡片容器决定。

static func build(card_id: String, entry: Dictionary) -> Control:
    var era_id := str(entry.get("era", ""))
    var face_node := CardFaceBase.create_face(card_id, true)
    CardFaceBase.add_corner_badges(
        face_node,
        str(CardDB.display_cost(card_id)),
        UI.enum_text("era", era_id) if era_id != "" else "",
        TranslationServer.translate("ui.game.detail.hero") if bool(entry.get("is_hero", false)) else "",
        false,
    )
    CardFaceBase.add_info(
        face_node,
        card_id,
        CardText.stat_text(entry.get("attack")),
        CardText.stat_text(entry.get("health")),
        CardText.playstyle_text(entry),
        true,
    )
    return face_node
