//! 棋盘随从主动技能的视图：技能列表、费用、次数、目标候选与门控。

mod common;

use common::{place, setup_with_gold, test_catalog};
use rule_engine::effects::{EffectCtx, Trigger};
use rule_engine::intent::RejectCode;
use rule_engine::state::GameState;
use rule_engine::view::view;

const GAUSS_SKILL: &str = "activate-friendly-growth-twice";
const BOXER_SHOUT: &str = "shout";

/// 座位 0：高斯（实例 100）+ 冰箱（实例 101，无技能）。
fn place_gauss_and_fridge(state: &mut GameState) {
    let catalog = test_catalog();
    place(&catalog, state, 0, 100, "gauss");
    place(&catalog, state, 0, 101, "einstein-fridge");
}

#[test]
fn exposes_active_ability_with_cost_and_uses() {
    let (engine, mut state) = setup_with_gold(221);
    place_gauss_and_fridge(&mut state);

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    let abilities = &seat_view.self_view.board[0].abilities;
    let ability = abilities.first().expect("高斯应下发主动技能");

    assert_eq!(ability.skill_id, GAUSS_SKILL);
    assert_eq!(ability.cost, 1, "费用取技能 cost");
    assert_eq!(ability.uses_left, 1, "uses_per_turn 缺省与本回合未使用");
    assert!(ability.requires_target, "成长需要指定友方目标");
    assert!(ability.can_use);
    assert_eq!(ability.disabled_reason, None);
}

#[test]
fn target_candidates_match_engine_pool() {
    let (engine, mut state) = setup_with_gold(222);
    place_gauss_and_fridge(&mut state);

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    let ability = &seat_view.self_view.board[0].abilities[0];

    let skill = engine
        .catalog
        .skill_by_name(GAUSS_SKILL)
        .expect("技能应在目录中");
    let effects = skill
        .effects
        .iter()
        .filter(|effect| effect.trigger == Trigger::OnActivate)
        .cloned()
        .collect::<Vec<_>>();
    let ctx = EffectCtx {
        seat: 0,
        source: Some(100),
        target: None,
        event_target: None,
        golden: false,
        level_override: None,
    };
    let expected = engine.target_candidates(&state, &effects, &ctx, false);

    assert!(!expected.is_empty(), "应存在合法友方目标");
    assert_eq!(
        ability.target_candidates, expected,
        "视图候选与引擎对同一随从、同一技能的预检结果一致"
    );
}

#[test]
fn filters_passive_skills_and_reads_uses_per_turn() {
    let (engine, mut state) = setup_with_gold(223);
    let catalog = test_catalog();
    place(&catalog, &mut state, 0, 100, "boxer");

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    let abilities = &seat_view.self_view.board[0].abilities;

    assert_eq!(
        abilities
            .iter()
            .map(|ability| (ability.skill_id.as_str(), ability.cost, ability.uses_left))
            .collect::<Vec<_>>(),
        vec![(BOXER_SHOUT, 1, 2)],
        "只下发带 cost 的主动技能；被动技能被过滤；次数取 uses_per_turn"
    );
}

#[test]
fn exhausted_uses_disable_ability() {
    let (engine, mut state) = setup_with_gold(224);
    place_gauss_and_fridge(&mut state);
    state.players[0]
        .minion_uses
        .insert((100, GAUSS_SKILL.to_string()), 1);

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    let ability = &seat_view.self_view.board[0].abilities[0];

    assert_eq!(ability.uses_left, 0);
    assert!(!ability.can_use);
    assert_eq!(ability.disabled_reason, Some(RejectCode::HeroPowerLimit));
}

#[test]
fn insufficient_gold_disables_ability() {
    let (engine, mut state) = setup_with_gold(225);
    place_gauss_and_fridge(&mut state);
    state.players[0].gold = 0;

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    let ability = &seat_view.self_view.board[0].abilities[0];

    assert_eq!(ability.uses_left, 1, "次数未用尽");
    assert!(!ability.can_use);
    assert_eq!(ability.disabled_reason, Some(RejectCode::NotEnoughGold));
}

#[test]
fn minion_without_active_skills_has_empty_abilities() {
    let (engine, mut state) = setup_with_gold(226);
    place_gauss_and_fridge(&mut state);

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    assert!(
        seat_view.self_view.board[1].abilities.is_empty(),
        "无技能随从不下发技能"
    );
}

#[test]
fn target_skill_without_candidates_is_no_valid_target() {
    let (engine, mut state) = setup_with_gold(227);
    let catalog = test_catalog();
    // 达·芬奇的手稿只能复制手牌中的随从；空手牌时没有合法落点（含自身）。
    place(&catalog, &mut state, 0, 100, "davinci");

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    let ability = &seat_view.self_view.board[0].abilities[0];

    assert!(ability.requires_target);
    assert!(ability.target_candidates.is_empty());
    assert!(!ability.can_use);
    assert_eq!(ability.disabled_reason, Some(RejectCode::NoValidTarget));
}

#[test]
fn recruit_done_disables_abilities() {
    let (engine, mut state) = setup_with_gold(228);
    place_gauss_and_fridge(&mut state);
    state.players[0].recruit_done = true;

    let seat_view = view(&engine, &state, 0).expect("座位 0 的视图应存在");
    let ability = &seat_view.self_view.board[0].abilities[0];

    assert!(!ability.can_use);
    assert_eq!(ability.disabled_reason, Some(RejectCode::InvalidStage));
}
