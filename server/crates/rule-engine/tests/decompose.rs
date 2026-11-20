//! T07：分解原语。
//!
//! 覆盖 `OnKill` / `OnFriendlyDeath` / `OnAllyKill` 三种触发、`original` /
//! `combat_start` / `pre_kill` 三种口径、`permanent` / `combat` 两种时长、
//! 金色双触发、多分解随从独立触发、缺失事件目标静默跳过，以及复生不撤销
//! 死亡快照。

mod common;

use common::{categories_for, effects_of, minion, setup_with_gold};
use rule_engine::combat::{CombatLog, CombatSetup, simulate_with};
use rule_engine::data::{Card, CardType, Playstyle, Skill};
use rule_engine::engine::Engine;
use rule_engine::rng::RngStreams;
use rule_engine::state::{InstanceId, MinionInstance, Seat};

fn add_card(
    engine: &mut Engine,
    id: &str,
    attack: i32,
    health: i32,
    skills: &[(Option<&str>, &str)],
) {
    add_card_at(engine, id, attack, health, "agaricales", skills);
}

fn add_card_at(
    engine: &mut Engine,
    id: &str,
    attack: i32,
    health: i32,
    taxon: &str,
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
            taxon: taxon.to_string(),
            categories: categories_for(Playstyle::Fungus),
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

fn battle(
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
        &mut RngStreams::from_seed([23u8; 32]),
        CombatSetup::default(),
    )
}

fn final_minion(log: &CombatLog, id: InstanceId) -> &MinionInstance {
    log.attacker_final
        .iter()
        .chain(log.defender_final.iter())
        .find(|minion| minion.instance_id == id)
        .expect("战斗结束时随从仍在快照中")
}

fn decomposed(engine: &mut Engine, id: &str, basis: &str, duration: &str) {
    add_card_at(
        engine,
        id,
        5,
        10,
        "hominidae",
        &[(
            None,
            &format!("OnKill|Decompose|self|basis={basis},duration={duration}"),
        )],
    );
}

#[test]
fn on_kill_original_permanent_adds_base_health() {
    let (mut engine, _state) = setup_with_gold(31);
    decomposed(&mut engine, "decomposer", "original", "permanent");
    add_card(&mut engine, "corpse", 0, 4, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "decomposer")],
        &[minion(&engine.catalog, 2, "corpse")],
        0,
    );

    let health = final_minion(&log, 1).stats.health;
    assert_eq!(health.permanent, 4, "original 取卡面基础生命值");
}

#[test]
fn when_filters_the_decomposed_victim() {
    // `when` 按被分解者（事件载荷）筛选：分类不符时不结算，符合同一次战斗内照常结算。
    let run = |victim_taxon: &str, victim_id: InstanceId| {
        let (mut engine, _state) = setup_with_gold(31);
        add_card_at(
            &mut engine,
            "decomposer",
            5,
            10,
            "hominidae",
            &[(
                None,
                "OnKill|Decompose|self|basis=original,duration=permanent,when=taxon=hominidae",
            )],
        );
        add_card_at(&mut engine, "corpse", 0, 4, victim_taxon, &[]);
        let log = battle(
            &engine,
            &[minion(&engine.catalog, 1, "decomposer")],
            &[minion(&engine.catalog, victim_id, "corpse")],
            0,
        );
        final_minion(&log, 1).stats.health.permanent
    };

    assert_eq!(run("hominidae", 2), 4, "被分解者符合 when 时正常获得生命值");
    assert_eq!(run("agaricales", 2), 0, "被分解者不符合 when 时不结算");
}

#[test]
fn combat_start_basis_reads_entry_snapshot() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "decomposer",
        5,
        10,
        &[(
            None,
            "OnKill|Decompose|self|basis=combat_start,duration=permanent",
        )],
    );
    add_card(&mut engine, "corpse", 0, 4, &[]);

    // 战前永久生命加成只进入 combat_start 快照，不改变卡面基础值。
    let mut corpse = minion(&engine.catalog, 2, "corpse");
    corpse.stats.health.add_permanent(3);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "decomposer")],
        &[corpse],
        0,
    );

    let health = final_minion(&log, 1).stats.health;
    assert_eq!(health.permanent, 7, "combat_start 取入场时的当前生命");
}

#[test]
fn pre_kill_basis_reads_health_before_lethal_damage() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "decomposer",
        6,
        10,
        &[(
            None,
            "OnKill|Decompose|self|basis=pre_kill,duration=permanent",
        )],
    );
    add_card(&mut engine, "corpse", 0, 10, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "decomposer")],
        &[minion(&engine.catalog, 2, "corpse")],
        0,
    );

    let health = final_minion(&log, 1).stats.health;
    assert_eq!(
        health.permanent, 4,
        "pre_kill 取致命伤害前的当前生命（10 → 4 → 击杀）"
    );
}

