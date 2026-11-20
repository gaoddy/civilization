//! Task 13：死亡、召唤与战斗开始效果。
//!
//! 覆盖 `OnCombatStart` 排序、延迟战吼、亡语 → 播种 → 复生、友方/敌方死亡触发、
//! 召唤（token / card）、复制、`OnAllySummon` 与单场 200 次触发上限。

mod common;

use std::collections::BTreeMap;

use common::{categories_for, effects_of, minion, reborn_count, setup_with_gold};
use rule_engine::combat::{CombatAction, CombatLog, CombatSetup, simulate_with};
use rule_engine::data::{Card, CardType, HeroCard, Playstyle, Skill, Token};
use rule_engine::effects::Duration;
use rule_engine::engine::Engine;
use rule_engine::rng::RngStreams;
use rule_engine::settle::Pairing;
use rule_engine::state::{InstanceId, MinionInstance, Phase, Seat};

/// 插入一张合成随从卡，携带若干 `(keyword, effects)` 技能。
fn add_card(
    engine: &mut Engine,
    id: &str,
    playstyle: Playstyle,
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
                keyword: keyword.map(str::to_string),
                effects: effects_of(effects),
                cost: None,
                uses_per_turn: None,
            },
        );
        skill_ids.push(skill_id);
    }
    engine.catalog.cards.insert(
        id.to_string(),
        Card {
            id: id.to_string(),
            card_type: CardType::Minion,
            taxon: "hominidae".to_string(),
            categories: categories_for(playstyle),
            subtype: None,
            level: 1,
            era: None,
            attack: Some(attack),
            health: Some(health),
            skills: skill_ids,
            is_scholar: false,
            cost: Some(1),
        },
    );
}

/// 把同 id 的随从卡镜像为英雄池条目：战斗开始的英雄技能只从英雄池读取。
fn mirror_as_hero(engine: &mut Engine, id: &str) {
    let card = engine
        .catalog
        .cards
        .get(id)
        .expect("镜像源卡应存在")
        .clone();
    engine.catalog.heroes.insert(
        id.to_string(),
        HeroCard {
            id: id.to_string(),
            categories: card.categories,
            skills: card.skills,
        },
    );
}

fn add_token(engine: &mut Engine, id: &str, attack: i32, health: i32) {
    engine.catalog.tokens.insert(
        id.to_string(),
        Token {
            id: id.to_string(),
            attack,
            health,
            keywords: Vec::new(),
            playstyle: None,
            taxon: "biology".to_string(),
        },
    );
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

fn mods_attack(minion: &MinionInstance) -> i32 {
    minion.stats.attack.permanent
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
    let (mut engine, _state) = setup_with_gold(31);
    add_token(&mut engine, "spawn", 2, 2);
    add_card(
        &mut engine,
        "ratter",
        Playstyle::Bacteria,
        1,
        1,
        &[
            (
                Some("deathrattle"),
                "OnDeath|Summon|self|token=spawn,count=1",
            ),
            (Some("reborn"), "[]"),
        ],
    );
    add_card(&mut engine, "brute", Playstyle::Mammal, 1, 10, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "ratter")],
        &[minion(&engine.catalog, 2, "brute")],
    );

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
    let (mut engine, _state) = setup_with_gold(31);
    add_token(&mut engine, "seedling", 1, 1);
    add_card(
        &mut engine,
        "sower",
        Playstyle::Plant,
        1,
        1,
        &[(Some("sow"), "[]")],
    );
    add_card(&mut engine, "killer", Playstyle::Mammal, 1, 10, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "sower")],
        &[minion(&engine.catalog, 2, "killer")],
    );

    assert_eq!(log.summon_count, 1);
    assert!(
        log.attacker_final.iter().any(|m| m.card_id == "seedling"),
        "播种应召唤一个幼苗 token"
    );
    assert!(log.pool_spent.is_empty(), "token 不扣池");
}

