mod common;

use common::{minion_id, place, set_shop, started, test_catalog};
use rule_engine::engine::Engine;
use rule_engine::intent::Intent;
use rule_engine::legal::legal_actions;
use rule_engine::state::{GameState, HandCard, MinionInstance, PendingChoice, Seat};

/// 枚举出的每条动作都必须能被引擎接受；用克隆状态验证，不污染原状态。
fn assert_every_action_accepted(engine: &Engine, state: &GameState, seat: Seat) {
    for intent in legal_actions(engine, state, seat) {
        let mut clone = state.clone();
        let result = engine.apply(&mut clone, seat, intent.clone());
        assert!(
            result.is_ok(),
            "合法动作被引擎拒绝：{intent:?} -> {:?}",
            result.err()
        );
    }
}

#[test]
fn every_legal_action_is_accepted_by_engine() {
    let (engine, state) = started(7);
    let actions = legal_actions(&engine, &state, 0);
    assert!(!actions.is_empty());
    assert_every_action_accepted(&engine, &state, 0);
}

#[test]
fn order_is_stable() {
    let (engine, state) = started(7);
    assert_eq!(
        legal_actions(&engine, &state, 0),
        legal_actions(&engine, &state, 0)
    );
}

#[test]
fn recruit_done_has_no_actions() {
    let (engine, mut state) = started(7);
    state.players[0].recruit_done = true;
    assert!(legal_actions(&engine, &state, 0).is_empty());
}

#[test]
fn effect_card_targets_include_on_use_responses() {
    let (engine, mut state) = started(7);
    let catalog = test_catalog();
    // 岩画师：每使用一张效果牌，使一个友方随从 +0/+1，需要目标。
    place(&catalog, &mut state, 0, 9001, "yanhuashi");
    // aolinpike-jidian 自身（friendly:all）不需要目标，但 OnUse 响应需要。
    state.players[0]
        .hand
        .push(HandCard::effect(9002, "aolinpike-jidian"));

    let actions = legal_actions(&engine, &state, 0);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            Intent::PlayCard {
                target: Some(_),
                ..
            }
        )),
        "岩画师在场时效果牌应带目标枚举出来：{actions:?}"
    );
    assert_every_action_accepted(&engine, &state, 0);
}

#[test]
fn discover_pending_with_full_hand_offers_no_choices() {
    let (engine, mut state) = started(7);
    let catalog = test_catalog();
    let hand_slots = engine.catalog.config.hand_slots;
    state.players[0].hand = (0..hand_slots)
        .map(|index| {
            HandCard::Minion(MinionInstance::new(
                &catalog,
                9_100 + index as u64,
                &minion_id(1),
                false,
            ))
        })
        .collect();
    state.pending.push_back(PendingChoice::Discover {
        seat: 0,
        candidates: vec![minion_id(1), minion_id(2), minion_id(3)],
        source: 0,
    });

    let actions = legal_actions(&engine, &state, 0);
    assert!(
        actions.is_empty(),
        "手牌已满时发现无处落手，不应给出任何动作：{actions:?}"
    );
}

#[test]
fn pending_target_offers_choose_and_abandon_only() {
    let (engine, mut state) = started(7);
    let catalog = test_catalog();
    // 棋盘上先放一个可被指定的友方随从。
    place(&catalog, &mut state, 0, 9001, &minion_id(1));
    // 手牌放一张带目标战吼的随从（bisheng：战吼使一个友方随从成长一次）。
    state.players[0]
        .hand
        .push(HandCard::Minion(MinionInstance::new(
            &catalog, 9002, "bisheng", false,
        )));
    state.next_instance_id = 9003;
    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect("带目标战吼的随从可以入场");
    assert!(
        matches!(state.pending.front(), Some(PendingChoice::Target { .. })),
        "入场后应留下目标待选"
    );

    let actions = legal_actions(&engine, &state, 0);
    assert!(actions.iter().any(|a| matches!(a, Intent::AbandonTarget)));
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Intent::ChooseTarget { .. })),
        "应给出至少一个目标候选"
    );
    assert!(
        actions
            .iter()
            .all(|a| matches!(a, Intent::ChooseTarget { .. } | Intent::AbandonTarget))
    );
    assert_every_action_accepted(&engine, &state, 0);
}

/// 枚举出来的冻结/解冻动作必须真的改变冻结状态。
///
/// 冻结已经冻结的槽位、解冻没有任何冻结的商店都不改变状态：这类空操作留在候选里，
/// 策略就能在一个回合内无限重复同一条动作（训练侧表现为顶满回合决策上限的死循环）。
#[test]
fn freeze_and_unfreeze_actions_change_state() {
    let (engine, mut state) = started(7);
    set_shop(&mut state, 0, &[Some(minion_id(1)), Some(minion_id(2))]);

    engine
        .apply(&mut state, 0, Intent::FreezeShop { slot: Some(0) })
        .expect("冻结 0 号应成功");
    let actions = legal_actions(&engine, &state, 0);
    assert!(
        !actions
            .iter()
            .any(|action| matches!(action, Intent::FreezeShop { slot: Some(0) })),
        "已冻结的槽位不应再出现在冻结候选里"
    );
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, Intent::FreezeShop { slot: Some(1) })),
        "未冻结的槽位仍应可冻结"
    );

    engine
        .apply(&mut state, 0, Intent::FreezeShop { slot: None })
        .expect("冻结全部应成功");
    let actions = legal_actions(&engine, &state, 0);
    assert!(
        !actions
            .iter()
            .any(|action| matches!(action, Intent::FreezeShop { .. })),
        "全部冻结后不应再有冻结候选"
    );
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, Intent::UnfreezeShop { slot: None })),
        "本回合冻结过，解冻全部应可用"
    );

    engine
        .apply(&mut state, 0, Intent::UnfreezeShop { slot: None })
        .expect("解冻全部应成功");
    let actions = legal_actions(&engine, &state, 0);
    assert!(
        !actions
            .iter()
            .any(|action| matches!(action, Intent::UnfreezeShop { .. })),
        "没有冻结槽位时不应再给出解冻候选"
    );
    assert!(
        actions
            .iter()
            .any(|action| matches!(action, Intent::FreezeShop { slot: None })),
        "解冻后应能重新冻结"
    );
}
