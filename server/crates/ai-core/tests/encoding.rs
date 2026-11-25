use std::collections::BTreeMap;

use ai_core::{Action, BOARD_SLOTS, CardIndex, Encoder, History, OBS_DIM, SEATS, SHOP_SLOTS};
use rule_engine::data::Catalog;
use rule_engine::engine::Engine;
use rule_engine::legal::legal_actions;
use rule_engine::playstyles;
use rule_engine::state::GameState;
use rule_engine::view::view;

fn catalog() -> Catalog {
    Catalog::from_json(
        include_str!("../../../data/v0.1/cards.json"),
        include_str!("../../../data/v0.1/heroes.json"),
        include_str!("../../../data/v0.1/skills.json"),
        include_str!("../../../data/v0.1/config.json"),
        include_str!("../../../data/v0.1/taxonomy.json"),
    )
    .expect("卡表")
}

/// 开局并进入第一回合招募的固定夹具。
fn started() -> (Engine, GameState, Catalog) {
    let catalog = catalog();
    let (engine, mut state) = Engine::new(catalog.clone(), [7; 32], playstyles::draw(7));
    let heroes: BTreeMap<u8, String> = catalog
        .heroes
        .values()
        .take(8)
        .enumerate()
        .map(|(index, hero)| (index as u8, hero.id.clone()))
        .collect();
    engine.start_game(&mut state, &heroes).expect("开局");
    (engine, state, catalog)
}

fn fnv(values: &[f32]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for value in values {
        hash ^= u64::from(value.to_bits());
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[test]
fn card_index_is_stable_and_complete() {
    let catalog = catalog();
    let index = CardIndex::build(&catalog);
    assert!(index.card("bisheng") >= 0);
    assert_eq!(index.card("bisheng"), index.card("bisheng"));
    assert_eq!(index.card("not-a-card"), -1);
    assert_eq!(index.card_count(), catalog.cards.len());
    assert_eq!(index.hero_count(), catalog.heroes.len());
    assert!(index.hero(catalog.heroes.values().next().expect("英雄").id.as_str()) >= 0);
}

#[test]
fn observation_uses_only_public_information() {
    let (engine, state, catalog) = started();
    let seat_view = view(&engine, &state, 0).expect("视图");
    let obs = Encoder::new(CardIndex::build(&catalog)).encode(&seat_view, &History::new());

    assert_eq!(obs.shop.len(), SHOP_SLOTS);
    assert_eq!(obs.players.len(), SEATS);
    assert_eq!(obs.board.len(), BOARD_SLOTS);
    // 没记录过对手棋盘时，所有历史槽位都是 -1。
    assert!(
        obs.players
            .iter()
            .all(|player| player.last_board.iter().all(|card| *card == -1))
    );
    assert_eq!(obs.hero, obs.players[0].hero);
    assert_eq!(obs.players.iter().filter(|p| p.alive > 0.0).count(), 8);
}

#[test]
fn history_only_fills_the_recorded_opponent() {
    let (engine, state, catalog) = started();
    let index = CardIndex::build(&catalog);
    let bisheng = index.card("bisheng");
    let mut history = History::new();
    history.record(3, &[bisheng, -1, -1, -1, -1, -1, -1]);

    let obs = Encoder::new(index).encode(&view(&engine, &state, 0).expect("视图"), &history);
    assert_eq!(obs.players[3].last_board[0], bisheng);
    assert!(
        obs.players[2].last_board.iter().all(|card| *card == -1),
        "没交手过的座位不应出现棋盘"
    );
}

#[test]
fn every_legal_action_round_trips() {
    let (engine, state, _) = started();
    let actions = legal_actions(&engine, &state, 0);
    assert!(!actions.is_empty());
    for intent in actions {
        let action = Action::from_intent(&intent).expect("意图应能转成动作");
        assert_eq!(action.to_intent(), Some(intent));
    }
}

#[test]
fn observation_layout_is_stable() {
    let (engine, state, catalog) = started();
    let obs = Encoder::new(CardIndex::build(&catalog))
        .encode(&view(&engine, &state, 0).expect("视图"), &History::new());
    let values = obs.to_vec();
    assert_eq!(values.len(), OBS_DIM);
    assert_eq!(values[0], 0.0);
    assert_eq!(values[1], 1.0 / 20.0);
    assert_eq!(values[2], 1.0);
    // 金样本：字段顺序或归一化常数一改，这里必然失败。
    assert_eq!(fnv(&values), 0x4dd7_7eda_1aba_56eb);
}
