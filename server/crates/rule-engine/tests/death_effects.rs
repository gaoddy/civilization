//! Task 13：死亡、召唤与战斗开始效果。
//!
//! 覆盖 `OnCombatStart` 排序、延迟战吼、亡语 → 播种 → 复生、友方/敌方死亡触发、
//! 召唤（token / card）、复制、`OnAllySummon` 与单场 200 次触发上限。

mod common;

use std::collections::BTreeMap;

use common::test_catalog;
use rule_engine::combat::{CombatAction, CombatLog, CombatSetup, simulate_with};
use rule_engine::data::{Card, CardType, Skill, Token, Tribe};
use rule_engine::engine::Engine;
use rule_engine::rng::RngStreams;
use rule_engine::state::{GameState, InstanceId, MinionInstance, Phase, Seat};

fn setup() -> (Engine, GameState) {
    let (engine, mut state) = Engine::new(test_catalog(), [31u8; 32]);
    state.phase = Phase::Recruit;
    state.turn = 1;
    for player in &mut state.players {
        player.hp = 30;
        player.gold = 10;
    }
    (engine, state)
}

/// 插入一张合成随从卡，携带若干 `(keyword, effects)` 技能。
fn add_card(
    engine: &mut Engine,
    id: &str,
    tribe: Tribe,
    attack: i32,
    health: i32,
    skills: &[(Option<&str>, &str)],
) {
    let mut skill_ids = Vec::new();
    for (index, (keyword, effects)) in skills.iter().enumerate() {
        let skill_id = format!("{id}-skill-{index}");
        engine.catalog.skills.insert(
            skill_id.clone(),
            Skill {
                id: skill_id.clone(),
                name: None,
                keyword: keyword.map(str::to_string),
                text: String::new(),
                effects: (*effects).to_string(),
                cost: None,
                uses_per_turn: None,
                match_text: None,
            },
        );
        skill_ids.push(skill_id);
    }
    engine.catalog.cards.insert(
        id.to_string(),
        Card {
            id: id.to_string(),
            card_type: CardType::Minion,
            name: id.to_string(),
            tribe,
            subtype: None,
            level: 1,
            era: None,
            attack: Some(attack),
            health: Some(health),
            skills: skill_ids,
            is_scholar: tribe == Tribe::Scholar,
            is_hero: false,
            cost: Some(1),
            match_text: None,
        },
    );
}

