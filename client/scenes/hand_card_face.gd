class_name HandCardFace
extends RefCounted
## 手牌卡面：费用/时代、★ 或奖励星级、两行介绍与当前身材。
##
## 元素组合固定在本组件内；手牌卡负责拖拽数据与悬停，不参与卡面构造。

static func build(card: Dictionary) -> Control:
    var card_id := str(card.get("cardId", ""))
    var entry := CardDB.card(card_id)
    var golden := bool(card.get("golden", false))
    var reward_level: Variant = card.get("rewardLevel")
    # 奖励牌在角标行显示锁定的发现星级；金色随从优先显示「★」。
    var badge_text := "★" if golden else ""
    if not golden and reward_level != null:
        badge_text = str(int(reward_level))
    var face_node := CardFaceBase.create_face(card_id, true)
    CardFaceBase.add_corner_badges(
        face_node,
        str(CardDB.display_cost(card_id)),
        UI.enum_text("era", str(entry.get("era", ""))),
        badge_text,
        false,
    )
    CardFaceBase.add_info(
        face_node,
        card_id,
        CardText.stat_text(card.get("attack")),
        CardText.stat_text(card.get("health")),
        CardText.playstyle_text(entry),
        true,
    )
    return face_node
