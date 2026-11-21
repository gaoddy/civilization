class_name GalleryCardFace
extends RefCounted
## 图鉴卡面：普通卡含攻防、费用/时代与两行介绍，不显示玩法；英雄卡只展示名称与玩法。
##
## 图鉴网格与图鉴详情共用同一条目；尺寸由调用方的卡片容器决定。

static func build(card_id: String, entry: Dictionary) -> Control:
    var era_id := str(entry.get("era", ""))
    var face_node := CardFaceBase.create_face(card_id, true)
    # 图鉴卡不显示玩法，信息区只留攻防、介绍与名称。
    CardFaceBase.add_info(
        face_node,
        card_id,
        CardText.stat_text(entry.get("attack")),
        CardText.stat_text(entry.get("health")),
        "",
        true,
    )
    CardFaceBase.add_frame(face_node, card_id)
    # 卡框压在信息区之上、角标之下：角标最后添加。
    CardFaceBase.add_corner_badges(
        face_node,
        str(CardDB.display_cost(card_id)),
        era_id,
        "",
        false,
    )
    return face_node


## 英雄卡面：只展示名称与玩法，没有费用、时代、攻击/生命、角标与技能文本。
static func build_hero(card_id: String, entry: Dictionary) -> Control:
    var face_node := CardFaceBase.create_face(card_id, false, true)
    CardFaceBase.add_info(
        face_node,
        card_id,
        "",
        "",
        CardText.playstyle_text(entry),
        false,
        false,
        null,
        false,
        true,
    )
    CardFaceBase.add_frame(face_node, card_id, true)
    return face_node
