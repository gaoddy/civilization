//! 随从主动技能：次数、费用、目标与来源。

mod common;

use common::{place, setup_with_gold, test_catalog, turn_started};
use rule_engine::effects::{TargetRef, Zone};
use rule_engine::hash::state_hash;
use rule_engine::intent::{Intent, RejectCode};
use rule_engine::state::{GameState, InstanceId, StatSourceOrigin};

const GAUSS_SKILL: &str = "activate-friendly-growth-twice";
const BOXER_PASSIVE: &str = "trigger-3-humans-self-plus-2-attack";

fn board_target(id: InstanceId) -> TargetRef {
    TargetRef {
        zone: Zone::Board,
        id,
    }
}

fn use_ability(minion: InstanceId, skill_id: &str, target: Option<TargetRef>) -> Intent {
    Intent::UseMinionAbility {
        minion,
        skill_id: skill_id.to_string(),
        target,
    }
}

/// 座位 0：高斯（实例 100）+ 冰箱（实例 101，无技能，作为目标）。
fn board_with_gauss(state: &mut GameState) {
    let catalog = test_catalog();
    place(&catalog, state, 0, 100, "gauss");
    place(&catalog, state, 0, 101, "einstein-fridge");
}

#[test]
fn activates_minion_ability_and_deducts_cost() {
    let (engine, mut state) = setup_with_gold(201);
    board_with_gauss(&mut state);
    state.players[0].gold = 5;

    let outcome = engine
        .apply(
            &mut state,
            0,
            use_ability(100, GAUSS_SKILL, Some(board_target(101))),
        )
        .expect("随从主动技能应能发动");

    assert!(outcome.events.is_empty(), "成长不产生事件");
    assert_eq!(state.players[0].gold, 4, "扣除技能费用 1");
    assert_eq!(
        state.players[0]
            .minion_uses
            .get(&(100, GAUSS_SKILL.to_string())),
        Some(&1),
        "该随从该技能本回合次数 +1"
    );
    let target = &state.players[0].board[1];
    assert_eq!(
        (target.stats.attack.growth, target.stats.health.growth),
        (2, 2),
        "成长两次施加到选定目标"
    );
}

#[test]
fn effect_source_is_the_minion_instance() {
    let (engine, mut state) = setup_with_gold(202);
    board_with_gauss(&mut state);
    state.players[0].gold = 5;

    engine
        .apply(
            &mut state,
            0,
            use_ability(100, GAUSS_SKILL, Some(board_target(101))),
        )
        .expect("随从主动技能应能发动");

    let skill_id = engine
        .catalog
        .skill_id(GAUSS_SKILL)
        .expect("技能应在目录中");
    let target = &state.players[0].board[1];
    assert!(
        target.stat_sources.iter().any(|source| source.origin
            == StatSourceOrigin::Card {
                card_id: Some("gauss".to_string()),
                skill_id: Some(skill_id),
            }),
        "成长来源应归因到发动技能的随从卡"
    );
}

#[test]
fn rejects_second_use_past_limit_without_changing_state() {
    let (engine, mut state) = setup_with_gold(203);
    board_with_gauss(&mut state);
    state.players[0].gold = 5;

    engine
        .apply(
            &mut state,
            0,
            use_ability(100, GAUSS_SKILL, Some(board_target(101))),
        )
        .expect("首次发动应成功");

    let before = state_hash(&state);
    let reject = engine
        .apply(
            &mut state,
            0,
            use_ability(100, GAUSS_SKILL, Some(board_target(101))),
        )
        .expect_err("超过 uses_per_turn 应被拒绝");

    assert_eq!(reject.code, RejectCode::HeroPowerLimit);
    assert_eq!(before, state_hash(&state), "拒绝时状态不变");
    assert_eq!(state.players[0].gold, 4, "拒绝时不扣金币");
    assert_eq!(
        state.players[0]
            .minion_uses
            .get(&(100, GAUSS_SKILL.to_string())),
        Some(&1),
        "拒绝时不增加次数"
    );
}

#[test]
fn rejects_when_gold_insufficient() {
    let (engine, mut state) = setup_with_gold(204);
    board_with_gauss(&mut state);
    state.players[0].gold = 0;

    let before = state_hash(&state);
    let reject = engine
        .apply(
            &mut state,
            0,
            use_ability(100, GAUSS_SKILL, Some(board_target(101))),
        )
        .expect_err("金币不足应被拒绝");

    assert_eq!(reject.code, RejectCode::NotEnoughGold);
    assert_eq!(before, state_hash(&state), "拒绝时状态不变");
    assert!(state.players[0].minion_uses.is_empty());
}

#[test]
fn rejects_skill_not_owned_by_minion_card() {
    let (engine, mut state) = setup_with_gold(205);
    let catalog = test_catalog();
    place(&catalog, &mut state, 0, 100, "boxer");
    state.players[0].gold = 5;

    let before = state_hash(&state);
    let reject = engine
        .apply(
            &mut state,
            0,
            use_ability(100, GAUSS_SKILL, Some(board_target(100))),
        )
        .expect_err("技能不属于该随从卡应被拒绝");

    assert_eq!(reject.code, RejectCode::HeroPowerLimit);
    assert_eq!(before, state_hash(&state), "拒绝时状态不变");
}

