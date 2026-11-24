//! `BoardMinionView.stats` 分层构成的集成测试。
//!
//! 覆盖：各层与 `current` 的自洽、攻击力为 0 的随从分层照常累计、受击差额进入 `damage`、
//! 招募期负血量被夹到 0 后由 `damage` 吸收，以及光环折叠后 `aura` 与当前量一致。

mod common;

use common::started;
use rule_engine::data::Catalog;
use rule_engine::engine::Engine;
use rule_engine::state::{
    AuraEffect, AuraSource, GameState, MinionInstance, StatSourceLayer, StatSourceOrigin,
    StatSystemSource,
};
use rule_engine::view::{BoardMinionView, view};

/// 卡面 3/5 的普通随从。
const FOUR: &str = "jianchihu";
/// 卡面 0/6、攻击力为 0 的随从。
const SCHOLAR: &str = "euclid";

/// 断言四项构成与 `damage`、`current` 自洽。
fn assert_consistent(layers: &[i32], damage: i32, current: i32) {
    let sum: i32 = layers.iter().sum();
    assert_eq!(
        sum - damage,
        current,
        "initial + permanent + turn + aura - damage 应等于 current"
    );
}

fn first_board_minion(engine: &Engine, state: &GameState) -> BoardMinionView {
    view(engine, state, 0)
        .expect("座位 0 的视图应存在")
        .self_view
        .board
        .into_iter()
        .next()
        .expect("棋盘上应有随从")
}

fn catalog(engine: &Engine) -> &Catalog {
    &engine.catalog
}

/// 直接改分层数值的测试夹具必须同步来源账本，否则视图的调试断言不成立。
fn record(minion: &mut MinionInstance, layer: StatSourceLayer, attack: i32, health: i32) {
    if attack == 0 && health == 0 {
        return;
    }
    minion.record_stat_source(
        layer,
        StatSourceOrigin::System(StatSystemSource::Unknown),
        attack,
        health,
    );
}

#[test]
fn layers_and_current_agree_without_damage() {
    let (engine, mut state) = started(7);
    let mut minion = MinionInstance::new(catalog(&engine), 1, FOUR, false);
    minion.stats.attack.add_growth(1);
    minion.stats.attack.add_permanent(2);
    minion.stats.attack.add_turn(3);
    minion.stats.health.add_growth(1);
    minion.stats.health.add_permanent(2);
    minion.stats.health.add_turn(-1);
    record(&mut minion, StatSourceLayer::Growth, 1, 1);
    record(&mut minion, StatSourceLayer::Permanent, 2, 2);
    record(&mut minion, StatSourceLayer::Turn, 3, -1);
    state.players[0].board.push(minion);

    let stats = first_board_minion(&engine, &state).stats;

    // 攻击：3 基础 + 1 成长 + 2 永久 + 3 回合。
    assert_eq!(stats.attack.initial, 3);
    assert_eq!(stats.attack.permanent, 3);
    assert_eq!(stats.attack.turn, 3);
    assert_eq!(stats.attack.aura, 0);
    assert_eq!(stats.attack.damage, 0);
    assert_eq!(stats.attack.current, 9);
    assert_consistent(
        &[
            stats.attack.initial,
            stats.attack.permanent,
            stats.attack.turn,
            stats.attack.aura,
        ],
        stats.attack.damage,
        stats.attack.current,
    );

    // 生命：5 基础 + 1 成长 + 2 永久 - 1 回合。
    assert_eq!(stats.health.initial, 5);
    assert_eq!(stats.health.permanent, 3);
    assert_eq!(stats.health.turn, -1);
    assert_eq!(stats.health.aura, 0);
    assert_eq!(stats.health.damage, 0);
    assert_eq!(stats.health.current, 7);
    assert_consistent(
        &[
            stats.health.initial,
            stats.health.permanent,
            stats.health.turn,
            stats.health.aura,
        ],
        stats.health.damage,
        stats.health.current,
    );
}

