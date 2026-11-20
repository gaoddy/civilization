mod common;

use common::{categories_for, effects_of, place, set_shop, setup, setup_with_gold};
use rule_engine::combat;
use rule_engine::data::{Card, CardType, Playstyle, Skill};
use rule_engine::effects::parse_effects;
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
            in_shop_pool: true,
            taxon: "hominidae".to_string(),
            categories: categories_for(playstyle),
            subtype: None,
            level: 1,
            era: None,
            attack: Some(0),
            health: Some(5),
            skills: vec![skill_id],
            is_scholar: false,
            cost: Some(1),
        },
    );
}

/// 注入一张无技能的普通卡，用于构造玩法计数与相邻测试的陪衬随从。
fn add_plain_card(engine: &mut Engine, card_id: &str, playstyle: Playstyle) {
    engine.catalog.cards.insert(
        card_id.to_string(),
        Card {
            id: card_id.to_string(),
            card_type: CardType::Minion,
            in_shop_pool: true,
            taxon: "hominidae".to_string(),
            categories: categories_for(playstyle),
            subtype: None,
            level: 1,
            era: None,
            attack: Some(1),
            health: Some(1),
            skills: Vec::new(),
            is_scholar: false,
            cost: Some(1),
        },
    );
}

/// 注入一张指定生命值的昆虫随从，避免光环测试依赖真实卡表的数值。
fn add_insect_card(engine: &mut Engine, card_id: &str, health: i32) {
    engine.catalog.cards.insert(
        card_id.to_string(),
        Card {
            id: card_id.to_string(),
            card_type: CardType::Minion,
            in_shop_pool: true,
            taxon: "hominidae".to_string(),
            categories: categories_for(Playstyle::Insect),
            subtype: None,
            level: 1,
            era: None,
            attack: Some(1),
            health: Some(health),
            skills: Vec::new(),
            is_scholar: false,
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
    // 固氮作物：你的植物随从获得 +1/+1（含自身）。
    place(&engine.catalog, &mut state, 0, 1, "nitrogen-crop");
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
    add_insect_card(&mut engine, "test-insect", 2);
    place(&engine.catalog, &mut state, 0, 2, "test-insect");
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
    let folded_insect = folded
        .iter()
        .find(|minion| minion.instance_id == 2)
        .expect("折叠后的棋盘应保留目标");
    assert_eq!(
        combat::effective_health(&engine.catalog, folded_insect),
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
    place(&engine.catalog, &mut state, 0, 1, "nitrogen-crop");
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
fn hero_aura_reads_hero_pool() {
    // 先从同名随从卡移除光环技能，再构建目录与引擎，避免加载后原地改动 `Catalog`。
    let mut catalog = common::test_catalog();
    catalog
        .cards
        .get_mut("darwin")
        .expect("达尔文随从卡应存在")
        .skills
        .retain(|skill| skill != "aura-biology-plus-1-plus-1");
    let (mut engine, mut state) = common::setup_with_catalog(catalog, 23);
    state.players[0].hero = Some("darwin".to_string());
    place(&engine.catalog, &mut state, 0, 1, "baiyi");

    engine.recompute_auras(&mut state);

    assert_eq!(
        state.auras.get(&1).map(|aura| (aura.attack, aura.health)),
        Some((1, 1)),
        "英雄光环只从英雄池读取"
    );
}

#[test]
fn recompute_is_idempotent() {
    let (mut engine, mut state) = setup(23);
    place(&engine.catalog, &mut state, 0, 1, "nitrogen-crop");
    place(&engine.catalog, &mut state, 0, 2, "juzi");

    engine.recompute_auras(&mut state);
    assert!(
        !state.auras.is_empty(),
        "夹具必须产生非空光环，否则幂等断言是空对空"
    );
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
    place(&engine.catalog, &mut state, 0, 1, "nitrogen-crop");
    place(&engine.catalog, &mut state, 0, 2, "juzi");
    place(&engine.catalog, &mut state, 3, 3, "juzi");
    state.players[3].eliminated = true;
    engine.recompute_auras(&mut state);
    assert!(
        !state.auras.is_empty(),
        "夹具必须产生非空光环，否则泄漏断言是空对空"
    );

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

#[test]
fn school_aura_scales_with_other_same_playstyle_count() {
    let (mut engine, mut state) = setup(23);
    add_aura_source(
        &mut engine,
        "test-fish-leader",
        Playstyle::Fish,
        "Aura|ModifyStats|friendly:all:playstyle=fish|attack=1,health=1,perAttack=1,duration=permanent",
    );
    add_plain_card(&mut engine, "test-fish-a", Playstyle::Fish);
    add_plain_card(&mut engine, "test-fish-b", Playstyle::Fish);
    place(&engine.catalog, &mut state, 0, 1, "test-fish-leader");
    place(&engine.catalog, &mut state, 0, 2, "test-fish-a");
    place(&engine.catalog, &mut state, 0, 3, "test-fish-b");

    engine.recompute_auras(&mut state);

    // 两个其他鱼纲 → 基础 1 + 每只 1×2 = 3 攻击。来源若被目标集合包含，
    // 也只按「其他随从」计数（自身不计），因此同样是 3 而不是 4。
    let source = state.auras.get(&1).expect("来源自身应获得基础加成");
    assert_eq!((source.attack, source.health), (3, 1));
    for id in [2, 3] {
        let aura = state.auras.get(&id).expect("其他鱼纲应获得群聚加成");
        assert_eq!(
            (aura.attack, aura.health),
            (3, 1),
            "两个其他鱼纲 → 基础 1 + 2 点额外攻击，生命只有基础 1"
        );
    }
}

#[test]
fn school_aura_count_is_capped_by_max_count() {
    let (mut engine, mut state) = setup(23);
    add_aura_source(
        &mut engine,
        "test-fish-leader",
        Playstyle::Fish,
        "Aura|ModifyStats|friendly:all:playstyle=fish|attack=1,perAttack=2,maxCount=1,duration=permanent",
    );
    add_plain_card(&mut engine, "test-fish-a", Playstyle::Fish);
    add_plain_card(&mut engine, "test-fish-b", Playstyle::Fish);
    place(&engine.catalog, &mut state, 0, 1, "test-fish-leader");
    place(&engine.catalog, &mut state, 0, 2, "test-fish-a");
    place(&engine.catalog, &mut state, 0, 3, "test-fish-b");

    engine.recompute_auras(&mut state);

    assert_eq!(
        state.auras.get(&2).map(|aura| aura.attack),
        Some(3),
        "maxCount=1 时只按一个其他随从计算"
    );
    assert_eq!(
        state.auras.get(&3).map(|aura| aura.attack),
        Some(3),
        "计数上限对所有目标一致"
    );
}

#[test]
fn adjacency_aura_only_buffs_left_and_right_neighbors() {
    let (mut engine, mut state) = setup(23);
    add_aura_source(
        &mut engine,
        "test-adjacent",
        Playstyle::Mammal,
        "Aura|ModifyStats|friendly:adjacent|attack=2,health=2,duration=permanent",
    );
    add_plain_card(&mut engine, "test-mammal-a", Playstyle::Mammal);
    add_plain_card(&mut engine, "test-mammal-b", Playstyle::Mammal);
    place(&engine.catalog, &mut state, 0, 1, "test-mammal-a");
    place(&engine.catalog, &mut state, 0, 2, "test-adjacent");
    place(&engine.catalog, &mut state, 0, 3, "test-mammal-b");

    engine.recompute_auras(&mut state);

    assert!(!state.auras.contains_key(&2), "携带者自身不获得相邻光环");
    for id in [1, 3] {
        let aura = state.auras.get(&id).expect("左右邻居应获得相邻光环");
        assert_eq!((aura.attack, aura.health), (2, 2));
    }
}

#[test]
fn aura_grant_keyword_keeps_armor_layer() {
    let (mut engine, mut state) = setup(23);
    add_aura_source(
        &mut engine,
        "test-armor-aura",
        Playstyle::Neutral,
        "Aura|GrantKeyword|friendly:all:other=true|keyword=armor 3,duration=permanent",
    );
    place(&engine.catalog, &mut state, 0, 1, "test-armor-aura");
    place(&engine.catalog, &mut state, 0, 2, "baiyi");

    engine.recompute_auras(&mut state);

    let aura = state.auras.get(&2).expect("目标应获得减伤光环");
    assert!(
        aura.keywords.contains("armor 3"),
        "光环授予关键词应保留层数，实为 {:?}",
        aura.keywords
    );

    let folded = fold_auras(
        &engine.catalog,
        &state.auras,
        &state.players[0].board.clone(),
    );
    let folded_target = folded
        .iter()
        .find(|minion| minion.instance_id == 2)
        .expect("折叠后的棋盘应保留目标");
    assert_eq!(
        combat::armor_value(&engine.catalog, folded_target),
        3,
        "折叠后应提供 3 点减伤，而不是退化为 1 点"
    );
}

#[test]
fn aura_armor_stacks_across_sources() {
    let (mut engine, mut state) = setup(23);
    for card_id in ["armor-aura-a", "armor-aura-b"] {
        add_aura_source(
            &mut engine,
            card_id,
            Playstyle::Neutral,
            "Aura|GrantKeyword|friendly:all:other=true|keyword=armor 1,duration=permanent",
        );
    }
    place(&engine.catalog, &mut state, 0, 1, "armor-aura-a");
    place(&engine.catalog, &mut state, 0, 2, "armor-aura-b");
    place(&engine.catalog, &mut state, 0, 3, "baiyi");

    engine.recompute_auras(&mut state);

    let aura = state.auras.get(&3).expect("目标应同时获得两个减伤光环");
    assert_eq!(
        aura.keywords
            .iter()
            .filter(|k| k.starts_with("armor"))
            .count(),
        1,
        "同层减伤应合并为一条关键词"
    );
    let folded = fold_auras(
        &engine.catalog,
        &state.auras,
        &state.players[0].board.clone(),
    );
    let folded_target = folded
        .iter()
        .find(|minion| minion.instance_id == 3)
        .expect("折叠后的棋盘应保留目标");
    assert_eq!(
        combat::armor_value(&engine.catalog, folded_target),
        2,
        "两个光环来源的同层减伤应累加"
    );
}

#[test]
fn non_aura_effects_reject_clustering_params_and_adjacent() {
    assert!(
        parse_effects("OnPlay|ModifyStats|self|attack=1,perAttack=1").is_err(),
        "perAttack 只能用于光环的 ModifyStats"
    );
    assert!(
        parse_effects("OnPlay|ModifyStats|self|maxCount=2").is_err(),
        "maxCount 只能用于光环的 ModifyStats"
    );
    assert!(
        parse_effects("OnCombatStart|DealDamage|friendly:adjacent|amount=1").is_err(),
        "adjacent 只能用于光环"
    );
}

#[test]
fn incremental_recompute_matches_full_recompute() {
    let (mut engine, mut state) = setup(23);
    add_aura_source(
        &mut engine,
        "test-mech-aura",
        Playstyle::Machine,
        "Aura|ModifyStats|friendly:all:playstyle=machine|attack=1,health=1,duration=permanent",
    );
    place(&engine.catalog, &mut state, 0, 1, "test-mech-aura");
    place(&engine.catalog, &mut state, 0, 2, "nanobot");
    place(&engine.catalog, &mut state, 1, 3, "test-mech-aura");
    place(&engine.catalog, &mut state, 1, 4, "nanobot");

    engine.recompute_auras(&mut state);

    // 只改座位 1：新增一个受光环影响的机械随从。
    place(&engine.catalog, &mut state, 1, 5, "nanobot");
    engine.recompute_auras_for(&mut state, 1);

    assert!(
        state.auras.contains_key(&1),
        "座位 0 的光环条目不应被座位 1 的重算移除"
    );
    let incremental = state.auras.clone();

    engine.recompute_auras(&mut state);

    assert_eq!(incremental, state.auras, "增量结果应与全量重算一致");
}

#[test]
fn incremental_recompute_drops_sold_minion_aura() {
    let (mut engine, mut state) = setup(23);
    add_aura_source(
        &mut engine,
        "test-mech-aura",
        Playstyle::Machine,
        "Aura|ModifyStats|friendly:all:playstyle=machine|attack=1,health=1,duration=permanent",
    );
    place(&engine.catalog, &mut state, 0, 1, "test-mech-aura");
    place(&engine.catalog, &mut state, 0, 2, "nanobot");

    engine.recompute_auras(&mut state);
    assert!(state.auras.contains_key(&2), "出售前目标应有光环");

    engine
        .apply(&mut state, 0, Intent::SellMinion { minion: 2 })
        .expect("出售应成功");
    assert!(
        !state.auras.contains_key(&2),
        "出售后不应残留已离场实例的光环条目"
    );
    let incremental = state.auras.clone();
    engine.recompute_auras(&mut state);
    assert_eq!(incremental, state.auras, "出售后增量结果应与全量重算一致");
}

#[test]
fn incremental_recompute_drops_aura_when_source_sold() {
    let (mut engine, mut state) = setup(23);
    add_aura_source(
        &mut engine,
        "test-mech-aura",
        Playstyle::Machine,
        "Aura|ModifyStats|friendly:all:playstyle=machine|attack=1,health=1,duration=permanent",
    );
    place(&engine.catalog, &mut state, 0, 1, "test-mech-aura");
    place(&engine.catalog, &mut state, 0, 2, "nanobot");

    engine.recompute_auras(&mut state);
    assert!(state.auras.contains_key(&2), "出售来源前目标应有光环");

    engine
        .apply(&mut state, 0, Intent::SellMinion { minion: 1 })
        .expect("出售来源应成功");
    assert!(
        !state.auras.contains_key(&2),
        "来源出售后目标不应残留光环条目"
    );
    let incremental = state.auras.clone();
    engine.recompute_auras(&mut state);
    assert_eq!(
        incremental, state.auras,
        "来源出售后增量结果应与全量重算一致"
    );
}

#[test]
fn incremental_recompute_adds_aura_when_source_bought() {
    let (mut engine, mut state) = setup_with_gold(23);
    add_aura_source(
        &mut engine,
        "test-mech-aura",
        Playstyle::Machine,
        "Aura|ModifyStats|friendly:all:playstyle=machine|attack=1,health=1,duration=permanent",
    );
    place(&engine.catalog, &mut state, 0, 1, "nanobot");
    // 测试注入的卡不在开局共享池内，且固定实例编号已用到 1；补池计数并让
    // 商店槽拿到不冲突的实例编号，买入预检才能通过。
    state.pool.insert("test-mech-aura".to_string(), 1);
    state.next_instance_id = 2;
    set_shop(&mut state, 0, &[Some("test-mech-aura".to_string())]);

    engine
        .apply(&mut state, 0, Intent::BuyMinion { slot: 0 })
        .expect("购买来源应成功");
    // 买入只进手牌；打出后来源才进入棋盘并触发增量重算。
    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect("打出来源应成功");

    assert!(state.auras.contains_key(&1), "来源入场后目标应获得光环条目");
    let incremental = state.auras.clone();
    engine.recompute_auras(&mut state);
    assert_eq!(
        incremental, state.auras,
        "来源入场后增量结果应与全量重算一致"
    );
}

#[test]
fn incremental_recompute_clears_eliminated_seat() {
    let (mut engine, mut state) = setup(23);
    add_aura_source(
        &mut engine,
        "test-mech-aura",
        Playstyle::Machine,
        "Aura|ModifyStats|friendly:all:playstyle=machine|attack=1,health=1,duration=permanent",
    );
    place(&engine.catalog, &mut state, 0, 1, "test-mech-aura");
    place(&engine.catalog, &mut state, 0, 2, "nanobot");

    engine.recompute_auras(&mut state);
    assert!(
        state.auras.contains_key(&1) && state.auras.contains_key(&2),
        "出局前该座位的来源与目标都应有光环"
    );

    state.players[0].eliminated = true;
    engine.recompute_auras_for(&mut state, 0);
    assert!(
        state.players[0]
            .board
            .iter()
            .all(|minion| !state.auras.contains_key(&minion.instance_id)),
        "出局座位的棋盘光环条目应全部清除"
    );
    let incremental = state.auras.clone();
    engine.recompute_auras(&mut state);
    assert_eq!(incremental, state.auras, "出局座位增量结果应与全量重算一致");
}
