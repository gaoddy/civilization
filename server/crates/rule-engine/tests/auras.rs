mod common;

use common::test_catalog;
use rule_engine::combat;
use rule_engine::data::{Card, CardType, Skill, Tribe};
use rule_engine::engine::Engine;
use rule_engine::hash::state_hash;
use rule_engine::intent::Intent;
use rule_engine::settle::{Pairing, fold_auras, mirror_ghost_with_auras};
use rule_engine::state::{GameState, InstanceId, MinionInstance, Phase, Seat};

fn setup() -> (Engine, GameState) {
    let (engine, mut state) = Engine::new(test_catalog(), [23u8; 32]);
    state.phase = Phase::Recruit;
    state.turn = 1;
    for player in &mut state.players {
        player.hp = 30;
    }
    (engine, state)
}

fn minion(instance_id: InstanceId, card_id: &str) -> MinionInstance {
    MinionInstance {
        instance_id,
        card_id: card_id.to_string(),
        golden: false,
        growth: 0,
        growth_health_only: 0,
        breed: 0,
        breed_applied: 0,
        contagion: 0,
        granted: Vec::new(),
        mods: Vec::new(),
        shield_used: false,
        reborn_used: false,
        attacks_left: 0,
        cannot_attack: false,
    }
}

fn place(state: &mut GameState, seat: Seat, id: InstanceId, card_id: &str) {
    state.players[usize::from(seat)]
        .board
        .push(minion(id, card_id));
}

fn board_minion(state: &GameState, seat: Seat, id: InstanceId) -> &MinionInstance {
    state.players[usize::from(seat)]
        .board
        .iter()
        .find(|minion| minion.instance_id == id)
        .expect("目标随从应在棋盘上")
}

/// 注入一张自定义光环来源卡（非英雄），用于数据表中不存在的负值与重复关键词场景。
fn add_aura_source(engine: &mut Engine, card_id: &str, tribe: Tribe, effects: &str) {
    let skill_id = format!("{card_id}-skill");
    engine.catalog.skills.insert(
        skill_id.clone(),
        Skill {
            id: skill_id.clone(),
            name: None,
            keyword: None,
            text: effects.to_string(),
            effects: effects.to_string(),
            cost: None,
            uses_per_turn: None,
            match_text: None,
        },
    );
    engine.catalog.cards.insert(
        card_id.to_string(),
        Card {
            id: card_id.to_string(),
            card_type: CardType::Minion,
            name: card_id.to_string(),
            tribe,
            subtype: None,
            level: 1,
            era: None,
            attack: Some(0),
            health: Some(5),
            skills: vec![skill_id],
            is_scholar: false,
            is_hero: false,
            cost: Some(1),
            match_text: None,
        },
    );
}

#[test]
fn aura_buffs_all_friendly_machines_except_self_if_other() {
    let (mut engine, mut state) = setup();
    add_aura_source(
        &mut engine,
        "test-mech-other",
        Tribe::Machine,
        "Aura|ModifyStats|friendly:all:tribe=机械:other=true|attack=2,health=2,duration=permanent",
    );
    // 来源自身是机械，`other=true` 应排除；另一台机械获得 +2/+2。
    place(&mut state, 0, 1, "test-mech-other");
    place(&mut state, 0, 2, "shilunche");

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
    let (mut engine, mut state) = setup();
    // 青蒿：你的植物随从获得 +1/+1（含自身）。
    place(&mut state, 0, 1, "qinghao");
    place(&mut state, 0, 2, "juzi");

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
    let (mut engine, mut state) = setup();
    add_aura_source(
        &mut engine,
        "test-minus",
        Tribe::General,
        "Aura|ModifyStats|friendly:all:tribe=生物:other=true|health=-5,duration=permanent",
    );
    place(&mut state, 0, 1, "test-minus");
    place(&mut state, 0, 2, "baiyi");
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
    assert!(board_minion(&state, 0, 2).mods.is_empty());
}

#[test]
fn same_keyword_from_two_sources_applies_once() {
    let (mut engine, mut state) = setup();
    add_aura_source(
        &mut engine,
        "test-taunt-a",
        Tribe::General,
        "Aura|GrantKeyword|friendly:all|keyword=嘲讽,duration=permanent",
    );
    add_aura_source(
        &mut engine,
        "test-taunt-b",
        Tribe::General,
        "Aura|GrantKeyword|friendly:all|keyword=嘲讽,duration=permanent",
    );
    place(&mut state, 0, 1, "test-taunt-a");
    place(&mut state, 0, 2, "test-taunt-b");
    place(&mut state, 0, 3, "baiyi");

    engine.recompute_auras(&mut state);

    let aura = state.auras.get(&3).expect("目标应获得关键词光环");
    assert_eq!(aura.keywords.len(), 1);
    assert!(aura.keywords.contains("嘲讽"));
}

