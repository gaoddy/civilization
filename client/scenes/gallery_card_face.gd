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
        era_id,
        "",
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
    CardFaceBase.add_frame(face_node, card_id)
    return face_node


## 英雄卡面：只展示身份与技能，没有费用、时代与攻击/生命。
static func build_hero(card_id: String, entry: Dictionary) -> Control:
    var face_node := CardFaceBase.create_face(card_id, true, true)
    CardFaceBase.add_corner_badges(
        face_node,
        "",
        "",
        TranslationServer.translate("ui.game.detail.hero"),
        false,
    )
    CardFaceBase.add_info(
        face_node,
        card_id,
        "",
        "",
        CardText.playstyle_text(entry),
        true,
        false,
        null,
        false,
        true,
    )
    CardFaceBase.add_frame(face_node, card_id, true)
    return face_node