#[test]
fn summon_card_deducts_pool_and_waits_when_empty() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "summoner",
        Playstyle::Mammal,
        1,
        1,
        &[(
            Some("deathrattle"),
            "OnDeath|Summon|self|card=termite,count=1",
        )],
    );
    add_card(&mut engine, "termite", Playstyle::Mammal, 1, 1, &[]);
    add_card(&mut engine, "stomper", Playstyle::Mammal, 5, 10, &[]);

    let mut pool = BTreeMap::new();
    pool.insert("termite".to_string(), 1u16);
    let with_copy = battle_with(
        &engine,
        &[minion(&engine.catalog, 1, "summoner")],
        &[minion(&engine.catalog, 2, "stomper")],
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
        &[minion(&engine.catalog, 3, "summoner")],
        &[minion(&engine.catalog, 4, "stomper")],
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
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "watcher",
        Playstyle::Mammal,
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
        Playstyle::Mammal,
        0,
        1,
        &[(Some("taunt"), "[]")],
    );
    add_card(
        &mut engine,
        "enemy-watcher",
        Playstyle::Mammal,
        0,
        10,
        &[(
            None,
            "OnFriendlyDeath|ModifyStats|self|attack=1,duration=permanent",
        )],
    );
    add_card(&mut engine, "brute", Playstyle::Mammal, 5, 5, &[]);

    let log = battle_first(
        &engine,
        &[
            minion(&engine.catalog, 1, "watcher"),
            minion(&engine.catalog, 2, "victim"),
        ],
        &[
            minion(&engine.catalog, 3, "enemy-watcher"),
            minion(&engine.catalog, 4, "brute"),
        ],
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
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "avenger",
        Playstyle::Mammal,
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
        Playstyle::Mammal,
        0,
        1,
        &[(Some("taunt"), "[]")],
    );

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "avenger")],
        &[minion(&engine.catalog, 2, "prey")],
    );

    assert_eq!(mods_attack(final_minion(&log, 1)), 2);
}

// ---------------------------------------------------------------------------
// 复苏与回手
// ---------------------------------------------------------------------------

fn action_count(log: &CombatLog, predicate: impl Fn(&CombatAction) -> bool) -> usize {
    log.actions
        .iter()
        .filter(|action| predicate(action))
        .count()
}

#[test]
fn revive_restores_with_bonus_and_only_once() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "reviver",
        Playstyle::Mammal,
        3,
        2,
        &[(None, "OnDeath|Revive|self|attack=2,health=2")],
    );
    add_card(&mut engine, "brute", Playstyle::Mammal, 10, 100, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "reviver")],
        &[minion(&engine.catalog, 2, "brute")],
    );

    let revived = action_count(
        &log,
        |action| matches!(action, CombatAction::Revived { minion } if *minion == 1),
    );
    assert_eq!(revived, 1, "复苏只结算一次");
    assert_eq!(
        deaths(&log).iter().filter(|id| **id == 1).count(),
        2,
        "随从死亡两次：第一次复苏，第二次不再复苏"
    );

    let final_minion = final_minion(&log, 1);
    assert_eq!(final_minion.stats.attack.layers(), 5, "3/2 以 +2/+2 复苏");
    assert_eq!(final_minion.stats.health.layers(), 4, "3/2 以 +2/+2 复苏");
    assert!(final_minion.revive_used, "复苏标记已使用");
    assert!(final_minion.reborn_used, "复苏后不再触发复生");
}

#[test]
fn recall_returns_minion_to_hand_with_growth() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "recallee",
        Playstyle::Mammal,
        1,
        1,
        &[
            (None, "OnCombatStart|Growth|self|times=2"),
            (None, "OnDeath|Recall|self|to=hand"),
        ],
    );
    add_card(&mut engine, "brute", Playstyle::Mammal, 10, 100, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "recallee")],
        &[minion(&engine.catalog, 2, "brute")],
    );

    assert_eq!(log.hand_minions.len(), 1, "死亡后回手一张");
    let hand = &log.hand_minions[0];
    assert_eq!(hand.seat, 0);
    assert_eq!(hand.minion.card_id, "recallee");
    assert_eq!(hand.minion.stats.attack.growth, 2, "保留成长");
    assert_eq!(hand.minion.stats.health.growth, 2, "保留成长");
    assert_eq!(
        hand.minion.stats.attack.layers(),
        3,
        "入场按当前成长后的数值"
    );
    assert_eq!(
        hand.minion.stats.health.layers(),
        3,
        "入场按当前成长后的数值"
    );
    assert_eq!(hand.minion.stats.health.current, 3, "回手清除战斗伤害");
    assert_eq!(
        action_count(&log, |action| matches!(
            action,
            CombatAction::Recalled { minion } if *minion == 1
        )),
        1
    );
}

