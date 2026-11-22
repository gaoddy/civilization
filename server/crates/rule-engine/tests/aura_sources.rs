mod common;

use std::collections::BTreeMap;

use common::{categories_for, effects_of, place, setup};
use rule_engine::data::{Card, CardType, Playstyle, Skill};
use rule_engine::engine::Engine;
use rule_engine::settle::fold_auras;
use rule_engine::state::{AuraEffect, StatSourceLayer, StatSourceOrigin, StatSystemSource};

/// 与任务书 JSON 语义等价的测试 DSL：友方全体光环加攻防。
fn aura_effects(attack: i32, health: i32) -> String {
    format!("Aura|ModifyStats|friendly:all|attack={attack},health={health}")
}

fn add_aura_card(engine: &mut Engine, card_id: &str, attack: i32, health: i32) {
    let skill_id = engine.catalog.insert_skill(Skill {
        id: format!("{card_id}-skill"),
        keyword: None,
        effects: effects_of(&aura_effects(attack, health)),
        cost: None,
        uses_per_turn: None,
    });
    engine.catalog.cards.insert(
        card_id.to_string(),
        Card {
            id: card_id.to_string(),
            card_type: CardType::Minion,
            in_shop_pool: true,
            taxon: "hominidae".to_string(),
            categories: categories_for(Playstyle::Mammal),
            subtype: None,
            level: 1,
            era: None,
            attack: Some(1),
            health: Some(1),
            skills: vec![skill_id],
            cost: Some(1),
        },
    );
}

fn add_plain_card(engine: &mut Engine, card_id: &str, health: i32) {
    engine.catalog.cards.insert(
        card_id.to_string(),
        Card {
            id: card_id.to_string(),
            card_type: CardType::Minion,
            in_shop_pool: true,
            taxon: "hominidae".to_string(),
            categories: categories_for(Playstyle::Mammal),
            subtype: None,
            level: 1,
            era: None,
            attack: Some(1),
            health: Some(health),
            skills: Vec::new(),
            cost: Some(1),
        },
    );
}

#[test]
fn same_card_aura_sources_merge() {
    let (mut engine, mut state) = setup(9);
    add_aura_card(&mut engine, "aura-a", 1, 1);
    place(&engine.catalog, &mut state, 0, 10, "aura-a");
    place(&engine.catalog, &mut state, 0, 11, "aura-a");
    place(&engine.catalog, &mut state, 0, 12, "jianchihu");
    engine.recompute_auras(&mut state);

    let target_id = state.players[0].board[2].instance_id;
    let breakdown = engine.player_auras(&state, &state.players[0]);
    let sources = breakdown.sources.get(&target_id).unwrap();
    let total: (i32, i32) = sources
        .iter()
        .fold((0, 0), |acc, s| (acc.0 + s.attack, acc.1 + s.health));
    let aura = breakdown.totals.get(&target_id).unwrap();
    assert_eq!(total, (aura.attack, aura.health));
    assert_eq!(sources.iter().filter(|s| s.card_id == "aura-a").count(), 1);
    assert_eq!((sources[0].attack, sources[0].health), (2, 2));
}

#[test]
fn folded_board_carries_aura_sources() {
    let (mut engine, mut state) = setup(9);
    add_aura_card(&mut engine, "aura-a", 1, 1);
    place(&engine.catalog, &mut state, 0, 10, "jianchihu");
    place(&engine.catalog, &mut state, 0, 11, "aura-a");
    engine.recompute_auras(&mut state);

    let board = state.players[0].board.clone();
    let folded = fold_auras(&engine.catalog, &state.auras, &state.aura_sources, &board);
    let target = &folded[0];
    let aura: Vec<_> = target
        .stat_sources
        .iter()
        .filter(|s| s.layer == StatSourceLayer::Aura)
        .collect();
    assert_eq!(aura.len(), 1);
    assert_eq!((aura[0].attack, aura[0].health), (1, 1));
}

#[test]
fn health_clamp_adjusts_negative_source() {
    let (mut engine, mut state) = setup(9);
    add_aura_card(&mut engine, "aura-minus", 0, -5);
    add_plain_card(&mut engine, "target", 3);
    place(&engine.catalog, &mut state, 0, 10, "target");
    place(&engine.catalog, &mut state, 0, 11, "aura-minus");
    engine.recompute_auras(&mut state);

    let board = state.players[0].board.clone();
    let folded = fold_auras(&engine.catalog, &state.auras, &state.aura_sources, &board);
    let target = &folded[0];
    assert_eq!(target.stats.health.aura, -2, "3 点生命遇 -5 光环应夹到 -2");
    let sum: i32 = target
        .stat_sources
        .iter()
        .filter(|s| s.layer == StatSourceLayer::Aura)
        .map(|s| s.health)
        .sum();
    assert_eq!(sum, target.stats.health.aura);
}

#[test]
fn health_clamp_without_negative_source_appends_aura_floor() {
    let (mut engine, mut state) = setup(9);
    add_plain_card(&mut engine, "target", 3);
    place(&engine.catalog, &mut state, 0, 10, "target");

    let mut auras = BTreeMap::new();
    auras.insert(
        10,
        AuraEffect {
            attack: 0,
            health: -5,
            ..AuraEffect::default()
        },
    );
    let sources = BTreeMap::new();
    let folded = fold_auras(&engine.catalog, &auras, &sources, &state.players[0].board);

    let target = &folded[0];
    assert_eq!(target.stats.health.aura, -2);
    let rows: Vec<_> = target
        .stat_sources
        .iter()
        .filter(|s| s.layer == StatSourceLayer::Aura)
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].origin,
        StatSourceOrigin::System(StatSystemSource::AuraFloor)
    );
    assert_eq!(rows[0].health, -2);
}

#[test]
fn hero_aura_uses_hero_card_id() {
    let (engine, mut state) = setup(23);
    state.players[0].hero = Some("darwin".to_string());
    place(&engine.catalog, &mut state, 0, 1, "baiyi");
    engine.recompute_auras(&mut state);

    let breakdown = engine.player_auras(&state, &state.players[0]);
    let sources = breakdown.sources.get(&1).expect("英雄光环应有逐来源");
    assert!(
        sources
            .iter()
            .any(|s| s.card_id == "darwin" && (s.attack, s.health) == (1, 1)),
        "英雄光环应按英雄卡 id 归因，实为 {sources:?}"
    );
}

#[test]
fn recompute_for_clears_stale_aura_sources_with_totals() {
    let (mut engine, mut state) = setup(9);
    add_aura_card(&mut engine, "aura-a", 1, 1);
    place(&engine.catalog, &mut state, 0, 10, "jianchihu");
    place(&engine.catalog, &mut state, 0, 11, "aura-a");
    engine.recompute_auras(&mut state);
    assert!(state.auras.contains_key(&10));
    assert!(state.aura_sources.contains_key(&10));

    state.players[0]
        .board
        .retain(|minion| minion.instance_id != 10);
    engine.recompute_auras_for(&mut state, 0);

    assert!(!state.auras.contains_key(&10), "陈旧聚合条目应被清理");
    assert!(
        !state.aura_sources.contains_key(&10),
        "陈旧逐来源条目应被清理"
    );
}