fn add_token(engine: &mut Engine, id: &str, attack: i32, health: i32) {
    engine.catalog.tokens.insert(
        id.to_string(),
        Token {
            id: id.to_string(),
            name: id.to_string(),
            attack,
            health,
            keywords: Vec::new(),
        },
    );
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

fn rng() -> RngStreams {
    RngStreams::from_seed([13u8; 32])
}

fn battle_first(
    engine: &Engine,
    attacker: &[MinionInstance],
    defender: &[MinionInstance],
    first: Seat,
) -> CombatLog {
    simulate_with(
        engine,
        attacker,
        defender,
        0,
        1,
        first,
        &mut rng(),
        CombatSetup::default(),
    )
}

fn battle_with(
    engine: &Engine,
    attacker: &[MinionInstance],
    defender: &[MinionInstance],
    setup: CombatSetup<'_>,
) -> CombatLog {
    simulate_with(engine, attacker, defender, 0, 1, 0, &mut rng(), setup)
}

fn battle(engine: &Engine, attacker: &[MinionInstance], defender: &[MinionInstance]) -> CombatLog {
    battle_first(engine, attacker, defender, 0)
}

fn deaths(log: &CombatLog) -> Vec<InstanceId> {
    log.actions
        .iter()
        .filter_map(|action| match action {
            CombatAction::Death { minion } => Some(*minion),
            _ => None,
        })
        .collect()
}

fn reborn_count(log: &CombatLog) -> usize {
    log.actions
        .iter()
        .filter(|action| matches!(action, CombatAction::Reborn { .. }))
        .count()
}

fn mods_attack(minion: &MinionInstance) -> i32 {
    minion
        .mods
        .iter()
        .fold(0i32, |sum, modifier| sum.saturating_add(modifier.attack))
}

fn final_minion(log: &CombatLog, id: InstanceId) -> &MinionInstance {
    log.attacker_final
        .iter()
        .chain(log.defender_final.iter())
        .find(|minion| minion.instance_id == id)
        .expect("死亡或复生的随从仍应存在于战斗结束快照中")
}

// ---------------------------------------------------------------------------
// 死亡结算
// ---------------------------------------------------------------------------

#[test]
fn deathrattle_runs_before_reborn() {
    let (mut engine, _state) = setup();
    add_token(&mut engine, "spawn", 2, 2);
    add_card(
        &mut engine,
        "ratter",
        Tribe::Biology,
        1,
        1,
        &[
            (Some("亡语"), "OnDeath|Summon|self|token=spawn,count=1"),
            (Some("复生"), "[]"),
        ],
    );
    add_card(&mut engine, "brute", Tribe::Human, 1, 10, &[]);

    let log = battle(&engine, &[minion(1, "ratter")], &[minion(2, "brute")]);

    assert_eq!(deaths(&log).first(), Some(&1), "首次死亡来自带亡语的随从");
    assert_eq!(reborn_count(&log), 1, "复生恰好触发一次");
    assert!(log.summon_count >= 1, "亡语在复生前完成召唤");
    assert!(
        log.attacker_final.iter().any(|m| m.card_id == "spawn"),
        "召唤物应出现在攻击方战斗快照中"
    );
}

#[test]
fn sow_summons_seedling_on_death() {
    let (mut engine, _state) = setup();
    add_token(&mut engine, "seedling", 1, 1);
    add_card(
        &mut engine,
        "sower",
        Tribe::Plant,
        1,
        1,
        &[(Some("播种"), "[]")],
    );
    add_card(&mut engine, "killer", Tribe::Human, 1, 10, &[]);

    let log = battle(&engine, &[minion(1, "sower")], &[minion(2, "killer")]);

    assert_eq!(log.summon_count, 1);
    assert!(
        log.attacker_final.iter().any(|m| m.card_id == "seedling"),
        "播种应召唤一个幼苗 token"
    );
    assert!(log.pool_spent.is_empty(), "token 不扣池");
}

#[test]
fn summon_card_deducts_pool_and_waits_when_empty() {
    let (mut engine, _state) = setup();
    add_card(
        &mut engine,
        "summoner",
        Tribe::Human,
        1,
        1,
        &[(Some("亡语"), "OnDeath|Summon|self|card=termite,count=1")],
    );
    add_card(&mut engine, "termite", Tribe::Human, 1, 1, &[]);
    add_card(&mut engine, "stomper", Tribe::Human, 5, 10, &[]);

    let mut pool = BTreeMap::new();
    pool.insert("termite".to_string(), 1u16);
    let with_copy = battle_with(
        &engine,
        &[minion(1, "summoner")],
        &[minion(2, "stomper")],
        CombatSetup {
            pool: Some(&pool),
            ..CombatSetup::default()
        },
    );
    assert_eq!(with_copy.summon_count, 1);
    assert_eq!(
        with_copy.pool_spent.get("termite"),
        Some(&1),
        "池中有份数时召唤 card 扣 1"
    );

    let empty_pool = BTreeMap::new();
    let without_copy = battle_with(
        &engine,
        &[minion(3, "summoner")],
        &[minion(4, "stomper")],
        CombatSetup {
            pool: Some(&empty_pool),
            ..CombatSetup::default()
        },
    );
    assert_eq!(without_copy.summon_count, 1, "池空仍创建副本");
    assert!(
        !without_copy.pool_spent.contains_key("termite"),
        "池空创建副本不扣池"
    );
}

#[test]
fn friendly_death_triggers_only_for_owner() {
    let (mut engine, _state) = setup();
    add_card(
        &mut engine,
        "watcher",
        Tribe::Human,
        0,
        10,
        &[(
            None,
            "OnFriendlyDeath|ModifyStats|self|attack=1,duration=permanent",
        )],
    );
    add_card(
        &mut engine,
        "victim",
        Tribe::Human,
        0,
        1,
        &[(Some("嘲讽"), "[]")],
    );
    add_card(
        &mut engine,
        "enemy-watcher",
        Tribe::Human,
        0,
        10,
        &[(
            None,
            "OnFriendlyDeath|ModifyStats|self|attack=1,duration=permanent",
        )],
    );
    add_card(&mut engine, "brute", Tribe::Human, 5, 5, &[]);

    let log = battle_first(
        &engine,
        &[minion(1, "watcher"), minion(2, "victim")],
        &[minion(3, "enemy-watcher"), minion(4, "brute")],
        1,
    );

    assert_eq!(mods_attack(final_minion(&log, 1)), 1, "死者的友方获得成长");
    assert_eq!(
        mods_attack(final_minion(&log, 3)),
        0,
        "敌方随从不应触发 OnFriendlyDeath"
    );
}

#[test]
fn enemy_death_triggers_for_opponent() {
    let (mut engine, _state) = setup();
    add_card(
        &mut engine,
        "avenger",
        Tribe::Human,
        10,
        10,
        &[(
            None,
            "OnEnemyDeath|ModifyStats|self|attack=2,duration=permanent",
        )],
    );
    add_card(
        &mut engine,
        "prey",
        Tribe::Human,
        0,
        1,
        &[(Some("嘲讽"), "[]")],
    );

    let log = battle(&engine, &[minion(1, "avenger")], &[minion(2, "prey")]);

    assert_eq!(mods_attack(final_minion(&log, 1)), 2);
}

// ---------------------------------------------------------------------------
// 战斗开始
// ---------------------------------------------------------------------------

#[test]
fn combat_start_order_heroes_then_minions() {
    let (mut engine, _state) = setup();
    add_card(
        &mut engine,
        "hero",
        Tribe::Human,
        0,
        30,
        &[(None, "OnCombatStart|DealDamage|enemy:all|amount=5")],
    );
    add_card(
        &mut engine,
        "minion0",
        Tribe::Human,
        0,
        5,
        &[(
            None,
            "OnCombatStart|ModifyStats|self|attack=1,duration=permanent",
        )],
    );
    add_card(
        &mut engine,
        "defender0",
        Tribe::Human,
        0,
        1,
        &[(
            None,
            "OnCombatStart|ModifyStats|self|attack=1,duration=permanent",
        )],
    );

    let log = battle_with(
        &engine,
        &[minion(1, "minion0")],
        &[minion(2, "defender0")],
        CombatSetup {
            attacker_hero: Some("hero"),
            ..CombatSetup::default()
        },
    );

    assert_eq!(
        mods_attack(final_minion(&log, 1)),
        1,
        "先手方随从效果已结算"
    );
    assert_eq!(
        mods_attack(final_minion(&log, 2)),
        0,
        "英雄被动先于随从结算，被击杀的随从不再触发"
    );
}

#[test]
fn delayed_battlecry_enemy_target_runs_at_combat_start() {
    let (mut engine, _state) = setup();
    add_card(
        &mut engine,
        "delayed",
        Tribe::Human,
        1,
        1,
        &[(Some("战吼"), "OnPlay|DealDamage|enemy:random|amount=3")],
    );
    add_card(&mut engine, "target", Tribe::Human, 0, 3, &[]);

    let log = battle(&engine, &[minion(1, "delayed")], &[minion(2, "target")]);

    assert_eq!(
        log.actions.first(),
        Some(&CombatAction::Death { minion: 2 }),
        "延迟战吼在行动循环前结算并击杀目标"
    );
    assert_eq!(log.action_count, 0);
    assert!(log.defender_survivors.is_empty());
}

#[test]
fn delayed_battlecry_cancelled_if_source_left() {
    let (mut engine, _state) = setup();
    add_card(
        &mut engine,
        "hero",
        Tribe::Human,
        0,
        30,
        &[(None, "OnCombatStart|DealDamage|enemy:all|amount=5")],
    );
    add_card(
        &mut engine,
        "delayed",
        Tribe::Human,
        1,
        1,
        &[(Some("战吼"), "OnPlay|DealDamage|enemy:random|amount=10")],
    );
    add_card(&mut engine, "tank", Tribe::Human, 0, 10, &[]);

    let log = battle_with(
        &engine,
        &[minion(1, "tank")],
        &[minion(2, "delayed")],
        CombatSetup {
            attacker_hero: Some("hero"),
            ..CombatSetup::default()
        },
    );

    assert!(
        log.attacker_survivors.contains(&1),
        "来源随从离场后延迟战吼取消，坦克不应死亡"
    );
}

#[test]
fn copy_minion_to_hand_does_not_deduct_pool() {
    let (mut engine, _state) = setup();
    add_card(
        &mut engine,
        "copier",
        Tribe::Human,
        0,
        10,
        &[(None, "OnCombatStart|Copy|self|to=hand")],
    );

    let log = battle_with(
        &engine,
        &[minion(1, "copier")],
        &[],
        CombatSetup {
            hand_slots: 10,
            ..CombatSetup::default()
        },
    );

    assert_eq!(log.hand_copies.len(), 1);
    assert_eq!(log.hand_copies[0].seat, 0);
    assert_eq!(log.hand_copies[0].card_id, "copier");
    assert!(log.pool_spent.is_empty(), "复制品不扣牌池");
}

#[test]
fn on_ally_summon_fires_after_summon() {
    let (mut engine, _state) = setup();
    add_token(&mut engine, "spawn", 1, 1);
    add_card(
        &mut engine,
        "summoner",
        Tribe::Human,
        1,
        5,
        &[(None, "OnCombatStart|Summon|self|token=spawn,count=1")],
    );
    add_card(
        &mut engine,
        "observer",
        Tribe::Human,
        0,
        10,
        &[(
            None,
            "OnAllySummon|ModifyStats|self|attack=1,duration=permanent",
        )],
    );

    let log = battle(
        &engine,
        &[minion(1, "summoner"), minion(2, "observer")],
        &[],
    );

    assert_eq!(log.summon_count, 1);
    assert_eq!(
        mods_attack(final_minion(&log, 2)),
        1,
        "召唤后友方随从的 OnAllySummon 触发"
    );
}

#[test]
fn trigger_budget_caps_effects() {
    let (mut engine, _state) = setup();
    add_token(&mut engine, "spawn", 0, 1);
    add_card(
        &mut engine,
        "loop",
        Tribe::Human,
        0,
        10,
        &[
            (None, "OnCombatStart|Summon|self|token=spawn,count=1"),
            (None, "OnAllySummon|Summon|self|token=spawn,count=1"),
        ],
    );

    let log = battle(&engine, &[minion(1, "loop")], &[]);

    assert_eq!(log.trigger_count, 200, "单场战斗触发上限为 200");
}

// ---------------------------------------------------------------------------
// 回合开始传染致死
// ---------------------------------------------------------------------------

#[test]
fn turn_start_contagion_death_fires_deathrattle_and_sow() {
    let (mut engine, mut state) = setup();
    add_token(&mut engine, "seedling", 1, 1);
    add_card(
        &mut engine,
        "dying",
        Tribe::Plant,
        1,
        1,
        &[
            (
                Some("亡语"),
                "OnDeath|ModifyStats|friendly:all|attack=1,duration=permanent",
            ),
            (Some("播种"), "[]"),
        ],
    );
    add_card(&mut engine, "friend", Tribe::Plant, 0, 5, &[]);

    state.players[0].board.push(minion(1, "dying"));
    state.players[0].board.push(minion(2, "friend"));
    state.players[0].board[0].contagion = 5;
    // 测试使用固定实例编号；把分配器推进到其后，避免召唤物与手写编号冲突。
    state.next_instance_id = 3;

    engine.start_turn(&mut state).expect("开始回合应成功");

    let board = &state.players[0].board;
    assert!(
        board.iter().all(|m| m.instance_id != 1),
        "传染致死的随从已离场"
    );
    assert_eq!(
        mods_attack(
            board
                .iter()
                .find(|m| m.instance_id == 2)
                .expect("友方随从仍应存活")
        ),
        1,
        "回合开始死亡触发亡语"
    );
    assert!(
        board.iter().any(|m| m.card_id == "seedling"),
        "播种在招募侧召唤幼苗"
    );
}