#[test]
fn recall_is_dropped_when_hand_is_full() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "recallee",
        Playstyle::Mammal,
        1,
        1,
        &[(None, "OnDeath|Recall|self|to=hand")],
    );
    add_card(&mut engine, "brute", Playstyle::Mammal, 10, 100, &[]);

    let log = battle_with(
        &engine,
        &[minion(&engine.catalog, 1, "recallee")],
        &[minion(&engine.catalog, 2, "brute")],
        CombatSetup {
            attacker_hand: 3,
            hand_slots: 3,
            ..CombatSetup::default()
        },
    );

    assert!(log.hand_minions.is_empty(), "手牌已满时不记录回手");
    assert_eq!(
        action_count(&log, |action| matches!(
            action,
            CombatAction::Recalled { .. }
        )),
        0
    );
}

#[test]
fn golden_revive_resurrects_only_once() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "reviver",
        Playstyle::Mammal,
        3,
        2,
        &[(None, "OnDeath|Revive|self|attack=2,health=2")],
    );
    add_card(&mut engine, "brute", Playstyle::Mammal, 10, 100, &[]);
    // 金色 3/2 → 卡面 6/4；复苏 +2/+2 仍只结算一次。
    let golden = MinionInstance::new(&engine.catalog, 1, "reviver", true);

    let log = battle(&engine, &[golden], &[minion(&engine.catalog, 2, "brute")]);

    assert_eq!(
        action_count(&log, |action| matches!(
            action,
            CombatAction::Revived { minion } if *minion == 1
        )),
        1,
        "金色复苏只复活一次"
    );
    let final_minion = final_minion(&log, 1);
    assert_eq!(final_minion.stats.attack.layers(), 8, "6/4 以 +2/+2 复苏");
    assert_eq!(final_minion.stats.health.layers(), 6, "6/4 以 +2/+2 复苏");
}

#[test]
fn reborn_and_revive_are_mutually_exclusive_on_one_death() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "both",
        Playstyle::Mammal,
        0,
        1,
        &[
            (None, "OnCombatStart|DealDamage|self|amount=1"),
            (Some("reborn"), "[]"),
            (None, "OnDeath|Revive|self|attack=1,health=1"),
        ],
    );
    add_card(&mut engine, "brute", Playstyle::Mammal, 0, 100, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "both")],
        &[minion(&engine.catalog, 2, "brute")],
    );

    assert_eq!(deaths(&log).iter().filter(|id| **id == 1).count(), 1);
    assert_eq!(reborn_count(&log), 1, "复生生效");
    assert_eq!(
        action_count(&log, |action| matches!(
            action,
            CombatAction::Revived { .. }
        )),
        0,
        "同一次死亡不会既复生又复苏"
    );
    assert!(final_minion(&log, 1).health() > 0, "复生后仍存活");
}

#[test]
fn recall_preserves_granted_keywords_golden_and_contagion() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "recallee",
        Playstyle::Mammal,
        1,
        1,
        &[
            (
                None,
                "OnCombatStart|GrantKeyword|self|keyword=taunt,duration=permanent",
            ),
            (None, "OnDeath|Recall|self|to=hand"),
        ],
    );
    add_card(&mut engine, "brute", Playstyle::Mammal, 10, 100, &[]);

    let mut golden = MinionInstance::new(&engine.catalog, 1, "recallee", true);
    golden.contagion = 3;

    let log = battle(&engine, &[golden], &[minion(&engine.catalog, 2, "brute")]);

    assert_eq!(log.hand_minions.len(), 1);
    let hand = &log.hand_minions[0].minion;
    assert!(hand.golden, "保留金色");
    assert_eq!(hand.contagion, 3, "保留传染层数");
    assert!(
        hand.granted
            .iter()
            .any(|(keyword, duration)| keyword == "taunt" && *duration == Duration::Permanent),
        "保留已授予关键词"
    );
}

#[test]
fn recall_with_other_destination_is_not_returned_to_hand() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "recallee",
        Playstyle::Mammal,
        1,
        1,
        &[(None, "OnDeath|Recall|self|to=board")],
    );
    add_card(&mut engine, "brute", Playstyle::Mammal, 10, 100, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "recallee")],
        &[minion(&engine.catalog, 2, "brute")],
    );

    assert!(log.hand_minions.is_empty(), "to 非 hand 时不回手");
    assert_eq!(
        action_count(&log, |action| matches!(
            action,
            CombatAction::Recalled { .. }
        )),
        0
    );
}

