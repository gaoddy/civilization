//! 机器人 brain 的确定性行为测试。
//!
//! 用固定 JSON 样例构造协议 `PlayerView`，只经 [`sim::bot::brain::next_intent`]
//! 做决策，不启网络、不跑引擎。

use protocol::intents::Intent;
use protocol::view::{Phase, PlayerView};
use serde_json::{Value, json};
use sim::bot::brain::{next_intent, recover_after_error};

/// 构造一份最小但完整的 `PlayerView`。
///
/// `shop` 是 `self.shop` 的槽位内容，`shop_slots` 是对应的可购买性；
/// `hand_cards`/`pending_choices` 为空数组以外的内容直接由调用方给出。
fn view(
    gold: u64,
    shop: Value,
    shop_slots: Value,
    hand_cards: Value,
    pending_choices: Value,
    can_end_recruit: bool,
) -> PlayerView {
    let json = json!({
        "roomId": "room_0",
        "phase": "Recruit",
        "turn": 1,
        "phaseDeadline": "2026-01-01T00:00:00Z",
        "self": {
            "playerId": "p0",
            "seat": 0,
            "hp": 30,
            "era": 1,
            "gold": gold,
            "heroCardId": "adam-smith",
            "heroPower": { "skillId": null, "usesLeft": 0 },
            "shop": shop,
            "hand": [],
            "board": [],
            "rerollCost": 1,
            "upgradeCost": 4,
            "recruited": false,
            "pendingChoices": pending_choices,
            "affordances": {
                "shopSlots": shop_slots,
                "handCards": hand_cards,
                "boardMinions": [],
                "canReroll": false,
                "canUpgradeEra": false,
                "upgradeDisabledReason": null,
                "canUseHeroPower": false,
                "canEndRecruit": can_end_recruit
            }
        },
        "players": [],
        "enabledFactions": ["人类", "自然", "机械", "经济", "文化"]
    });
    serde_json::from_value(json).expect("样例视图应当能反序列化为 PlayerView")
}

fn catalog() -> rule_engine::data::Catalog {
    sim::embedded_catalog().expect("内嵌卡表应能加载")
}

#[test]
fn consumes_pending_choice_first() {
    // 即使商店有买得起的最贵卡，也必须先消费待选。
    let view = view(
        10,
        json!([{ "index": 0, "cardId": "yuandandan", "frozen": false }]),
        json!([{ "index": 0, "buyable": true, "disabledReason": null }]),
        json!([]),
        json!([{ "kind": "discover", "candidates": ["a", "b", "c"] }]),
        true,
    );

    let intent = next_intent(&view, &catalog(), 0).expect("有待选时应当给出意图");
    match intent {
        Intent::ChooseDiscover { choice } => assert_eq!(choice, 0),
        other => panic!("期望 ChooseDiscover，实际 {other:?}"),
    }
}

#[test]
fn buys_most_expensive_buyable_slot() {
    // adam-smith 是随从（买价固定 3），aolimpike-jidian 是 2 费效果牌，
    // yuandandan 是 4 费效果牌：应选最贵的 yuandandan（下标 1）。
    let view = view(
        10,
        json!([
            { "index": 0, "cardId": "adam-smith", "frozen": false },
            { "index": 1, "cardId": "yuandandan", "frozen": false },
            { "index": 2, "cardId": "aolimpike-jidian", "frozen": false }
        ]),
        json!([
            { "index": 0, "buyable": true, "disabledReason": null },
            { "index": 1, "buyable": true, "disabledReason": null },
            { "index": 2, "buyable": true, "disabledReason": null }
        ]),
        json!([]),
        json!([]),
        true,
    );

    let intent = next_intent(&view, &catalog(), 0).expect("有可买槽位时应当给出意图");
    match intent {
        Intent::BuyMinion {
            slot,
            target_minion_id,
        } => {
            assert_eq!(slot, 1);
            assert_eq!(target_minion_id, None, "M1 机器人不填目标");
        }
        other => panic!("期望 BuyMinion，实际 {other:?}"),
    }
}

#[test]
fn ends_recruit_when_nothing_else_is_possible() {
    let view = view(0, json!([]), json!([]), json!([]), json!([]), true);

    let intent = next_intent(&view, &catalog(), 0).expect("无可做时应当结束招募");
    assert!(matches!(intent, Intent::EndRecruit), "实际 {intent:?}");
}

#[test]
fn recovers_from_rejected_intent_by_ending_recruit() {
    // 服务端拒绝意图后机器人不能停在原地：仍在招募且可结束时补发 EndRecruit。
    let view = view(10, json!([]), json!([]), json!([]), json!([]), true);

    let intent = recover_after_error(&view).expect("招募中收到错误后应当结束招募");
    assert!(matches!(intent, Intent::EndRecruit), "实际 {intent:?}");
}

#[test]
fn does_not_recover_outside_recruit() {
    let mut combat = view(10, json!([]), json!([]), json!([]), json!([]), true);
    combat.phase = Phase::Combat;
    assert!(recover_after_error(&combat).is_none());

    let mut recruited = view(10, json!([]), json!([]), json!([]), json!([]), true);
    recruited.self_.recruited = true;
    assert!(recover_after_error(&recruited).is_none());
}