#[test]
fn conditional_aura_requires_condition() {
    let (mut engine, mut state) = setup();
    // 燧石矛兵：友方人类 >= 2 时自身获得 +1 攻击力。
    place(&mut state, 0, 1, "suishimaobing");

    engine.recompute_auras(&mut state);
    assert!(!state.auras.contains_key(&1), "条件不满足时条件光环不生效");

    place(&mut state, 0, 2, "buluozhanshi");
    engine.recompute_auras(&mut state);
    assert_eq!(
        state.auras.get(&1).map(|aura| (aura.attack, aura.health)),
        Some((1, 0)),
        "第二名人类入场后条件成立"
    );
}

#[test]
fn aura_does_not_stack_self() {
    let (mut engine, mut state) = setup();
    // 亚里士多德：你的随从获得 +0/+1（含自身），只应生效一次。
    place(&mut state, 0, 1, "aristotle");

    engine.recompute_auras(&mut state);

    let aura = state.auras.get(&1).expect("含自身的光环应作用于来源");
    assert_eq!((aura.attack, aura.health), (0, 1));
}

#[test]
fn aura_never_crosses_seats() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "qinghao");
    place(&mut state, 1, 2, "juzi");

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
    let (mut engine, mut state) = setup();
    // 达尔文（英雄）：你的生物随从获得 +1/+1。
    state.players[0].hero = Some("darwin".to_string());
    place(&mut state, 0, 1, "baiyi");

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
    assert!(board_minion(&state, 0, 1).mods.is_empty());
    assert!(board_minion(&state, 0, 1).granted.is_empty());
}

#[test]
fn recompute_is_idempotent() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "qinghao");
    place(&mut state, 0, 2, "juzi");

    engine.recompute_auras(&mut state);
    let first = state.auras.clone();
    engine.recompute_auras(&mut state);

    assert_eq!(state.auras, first);
}

#[test]
fn apply_recomputes_auras() {
    let (mut engine, mut state) = setup();
    state.players[0].hero = Some("darwin".to_string());
    place(&mut state, 0, 1, "baiyi");

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
    let (_engine, state) = setup();
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
fn mirror_ghost_from_previous_board_includes_auras() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "qinghao");
    place(&mut state, 0, 2, "juzi");
    engine.recompute_auras(&mut state);

    // 上一场快照保留原始实例编号，光环缓存可按同一 InstanceId 命中。
    let raw = state.players[0].board.clone();
    state.previous_board.insert(0, raw.clone());

    let ghost = mirror_ghost_with_auras(&engine.catalog, &state, 0);
    let ghost_juzi = ghost
        .iter()
        .find(|minion| minion.instance_id == 2)
        .expect("幽灵应包含上一场随从");
    assert_eq!(
        combat::effective_health(&engine.catalog, ghost_juzi),
        4,
        "幽灵应叠加 +1 生命光环（基础 3 + 光环 1）"
    );
    assert_eq!(
        combat::effective_attack(&engine.catalog, ghost_juzi),
        2,
        "幽灵应叠加 +1 攻击光环（基础 1 + 光环 1）"
    );
}

#[test]
fn mirror_fight_does_not_mutate_stored_state() {
    let (mut engine, mut state) = setup();
    place(&mut state, 0, 1, "qinghao");
    place(&mut state, 0, 2, "juzi");
    engine.recompute_auras(&mut state);

    let before = state.players[0].board.clone();
    state.previous_board.insert(0, before.clone());
    state.phase = Phase::Combat;
    state.pairings = vec![Pairing {
        a: 0,
        b: 0,
        mirror: true,
    }];

    engine
        .resolve_combat(&mut state)
        .expect("镜像战斗应成功结算");

    assert_eq!(state.players[0].board, before, "战斗不得修改持久随从实例");
    assert_eq!(
        state.previous_board.get(&0),
        Some(&before),
        "上一场快照应保持原始（未折叠光环）内容"
    );
    for minion in &state.players[0].board {
        assert!(minion.mods.is_empty(), "光环不得写入持久 mods");
        assert!(minion.granted.is_empty(), "光环不得写入持久 granted");
    }
}

#[test]
fn aura_top_attack_picks_highest_attack() {
    let (mut engine, mut state) = setup();
    add_aura_source(
        &mut engine,
        "test-top-attack",
        Tribe::General,
        "Aura|ModifyStats|friendly:topAttack:other=true|attack=1,duration=permanent",
    );
    place(&mut state, 0, 1, "test-top-attack");
    place(&mut state, 0, 2, "baiyi");
    place(&mut state, 0, 3, "menghu");

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
    let (mut engine, mut state) = setup();
    add_aura_source(
        &mut engine,
        "test-single",
        Tribe::General,
        "Aura|ModifyStats|friendly:single|attack=1,duration=permanent",
    );
    place(&mut state, 0, 1, "test-single");
    place(&mut state, 0, 2, "baiyi");

    engine.recompute_auras(&mut state);

    assert!(
        state.auras.is_empty(),
        "没有显式目标的光环 single 不应产生任何加成"
    );
}