#[test]
fn ghost_recall_is_not_written_into_solo_players_hand() {
    let (mut engine, mut state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "ghost-recallee",
        Playstyle::Mammal,
        1,
        1,
        &[(None, "OnDeath|Recall|self|to=hand")],
    );
    add_card(&mut engine, "brute", Playstyle::Mammal, 10, 100, &[]);

    state.phase = Phase::Combat;
    state.turn = 2;
    state.first_seat = 0;
    state.last_winner = None;
    state.pairings = vec![Pairing {
        a: 0,
        b: 0,
        ghost: Some(3),
    }];
    state.players[0]
        .board
        .push(minion(&engine.catalog, 1, "brute"));
    state.players[3].eliminated = true;
    state.players[3]
        .board
        .push(minion(&engine.catalog, 2, "ghost-recallee"));

    engine.resolve_combat(&mut state).expect("幽灵战应成功");

    assert!(
        state.players[0].hand.is_empty(),
        "幽灵侧回手不得写进落单玩家手牌"
    );
    assert!(state.players[3].hand.is_empty(), "幽灵座位也不写回手牌");
}

#[test]
fn ghost_recall_is_not_reported_as_combat_action() {
    let (mut engine, mut state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "ghost-recallee",
        Playstyle::Mammal,
        1,
        1,
        &[(None, "OnDeath|Recall|self|to=hand")],
    );
    add_card(&mut engine, "brute", Playstyle::Mammal, 10, 100, &[]);

    state.phase = Phase::Combat;
    state.turn = 2;
    state.first_seat = 0;
    state.last_winner = None;
    state.pairings = vec![Pairing {
        a: 0,
        b: 0,
        ghost: Some(3),
    }];
    state.players[0]
        .board
        .push(minion(&engine.catalog, 1, "brute"));
    state.players[3].eliminated = true;
    state.players[3]
        .board
        .push(minion(&engine.catalog, 2, "ghost-recallee"));

    let outcome = engine.resolve_combat(&mut state).expect("幽灵战应成功");
    let report = outcome.reports.get(&0).expect("幽灵战应有报告");
    let recalled: Vec<InstanceId> = report
        .actions
        .iter()
        .filter_map(|action| match action {
            CombatAction::Recalled { minion } => Some(*minion),
            _ => None,
        })
        .collect();
    assert!(
        recalled.is_empty(),
        "幽灵侧回手没有对应手牌写入，不应出现在回放动作中：{recalled:?}"
    );
}

#[test]
fn ghost_copy_to_hand_is_not_written_into_solo_players_hand() {
    let (mut engine, mut state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "ghost-copier",
        Playstyle::Mammal,
        0,
        100,
        &[(None, "OnCombatStart|Copy|self|to=hand")],
    );
    add_card(&mut engine, "brute", Playstyle::Mammal, 10, 100, &[]);

    state.phase = Phase::Combat;
    state.turn = 2;
    state.first_seat = 0;
    state.last_winner = None;
    state.pairings = vec![Pairing {
        a: 0,
        b: 0,
        ghost: Some(3),
    }];
    state.players[0]
        .board
        .push(minion(&engine.catalog, 1, "brute"));
    state.players[3].eliminated = true;
    state.players[3]
        .board
        .push(minion(&engine.catalog, 2, "ghost-copier"));

    engine.resolve_combat(&mut state).expect("幽灵战应成功");

    assert!(
        state.players[0].hand.is_empty(),
        "幽灵侧复制不得写进落单玩家手牌"
    );
    assert!(state.players[3].hand.is_empty(), "幽灵座位也不写回复制牌");
}

#[test]
fn revive_does_not_replay_deathrattle_within_one_death_event() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "reviver",
        Playstyle::Mammal,
        0,
        1,
        &[
            (None, "OnCombatStart|DealDamage|self|amount=1"),
            (
                Some("deathrattle"),
                "OnDeath|ModifyStats|friendly:all|attack=1,duration=permanent",
            ),
            (None, "OnDeath|Revive|self|attack=1,health=1"),
        ],
    );
    add_card(&mut engine, "friend", Playstyle::Mammal, 0, 10, &[]);
    add_card(&mut engine, "brute", Playstyle::Mammal, 0, 100, &[]);

    let log = battle(
        &engine,
        &[
            minion(&engine.catalog, 1, "reviver"),
            minion(&engine.catalog, 2, "friend"),
        ],
        &[minion(&engine.catalog, 3, "brute")],
    );

    assert_eq!(deaths(&log).iter().filter(|id| **id == 1).count(), 1);
    assert_eq!(
        action_count(&log, |action| matches!(
            action,
            CombatAction::Revived { minion } if *minion == 1
        )),
        1,
        "复苏只结算一次"
    );
    assert_eq!(
        mods_attack(final_minion(&log, 2)),
        1,
        "同一次死亡事件内亡语只触发一次"
    );
}

