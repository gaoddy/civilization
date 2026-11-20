//! 机器人 brain 的确定性行为测试。
//!
//! 用固定 JSON 样例构造协议 `PlayerView`，只经 [`sim::bot::brain::next_intent`]
//! 做决策，不启网络、不跑引擎。

mod common;

use common::catalog;
use protocol::intents::Intent;
use protocol::view::{Phase, PlayerView};
use protocol::{TargetRef, TargetZone};
use serde_json::{Value, json};
use sim::bot::brain::{BlockedTargets, next_intent, recover_after_error, retry_with_target};

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
            "heroPowers": [],
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
                "canUnfreeze": false,
                "canUpgradeEra": false,
                "upgradeDisabledReason": null,
                "canEndRecruit": can_end_recruit
            }
        },
        "players": [],
        "enabledFactions": ["mammal", "machine", "plant", "virus", "insect"]
    });
    serde_json::from_value(json).expect("样例视图应当能反序列化为 PlayerView")
}

/// 给视图补上手牌与棋盘，用于需要目标的效果牌决策。
fn with_hand_and_board(mut view: PlayerView, hand_cards: Value, board: Value) -> PlayerView {
    view.self_.hand = serde_json::from_value(hand_cards).expect("手牌样例应当能反序列化");
    view.self_.board = serde_json::from_value(board).expect("棋盘样例应当能反序列化");
    view
}

/// 棋盘目标。
fn board_target(id: &str) -> TargetRef {
    TargetRef {
        zone: TargetZone::Board,
        id: id.to_string(),
    }
}

/// 需要目标的手牌样例；`candidates` 为服务端下发的合法目标。
fn hand_effect(index: i64, instance_id: &str, candidates: Value) -> Value {
    json!({
        "attack": null,
        "cardId": "yuandandan",
        "disabledReason": null,
        "golden": false,
        "health": null,
        "index": index,
        "instanceId": instance_id,
        "playable": true,
        "requiresTarget": true,
        "rewardLevel": null,
        "targetCandidates": candidates
    })
}

/// 目标候选的 JSON 片段。
fn target_json(zone: &str, id: &str) -> Value {
    json!({ "zone": zone, "id": id })
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
        "sellable": true,
        "stealth": false
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

    let intent = next_intent(&view, &catalog(), 0, &BlockedTargets::default())
        .expect("有待选时应当给出意图");
    match intent {
        Intent::ChooseDiscover { choice } => assert_eq!(choice, 0),
        other => panic!("期望 ChooseDiscover，实际 {other:?}"),
    }
}

#[test]
fn pending_target_starts_from_first_board_minion_not_source() {
    // 战吼的合法池可能带筛选（例如「一个友方鱼纲随从」），来源随从在棋盘末尾时
    // 未必在池内。目标待选从棋盘第一个随从开始，被拒的候选记入阻塞集合后由
    // 下一次决策依次换下一个，能覆盖整个棋盘。
    let pending = json!([{
        "kind": "target",
        "sourceMinionId": "minion-9",
        "candidates": [
            target_json("board", "minion-1"),
            target_json("board", "minion-2"),
            target_json("board", "minion-9"),
        ],
    }]);
    let board = json!([
        board_minion("minion-1", 0),
        board_minion("minion-2", 1),
        board_minion("minion-9", 2),
    ]);
    let view = with_hand_and_board(
        view(10, json!([]), json!([]), json!([]), pending, false),
        json!([]),
        board,
    );

    let intent =
        next_intent(&view, &catalog(), 0, &BlockedTargets::default()).expect("有待选时应给出意图");
    match intent {
        Intent::ChooseTarget { target } => {
            assert_eq!(target.id, "minion-1");
            assert_eq!(target.zone, TargetZone::Board);
        }
        other => panic!("期望 ChooseTarget，实际 {other:?}"),
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

    let intent = next_intent(&view, &catalog(), 0, &BlockedTargets::default())
        .expect("有可买槽位时应当给出意图");
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

    let intent = next_intent(&view, &catalog(), 0, &BlockedTargets::default())
        .expect("有可打手牌时应当给出意图");
    match intent {
        Intent::PlayCard {
            hand_index,
            position,
            target,
        } => {
            assert_eq!(hand_index, 0);
            assert_eq!(position, None, "机器人不指定落点，一律追加");
            assert!(target.is_none(), "视图未声明需要目标时不填目标");
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
            "instanceId": "2001",
            "cardId": "yuandandan",
            "golden": false,
            "attack": null,
            "health": null,
            "rewardLevel": null,
            "disabledReason": null,
            "playable": true,
            "requiresTarget": true,
            "targetCandidates": [target_json("board", "mi_1")]
        }]),
        json!([board_minion("mi_1", 0)]),
    );

    let intent = next_intent(&view, &catalog(), 0, &BlockedTargets::default())
        .expect("有可打手牌时应当给出意图");
    match intent {
        Intent::PlayCard {
            hand_index, target, ..
        } => {
            assert_eq!(hand_index, 0);
            assert_eq!(
                target.as_ref().map(|target| target.id.as_str()),
                Some("mi_1"),
                "需要目标的牌应预填服务端下发的第一个候选"
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
            "instanceId": "2001",
            "cardId": "yuandandan",
            "golden": false,
            "attack": null,
            "health": null,
            "rewardLevel": null,
            "disabledReason": null,
            "playable": true,
            "requiresTarget": true,
            "targetCandidates": []
        }]),
        json!([]),
    );

    let intent = next_intent(&view, &catalog(), 0, &BlockedTargets::default())
        .expect("有可打手牌时应当给出意图");
    match intent {
        Intent::PlayCard { target, .. } => {
            assert!(target.is_none(), "候选为空时不填目标")
        }
        other => panic!("期望 PlayCard，实际 {other:?}"),
    }
}

