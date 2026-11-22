mod common;

use common::{place, setup};
use rule_engine::state::{StatSource, StatSourceLayer, StatSourceOrigin, StatSystemSource};

fn card_origin(card_id: &str) -> StatSourceOrigin {
    StatSourceOrigin::Card {
        card_id: Some(card_id.to_string()),
        skill_id: None,
    }
}

#[test]
fn sources_merge_by_layer_and_origin() {
    let (engine, mut state) = setup(3);
    place(&engine.catalog, &mut state, 0, 1, "jianchihu");
    let minion = &mut state.players[0].board[0];

    minion.record_stat_source(StatSourceLayer::Permanent, card_origin("a"), 1, 2);
    minion.record_stat_source(StatSourceLayer::Permanent, card_origin("a"), 1, 1);
    minion.record_stat_source(StatSourceLayer::Permanent, card_origin("b"), 0, 3);

    assert_eq!(minion.stat_sources.len(), 2);
    let a = minion
        .stat_sources
        .iter()
        .find(|s| s.origin == card_origin("a"))
        .unwrap();
    assert_eq!((a.attack, a.health), (2, 3));
}

#[test]
fn growth_sources_removed_independently() {
    let (engine, mut state) = setup(3);
    place(&engine.catalog, &mut state, 0, 1, "jianchihu");
    let minion = &mut state.players[0].board[0];

    minion.record_stat_source(StatSourceLayer::Growth, card_origin("g"), 1, 1);
    minion.record_stat_source(StatSourceLayer::Permanent, card_origin("p"), 2, 2);
    minion.remove_growth_sources();

    assert_eq!(minion.stat_sources.len(), 1);
    assert_eq!(minion.stat_sources[0].layer, StatSourceLayer::Permanent);
}

#[test]
fn turn_and_aura_sources_clear() {
    let (engine, mut state) = setup(3);
    place(&engine.catalog, &mut state, 0, 1, "jianchihu");
    let minion = &mut state.players[0].board[0];

    minion.record_stat_source(StatSourceLayer::Turn, card_origin("t"), 1, 1);
    minion.record_stat_source(
        StatSourceLayer::Aura,
        StatSourceOrigin::System(StatSystemSource::AuraFloor),
        0,
        1,
    );
    minion.clear_turn_sources();
    minion.replace_aura_sources(vec![StatSource {
        layer: StatSourceLayer::Aura,
        origin: card_origin("aura"),
        attack: 1,
        health: 0,
    }]);

    assert_eq!(minion.stat_sources.len(), 1);
    assert_eq!(minion.stat_sources[0].layer, StatSourceLayer::Aura);
}
