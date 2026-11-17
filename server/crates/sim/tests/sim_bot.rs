//! 机器人 brain 的确定性行为测试。
//!
//! 用固定 JSON 样例构造协议 `PlayerView`，只经 [`sim::bot::brain::next_intent`]
//! 做决策，不启网络、不跑引擎。

use protocol::intents::Intent;
use protocol::view::{Phase, PlayerView};
use serde_json::{Value, json};
use sim::bot::brain::{next_intent, recover_after_error, retry_with_target};

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
        "boardSlots": 7,
        "hpCap": 40,
        "combatLog": [],
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
        "enabledFactions": ["human", "machine", "biology", "plant", "scholar"]
    });
    serde_json::from_value(json).expect("样例视图应当能反序列化为 PlayerView")
}

fn catalog() -> rule_engine::data::Catalog {
    sim::embedded_catalog().expect("内嵌卡表应能加载")
}

/// 给视图补上手牌与棋盘，用于需要目标的效果牌决策。
fn with_hand_and_board(mut view: PlayerView, hand_cards: Value, board: Value) -> PlayerView {
    view.self_.hand = serde_json::from_value(hand_cards).expect("手牌样例应当能反序列化");
    view.self_.board = serde_json::from_value(board).expect("棋盘样例应当能反序列化");
    view
}

/// 棋盘随从样例；`index` 与 `minionId` 由调用方给出。
fn board_minion(minion_id: &str, index: u64) -> Value {
    json!({
        "attack": 1,
        "cardId": "adam-smith",
        "golden": false,
        "health": 1,
        "index": index,
        "keywords": [],
        "minionId": minion_id,
        "sellable": true
    })
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
        Intent::BuyMinion { slot } => assert_eq!(slot, 1),
        other => panic!("期望 BuyMinion，实际 {other:?}"),
    }
}

#[test]
fn plays_hand_before_buying() {
    // 买入一律入手牌，机器人必须先把手牌随从打上棋盘，避免手牌堵塞购买。
    let view = view(
        10,
        json!([{ "index": 0, "cardId": "adam-smith", "frozen": false }]),
        json!([{ "index": 0, "buyable": true, "disabledReason": null }]),
        json!([{ "index": 0, "playable": true, "disabledReason": null }]),
        json!([]),
        true,
    );

    let intent = next_intent(&view, &catalog(), 0).expect("有可打手牌时应当给出意图");
    match intent {
        Intent::PlayCard {
            hand_index,
            position,
            target_minion_id,
        } => {
            assert_eq!(hand_index, 0);
            assert_eq!(position, None, "机器人不指定落点，一律追加");
            assert_eq!(target_minion_id, None, "视图未声明需要目标时不填目标");
        }
        other => panic!("期望 PlayCard，实际 {other:?}"),
    }
}

#[test]
fn targets_first_board_minion_when_hand_requires_target() {
    // 需要目标的牌不带目标会被服务端拒绝（NO_VALID_TARGET），白跑一轮往返回合。
    // 棋盘非空时机器人应直接带上己方第一个随从。
    let view = view(
        10,
        json!([]),
        json!([]),
        json!([{ "index": 0, "playable": true, "disabledReason": null }]),
        json!([]),
        true,
    );
    let view = with_hand_and_board(
        view,
        json!([{
            "index": 0,
            "cardId": "yuandandan",
            "disabledReason": null,
            "playable": true,
            "requiresTarget": true
        }]),
        json!([board_minion("mi_1", 0)]),
    );

    let intent = next_intent(&view, &catalog(), 0).expect("有可打手牌时应当给出意图");
    match intent {
        Intent::PlayCard {
            hand_index,
            target_minion_id,
            ..
        } => {
            assert_eq!(hand_index, 0);
            assert_eq!(
                target_minion_id.as_deref(),
                Some("mi_1"),
                "需要目标的牌应预填己方第一个随从"
            );
        }
        other => panic!("期望 PlayCard，实际 {other:?}"),
    }
}

