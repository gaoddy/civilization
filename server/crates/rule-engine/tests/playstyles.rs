//! 开局玩法抽取与 `Engine::new` 的玩法参数。

mod common;

use std::collections::BTreeSet;

use common::test_catalog;
use rule_engine::data::Playstyle;
use rule_engine::engine::Engine;
use rule_engine::playstyles::draw;
use rule_engine::rng::{RngStreams, Stream};

#[test]
fn same_seed_draws_same_five_playstyles() {
    assert_eq!(draw(7), draw(7));
    assert_eq!(draw(0), draw(0));
    assert_ne!(draw(7), draw(8), "不同种子应产生不同组合");
}

#[test]
fn draw_is_five_distinct_non_neutral_sorted_playstyles() {
    for seed in [0, 1, 7, 42, u64::MAX] {
        let picked = draw(seed);
        assert_eq!(picked.len(), 5);
        assert!(!picked.contains(&Playstyle::Neutral), "不能抽到 Neutral");
        let unique: BTreeSet<Playstyle> = picked.iter().copied().collect();
        assert_eq!(unique.len(), 5, "五个玩法互不相同");
        let mut sorted = picked;
        sorted.sort();
        assert_eq!(picked, sorted, "结果按枚举顺序排序");
    }
}

#[test]
fn engine_state_uses_passed_playstyles() {
    let playstyles = [
        Playstyle::Mammal,
        Playstyle::Bird,
        Playstyle::Reptile,
        Playstyle::Fish,
        Playstyle::Insect,
    ];
    let (_, state) = Engine::new(test_catalog(), [9; 32], playstyles);
    let expected: BTreeSet<Playstyle> = playstyles.into_iter().collect();
    assert_eq!(state.enabled_playstyles, expected);

    // 玩法抽取不消费引擎随机流：`Engine::new` 只消耗首座位抽取。
    let mut fresh = RngStreams::from_seed([9; 32]);
    let expected_first = fresh.below(Stream::Matchmaking, 8) as u8;
    assert_eq!(state.first_seat, expected_first);
    assert_eq!(state.rng, fresh);
}

#[test]
#[should_panic(expected = "enabled_playstyles 不能包含 Neutral")]
fn engine_rejects_neutral_playstyle() {
    let playstyles = [
        Playstyle::Neutral,
        Playstyle::Bird,
        Playstyle::Reptile,
        Playstyle::Fish,
        Playstyle::Insect,
    ];
    let _ = Engine::new(test_catalog(), [1; 32], playstyles);
}

#[test]
#[should_panic(expected = "enabled_playstyles 不能包含重复玩法")]
fn engine_rejects_duplicate_playstyle() {
    let playstyles = [
        Playstyle::Mammal,
        Playstyle::Mammal,
        Playstyle::Reptile,
        Playstyle::Fish,
        Playstyle::Insect,
    ];
    let _ = Engine::new(test_catalog(), [1; 32], playstyles);
}