#[test]
fn zero_attack_layers_accumulate_normally() {
    let (engine, mut state) = started(7);
    let mut minion = MinionInstance::new(catalog(&engine), 1, SCHOLAR, false);
    minion.stats.attack.add_permanent(4);
    minion.stats.health.add_growth(2);
    record(&mut minion, StatSourceLayer::Permanent, 4, 0);
    record(&mut minion, StatSourceLayer::Growth, 0, 2);
    state.players[0].board.push(minion);

    let stats = first_board_minion(&engine, &state).stats;

    assert_eq!(
        stats.attack,
        rule_engine::view::StatBreakdownView {
            initial: 0,
            permanent: 4,
            turn: 0,
            aura: 0,
            damage: 0,
            current: 4,
        },
        "攻击力为 0 的随从分层照常累计"
    );
    assert_eq!(stats.health.initial, 6);
    assert_eq!(stats.health.permanent, 2);
    assert_eq!(stats.health.current, 8);
}

#[test]
fn damage_is_absorbed_into_breakdown() {
    let (engine, mut state) = started(7);
    let mut minion = MinionInstance::new(catalog(&engine), 1, FOUR, false);
    minion.stats.health.add_permanent(1);
    minion.stats.health.apply_damage(3);
    record(&mut minion, StatSourceLayer::Permanent, 0, 1);
    state.players[0].board.push(minion);

    let stats = first_board_minion(&engine, &state).stats;

    assert_eq!(stats.health.damage, 3);
    assert_eq!(stats.health.current, 3);
    assert_consistent(
        &[
            stats.health.initial,
            stats.health.permanent,
            stats.health.turn,
            stats.health.aura,
        ],
        stats.health.damage,
        stats.health.current,
    );
}

#[test]
fn negative_current_is_clamped_and_absorbed_by_damage() {
    let (engine, mut state) = started(7);
    let mut minion = MinionInstance::new(catalog(&engine), 1, FOUR, false);
    minion.stats.health.apply_damage(7);
    assert_eq!(minion.stats.health.current, -2);
    state.players[0].board.push(minion);

    let stats = first_board_minion(&engine, &state).stats;

    assert_eq!(stats.health.current, 0, "协议下限把负当前量夹到 0");
    assert_eq!(
        stats.health.damage, 5,
        "层和（5）到夹紧后当前量（0）的差额计入受击"
    );
    assert_consistent(
        &[
            stats.health.initial,
            stats.health.permanent,
            stats.health.turn,
            stats.health.aura,
        ],
        stats.health.damage,
        stats.health.current,
    );
}

#[test]
fn folded_aura_is_reflected_in_breakdown() {
    let (engine, mut state) = started(7);
    state.players[0]
        .board
        .push(MinionInstance::new(catalog(&engine), 1, FOUR, false));
    state.auras.insert(
        1,
        AuraEffect {
            attack: 2,
            health: 1,
            keywords: Default::default(),
            floor_one: false,
        },
    );
    state.aura_sources.insert(
        1,
        vec![AuraSource {
            card_id: FOUR.to_string(),
            attack: 2,
            health: 1,
        }],
    );

    let stats = first_board_minion(&engine, &state).stats;

    assert_eq!(stats.attack.aura, 2);
    assert_eq!(stats.attack.current, 5);
    assert_eq!(stats.health.aura, 1);
    assert_eq!(stats.health.current, 6);
    assert_consistent(
        &[
            stats.attack.initial,
            stats.attack.permanent,
            stats.attack.turn,
            stats.attack.aura,
        ],
        stats.attack.damage,
        stats.attack.current,
    );
    assert_consistent(
        &[
            stats.health.initial,
            stats.health.permanent,
            stats.health.turn,
            stats.health.aura,
        ],
        stats.health.damage,
        stats.health.current,
    );
}