#[test]
fn omits_target_when_hand_requires_target_but_board_is_empty() {
    // 目标池为空时服务端允许落空，机器人不应凭空编造目标。
    let view = view(
        10,
        json!([]),
        json!([]),
        json!([{ "index": 0, "playable": true, "disabledReason": null }]),
        json!([]),
        true,
    );
    let view = with_hand_and_board(
        view,
        json!([{
            "index": 0,
            "cardId": "yuandandan",
            "disabledReason": null,
            "playable": true,
            "requiresTarget": true
        }]),
        json!([]),
    );

    let intent = next_intent(&view, &catalog(), 0).expect("有可打手牌时应当给出意图");
    match intent {
        Intent::PlayCard {
            target_minion_id, ..
        } => assert_eq!(target_minion_id, None, "棋盘为空时不填目标"),
        other => panic!("期望 PlayCard，实际 {other:?}"),
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

/// 给视图补一个己方随从，用于目标补选。
fn with_board(mut view: PlayerView, minion_id: &str) -> PlayerView {
    view.self_.board.push(protocol::view::BoardMinion {
        attack: 1,
        card_id: "adam-smith".to_string(),
        golden: false,
        health: 1,
        index: 0,
        keywords: Vec::new(),
        minion_id: minion_id.to_string(),
        sellable: true,
        stats: None,
    });
    view
}

#[test]
fn retries_rejected_intent_with_friendly_target() {
    // 服务端返回 NO_VALID_TARGET 后，机器人应给被拒意图补上己方随从并重发。
    let view = with_board(
        view(10, json!([]), json!([]), json!([]), json!([]), true),
        "minion-7",
    );

    let retried = retry_with_target(
        &view,
        &Intent::PlayCard {
            hand_index: 2,
            position: None,
            target_minion_id: None,
        },
    )
    .expect("有己方随从时应能补选");

    match retried {
        Intent::PlayCard {
            hand_index,
            position,
            target_minion_id,
        } => {
            assert_eq!(hand_index, 2, "重发保留原手牌下标");
            assert_eq!(position, None, "补选目标不改落点");
            assert_eq!(target_minion_id.as_deref(), Some("minion-7"));
        }
        other => panic!("期望 PlayCard，实际 {other:?}"),
    }
}

#[test]
fn retries_play_and_hero_power_with_target() {
    let view = with_board(
        view(10, json!([]), json!([]), json!([]), json!([]), true),
        "minion-3",
    );

    let played = retry_with_target(
        &view,
        &Intent::PlayCard {
            hand_index: 1,
            position: None,
            target_minion_id: None,
        },
    )
    .expect("手牌也可补选目标");
    assert!(matches!(
        played,
        Intent::PlayCard {
            hand_index: 1,
            position: None,
            target_minion_id: Some(ref id),
            ..
        } if id == "minion-3"
    ));

    let powered = retry_with_target(
        &view,
        &Intent::UseHeroPower {
            skill_id: "light-speed-strike".to_string(),
            target_minion_id: None,
        },
    )
    .expect("技能也可补选目标");
    assert!(matches!(
        powered,
        Intent::UseHeroPower {
            ref skill_id,
            target_minion_id: Some(ref id),
        } if skill_id == "light-speed-strike" && id == "minion-3"
    ));
}

#[test]
fn does_not_retry_without_board_or_for_targetless_intent() {
    let empty = view(10, json!([]), json!([]), json!([]), json!([]), true);
    assert!(
        retry_with_target(
            &empty,
            &Intent::PlayCard {
                hand_index: 0,
                position: None,
                target_minion_id: None,
            }
        )
        .is_none(),
        "没有己方随从时无法补选"
    );

    let view = with_board(empty, "minion-1");
    assert!(
        retry_with_target(&view, &Intent::RerollShop).is_none(),
        "不需要目标的意图不重发"
    );
}

#[test]
fn does_not_retry_an_intent_that_already_targets() {
    // 服务端可按过滤器拒绝补选的目标（例如要求的种族与首个随从不符）。
    // 补选后的意图再被拒时必须放弃，否则会「补选→拒绝→再补选」无限空转。
    let view = with_board(
        view(10, json!([]), json!([]), json!([]), json!([]), true),
        "minion-3",
    );

    assert!(
        retry_with_target(
            &view,
            &Intent::PlayCard {
                hand_index: 0,
                position: None,
                target_minion_id: Some("minion-3".to_string()),
            }
        )
        .is_none(),
        "已带目标的打牌意图再被拒时不得重发"
    );
    assert!(
        retry_with_target(
            &view,
            &Intent::UseHeroPower {
                skill_id: "light-speed-strike".to_string(),
                target_minion_id: Some("minion-3".to_string()),
            }
        )
        .is_none(),
        "已带目标的技能意图再被拒时不得重发"
    );
}
