//! T07：击杀归属。
//!
//! 覆盖攻击击杀、反击击杀、效果伤害击杀、友方击杀只触发 `OnKill`、
//! 同时死亡（互相击杀）不触发、复生后再次击杀，以及无来源伤害不产生击杀。

mod common;

use common::{effects_of, minion, setup_with_gold};
use rule_engine::combat::{CombatAction, CombatLog, CombatSetup, simulate_with};
use rule_engine::data::{Card, CardType, Playstyle, Skill};
use rule_engine::engine::Engine;
use rule_engine::rng::RngStreams;
use rule_engine::state::{InstanceId, MinionInstance, Seat};

/// 插入一张合成随从卡，携带若干 `(keyword, effects)` 技能。
fn add_card(
    engine: &mut Engine,
    id: &str,
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
            playstyle: Playstyle::Mammal,
            taxon: "hominidae".to_string(),
            specialty: Vec::new(),
            subtype: None,
            level: 1,
            era: None,
            attack: Some(attack),
            health: Some(health),
            skills: skill_ids,
            is_scholar: false,
            is_hero: false,
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
        &mut RngStreams::from_seed([17u8; 32]),
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

fn deaths(log: &CombatLog) -> Vec<InstanceId> {
    log.actions
        .iter()
        .filter_map(|action| match action {
            CombatAction::Death { minion } => Some(*minion),
            _ => None,
        })
        .collect()
}

#[test]
fn attack_kill_triggers_on_kill_for_attacker() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "hunter",
        5,
        5,
        &[(None, "OnKill|ModifyStats|self|attack=1,duration=permanent")],
    );
    add_card(&mut engine, "prey", 0, 2, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "hunter")],
        &[minion(&engine.catalog, 2, "prey")],
        0,
    );

    assert_eq!(deaths(&log), vec![2]);
    assert_eq!(
        final_minion(&log, 1).stats.attack.permanent,
        1,
        "攻击击杀应触发击杀者的 OnKill"
    );
}

#[test]
fn counterattack_kill_triggers_on_kill_for_defender() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(&mut engine, "weakling", 1, 1, &[]);
    add_card(
        &mut engine,
        "guard",
        3,
        10,
        &[(None, "OnKill|ModifyStats|self|attack=1,duration=permanent")],
    );

    // 防守方先手：弱鸡主动攻击守卫并被反击打死。
    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "weakling")],
        &[minion(&engine.catalog, 2, "guard")],
        1,
    );

    assert_eq!(deaths(&log).first(), Some(&1));
    assert_eq!(
        final_minion(&log, 2).stats.attack.permanent,
        1,
        "反击击杀应归因给反击者"
    );
}

#[test]
fn effect_damage_kill_triggers_on_kill_for_caster() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "caster",
        1,
        3,
        &[
            (
                Some("battlecry"),
                "OnCombatStart|DealDamage|enemy:all|amount=9",
            ),
            (None, "OnKill|ModifyStats|self|attack=2,duration=permanent"),
        ],
    );
    add_card(&mut engine, "prey", 0, 2, &[]);

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "caster")],
        &[minion(&engine.catalog, 2, "prey")],
        0,
    );

    // 战斗开始效果打死猎物：击杀归因施放者。
    assert_eq!(deaths(&log).first(), Some(&2));
    assert_eq!(
        final_minion(&log, 1).stats.attack.permanent,
        2,
        "效果伤害击杀应触发施放者的 OnKill"
    );
}

#[test]
fn hero_effect_has_no_kill_attribution() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(&mut engine, "caster", 1, 3, &[]);
    add_card(
        &mut engine,
        "prey",
        0,
        2,
        &[(None, "OnKill|ModifyStats|self|attack=2,duration=permanent")],
    );
    add_card(
        &mut engine,
        "hero",
        0,
        0,
        &[(Some("hero"), "OnCombatStart|DealDamage|enemy:all|amount=9")],
    );

    let log = simulate_with(
        &engine,
        &[minion(&engine.catalog, 1, "caster")],
        &[minion(&engine.catalog, 2, "prey")],
        0,
        1,
        0,
        &mut RngStreams::from_seed([17u8; 32]),
        CombatSetup {
            attacker_hero: Some("hero"),
            ..CombatSetup::default()
        },
    );

    assert!(deaths(&log).contains(&2), "英雄被动打死了猎物");
    assert_eq!(
        final_minion(&log, 2).stats.attack.permanent,
        0,
        "英雄技能不归因随从，不产生击杀"
    );
}

#[test]
fn friendly_kill_triggers_on_kill_but_not_on_ally_kill() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "bomber",
        0,
        5,
        &[
            (
                Some("battlecry"),
                "OnCombatStart|DealDamage|friendly:all:other=true|amount=9",
            ),
            (None, "OnKill|ModifyStats|self|attack=1,duration=permanent"),
            (
                None,
                "OnAllyKill|ModifyStats|self|attack=10,duration=permanent",
            ),
        ],
    );
    add_card(
        &mut engine,
        "ally",
        0,
        1,
        &[(
            None,
            "OnAllyKill|ModifyStats|self|attack=3,duration=permanent",
        )],
    );

    let log = battle(
        &engine,
        &[
            minion(&engine.catalog, 1, "bomber"),
            minion(&engine.catalog, 2, "ally"),
        ],
        &[],
        0,
    );

    assert!(deaths(&log).contains(&2), "友方随从被击杀");
    assert_eq!(
        final_minion(&log, 1).stats.attack.permanent,
        1,
        "击杀友方只触发 OnKill，不触发 OnAllyKill"
    );
}

