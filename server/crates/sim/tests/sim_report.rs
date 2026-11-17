//! 结束存活人数采样：用真实引擎状态走完全局，断言 `stats.survivors_at_end`
//! 等于 `GameOver` 时刻的真实存活人数。

mod common;

use common::catalog;
use rule_engine::engine::Engine;
use rule_engine::state::{GameState, Phase};

fn alive(state: &GameState) -> u8 {
    state
        .players
        .iter()
        .filter(|player| !player.eliminated)
        .count() as u8
}

#[test]
fn survivors_at_end_matches_real_survivors_of_each_game() {
    // seed 1..=8 覆盖多种终局；每局独立重建，断言采样值 = GameOver 时的真实存活数，
    // 且至少有一局结束时仍有 2 名及以上存活者（能发现「用 0 当哨兵」的缺陷）。
    let mut seen = std::collections::BTreeSet::new();
    for seed in 1..=8u64 {
        let (mut engine, mut state) = Engine::new(
            catalog(),
            sim_seed(seed),
            rule_engine::playstyles::draw(seed),
        );
        let log = sim::play_game(&mut engine, &mut state, seed, 8).expect("单局应能完成");
        assert_eq!(state.phase, Phase::GameOver);
        assert_eq!(
            log.stats.survivors_at_end,
            alive(&state),
            "种子 {seed}：survivors_at_end 应等于 GameOver 时刻的存活人数"
        );
        seen.insert(log.stats.survivors_at_end);
    }
    assert!(
        seen.iter().any(|survivors| *survivors >= 2),
        "样本中应至少有一局结束时仍有 2 名及以上存活者，实际 {seen:?}"
    );
}

#[test]
fn survivors_at_end_keeps_nonzero_value() {
    // 构造「结束时仍有 1 名以上存活者」的 GameOver 状态：只有座位 0 存活。
    // 采样只依赖 `GameState`，确认实现不会把真实存活数覆盖成 0 或停留在默认值。
    let (_, mut state) = Engine::new(catalog(), sim_seed(1), rule_engine::playstyles::draw(1));
    for player in state.players.iter_mut().skip(1) {
        player.eliminated = true;
    }
    state.phase = Phase::GameOver;
    assert_eq!(sim::alive_players(&state), 1);
    for player in state.players.iter_mut().skip(2) {
        player.eliminated = false;
    }
    assert_eq!(sim::alive_players(&state), 7);
}

fn sim_seed(seed: u64) -> [u8; 32] {
    sim::seed_from_u64(seed)
}