// ---------------------------------------------------------------------------
// 战斗开始
// ---------------------------------------------------------------------------

#[test]
fn combat_start_order_heroes_then_minions() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "hero",
        Playstyle::Mammal,
        0,
        30,
        &[(None, "OnCombatStart|DealDamage|enemy:all|amount=5")],
    );
    mirror_as_hero(&mut engine, "hero");
    add_card(
        &mut engine,
        "minion0",
        Playstyle::Mammal,
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
        Playstyle::Mammal,
        0,
        1,
        &[(
            None,
            "OnCombatStart|ModifyStats|self|attack=1,duration=permanent",
        )],
    );

    let log = battle_with(
        &engine,
        &[minion(&engine.catalog, 1, "minion0")],
        &[minion(&engine.catalog, 2, "defender0")],
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
fn combat_start_hero_effects_read_hero_pool() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "hero-only",
        Playstyle::Mammal,
        0,
        30,
        &[(None, "OnCombatStart|DealDamage|enemy:all|amount=5")],
    );
    mirror_as_hero(&mut engine, "hero-only");
    // 同名随从卡移除技能，仅英雄池保留：战斗开始仍应打出 5 点伤害。
    engine
        .catalog
        .cards
        .get_mut("hero-only")
        .expect("同名随从卡应存在")
        .skills
        .clear();
    add_card(&mut engine, "tank", Playstyle::Mammal, 0, 10, &[]);
    add_card(&mut engine, "defender", Playstyle::Mammal, 0, 5, &[]);

    let log = battle_with(
        &engine,
        &[minion(&engine.catalog, 1, "tank")],
        &[minion(&engine.catalog, 2, "defender")],
        CombatSetup {
            attacker_hero: Some("hero-only"),
            ..CombatSetup::default()
        },
    );

    assert!(deaths(&log).contains(&2), "英雄战斗开始技能只从英雄池读取");
}

#[test]
fn delayed_battlecry_enemy_target_runs_at_combat_start() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "delayed",
        Playstyle::Mammal,
        1,
        1,
        &[(Some("battlecry"), "OnPlay|DealDamage|enemy:random|amount=3")],
    );
    add_card(&mut engine, "target", Playstyle::Mammal, 0, 3, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "delayed")],
        &[minion(&engine.catalog, 2, "target")],
    );

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
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "hero",
        Playstyle::Mammal,
        0,
        30,
        &[(None, "OnCombatStart|DealDamage|enemy:all|amount=5")],
    );
    mirror_as_hero(&mut engine, "hero");
    add_card(
        &mut engine,
        "delayed",
        Playstyle::Mammal,
        1,
        1,
        &[(
            Some("battlecry"),
            "OnPlay|DealDamage|enemy:random|amount=10",
        )],
    );
    add_card(&mut engine, "tank", Playstyle::Mammal, 0, 10, &[]);

    let log = battle_with(
        &engine,
        &[minion(&engine.catalog, 1, "tank")],
        &[minion(&engine.catalog, 2, "delayed")],
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
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "copier",
        Playstyle::Mammal,
        0,
        10,
        &[(None, "OnCombatStart|Copy|self|to=hand")],
    );

    let log = battle_with(
        &engine,
        &[minion(&engine.catalog, 1, "copier")],
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
    let (mut engine, _state) = setup_with_gold(31);
    add_token(&mut engine, "spawn", 1, 1);
    add_card(
        &mut engine,
        "summoner",
        Playstyle::Mammal,
        1,
        5,
        &[(None, "OnCombatStart|Summon|self|token=spawn,count=1")],
    );
    add_card(
        &mut engine,
        "observer",
        Playstyle::Mammal,
        0,
        10,
        &[(
            None,
            "OnAllySummon|ModifyStats|self|attack=1,duration=permanent",
        )],
    );

    let log = battle(
        &engine,
        &[
            minion(&engine.catalog, 1, "summoner"),
            minion(&engine.catalog, 2, "observer"),
        ],
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
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "loop-a",
        Playstyle::Mammal,
        0,
        10,
        &[
            (None, "OnCombatStart|Growth|self|times=1"),
            (None, "OnAllyGrowth|Growth|self|times=1"),
        ],
    );
    add_card(
        &mut engine,
        "loop-b",
        Playstyle::Mammal,
        0,
        10,
        &[(None, "OnAllyGrowth|Growth|self|times=1")],
    );

    let log = battle(
        &engine,
        &[
            minion(&engine.catalog, 1, "loop-a"),
            minion(&engine.catalog, 2, "loop-b"),
        ],
        &[],
    );

    assert_eq!(log.trigger_count, 200, "单场战斗触发上限为 200");
}

