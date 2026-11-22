//! T04：视图层逐来源加成的合并、过滤与排序。
//!
//! 覆盖：成长与永久合并为永久层、层内按数值降序、来源和等于聚合层，
//! 某层聚合为 0 时该层来源全部隐藏，以及合并键的技能维度、固定层序与系统来源形态。

mod common;

use common::{categories_for, effects_of, place, setup};
use rule_engine::data::{Card, CardType, Playstyle, Skill};
use rule_engine::effects::{EffectCtx, Trigger};
use rule_engine::engine::Engine;
use rule_engine::state::{StatSourceLayer, StatSourceOrigin, StatSystemSource};
use rule_engine::view::{StatSourceKindView, StatSourceLayerView, view};

/// 向目录插入一张打出来给全体友方加攻防的随从牌。
fn add_buff_card(engine: &mut Engine, card_id: &str, attack: i32, health: i32) {
    let id = engine.catalog.insert_skill(Skill {
        id: format!("{card_id}-skill"),
        keyword: None,
        effects: effects_of(&format!(
            "OnPlay|ModifyStats|friendly:all|attack={attack},health={health}"
        )),
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
            skills: vec![id],
            cost: Some(1),
        },
    );
}

/// 每次执行效果都用一份新的上下文；`EffectCtx` 会被 `run_effects` 消费。
fn ctx() -> EffectCtx {
    EffectCtx {
        seat: 0,
        source: None,
        target: None,
        event_target: None,
        golden: false,
        level_override: None,
    }
}

#[test]
fn view_sorts_and_sums_sources() {
    let (mut engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 10, "jianchihu");
    add_buff_card(&mut engine, "small", 1, 0);
    add_buff_card(&mut engine, "big", 0, 3);
    engine.fire_card(&mut state, "small", Trigger::OnPlay, ctx());
    engine.fire_card(&mut state, "big", Trigger::OnPlay, ctx());

    let board = view(&engine, &state, 0).unwrap().self_view.board;
    let stats = &board[0].stats;
    let permanent: Vec<_> = stats
        .sources
        .iter()
        .filter(|source| source.layer == StatSourceLayerView::Permanent)
        .collect();
    assert_eq!(permanent.len(), 2);
    // `big` 的 |0| + |3| 大于 `small` 的 |1| + |0|，排在前面。
    assert_eq!(permanent[0].card_id.as_deref(), Some("big"));
    let sum_attack: i32 = permanent.iter().map(|source| source.attack).sum();
    let sum_health: i32 = permanent.iter().map(|source| source.health).sum();
    assert_eq!(sum_attack, stats.attack.permanent);
    assert_eq!(sum_health, stats.health.permanent);
}

#[test]
fn zero_layer_has_no_sources() {
    let (mut engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 10, "jianchihu");
    add_buff_card(&mut engine, "plus", 1, 0);
    add_buff_card(&mut engine, "minus", -1, 0);
    engine.fire_card(&mut state, "plus", Trigger::OnPlay, ctx());
    engine.fire_card(&mut state, "minus", Trigger::OnPlay, ctx());

    let board = view(&engine, &state, 0).unwrap().self_view.board;
    let stats = &board[0].stats;
    assert_eq!(stats.attack.permanent, 0);
    assert!(
        stats
            .sources
            .iter()
            .all(|source| source.layer != StatSourceLayerView::Permanent)
    );
}

/// 往目录插入一个空技能，返回其整数标识。
fn add_skill(engine: &mut Engine, skill_id: &str) -> rule_engine::data::SkillId {
    engine.catalog.insert_skill(Skill {
        id: skill_id.to_string(),
        keyword: None,
        effects: Vec::new(),
        cost: None,
        uses_per_turn: None,
    })
}

/// 同一张卡的两个技能必须各成一行；合并键若退化为 `(kind, card_id)` 会并成一行。
#[test]
fn same_card_different_skills_stay_separate() {
    let (mut engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 12, "jianchihu");
    let first = add_skill(&mut engine, "twin-first");
    let second = add_skill(&mut engine, "twin-second");

    let minion = &mut state.players[0].board[0];
    minion.stats.attack.add_permanent(3);
    minion.record_stat_source(
        StatSourceLayer::Permanent,
        StatSourceOrigin::Card {
            card_id: Some("twin".to_string()),
            skill_id: Some(first),
        },
        1,
        0,
    );
    minion.record_stat_source(
        StatSourceLayer::Permanent,
        StatSourceOrigin::Card {
            card_id: Some("twin".to_string()),
            skill_id: Some(second),
        },
        2,
        0,
    );

    let board = view(&engine, &state, 0).unwrap().self_view.board;
    let stats = &board[0].stats;
    let permanent: Vec<_> = stats
        .sources
        .iter()
        .filter(|source| source.layer == StatSourceLayerView::Permanent)
        .collect();
    assert_eq!(
        permanent.len(),
        2,
        "同卡不同技能必须分行，技能 id 是合并键的一部分"
    );
    // 数值降序：+2 的技能排在 +1 之前；技能 id 经 `Catalog::skill_name` 还原。
    assert_eq!(permanent[0].skill_id.as_deref(), Some("twin-second"));
    assert_eq!(permanent[1].skill_id.as_deref(), Some("twin-first"));
    assert!(
        permanent
            .iter()
            .all(|source| source.card_id.as_deref() == Some("twin"))
    );
    let sum_attack: i32 = permanent.iter().map(|source| source.attack).sum();
    assert_eq!(sum_attack, stats.attack.permanent);
}