#[test]
fn ally_kill_triggers_for_all_living_allies_including_killer() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "hunter",
        5,
        5,
        &[(
            None,
            "OnAllyKill|ModifyStats|self|attack=1,duration=permanent",
        )],
    );
    add_card(
        &mut engine,
        "cheerer",
        0,
        5,
        &[(
            None,
            "OnAllyKill|ModifyStats|self|attack=2,duration=permanent",
        )],
    );
    add_card(&mut engine, "prey", 0, 2, &[]);

    let log = battle(
        &engine,
        &[
            minion(&engine.catalog, 1, "hunter"),
            minion(&engine.catalog, 2, "cheerer"),
        ],
        &[minion(&engine.catalog, 3, "prey")],
        0,
    );

    assert!(deaths(&log).contains(&3), "猎物应被击杀");
    assert_eq!(
        final_minion(&log, 1).stats.attack.permanent,
        1,
        "击杀者自身也响应 OnAllyKill"
    );
    assert_eq!(
        final_minion(&log, 2).stats.attack.permanent,
        2,
        "其他存活友方响应 OnAllyKill"
    );
}

#[test]
fn mutual_death_triggers_no_kill_hooks() {
    let (mut engine, _state) = setup_with_gold(31);
    // 攻击者击杀目标后死于自身的 OnDealDamage 效果：同一次交换双方都死亡。
    add_card(
        &mut engine,
        "suicide",
        5,
        10,
        &[
            (None, "OnDealDamage|DealDamage|self|amount=99"),
            (None, "OnKill|ModifyStats|self|attack=1,duration=permanent"),
        ],
    );
    add_card(
        &mut engine,
        "witness",
        0,
        10,
        &[(
            None,
            "OnAllyKill|ModifyStats|self|attack=1,duration=permanent",
        )],
    );
    add_card(&mut engine, "victim", 0, 1, &[]);

    let log = battle(
        &engine,
        &[
            minion(&engine.catalog, 1, "suicide"),
            minion(&engine.catalog, 2, "witness"),
        ],
        &[minion(&engine.catalog, 3, "victim")],
        0,
    );

    assert!(deaths(&log).contains(&1), "攻击者自身死亡");
    assert!(deaths(&log).contains(&3), "被击杀者死亡");
    assert_eq!(
        final_minion(&log, 2).stats.attack.permanent,
        0,
        "互相击杀的友方存活随从不应响应 OnAllyKill"
    );
}

#[test]
fn reborn_then_kill_triggers_on_kill_again() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "reaper",
        1,
        10,
        &[(None, "OnKill|ModifyStats|self|attack=1,duration=permanent")],
    );
    add_card(
        &mut engine,
        "phoenix",
        0,
        1,
        &[(Some("reborn"), "[]"), (Some("taunt"), "[]")],
    );

    let log = battle(
        &engine,
        &[minion(&engine.catalog, 1, "reaper")],
        &[minion(&engine.catalog, 2, "phoenix")],
        0,
    );

    assert!(
        log.actions
            .iter()
            .any(|action| matches!(action, CombatAction::Reborn { minion: 2 })),
        "被击杀者应复生"
    );
    assert_eq!(
        deaths(&log).iter().filter(|id| **id == 2).count(),
        2,
        "复生后再次被击杀"
    );
    assert_eq!(
        final_minion(&log, 1).stats.attack.permanent,
        2,
        "两次击杀各触发一次 OnKill"
    );
}

#[test]
fn kill_hook_chain_processes_new_kill_records() {
    let (mut engine, _state) = setup_with_gold(31);
    add_card(
        &mut engine,
        "archer",
        5,
        5,
        &[(None, "OnKill|DealDamage|enemy:random|amount=9")],
    );
    add_card(
        &mut engine,
        "witness",
        0,
        10,
        &[(
            None,
            "OnAllyKill|ModifyStats|self|attack=1,duration=permanent",
        )],
    );
    add_card(&mut engine, "prey-a", 0, 2, &[]);
    add_card(&mut engine, "prey-b", 0, 2, &[]);

    let log = battle(
        &engine,
        &[
            minion(&engine.catalog, 1, "archer"),
            minion(&engine.catalog, 2, "witness"),
        ],
        &[
            minion(&engine.catalog, 3, "prey-a"),
            minion(&engine.catalog, 4, "prey-b"),
        ],
        0,
    );

    assert!(deaths(&log).contains(&3));
    assert!(
        deaths(&log).contains(&4),
        "OnKill 效果造成的第二次击杀应进入死亡队列"
    );
    assert_eq!(
        final_minion(&log, 2).stats.attack.permanent,
        2,
        "两条击杀记录都触发 OnAllyKill"
    );
}