#[test]
fn combat_duration_uses_turn_layer() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "decomposer",
        5,
        10,
        &[(None, "OnKill|Decompose|self|basis=original,duration=combat")],
    );
    add_card(&mut engine, "corpse", 0, 4, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "decomposer")],
        &[minion(&engine.catalog, 2, "corpse")],
        0,
    );

    let health = final_minion(&log, 1).stats.health;
    assert_eq!(health.permanent, 0, "combat 时长不落永久层");
    assert_eq!(health.turn, 4, "combat 时长写入本回合层");
}

#[test]
fn golden_decomposer_triggers_twice() {
    let (mut engine, _state) = setup_with_gold(31);
    decomposed(&mut engine, "decomposer", "original", "permanent");
    add_card(&mut engine, "corpse", 0, 4, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "decomposer")],
        &[minion(&engine.catalog, 2, "corpse")],
        0,
    );
    let golden_log = battle(
        &engine,
        &[MinionInstance::new(&engine.catalog, 11, "decomposer", true)],
        &[minion(&engine.catalog, 2, "corpse")],
        0,
    );

    assert_eq!(final_minion(&log, 1).stats.health.permanent, 4);
    assert_eq!(
        final_minion(&golden_log, 11).stats.health.permanent,
        8,
        "金色来源整段效果执行两遍"
    );
}

#[test]
fn multiple_decomposers_trigger_independently_on_ally_kill() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(&mut engine, "killer", 5, 10, &[]);
    add_card(
        &mut engine,
        "d1",
        0,
        10,
        &[(
            None,
            "OnAllyKill|Decompose|self|basis=original,duration=permanent",
        )],
    );
    add_card(
        &mut engine,
        "d2",
        0,
        10,
        &[(
            None,
            "OnAllyKill|Decompose|self|basis=original,duration=permanent",
        )],
    );
    add_card(&mut engine, "prey", 0, 3, &[]);

    let log = battle(
        &engine,
        &[
            minion(&engine.catalog, 1, "killer"),
            minion(&engine.catalog, 2, "d1"),
            minion(&engine.catalog, 3, "d2"),
        ],
        &[minion(&engine.catalog, 4, "prey")],
        0,
    );

    assert_eq!(
        final_minion(&log, 2).stats.health.permanent,
        3,
        "第一个分解随从独立结算"
    );
    assert_eq!(
        final_minion(&log, 3).stats.health.permanent,
        3,
        "第二个分解随从独立结算"
    );
}

#[test]
fn friendly_death_trigger_decomposes_dead_ally() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "watcher",
        0,
        10,
        &[(
            None,
            "OnFriendlyDeath|Decompose|self|basis=original,duration=permanent",
        )],
    );
    add_card(&mut engine, "martyr", 0, 3, &[(Some("taunt"), "[]")]);
    add_card(&mut engine, "executioner", 5, 10, &[]);

    let log = battle(
        &engine,
        &[
            minion(&engine.catalog, 1, "watcher"),
            minion(&engine.catalog, 2, "martyr"),
        ],
        &[minion(&engine.catalog, 3, "executioner")],
        1,
    );

    assert_eq!(
        final_minion(&log, 1).stats.health.permanent,
        3,
        "友方死亡触发分解，取死者基础生命"
    );
}

#[test]
fn missing_event_target_is_silent() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "idle",
        0,
        10,
        &[(
            None,
            "OnCombatStart|Decompose|self|basis=original,duration=permanent",
        )],
    );
    add_card(&mut engine, "wall", 0, 10, &[]);

    // 战斗开始没有事件目标：不获得生命值、不报错。
    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "idle")],
        &[minion(&engine.catalog, 2, "wall")],
        0,
    );

    let health = final_minion(&log, 1).stats.health;
    assert_eq!(health.permanent, 0);
}

#[test]
fn reborn_does_not_reset_death_snapshot() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "watcher",
        0,
        10,
        &[(
            None,
            "OnFriendlyDeath|Decompose|self|basis=pre_kill,duration=permanent",
        )],
    );
    add_card(
        &mut engine,
        "phoenix",
        0,
        6,
        &[(Some("reborn"), "[]"), (Some("taunt"), "[]")],
    );
    add_card(&mut engine, "executioner", 4, 10, &[]);

    let log = battle(
        &engine,
        &[
            minion(&engine.catalog, 1, "watcher"),
            minion(&engine.catalog, 2, "phoenix"),
        ],
        &[minion(&engine.catalog, 3, "executioner")],
        1,
    );

    // 第一次死亡：6 → 2 → 致命，快照 2；复生后第二次死亡：1 → 致命，快照 1。
    assert_eq!(
        final_minion(&log, 1).stats.health.permanent,
        3,
        "复生不撤销已触发的死亡事件，快照仍按死前生命结算"
    );
}
