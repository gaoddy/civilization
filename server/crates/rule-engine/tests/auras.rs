mod common;

use common::{effects_of, place, setup};
use rule_engine::combat;
use rule_engine::data::{Card, CardType, Playstyle, Skill};
use rule_engine::engine::Engine;
use rule_engine::hash::state_hash;
use rule_engine::intent::Intent;
use rule_engine::settle::{Pairing, fold_auras};
use rule_engine::state::{GameState, InstanceId, MinionInstance, Phase, Seat};

fn board_minion(state: &GameState, seat: Seat, id: InstanceId) -> &MinionInstance {
    state.players[usize::from(seat)]
        .board
        .iter()
        .find(|minion| minion.instance_id == id)
        .expect("目标随从应在棋盘上")
}

/// 注入一张自定义光环来源卡（非英雄），用于数据表中不存在的负值与重复关键词场景。
fn add_aura_source(engine: &mut Engine, card_id: &str, playstyle: Playstyle, effects: &str) {
    let skill_id = format!("{card_id}-skill");
    engine.catalog.skills.insert(
        skill_id.clone(),
        Skill {
            id: skill_id.clone(),
            keyword: None,
            effects: effects_of(effects),
            cost: None,
            uses_per_turn: None,
        },
    );
    engine.catalog.cards.insert(
        card_id.to_string(),
        Card {
            id: card_id.to_string(),
            card_type: CardType::Minion,
            playstyle,
            taxon: "hominidae".to_string(),
            specialty: Vec::new(),
            subtype: None,
            level: 1,
            era: None,
            attack: Some(0),
            health: Some(5),
            skills: vec![skill_id],
            is_scholar: false,
            is_hero: false,
            cost: Some(1),
        },
    );
}

#[test]
fn aura_buffs_all_friendly_machines_except_self_if_other() {
    let (mut engine, mut state) = setup(23);
    add_aura_source(
        &mut engine,
        "test-mech-other",
        Playstyle::Machine,
        "Aura|ModifyStats|friendly:all:playstyle=machine:other=true|attack=2,health=2,duration=permanent",
    );
    // 来源自身是机械，`other=true` 应排除；另一台机械获得 +2/+2。
    place(&engine.catalog, &mut state, 0, 1, "test-mech-other");
    place(&engine.catalog, &mut state, 0, 2, "nanobot");

    engine.recompute_auras(&mut state);

    assert!(
        !state.auras.contains_key(&1),
        "other=true 的光环不应作用于来源自身"
    );
    let aura = state.auras.get(&2).expect("另一台机械应获得光环");
    assert_eq!((aura.attack, aura.health), (2, 2));
    assert!(aura.floor_one, "被光环作用的目标应标记生命下限");
}

#[test]
fn aura_recomputes_when_source_leaves() {
    let (mut engine, mut state) = setup(23);
    // 青蒿：你的植物随从获得 +1/+1（含自身）。
    place(&engine.catalog, &mut state, 0, 1, "qinghao");
    place(&engine.catalog, &mut state, 0, 2, "juzi");

    engine.recompute_auras(&mut state);
    assert_eq!(
        state.auras.get(&2).map(|aura| (aura.attack, aura.health)),
        Some((1, 1))
    );

    state.players[0]
        .board
        .retain(|minion| minion.instance_id != 1);
    engine.recompute_auras(&mut state);

    assert!(!state.auras.contains_key(&2), "来源离场后光环加成立即消失");
}

#[test]
fn aura_health_floor_is_one() {
    let (mut engine, mut state) = setup(23);
    add_aura_source(
        &mut engine,
        "test-minus",
        Playstyle::Neutral,
        "Aura|ModifyStats|friendly:all:playstyle=insect:other=true|health=-5,duration=permanent",
    );
    place(&engine.catalog, &mut state, 0, 1, "test-minus");
    place(&engine.catalog, &mut state, 0, 2, "baiyi");
    assert_eq!(
        combat::effective_health(&engine.catalog, board_minion(&state, 0, 2)),
        2
    );

    engine.recompute_auras(&mut state);
    let aura = state.auras.get(&2).expect("负向光环仍应记录");
    assert_eq!(aura.health, -5);

    let folded = fold_auras(
        &engine.catalog,
        &state.auras,
        &state.players[0].board.clone(),
    );
    let folded_baiyi = folded
        .iter()
        .find(|minion| minion.instance_id == 2)
        .expect("折叠后的棋盘应保留目标");
    assert_eq!(
        combat::effective_health(&engine.catalog, folded_baiyi),
        1,
        "光环导致的负生命应被夹到 1，而不是致死"
    );
    assert_eq!(
        combat::effective_health(&engine.catalog, board_minion(&state, 0, 2)),
        2,
        "光环不应写入持久状态"
    );
    assert_eq!(
        (
            board_minion(&state, 0, 2).stats.attack.aura,
            board_minion(&state, 0, 2).stats.health.aura
        ),
        (0, 0),
        "光环不得写入持久实例"
    );
}