#[test]
fn ends_recruit_when_nothing_else_is_possible() {
    let view = view(0, json!([]), json!([]), json!([]), json!([]), true);

    let intent = next_intent(&view, &catalog(), 0, &BlockedTargets::default())
        .expect("无可做时应当结束招募");
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

#[test]
fn skips_hand_blocked_earlier_in_the_turn() {
    // 整盘都没有合法目标、本回合放弃该牌后，直接换下一张。
    let view = view(
        10,
        json!([]),
        json!([]),
        json!([
            { "index": 0, "playable": true, "disabledReason": null },
            { "index": 1, "playable": true, "disabledReason": null }
        ]),
        json!([]),
        true,
    );
    let mut blocked = BlockedTargets::default();
    blocked.block_hand(0);

    let intent = next_intent(&view, &catalog(), 0, &blocked).expect("还有另一张可打手牌");
    match intent {
        Intent::PlayCard { hand_index, .. } => assert_eq!(hand_index, 1, "应跳过已放弃的手牌"),
        other => panic!("期望 PlayCard，实际 {other:?}"),
    }
}

#[test]
fn ends_recruit_after_blocked_limit() {
    // 连续放弃多张后不再试错：达到上限时直接结束招募，避免空转。
    let view = view(
        0,
        json!([]),
        json!([]),
        json!([{ "index": 0, "playable": true, "disabledReason": null }]),
        json!([]),
        true,
    );
    let mut blocked = BlockedTargets::default();
    for hand_index in 0..3 {
        blocked.block_hand(hand_index);
    }

    let intent = next_intent(&view, &catalog(), 0, &blocked).expect("达到上限时应结束招募");
    assert!(matches!(intent, Intent::EndRecruit), "实际 {intent:?}");
}

#[test]
fn rotates_pending_target_past_rejected_minion() {
    // ChooseTarget 被拒只跳过该候选，下一次决策换下一个己方随从。
    let pending = json!([{
        "kind": "target",
        "sourceMinionId": "minion-2",
        "candidates": [
            target_json("board", "minion-1"),
            target_json("board", "minion-2"),
        ],
    }]);
    let board = json!([board_minion("minion-1", 0), board_minion("minion-2", 1)]);
    let view = with_hand_and_board(
        view(10, json!([]), json!([]), json!([]), pending, false),
        json!([]),
        board,
    );
    let mut blocked = BlockedTargets::default();
    blocked.block_target(&board_target("minion-1"));

    let intent = next_intent(&view, &catalog(), 0, &blocked).expect("还有下一个候选");
    match intent {
        Intent::ChooseTarget { target } => assert_eq!(target.id, "minion-2"),
        other => panic!("期望 ChooseTarget，实际 {other:?}"),
    }
}

#[test]
fn stops_when_every_board_minion_is_blocked() {
    // 全部候选都被拒绝时不再发意图，交给回合计时推进，避免无限空转。
    let pending = json!([{
        "kind": "target",
        "sourceMinionId": "minion-2",
        "candidates": [
            target_json("board", "minion-1"),
            target_json("board", "minion-2"),
        ],
    }]);
    let board = json!([board_minion("minion-1", 0), board_minion("minion-2", 1)]);
    let view = with_hand_and_board(
        view(0, json!([]), json!([]), json!([]), pending, false),
        json!([]),
        board,
    );
    let mut blocked = BlockedTargets::default();
    blocked.block_target(&board_target("minion-1"));
    blocked.block_target(&board_target("minion-2"));

    assert!(
        !blocked.has_unblocked_target(&[board_target("minion-1"), board_target("minion-2")]),
        "全部候选被拒后应报告没有可用目标"
    );
    assert!(
        next_intent(&view, &catalog(), 0, &blocked).is_none(),
        "没有合法候选时不应再发意图"
    );
}

#[test]
fn blocked_hand_counts_distinct_cards() {
    let mut blocked = BlockedTargets::default();
    assert_eq!(blocked.hand_count(), 0);

    blocked.block_hand(4);
    blocked.block_hand(4);
    assert_eq!(blocked.hand_count(), 1, "同一张牌重复放弃只记一次");
    assert!(blocked.blocks_hand(4));
}

#[test]
fn retries_rejected_play_with_next_board_minion() {
    // 预填的随从不在目标池内时，沿棋盘换下一个继续尝试，而不是直接放弃这张牌。
    let board = json!([board_minion("minion-1", 0), board_minion("minion-2", 1)]);
    let hand = json!([hand_effect(
        2,
        "2003",
        json!([
            target_json("board", "minion-1"),
            target_json("board", "minion-2")
        ])
    )]);
    let view = with_hand_and_board(
        view(10, json!([]), json!([]), json!([]), json!([]), true),
        hand,
        board,
    );

    let retried = retry_with_target(
        &view,
        &Intent::PlayCard {
            hand_index: 2,
            position: None,
            target: Some(board_target("minion-1")),
        },
    )
    .expect("棋盘上还有下一个随从");

    match retried {
        Intent::PlayCard {
            hand_index,
            position,
            target,
        } => {
            assert_eq!(hand_index, 2, "重试保留原手牌下标");
            assert_eq!(position, None, "换目标不改落点");
            assert_eq!(
                target.as_ref().map(|target| target.id.as_str()),
                Some("minion-2")
            );
        }
        other => panic!("期望 PlayCard，实际 {other:?}"),
    }
}

#[test]
fn retries_from_first_minion_when_target_left_board() {
    // 被拒的目标已不在棋盘上（例如被三连合并）时从头开始，避免越界。
    let board = json!([board_minion("minion-1", 0)]);
    let hand = json!([hand_effect(
        0,
        "2001",
        json!([target_json("board", "minion-1")])
    )]);
    let view = with_hand_and_board(
        view(10, json!([]), json!([]), json!([]), json!([]), true),
        hand,
        board,
    );

    let retried = retry_with_target(
        &view,
        &Intent::PlayCard {
            hand_index: 0,
            position: None,
            target: Some(board_target("minion-gone")),
        },
    )
    .expect("应从第一个随从重新尝试");
    assert!(matches!(
        retried,
        Intent::PlayCard {
            target: Some(ref target),
            ..
        } if target.id == "minion-1"
    ));
}

#[test]
fn stops_retrying_after_last_board_minion() {
    // 棋盘遍历完仍无合法目标时返回 None，由调用方本回合放弃这张牌。
    let board = json!([board_minion("minion-1", 0), board_minion("minion-2", 1)]);
    let hand = json!([hand_effect(
        0,
        "2001",
        json!([
            target_json("board", "minion-1"),
            target_json("board", "minion-2")
        ])
    )]);
    let view = with_hand_and_board(
        view(10, json!([]), json!([]), json!([]), json!([]), true),
        hand,
        board,
    );

    assert!(
        retry_with_target(
            &view,
            &Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: Some(board_target("minion-2")),
            }
        )
        .is_none(),
        "最后一个候选也失败后不再重试"
    );
}

#[test]
fn retry_ignores_non_play_intents() {
    let board = json!([board_minion("minion-1", 0)]);
    let view = with_hand_and_board(
        view(10, json!([]), json!([]), json!([]), json!([]), true),
        json!([]),
        board,
    );

    assert!(
        retry_with_target(
            &view,
            &Intent::ChooseTarget {
                target: board_target("minion-1"),
            }
        )
        .is_none(),
        "目标待选走候选轮换，不沿棋盘重发"
    );
}