#[test]
fn rejects_passive_skill() {
    let (engine, mut state) = setup_with_gold(206);
    let catalog = test_catalog();
    place(&catalog, &mut state, 0, 100, "boxer");
    state.players[0].gold = 5;

    let before = state_hash(&state);
    let reject = engine
        .apply(&mut state, 0, use_ability(100, BOXER_PASSIVE, None))
        .expect_err("被动技能不能主动发动");

    assert_eq!(reject.code, RejectCode::NotActiveSkill);
    assert_eq!(before, state_hash(&state), "拒绝时状态不变");
}

#[test]
fn rejects_target_not_on_board() {
    let (engine, mut state) = setup_with_gold(207);
    board_with_gauss(&mut state);
    state.players[0].gold = 5;

    let before = state_hash(&state);
    let reject = engine
        .apply(
            &mut state,
            0,
            use_ability(100, GAUSS_SKILL, Some(board_target(9_999))),
        )
        .expect_err("目标不在本人棋盘应被拒绝");

    assert_eq!(reject.code, RejectCode::NoValidTarget);
    assert_eq!(before, state_hash(&state), "拒绝时状态不变");
}

#[test]
fn rejects_target_on_other_board() {
    let (engine, mut state) = setup_with_gold(208);
    let catalog = test_catalog();
    place(&catalog, &mut state, 0, 100, "gauss");
    place(&catalog, &mut state, 1, 500, "einstein-fridge");
    state.players[0].gold = 5;

    let before = state_hash(&state);
    let reject = engine
        .apply(
            &mut state,
            0,
            use_ability(100, GAUSS_SKILL, Some(board_target(500))),
        )
        .expect_err("目标属于其他玩家应被拒绝");

    assert_eq!(reject.code, RejectCode::TargetNotAllowed);
    assert_eq!(before, state_hash(&state), "拒绝时状态不变");
}

#[test]
fn rejects_target_outside_effect_pool() {
    let (engine, mut state) = setup_with_gold(209);
    let catalog = test_catalog();
    // 拉马克的技能只对友方动物生效；冰箱是机械，区域合法但不在效果池内。
    place(&catalog, &mut state, 0, 100, "lamarck");
    place(&catalog, &mut state, 0, 101, "einstein-fridge");
    state.players[0].gold = 5;

    let before = state_hash(&state);
    let reject = engine
        .apply(
            &mut state,
            0,
            use_ability(100, "biology-evolve", Some(board_target(101))),
        )
        .expect_err("目标不在技能效果池内应被拒绝");

    assert_eq!(reject.code, RejectCode::NoValidTarget);
    assert_eq!(before, state_hash(&state), "拒绝时状态不变");
}

#[test]
fn resets_uses_on_next_recruit_turn() {
    let (engine, mut state) = turn_started(210);
    let catalog = test_catalog();
    place(&catalog, &mut state, 0, 100, "gauss");
    place(&catalog, &mut state, 0, 101, "einstein-fridge");
    state.players[0].gold = 5;

    engine
        .apply(
            &mut state,
            0,
            use_ability(100, GAUSS_SKILL, Some(board_target(101))),
        )
        .expect("本回合首次发动应成功");
    assert_eq!(
        state.players[0]
            .minion_uses
            .get(&(100, GAUSS_SKILL.to_string())),
        Some(&1)
    );

    engine.start_turn(&mut state).expect("开始下一回合");
    assert!(
        state.players[0].minion_uses.is_empty(),
        "进入新招募回合后随从技能次数清零"
    );

    state.players[0].gold = 5;
    engine
        .apply(
            &mut state,
            0,
            use_ability(100, GAUSS_SKILL, Some(board_target(101))),
        )
        .expect("新回合应能再次发动");
    assert_eq!(
        state.players[0]
            .minion_uses
            .get(&(100, GAUSS_SKILL.to_string())),
        Some(&1)
    );
}

#[test]
fn multiple_minions_track_uses_independently() {
    let (engine, mut state) = setup_with_gold(211);
    let catalog = test_catalog();
    place(&catalog, &mut state, 0, 100, "gauss");
    place(&catalog, &mut state, 0, 101, "gauss");
    state.players[0].gold = 5;

    engine
        .apply(
            &mut state,
            0,
            use_ability(100, GAUSS_SKILL, Some(board_target(101))),
        )
        .expect("第一个随从发动应成功");
    engine
        .apply(
            &mut state,
            0,
            use_ability(101, GAUSS_SKILL, Some(board_target(100))),
        )
        .expect("第二个随从应独立计数");

    assert_eq!(
        state.players[0]
            .minion_uses
            .get(&(100, GAUSS_SKILL.to_string())),
        Some(&1)
    );
    assert_eq!(
        state.players[0]
            .minion_uses
            .get(&(101, GAUSS_SKILL.to_string())),
        Some(&1)
    );
}