#[test]
fn same_keyword_from_two_sources_applies_once() {
    let (mut engine, mut state) = setup(23);
    add_aura_source(
        &mut engine,
        "test-taunt-a",
        Playstyle::Neutral,
        "Aura|GrantKeyword|friendly:all|keyword=taunt,duration=permanent",
    );
    add_aura_source(
        &mut engine,
        "test-taunt-b",
        Playstyle::Neutral,
        "Aura|GrantKeyword|friendly:all|keyword=taunt,duration=permanent",
    );
    place(&engine.catalog, &mut state, 0, 1, "test-taunt-a");
    place(&engine.catalog, &mut state, 0, 2, "test-taunt-b");
    place(&engine.catalog, &mut state, 0, 3, "baiyi");

    engine.recompute_auras(&mut state);

    let aura = state.auras.get(&3).expect("目标应获得关键词光环");
    assert_eq!(aura.keywords.len(), 1);
    assert!(aura.keywords.contains("taunt"));
}

#[test]
fn conditional_aura_requires_condition() {
    let (mut engine, mut state) = setup(23);
    // 燧石矛兵：友方人类 >= 2 时自身获得 +1 攻击力。
    place(&engine.catalog, &mut state, 0, 1, "suishimaobing");

    engine.recompute_auras(&mut state);
    assert!(!state.auras.contains_key(&1), "条件不满足时条件光环不生效");

    place(&engine.catalog, &mut state, 0, 2, "buluozhanshi");
    engine.recompute_auras(&mut state);
    assert_eq!(
        state.auras.get(&1).map(|aura| (aura.attack, aura.health)),
        Some((1, 0)),
        "第二名人类入场后条件成立"
    );
}

#[test]
fn aura_does_not_stack_self() {
    let (mut engine, mut state) = setup(23);
    // 亚里士多德：你的随从获得 +0/+1（含自身），只应生效一次。
    place(&engine.catalog, &mut state, 0, 1, "aristotle");

    engine.recompute_auras(&mut state);

    let aura = state.auras.get(&1).expect("含自身的光环应作用于来源");
    assert_eq!((aura.attack, aura.health), (0, 1));
}

#[test]
fn aura_never_crosses_seats() {
    let (mut engine, mut state) = setup(23);
    place(&engine.catalog, &mut state, 0, 1, "qinghao");
    place(&engine.catalog, &mut state, 1, 2, "juzi");

    engine.recompute_auras(&mut state);

    assert_eq!(
        state.auras.get(&1).map(|aura| (aura.attack, aura.health)),
        Some((1, 1)),
        "光环对来源自身生效"
    );
    assert!(
        !state.auras.contains_key(&2),
        "光环不能跨座位作用于其他玩家的棋盘"
    );
}

#[test]
fn aura_folds_into_combat_snapshot_only() {
    let (mut engine, mut state) = setup(23);
    // 达尔文（英雄）：你的生物随从获得 +1/+1。
    state.players[0].hero = Some("darwin".to_string());
    place(&engine.catalog, &mut state, 0, 1, "baiyi");

    engine.recompute_auras(&mut state);
    assert_eq!(
        state.auras.get(&1).map(|aura| (aura.attack, aura.health)),
        Some((1, 1))
    );

    let stored_attack = combat::effective_attack(&engine.catalog, board_minion(&state, 0, 1));
    let stored_health = combat::effective_health(&engine.catalog, board_minion(&state, 0, 1));
    let folded = fold_auras(
        &engine.catalog,
        &state.auras,
        &state.players[0].board.clone(),
    );
    let folded_minion = folded
        .iter()
        .find(|minion| minion.instance_id == 1)
        .expect("折叠后的棋盘应保留随从");

    assert_eq!(
        combat::effective_attack(&engine.catalog, folded_minion),
        stored_attack + 1
    );
    assert_eq!(
        combat::effective_health(&engine.catalog, folded_minion),
        stored_health + 1
    );
    assert_eq!(
        combat::effective_attack(&engine.catalog, board_minion(&state, 0, 1)),
        stored_attack,
        "持久状态的属性不因折叠改变"
    );
    assert_eq!(
        board_minion(&state, 0, 1).stats.attack.aura,
        0,
        "光环不写回持久实例的光环层"
    );
    assert!(board_minion(&state, 0, 1).granted.is_empty());
}