/// 层序固定为光环 → 永久 → 本回合，不能退化成按权重全局排序。
#[test]
fn layer_order_wins_over_weight() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 13, "jianchihu");

    let minion = &mut state.players[0].board[0];
    // 权重：永久 5 > 本回合 2 > 光环 1；若按权重全局排序会得到永久→本回合→光环。
    minion.stats.attack.set_aura(1);
    minion.stats.attack.add_permanent(5);
    minion.stats.attack.add_turn(2);
    minion.record_stat_source(
        StatSourceLayer::Aura,
        StatSourceOrigin::Card {
            card_id: Some("aura-card".to_string()),
            skill_id: None,
        },
        1,
        0,
    );
    minion.record_stat_source(
        StatSourceLayer::Permanent,
        StatSourceOrigin::Card {
            card_id: Some("permanent-card".to_string()),
            skill_id: None,
        },
        5,
        0,
    );
    minion.record_stat_source(
        StatSourceLayer::Turn,
        StatSourceOrigin::Card {
            card_id: Some("turn-card".to_string()),
            skill_id: None,
        },
        2,
        0,
    );

    let board = view(&engine, &state, 0).unwrap().self_view.board;
    let stats = &board[0].stats;
    let layers: Vec<_> = stats.sources.iter().map(|source| source.layer).collect();
    assert_eq!(
        layers,
        vec![
            StatSourceLayerView::Aura,
            StatSourceLayerView::Permanent,
            StatSourceLayerView::Turn,
        ],
        "层序固定为光环→永久→本回合"
    );
}

/// 系统来源按枚举映射 `kind`，且不携带卡牌 / 技能 id。
#[test]
fn system_sources_carry_kind_without_card_or_skill() {
    let (engine, mut state) = setup(11);
    place(&engine.catalog, &mut state, 0, 14, "jianchihu");

    let minion = &mut state.players[0].board[0];
    minion.stats.attack.add_permanent(1);
    minion.stats.health.add_permanent(2);
    minion.stats.attack.add_turn(1);
    minion.stats.health.set_aura(1);
    minion.record_stat_source(
        StatSourceLayer::Permanent,
        StatSourceOrigin::System(StatSystemSource::Evolve),
        1,
        0,
    );
    minion.record_stat_source(
        StatSourceLayer::Permanent,
        StatSourceOrigin::System(StatSystemSource::Breed),
        0,
        2,
    );
    minion.record_stat_source(
        StatSourceLayer::Turn,
        StatSourceOrigin::System(StatSystemSource::Contagion),
        1,
        0,
    );
    minion.record_stat_source(
        StatSourceLayer::Aura,
        StatSourceOrigin::System(StatSystemSource::AuraFloor),
        0,
        1,
    );

    let board = view(&engine, &state, 0).unwrap().self_view.board;
    let stats = &board[0].stats;
    for (kind, layer, attack, health) in [
        (
            StatSourceKindView::Evolve,
            StatSourceLayerView::Permanent,
            1,
            0,
        ),
        (
            StatSourceKindView::Breed,
            StatSourceLayerView::Permanent,
            0,
            2,
        ),
        (
            StatSourceKindView::Contagion,
            StatSourceLayerView::Turn,
            1,
            0,
        ),
        (
            StatSourceKindView::AuraFloor,
            StatSourceLayerView::Aura,
            0,
            1,
        ),
    ] {
        let row = stats
            .sources
            .iter()
            .find(|source| source.kind == kind)
            .unwrap_or_else(|| panic!("应存在 {kind:?} 来源"));
        assert_eq!(row.layer, layer);
        assert_eq!(row.card_id, None, "系统来源不带卡牌 id");
        assert_eq!(row.skill_id, None, "系统来源不带技能 id");
        assert_eq!((row.attack, row.health), (attack, health));
    }
    // 层序固定；永久层内按权重降序：Breed(2) 在 Evolve(1) 之前。
    let layers: Vec<_> = stats.sources.iter().map(|source| source.layer).collect();
    assert_eq!(
        layers,
        vec![
            StatSourceLayerView::Aura,
            StatSourceLayerView::Permanent,
            StatSourceLayerView::Permanent,
            StatSourceLayerView::Turn,
        ]
    );
    let kinds: Vec<_> = stats.sources.iter().map(|source| source.kind).collect();
    assert_eq!(
        kinds,
        vec![
            StatSourceKindView::AuraFloor,
            StatSourceKindView::Breed,
            StatSourceKindView::Evolve,
            StatSourceKindView::Contagion,
        ]
    );
}
