class_name ShopCardFace
extends RefCounted
## 商店槽卡面：费用与时代角标、冻结表现，不显示介绍。
##
## 元素组合固定在本组件内；商店视图只负责外层卡片的点击与可用性。

static func build(card_id: String, entry: Dictionary, frozen := false) -> Control:
    var face_node := CardFaceBase.create_face(card_id, false)
    CardFaceBase.add_corner_badges(
        face_node,
        str(CardDB.display_cost(card_id)),
        str(entry.get("era", "")),
        "",
        false,
    )
    CardFaceBase.add_info(
        face_node,
        card_id,
        CardText.stat_text(entry.get("attack")),
        CardText.stat_text(entry.get("health")),
        CardText.playstyle_text(entry),
        false,
    )
    CardFaceBase.add_frame(face_node, card_id)
    if frozen:
        CardFaceBase.add_freeze(face_node)
    return face_node
