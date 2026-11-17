mod common;

use common::{heroes, test_catalog, test_playstyles};
use rule_engine::engine::Engine;
use rule_engine::hash::state_hash;
use rule_engine::rng::{RngStreams, Stream};

#[test]
fn same_seed_same_stream_sequence() {
    let mut a = RngStreams::from_seed([7; 32]);
    let mut b = RngStreams::from_seed([7; 32]);
    for _ in 0..100 {
        assert_eq!(a.next_u64(Stream::Shop), b.next_u64(Stream::Shop));
    }
}

#[test]
fn streams_are_independent() {
    let mut a = RngStreams::from_seed([7; 32]);
    let mut b = RngStreams::from_seed([7; 32]);
    let _ = a.next_u64(Stream::Shop);
    assert_eq!(a.next_u64(Stream::Combat), b.next_u64(Stream::Combat));
}

#[test]
fn same_seed_same_initial_hash_and_turn() {
    let (mut e1, mut s1) = Engine::new(test_catalog(), [9; 32], test_playstyles());
    let (mut e2, mut s2) = Engine::new(test_catalog(), [9; 32], test_playstyles());
    assert_eq!(state_hash(&s1), state_hash(&s2));
    e1.start_game(&mut s1, &heroes()).unwrap();
    e2.start_game(&mut s2, &heroes()).unwrap();
    e1.start_turn(&mut s1).unwrap();
    e2.start_turn(&mut s2).unwrap();
    assert_eq!(state_hash(&s1), state_hash(&s2));
}

#[test]
fn hash_changes_when_state_changes() {
    let (_, mut s) = Engine::new(test_catalog(), [9; 32], test_playstyles());
    let before = state_hash(&s);
    s.players[0].hp -= 1;
    assert_ne!(before, state_hash(&s));
}

#[test]
fn start_game_rejects_missing_seat() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [9; 32], test_playstyles());
    let mut heroes = heroes();
    heroes.remove(&7);
    assert!(engine.start_game(&mut state, &heroes).is_err());
}

#[test]
fn start_game_rejects_non_hero_card() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [9; 32], test_playstyles());
    let mut heroes = heroes();
    let non_hero = test_catalog()
        .cards
        .values()
        .find(|card| !card.is_hero)
        .expect("数据中应存在非英雄卡")
        .id
        .clone();
    heroes.insert(3, non_hero);
    assert!(engine.start_game(&mut state, &heroes).is_err());
}

#[test]
fn start_game_rejects_unknown_card_id() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [9; 32], test_playstyles());
    let mut heroes = heroes();
    heroes.insert(3, "does-not-exist".to_string());
    assert!(engine.start_game(&mut state, &heroes).is_err());
}

#[test]
fn start_game_allows_duplicate_heroes() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [9; 32], test_playstyles());
    let mut heroes = heroes();
    let first = heroes.get(&0).expect("座位 0 应有英雄").clone();
    heroes.insert(1, first.clone());
    engine
        .start_game(&mut state, &heroes)
        .expect("多人局允许重复选择同一张英雄卡");
    assert_eq!(state.players[0].hero.as_deref(), Some(first.as_str()));
    assert_eq!(state.players[1].hero.as_deref(), Some(first.as_str()));
    assert_eq!(state.phase, rule_engine::state::Phase::Recruit);
}

#[test]
fn start_game_rejects_truncated_players_without_mutating_state() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [9; 32], test_playstyles());
    state.players.pop();
    let before = state_hash(&state);
    assert!(engine.start_game(&mut state, &heroes()).is_err());
    assert_eq!(before, state_hash(&state));
}
