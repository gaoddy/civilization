mod common;

use common::{heroes, test_catalog};
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
    let (mut e1, mut s1) = Engine::new(test_catalog(), [9; 32]);
    let (mut e2, mut s2) = Engine::new(test_catalog(), [9; 32]);
    assert_eq!(state_hash(&s1), state_hash(&s2));
    e1.start_game(&mut s1, &heroes()).unwrap();
    e2.start_game(&mut s2, &heroes()).unwrap();
    e1.start_turn(&mut s1).unwrap();
    e2.start_turn(&mut s2).unwrap();
    assert_eq!(state_hash(&s1), state_hash(&s2));
}

#[test]
fn hash_changes_when_state_changes() {
    let (_, mut s) = Engine::new(test_catalog(), [9; 32]);
    let before = state_hash(&s);
    s.players[0].hp -= 1;
    assert_ne!(before, state_hash(&s));
}