#[test]
fn recompute_is_idempotent() {
    let (mut engine, mut state) = setup(23);
    place(&engine.catalog, &mut state, 0, 1, "qinghao");
    place(&engine.catalog, &mut state, 0, 2, "juzi");

    engine.recompute_auras(&mut state);
    let first = state.auras.clone();
    engine.recompute_auras(&mut state);

    assert_eq!(state.auras, first);
}

#[test]
fn apply_recomputes_auras() {
    let (mut engine, mut state) = setup(23);
    state.players[0].hero = Some("darwin".to_string());
    place(&engine.catalog, &mut state, 0, 1, "baiyi");

    engine
        .apply(&mut state, 0, Intent::FreezeShop { slot: None })
        .expect("冻结全部槽位应成功");

    assert_eq!(
        state.auras.get(&1).map(|aura| (aura.attack, aura.health)),
        Some((1, 1)),
        "成功的招募意图应触发光环重算"
    );
}

#[test]
fn state_hash_includes_auras() {
    let (_engine, state) = setup(23);
    let mut with_aura = state.clone();
    with_aura.auras.insert(
        1,
        rule_engine::state::AuraEffect {
            attack: 1,
            health: 0,
            keywords: Default::default(),
            floor_one: true,
        },
    );

    assert_ne!(state_hash(&state), state_hash(&with_aura));
}

#[test]
fn ghost_fight_does_not_leak_auras_into_stored_state() {
    let (mut engine, mut state) = setup(23);
    place(&engine.catalog, &mut state, 0, 1, "qinghao");
    place(&engine.catalog, &mut state, 0, 2, "juzi");
    place(&engine.catalog, &mut state, 3, 3, "juzi");
    state.players[3].eliminated = true;
    engine.recompute_auras(&mut state);

    let before = state.players[0].board.clone();
    let before_ghost = state.players[3].board.clone();
    let before_auras = state.auras.clone();
    state.phase = Phase::Combat;
    state.pairings = vec![Pairing {
        a: 0,
        b: 0,
        ghost: Some(3),
    }];

    engine
        .resolve_combat(&mut state)
        .expect("幽灵战斗应成功结算");

    // 落单座位照常写回战斗成长/传染；本夹具的两张卡都不产生成长或传染，
    // 因此持久棋盘应原样保留，用来确认光环折叠本身没有泄漏。
    assert_eq!(
        state.players[0].board, before,
        "无成长/传染的战斗不应改写落单座位持久棋盘"
    );
    assert_eq!(
        state.players[3].board, before_ghost,
        "幽灵座位棋盘不得被写回"
    );
    assert_eq!(state.auras, before_auras, "幽灵光环不得写回共享缓存");
    for minion in &state.players[0].board {
        assert_eq!(minion.stats.attack.aura, 0, "光环不得写入持久实例");
        assert!(minion.granted.is_empty(), "光环不得写入持久 granted");
    }
}

#[test]
fn aura_top_attack_picks_highest_attack() {
    let (mut engine, mut state) = setup(23);
    add_aura_source(
        &mut engine,
        "test-top-attack",
        Playstyle::Neutral,
        "Aura|ModifyStats|friendly:topAttack:other=true|attack=1,duration=permanent",
    );
    place(&engine.catalog, &mut state, 0, 1, "test-top-attack");
    place(&engine.catalog, &mut state, 0, 2, "baiyi");
    place(&engine.catalog, &mut state, 0, 3, "menghu");

    engine.recompute_auras(&mut state);

    assert!(
        !state.auras.contains_key(&2),
        "低攻击力目标不应被 topAttack 选中"
    );
    assert_eq!(
        state.auras.get(&3).map(|aura| aura.attack),
        Some(1),
        "topAttack 应选中攻击力最高的随从"
    );
}

#[test]
fn aura_single_without_target_has_no_targets() {
    let (mut engine, mut state) = setup(23);
    add_aura_source(
        &mut engine,
        "test-single",
        Playstyle::Neutral,
        "Aura|ModifyStats|friendly:single|attack=1,duration=permanent",
    );
    place(&engine.catalog, &mut state, 0, 1, "test-single");
    place(&engine.catalog, &mut state, 0, 2, "baiyi");

    engine.recompute_auras(&mut state);

    assert!(
        state.auras.is_empty(),
        "没有显式目标的光环 single 不应产生任何加成"
    );
}
