class_name HandCardFace
extends RefCounted
## 手牌卡面：费用/时代、★ 或奖励星级、完整介绍与当前身材。
##
## 元素组合固定在本组件内；手牌卡负责拖拽数据与悬停，不参与卡面构造。
## 介绍不截断：信息区向上扩展并压缩字号，角标最后添加，始终压在信息区之上。

static func build(card: Dictionary, measure_host: Node = null) -> Control:
    var card_id := str(card.get("cardId", ""))
    var entry := CardDB.card(card_id)
    var golden := bool(card.get("golden", false))
    var reward_level: Variant = card.get("rewardLevel")
    # 奖励牌在角标行显示锁定的发现星级；金色随从优先显示「★」。
    var badge_text := "★" if golden else ""
    if not golden and reward_level != null:
        badge_text = str(int(reward_level))
    var face_node := CardFaceBase.create_face(card_id, true)
    CardFaceBase.add_info(
        face_node,
        card_id,
        CardText.stat_text(card.get("attack")),
        CardText.stat_text(card.get("health")),
        CardText.playstyle_text(entry),
        true,
        true,
        measure_host,
        true,
        false,
        golden,
    )
    CardFaceBase.add_frame(face_node, card_id)
    # 角标最后添加：介绍需要时信息区会向上扩展，角标必须压在信息区之上。
    CardFaceBase.add_corner_badges(
        face_node,
        str(CardDB.display_cost(card_id)),
        str(entry.get("era", "")),
        badge_text,
        false,
    )
    return face_node