// ---------------------------------------------------------------------------
// 回合开始传染致死
// ---------------------------------------------------------------------------

#[test]
fn turn_start_contagion_death_fires_deathrattle_and_sow() {
    let (mut engine, mut state) = setup_with_gold(31);
    add_token(&mut engine, "seedling", 1, 1);
    add_card(
        &mut engine,
        "dying",
        Playstyle::Plant,
        1,
        1,
        &[
            (
                Some("deathrattle"),
                "OnDeath|ModifyStats|friendly:all|attack=1,duration=permanent",
            ),
            (Some("sow"), "[]"),
        ],
    );
    add_card(&mut engine, "friend", Playstyle::Plant, 0, 5, &[]);

    state.players[0]
        .board
        .push(minion(&engine.catalog, 1, "dying"));
    state.players[0]
        .board
        .push(minion(&engine.catalog, 2, "friend"));
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

/// §2.7.1：英雄被动与随从效果都按「先手方 → 后手方」结算。
///
/// 本用例让防守方成为先手方：
/// - 先手英雄（防守方）的范围伤害先击杀己方 5 血随从 P，后手英雄的 `both:all`
///   增益因此选不到已死亡的 P；
/// - 先手随从（防守方）的范围伤害在后手随从 N 自己的 `OnCombatStart` 之前击杀 N，
///   所以 N 只留下后手英雄施加的 +1 攻击。
#[test]
fn combat_start_heroes_follow_first_side_order() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "hero-a",
        Playstyle::Mammal,
        0,
        30,
        &[(None, "OnCombatStart|DealDamage|both:all|amount=5")],
    );
    mirror_as_hero(&mut engine, "hero-a");
    add_card(
        &mut engine,
        "hero-b",
        Playstyle::Mammal,
        0,
        30,
        &[(
            None,
            "OnCombatStart|ModifyStats|both:all|attack=1,duration=permanent",
        )],
    );
    mirror_as_hero(&mut engine, "hero-b");
    add_card(&mut engine, "p", Playstyle::Mammal, 0, 5, &[]);
    add_card(
        &mut engine,
        "n",
        Playstyle::Mammal,
        0,
        10,
        &[(
            None,
            "OnCombatStart|ModifyStats|self|attack=1,duration=permanent",
        )],
    );
    add_card(
        &mut engine,
        "d",
        Playstyle::Mammal,
        0,
        20,
        &[(None, "OnCombatStart|DealDamage|enemy:all|amount=10")],
    );

    // 防守方（座位 1）为先手方。
    let log = simulate_with(
        &engine,
        &[minion(&engine.catalog, 1, "n")],
        &[
            minion(&engine.catalog, 2, "p"),
            minion(&engine.catalog, 3, "d"),
        ],
        0,
        1,
        1,
        &mut rng(),
        CombatSetup {
            attacker_hero: Some("hero-b"),
            defender_hero: Some("hero-a"),
            ..CombatSetup::default()
        },
    );

    assert_eq!(
        mods_attack(final_minion(&log, 2)),
        0,
        "先手方（防守方）英雄先结算，已死亡的 P 不再获得后手英雄的增益"
    );
    assert_eq!(
        mods_attack(final_minion(&log, 1)),
        1,
        "先手方随从先结算，N 在被击杀前没有机会触发自身效果（只留下后手英雄的 +1）"
    );
    assert_eq!(
        mods_attack(final_minion(&log, 3)),
        1,
        "后手方英雄的增益仍应结算到存活随从上"
    );
}
