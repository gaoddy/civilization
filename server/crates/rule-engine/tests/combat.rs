mod common;

use std::collections::BTreeSet;

use common::test_catalog;
use rule_engine::combat::{
    CombatAction, CombatLog, effective_attack, effective_attack_with, effective_health,
    effective_health_with, keywords, simulate, simulate_with,
};
use rule_engine::data::{Card, CardType, Catalog, Skill, Tribe};
use rule_engine::effects::Duration;
use rule_engine::engine::Engine;
use rule_engine::hash::state_hash;
use rule_engine::rng::RngStreams;
use rule_engine::settle::Pairing;
use rule_engine::state::{AuraEffect, InstanceId, MinionInstance, Modifier, Phase, Seat};

/// 往测试目录里插入一张合成卡牌，避免测试依赖真实卡面数值。
fn add_card_with(
    catalog: &mut Catalog,
    id: &str,
    tribe: Tribe,
    attack: i32,
    health: i32,
    keyword_list: &[&str],
) {
    let mut skill_ids = Vec::new();
    for (index, keyword) in keyword_list.iter().enumerate() {
        let skill_id = format!("{id}-skill-{index}");
        catalog.skills.insert(
            skill_id.clone(),
            Skill {
                id: skill_id.clone(),
                name: None,
                keyword: Some((*keyword).to_string()),
                text: String::new(),
                effects: String::new(),
                cost: None,
                uses_per_turn: None,
                match_text: None,
            },
        );
        skill_ids.push(skill_id);
    }
    catalog.cards.insert(
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

fn add_card(catalog: &mut Catalog, id: &str, attack: i32, health: i32, keyword_list: &[&str]) {
    add_card_with(catalog, id, Tribe::Human, attack, health, keyword_list);
}

/// 给已有卡牌追加一个带效果串的技能，用于构造召唤、复制等战斗内效果。
fn add_effect_skill(
    catalog: &mut Catalog,
    card_id: &str,
    skill_id: &str,
    keyword: &str,
    effects: &str,
) {
    catalog.skills.insert(
        skill_id.to_string(),
        Skill {
            id: skill_id.to_string(),
            name: None,
            keyword: Some(keyword.to_string()),
            text: String::new(),
            effects: effects.to_string(),
            cost: None,
            uses_per_turn: None,
            match_text: None,
        },
    );
    catalog
        .cards
        .get_mut(card_id)
        .expect("卡牌应存在")
        .skills
        .push(skill_id.to_string());
}

fn add_seedling_token(catalog: &mut Catalog) {
    catalog.tokens.insert(
        "seedling".to_string(),
        rule_engine::data::Token {
            id: "seedling".to_string(),
            name: "幼苗".to_string(),
            attack: 1,
            health: 1,
            keywords: Vec::new(),
        },
    );
}

fn minion(catalog: &Catalog, instance_id: InstanceId, card_id: &str) -> MinionInstance {
    MinionInstance::new(catalog, instance_id, card_id, false)
}

fn rng() -> RngStreams {
    RngStreams::from_seed([7; 32])
}

fn attacks(log: &CombatLog) -> Vec<(InstanceId, InstanceId, i32, bool)> {
    log.actions
        .iter()
        .filter_map(|action| match action {
            CombatAction::Attack {
                attacker,
                target,
                damage,
                blocked,
            } => Some((*attacker, *target, *damage, *blocked)),
            _ => None,
        })
        .collect()
}

fn death_order(log: &CombatLog) -> Vec<InstanceId> {
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

fn contagion_count(log: &CombatLog) -> usize {
    log.actions
        .iter()
        .filter(|action| matches!(action, CombatAction::ContagionApplied { .. }))
        .count()
}

#[test]
fn attack_kills_target_without_counterattack() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 3, 3, &[]);
    add_card(&mut catalog, "b", 2, 3, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "b")],
        0,
        1,
        0,
        &mut rng(),
    );

    // A 打 B 3 点（B 死于 0 血）；死亡目标不再反击。
    assert_eq!(attacks(&log), vec![(1, 2, 3, false)], "只记录主伤害事件");
    assert_eq!(death_order(&log), vec![2]);
    assert_eq!(log.attacker_survivors, vec![1], "攻击方未受反击伤害");
    assert!(log.defender_survivors.is_empty());
    assert_eq!(log.winner, Some(0));
    assert_eq!(log.action_count, 1);
}

#[test]
fn surviving_target_counters_and_kills_attacker() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 2, 2, &[]);
    add_card(&mut catalog, "b", 3, 3, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "b")],
        0,
        1,
        0,
        &mut rng(),
    );

    assert_eq!(
        attacks(&log),
        vec![(1, 2, 2, false), (2, 1, 3, false)],
        "主伤害 2 后目标仍存活，反击 3 击杀攻击方"
    );
    assert_eq!(death_order(&log), vec![1]);
    assert!(log.attacker_survivors.is_empty());
    assert_eq!(log.defender_survivors, vec![2]);
    assert_eq!(log.winner, Some(1));
}

#[test]
fn taunt_forces_target() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 2, 10, &[]);
    add_card(&mut catalog, "taunt", 1, 10, &["嘲讽"]);
    add_card(&mut catalog, "plain", 1, 10, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "plain"), minion(&catalog, 3, "taunt")],
        0,
        1,
        0,
        &mut rng(),
    );

    assert_eq!(attacks(&log)[0], (1, 3, 2, false), "必须只攻击嘲讽随从");
}

#[test]
fn divine_shield_blocks_one_hit() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 2, 10, &["连击"]);
    add_card(&mut catalog, "b", 0, 5, &["护盾"]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "b")],
        0,
        1,
        0,
        &mut rng(),
    );

    let attack_list = attacks(&log);
    assert_eq!(attack_list[0], (1, 2, 0, true), "第一击被护盾抵消");
    assert!(
        attack_list.contains(&(1, 2, 2, false)),
        "护盾耗尽后同一随从再次命中并掉血"
    );
}

#[test]
fn windfury_attacks_twice() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a1", 0, 10, &["连击"]);
    add_card(&mut catalog, "a2", 0, 10, &[]);
    add_card(&mut catalog, "b", 0, 20, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a1"), minion(&catalog, 2, "a2")],
        &[minion(&catalog, 3, "b")],
        0,
        1,
        0,
        &mut rng(),
    );

    let attacker_ids: Vec<InstanceId> = attacks(&log)
        .into_iter()
        .map(|event| event.0)
        .filter(|id| *id != 3)
        .collect();
    assert_eq!(attacker_ids, vec![1, 1, 2], "连击随从攻击两次后指针才右移");
}

#[test]
fn optimal_attack_prefers_killable() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 3, 20, &["最优攻击"]);
    add_card(&mut catalog, "taunt", 1, 10, &["嘲讽"]);
    add_card(&mut catalog, "weak", 1, 2, &[]);
    add_card(&mut catalog, "sturdy", 1, 10, &[]);

    let prefers_kill = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "taunt"), minion(&catalog, 3, "weak")],
        0,
        1,
        0,
        &mut rng(),
    );
    assert_eq!(
        attacks(&prefers_kill)[0],
        (1, 3, 3, false),
        "最优攻击应无视嘲讽优先击杀可击杀目标"
    );

    let falls_back = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "taunt"), minion(&catalog, 4, "sturdy")],
        0,
        1,
        0,
        &mut rng(),
    );
    assert_eq!(
        attacks(&falls_back)[0],
        (1, 2, 3, false),
        "无可击杀目标时回退到嘲讽规则"
    );
}

#[test]
fn contagion_applies_after_damage() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 2, 10, &["传染"]);
    add_card(&mut catalog, "b", 0, 5, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "b")],
        0,
        1,
        0,
        &mut rng(),
    );

    assert_eq!(
        log.actions[0],
        CombatAction::ContagionApplied { target: 2 },
        "实际伤害大于 0 时先叠传染"
    );
    assert_eq!(
        log.actions[1],
        CombatAction::Attack {
            attacker: 1,
            target: 2,
            damage: 2,
            blocked: false,
        }
    );
    assert_eq!(contagion_count(&log), 1);
}

#[test]
fn contagion_skipped_when_blocked() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 2, 10, &["传染", "连击"]);
    add_card(&mut catalog, "b", 0, 5, &["护盾"]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "b")],
        0,
        1,
        0,
        &mut rng(),
    );

    assert_eq!(contagion_count(&log), 1, "只有造成实际伤害的第二击叠传染");
    assert_eq!(attacks(&log)[0], (1, 2, 0, true));
    assert_eq!(attacks(&log)[2], (1, 2, 2, false));
}

#[test]
fn reborn_returns_once_with_one_health() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 3, 10, &["连击"]);
    add_card(&mut catalog, "b", 0, 2, &["复生"]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "b")],
        0,
        1,
        0,
        &mut rng(),
    );

    assert_eq!(death_order(&log), vec![2, 2], "复生后再次死亡");
    assert_eq!(reborn_count(&log), 1, "每场战斗只复生一次");
    assert!(log.defender_survivors.is_empty());
    let reborn_index = log
        .actions
        .iter()
        .position(|action| matches!(action, CombatAction::Reborn { .. }))
        .expect("应有复生事件");
    assert_eq!(
        log.actions[reborn_index - 1],
        CombatAction::Death { minion: 2 }
    );
}

#[test]
fn dying_target_does_not_counterattack() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 3, 2, &[]);
    add_card(&mut catalog, "b", 2, 3, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "b")],
        0,
        1,
        0,
        &mut rng(),
    );

    // 攻击力 3 直接击杀 3 血目标；目标已死亡，不能反击攻击力 2 的 A。
    assert_eq!(death_order(&log), vec![2], "只有目标死亡");
    assert_eq!(log.attacker_survivors, vec![1], "A 未受反击伤害");
    assert!(log.defender_survivors.is_empty());
    assert_eq!(log.winner, Some(0));
    assert_eq!(log.action_count, 1);
}

fn zero_health(catalog: &Catalog, instance_id: InstanceId, card_id: &str) -> MinionInstance {
    let mut subject = minion(catalog, instance_id, card_id);
    subject.stats.health.add_permanent(-5);
    subject
}

#[test]
fn death_queue_orders_attacker_side_then_left_to_right() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "corpse", 0, 1, &[]);
    add_card(&mut catalog, "fighter", 0, 10, &[]);
    add_card(&mut catalog, "d1", 0, 1, &[]);
    add_card(&mut catalog, "d2", 0, 1, &["复生"]);

    let log = simulate(
        &catalog,
        &[
            zero_health(&catalog, 1, "corpse"),
            minion(&catalog, 2, "fighter"),
        ],
        &[
            zero_health(&catalog, 3, "d1"),
            zero_health(&catalog, 4, "d2"),
        ],
        0,
        1,
        0,
        &mut rng(),
    );

    assert_eq!(
        death_order(&log),
        vec![1, 3, 4],
        "攻击方零血随从先处理，再按防御方站位从左到右"
    );
    assert_eq!(reborn_count(&log), 1, "只有带复生的 d2 复活");
    let reborn_index = log
        .actions
        .iter()
        .position(|action| matches!(action, CombatAction::Reborn { minion } if *minion == 4))
        .expect("d2 应复生");
    assert_eq!(
        log.actions[reborn_index - 1],
        CombatAction::Death { minion: 4 }
    );
    assert_eq!(log.attacker_survivors, vec![2]);
    assert_eq!(log.defender_survivors, vec![4], "复生后的 d2 回到场上");
}

#[test]
fn combat_stops_at_200_actions() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 0, 1, &["连击"]);
    add_card(&mut catalog, "b", 0, 100, &[]);

    // 攻击方 100 个连击随从 = 200 个令牌，足以把行动数推到上限。
    let attacker_board: Vec<MinionInstance> = (1..=100)
        .map(|index| minion(&catalog, index, "a"))
        .collect();

    let log = simulate(
        &catalog,
        &attacker_board,
        &[minion(&catalog, 500, "b")],
        2,
        1,
        2,
        &mut rng(),
    );

    assert_eq!(log.action_count, 200, "行动数达到 200 必须停止");
    assert_eq!(attacks(&log).len(), 200);
    // 行动数到上限而双方仍有存活者：§8.3 视为平局，交给结算层按 §5.3 判定。
    assert_eq!(log.winner, None);
    assert_eq!(log.attacker_survivors.len(), 100);
    assert_eq!(log.defender_survivors, vec![500]);
}

#[test]
fn zero_attack_does_not_counterattack() {
    let mut catalog = test_catalog();
    add_card_with(&mut catalog, "scholar", Tribe::Scholar, 0, 10, &[]);
    add_card(&mut catalog, "b", 2, 3, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "scholar")],
        &[minion(&catalog, 2, "b")],
        0,
        1,
        0,
        &mut rng(),
    );

    assert_eq!(
        attacks(&log)[0],
        (1, 2, 0, false),
        "0 攻仍会出手，但造成 0 伤害"
    );
    assert_eq!(
        log.attacker_survivors,
        vec![1],
        "学者仍会承受反击但不会反击"
    );

    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 3, 10, &[]);
    add_card(&mut catalog, "defender", 0, 10, &[]);
    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "defender")],
        0,
        1,
        0,
        &mut rng(),
    );
    assert_eq!(attacks(&log)[0], (1, 2, 3, false));
    let defender_main = attacks(&log)
        .into_iter()
        .find(|event| event.0 == 2)
        .expect("防御方应轮到出手");
    assert_eq!(defender_main, (2, 1, 0, false), "0 攻防守方不反击");
}

#[test]
fn cannot_attack_is_skipped() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "bound", 5, 10, &[]);
    add_card(&mut catalog, "free", 5, 10, &[]);
    add_card(&mut catalog, "b", 0, 20, &[]);

    let mut bound = minion(&catalog, 1, "bound");
    bound.cannot_attack = true;
    // 防守方也禁攻，避免反击事件干扰「谁发起攻击」的断言。
    let mut defender = minion(&catalog, 3, "b");
    defender.cannot_attack = true;
    let log = simulate(
        &catalog,
        &[bound, minion(&catalog, 2, "free")],
        &[defender],
        0,
        1,
        0,
        &mut rng(),
    );

    assert!(
        attacks(&log).iter().all(|event| event.0 != 1),
        "禁攻随从被指针跳过"
    );
    assert_eq!(attacks(&log)[0].0, 2, "指针停在可攻击随从上");
}

#[test]
fn defender_acts_first_when_first_seat_is_defender() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 1, 10, &[]);
    add_card(&mut catalog, "b", 1, 10, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "b")],
        0,
        1,
        1,
        &mut rng(),
    );

    assert_eq!(attacks(&log)[0], (2, 1, 1, false), "先手方应为防御方");
}

#[test]
fn scholar_granted_windfury_still_attacks_once() {
    let mut catalog = test_catalog();
    add_card_with(&mut catalog, "scholar", Tribe::Scholar, 0, 10, &[]);
    add_card(&mut catalog, "b", 0, 20, &[]);

    let mut scholar = minion(&catalog, 1, "scholar");
    scholar.granted.push(("连击".to_string(), Duration::Combat));
    let log = simulate(
        &catalog,
        &[scholar],
        &[minion(&catalog, 2, "b")],
        0,
        1,
        0,
        &mut rng(),
    );

    let scholar_attacks = attacks(&log)
        .into_iter()
        .filter(|event| event.0 == 1)
        .count();
    assert_eq!(scholar_attacks, 1, "连击对学者不生效，只有 1 个令牌");
}

#[test]
fn reborn_attacker_keeps_its_spent_token() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 2, 2, &["复生"]);
    add_card(&mut catalog, "b", 3, 3, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "b")],
        0,
        1,
        0,
        &mut rng(),
    );

    // 核心规则 §5.2 第 7 步死亡结算后第 8 步才扣减令牌：
    // 复生重置令牌后再扣掉本次攻击的 1 个，因此本场不会再出手。
    let death_index = log
        .actions
        .iter()
        .position(|action| matches!(action, CombatAction::Death { minion } if *minion == 1))
        .expect("攻击方应死亡");
    assert_eq!(
        log.actions[death_index + 1],
        CombatAction::Reborn { minion: 1 }
    );
    let attacks_by_attacker = attacks(&log)
        .into_iter()
        .filter(|event| event.0 == 1)
        .count();
    assert_eq!(attacks_by_attacker, 1, "复生后令牌已耗尽，不再攻击");
    assert!(log.attacker_survivors.is_empty());
}

#[test]
fn effective_stats_follow_evaluation_order() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 2, 3, &[]);
    add_card_with(&mut catalog, "scholar", Tribe::Scholar, 9, 4, &[]);

    let mut monster = minion(&catalog, 1, "a");
    monster.golden = true;
    monster.stats.attack.set_initial(4);
    monster.stats.health.set_initial(6);
    monster.stats.attack.add_growth(2);
    monster.stats.health.add_growth(2);
    monster.stats.health.add_growth(1);
    monster.stats.attack.add_permanent(1);
    monster.stats.health.add_permanent(1);
    monster.stats.attack.add_turn(2);
    monster.stats.health.add_turn(3);
    // 金色卡面 2/3 -> 4/6，成长 +2 攻击、+2/+1 生命，修正 +3/+4。
    assert_eq!(effective_attack(&catalog, &monster), 9);
    assert_eq!(effective_health(&catalog, &monster), 13);

    let auras = [Modifier {
        attack: 5,
        health: -2,
        duration: Duration::Combat,
    }];
    assert_eq!(effective_attack_with(&catalog, &monster, &auras), 14);
    assert_eq!(effective_health_with(&catalog, &monster, &auras), 11);

    let mut scholar = minion(&catalog, 2, "scholar");
    scholar.stats.attack.add_growth(3);
    scholar.stats.health.add_growth(3);
    scholar.stats.attack.add_permanent(7);
    assert_eq!(effective_attack(&catalog, &scholar), 0, "学者攻击力强制 0");
    assert_eq!(effective_health(&catalog, &scholar), 7);

    let mut floored = minion(&catalog, 3, "a");
    floored.stats.attack.add_turn(-10);
    floored.stats.health.add_turn(-10);
    assert_eq!(effective_attack(&catalog, &floored), 0);
    assert_eq!(effective_health(&catalog, &floored), 0);

    let missing = minion(&catalog, 4, "missing-card");
    assert_eq!(effective_attack(&catalog, &missing), 0);
    assert_eq!(effective_health(&catalog, &missing), 0);
    assert!(keywords(&catalog, &missing).is_empty());
}

#[test]
fn keywords_merge_card_and_active_grants() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 1, 1, &["嘲讽"]);

    let mut subject = minion(&catalog, 1, "a");
    subject.granted = vec![
        ("护盾".to_string(), Duration::Permanent),
        ("连击".to_string(), Duration::Combat),
        ("复生".to_string(), Duration::Instant),
        ("未知关键词".to_string(), Duration::Permanent),
    ];

    let expected: BTreeSet<String> = ["护盾", "未知关键词", "嘲讽", "连击"]
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(keywords(&catalog, &subject), expected);
}

/// 构造一场真实配对战斗：座位 0、1 存活，其余标淘汰。
fn real_combat_state() -> (Engine, rule_engine::state::GameState) {
    let (mut engine, mut state) = Engine::new(test_catalog(), [4; 32]);
    add_card(&mut engine.catalog, "combat-big", 5, 5, &[]);
    add_card(&mut engine.catalog, "combat-small", 1, 1, &[]);
    state.phase = Phase::Combat;
    for seat in 2..8 {
        state.players[seat].eliminated = true;
    }
    state.players[0].board = vec![minion(&engine.catalog, 10, "combat-big")];
    state.players[1].board = vec![minion(&engine.catalog, 20, "combat-small")];
    // 座位 0 的随从带一条光环：报告里的 own_board / opponent_board 必须是
    // 折叠后的数值，而持久棋盘仍保留原始值。
    state.auras.insert(
        10,
        AuraEffect {
            attack: 2,
            health: 1,
            keywords: ["护盾"].into_iter().map(str::to_string).collect(),
            floor_one: true,
        },
    );
    state.pairings = vec![Pairing {
        a: 0,
        b: 1,
        ghost: None,
    }];
    (engine, state)
}

#[test]
fn resolve_combat_returns_one_report_per_participant() {
    let (mut engine, mut state) = real_combat_state();

    let outcome = engine.resolve_combat(&mut state).unwrap();

    assert_eq!(outcome.reports.len(), 2, "真实战斗双方各一条");
    let report_a = outcome.reports.get(&0).expect("座位 a 的报告");
    let report_b = outcome.reports.get(&1).expect("座位 b 的报告");
    assert!(!report_a.mirror && !report_b.mirror);
    assert_eq!(report_a.opponent_seat, Some(1));
    assert_eq!(report_b.opponent_seat, Some(0));
    assert_eq!(report_a.own_board.len(), 1, "own_board 是战前阵容");
    assert_eq!(report_a.own_board[0].instance_id, 10);
    assert_eq!(report_a.opponent_board.len(), 1);
    assert_eq!(report_a.opponent_board[0].instance_id, 20);
    assert_eq!(report_b.own_board[0].instance_id, 20);
    assert_eq!(report_b.opponent_board[0].instance_id, 10);
    // 报告阵容已折叠光环：5/5 + 2/1 = 7/6，并获得「护盾」。
    let folded_a = &report_a.own_board[0];
    assert_eq!(effective_attack(&engine.catalog, folded_a), 7);
    assert_eq!(effective_health(&engine.catalog, folded_a), 6);
    assert!(keywords(&engine.catalog, folded_a).contains("护盾"));
    // 座位 1 视角的 opponent_board 看到的是同一份折叠后数值。
    let folded_b_view = &report_b.opponent_board[0];
    assert_eq!(effective_attack(&engine.catalog, folded_b_view), 7);
    assert_eq!(effective_health(&engine.catalog, folded_b_view), 6);
    assert!(keywords(&engine.catalog, folded_b_view).contains("护盾"));
    // 持久棋盘保持原始数值：折叠只发生在报告副本上。
    assert_eq!(
        effective_attack(&engine.catalog, &state.players[0].board[0]),
        5
    );
    assert_eq!(
        effective_health(&engine.catalog, &state.players[0].board[0]),
        5
    );
    assert!(!keywords(&engine.catalog, &state.players[0].board[0]).contains("护盾"));
    assert_eq!(state.players[0].board[0].stats.attack.layers(), 5);
    assert_eq!(state.players[0].board[0].stats.health.layers(), 5);
    assert!(state.players[0].board[0].granted.is_empty());
    assert!(!report_a.actions.is_empty(), "行动序列随报告返回");
    assert_eq!(report_a.actions, report_b.actions, "双方看到同一份行动序列");
    assert_eq!(report_a.damage, report_b.damage, "双方看到同一份伤害");
    assert_eq!(report_a.winner, Some(0));
    assert_eq!(report_b.winner, Some(0));
    assert_eq!(
        outcome.events.first(),
        Some(&rule_engine::events::GameEvent::CombatResolved {
            winner: 0,
            loser: 1,
            damage: report_a.damage,
        }),
        "原有事件流不变"
    );
}

#[test]
fn ghost_report_names_ghost_owner() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [3; 32]);
    add_card(&mut engine.catalog, "combat-strong", 10, 10, &[]);
    add_card(&mut engine.catalog, "combat-weak", 1, 1, &[]);
    state.phase = Phase::Combat;
    for seat in 1..8 {
        state.players[seat].eliminated = true;
    }
    state.players[0].board = vec![minion(&engine.catalog, 1, "combat-strong")];
    state.players[3].board = vec![minion(&engine.catalog, 2, "combat-weak")];
    state.pairings = vec![Pairing {
        a: 0,
        b: 0,
        ghost: Some(3),
    }];

    let outcome = engine.resolve_combat(&mut state).unwrap();

    assert_eq!(outcome.reports.len(), 1, "幽灵战只有一条报告");
    let report = outcome.reports.get(&0).expect("座位 0 的报告");
    assert!(report.mirror);
    assert_eq!(report.opponent_seat, Some(3), "报告指向幽灵归属座位");
    assert_eq!(report.damage, 0);
    assert_eq!(report.own_board[0].instance_id, 1);
    assert_eq!(report.opponent_board[0].instance_id, 2, "对手阵容是幽灵");
    assert!(!report.actions.is_empty());
    assert_eq!(report.winner, Some(0));
}

fn state_hash_after_combat() -> u64 {
    let (mut engine, mut state) = real_combat_state();
    let outcome = engine.resolve_combat(&mut state).unwrap();
    // 保留报告并读取，模拟服务端需要这份数据；报告不得进入 GameState。
    assert_eq!(outcome.reports.len(), 2);
    let _actions = &outcome.reports.get(&0).unwrap().actions;
    state_hash(&state)
}

#[test]
fn combat_reports_do_not_change_state_hash() {
    // 两次相同的 seed 与相同操作序列：一份取用报告，一份只丢哈希前不触碰报告。
    let with_reports = state_hash_after_combat();
    let (mut engine, mut state) = real_combat_state();
    engine.resolve_combat(&mut state).unwrap();
    let discarded = state_hash(&state);
    assert_eq!(with_reports, discarded);
}

#[test]
fn combat_reports_do_not_mutate_state_fields() {
    let (mut engine, mut state) = real_combat_state();
    let before = state.clone();
    let outcome = engine.resolve_combat(&mut state).unwrap();
    let damage = outcome.reports.get(&0).expect("座位 0 的报告").damage;

    // 光环是战斗外的派生缓存；报告折叠只发生在副本上，缓存本身不变。
    assert_eq!(state.auras, before.auras, "报告构造不得改动光环缓存");
    // 该场景没有召唤；实例编号只由战斗模拟推进到「双方最大实例编号 + 1」，
    // 报告构造不得进一步推进或回退它。
    let max_board_id = before
        .players
        .iter()
        .flat_map(|player| player.board.iter())
        .map(|minion| minion.instance_id)
        .max()
        .unwrap_or(0);
    let expected_next_id = before.next_instance_id.max(max_board_id + 1);
    assert_eq!(
        state.next_instance_id, expected_next_id,
        "实例编号只允许由战斗模拟推进"
    );
    // 对手互指只由战斗结算写入，报告不得额外增删键值。
    let mut expected_opponents = before.previous_opponent.clone();
    expected_opponents.insert(0, Some(1));
    expected_opponents.insert(1, Some(0));
    assert_eq!(state.previous_opponent, expected_opponents);
    // 玩家状态只允许胜方 damage_dealt 变化；棋盘、生命等其它字段保持原样。
    let mut expected_players = before.players.clone();
    expected_players[0].damage_dealt += damage;
    assert_eq!(
        state.players, expected_players,
        "报告不得写回棋盘或其它玩家字段"
    );
}

#[test]
fn combat_state_hash_matches_golden_baseline() {
    // 场景：seed [4; 32]、座位 0 随从 10 带光环、配对 0 vs 1。
    // 随从属性改为分层模型后哈希编码变化，金标准同步更新；行为不变，
    // 任何 GameState 字段或随机流消耗顺序变化仍会使该值改变。
    let (mut engine, mut state) = real_combat_state();
    let outcome = engine.resolve_combat(&mut state).unwrap();
    assert_eq!(outcome.reports.len(), 2);
    assert_eq!(
        format!("{:016x}", state_hash(&state)),
        "dee6adc25ec516ff",
        "固定场景的 state_hash 金标准"
    );
}

#[test]
fn mixed_real_and_mirror_reports_cover_exactly_three_seats() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [5; 32]);
    add_card(&mut engine.catalog, "combat-a", 5, 5, &[]);
    add_card(&mut engine.catalog, "combat-b", 1, 1, &[]);
    add_card(&mut engine.catalog, "combat-c", 3, 3, &[]);
    state.phase = Phase::Combat;
    for seat in 3..8 {
        state.players[seat].eliminated = true;
    }
    state.players[0].board = vec![minion(&engine.catalog, 30, "combat-a")];
    state.players[1].board = vec![minion(&engine.catalog, 31, "combat-b")];
    state.players[2].board = vec![minion(&engine.catalog, 32, "combat-c")];
    state.pairings = vec![
        Pairing {
            a: 0,
            b: 1,
            ghost: None,
        },
        Pairing {
            a: 2,
            b: 2,
            ghost: Some(3),
        },
    ];

    let outcome = engine.resolve_combat(&mut state).unwrap();

    let seats: Vec<Seat> = outcome.reports.keys().copied().collect();
    assert_eq!(seats, vec![0, 1, 2], "键集合恰为三名参战座位");
    assert!(!outcome.reports.get(&0).unwrap().mirror);
    assert!(!outcome.reports.get(&1).unwrap().mirror);
    assert!(outcome.reports.get(&2).unwrap().mirror);
    assert_eq!(outcome.reports.get(&0).unwrap().opponent_seat, Some(1));
    assert_eq!(outcome.reports.get(&1).unwrap().opponent_seat, Some(0));
    assert_eq!(outcome.reports.get(&2).unwrap().opponent_seat, Some(3));
}

// ---- 动作后状态快照与召唤 ----

fn combat_log(
    catalog: &Catalog,
    attacker: &[MinionInstance],
    defender: &[MinionInstance],
) -> CombatLog {
    simulate(catalog, attacker, defender, 0, 1, 0, &mut rng())
}

#[test]
fn combat_states_parallel_actions() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 3, 3, &[]);
    add_card(&mut catalog, "b", 2, 5, &[]);

    let log = combat_log(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "b")],
    );

    assert!(!log.actions.is_empty(), "应有动作");
    assert_eq!(
        log.actions.len(),
        log.states.len(),
        "状态快照与动作同序同长"
    );
    for states in &log.states {
        assert!(!states.is_empty(), "每条动作至少记录一个受影响随从");
    }
}

#[test]
fn attack_states_match_field_after_damage() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 3, 3, &[]);
    add_card(&mut catalog, "b", 2, 5, &[]);

    let log = combat_log(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "b")],
    );

    let index = log
        .actions
        .iter()
        .position(|action| matches!(action, CombatAction::Attack { .. }))
        .expect("应至少有一次攻击");
    let states = &log.states[index];
    assert_eq!(states.len(), 2, "攻击记录攻方与受方各一条");
    assert_eq!(states[0].instance_id, 1, "第一条是攻方");
    assert_eq!(states[0].attack, 3);
    assert_eq!(states[0].health, 1, "攻方已被反击 2 点，取动作后的现场值");
    assert!(states[0].alive);
    assert_eq!(states[1].instance_id, 2, "第二条是受方");
    assert_eq!(states[1].health, 2, "受方动作后生命为 5 - 3");
    assert!(states[1].alive);
}

#[test]
fn summon_action_records_position_and_seat() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "killer", 10, 10, &[]);
    add_card(&mut catalog, "sower", 1, 1, &["播种"]);
    catalog.tokens.insert(
        "seedling".to_string(),
        rule_engine::data::Token {
            id: "seedling".to_string(),
            name: "幼苗".to_string(),
            attack: 1,
            health: 1,
            keywords: Vec::new(),
        },
    );

    let (engine, _state) = Engine::new(catalog.clone(), [7; 32]);
    let log = simulate_with(
        &engine,
        &[minion(&catalog, 1, "killer")],
        &[minion(&catalog, 2, "sower")],
        0,
        1,
        0,
        &mut rng(),
        Default::default(),
    );

    let index = log
        .actions
        .iter()
        .position(|action| matches!(action, CombatAction::Summon { .. }))
        .expect("播种亡语应产生召唤动作");
    let CombatAction::Summon {
        minion: summoned,
        seat,
        position,
        card_id,
        ..
    } = &log.actions[index]
    else {
        unreachable!("已确认是召唤动作");
    };
    assert_eq!(*seat, 1, "召唤方为防守方座位");
    assert_eq!(*position, 0, "阵亡随从不计入存活序列，召唤物是唯一存活随从");
    assert_eq!(card_id, "seedling");
    assert_eq!(
        log.states[index][0].instance_id, *summoned,
        "召唤的状态快照指向新随从"
    );
    assert!(log.states[index][0].alive, "新随从存活");
}

#[test]
fn deathrattle_summon_uses_alive_rank_and_stops_at_capacity() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "killer", 1000, 1000, &[]);
    add_card(&mut catalog, "summoner", 1, 1, &["嘲讽"]);
    add_card(&mut catalog, "filler", 0, 10000, &[]);
    add_effect_skill(
        &mut catalog,
        "summoner",
        "summoner-death-summon",
        "亡语",
        "OnDeath|Summon|self|token=seedling,count=2",
    );
    add_seedling_token(&mut catalog);

    let (engine, _state) = Engine::new(catalog.clone(), [7; 32]);
    let mut defender = vec![minion(&catalog, 100, "summoner")];
    defender.extend((1..7).map(|index| minion(&catalog, 100 + index, "filler")));
    let log = simulate_with(
        &engine,
        &[minion(&catalog, 1, "killer")],
        &defender,
        0,
        1,
        0,
        &mut rng(),
        Default::default(),
    );

    let positions: Vec<u8> = log
        .actions
        .iter()
        .filter_map(|action| match action {
            CombatAction::Summon {
                seat: 1, position, ..
            } => Some(*position),
            _ => None,
        })
        .collect();
    assert_eq!(positions.len(), 1, "存活已满后的第二次召唤作废");
    assert_eq!(positions[0], 6, "位置是存活序列下标，不把阵亡的召唤者计入");
    assert_eq!(log.summon_count, 1, "作废的召唤不计入召唤数");
}

#[test]
fn reborn_fizzles_when_summon_filled_the_board() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "killer", 1000, 1000, &[]);
    add_card(&mut catalog, "reborner", 1, 1, &["复生", "嘲讽"]);
    add_card(&mut catalog, "filler", 0, 10000, &[]);
    add_effect_skill(
        &mut catalog,
        "reborner",
        "reborner-death-summon",
        "亡语",
        "OnDeath|Summon|self|token=seedling,count=1",
    );
    add_seedling_token(&mut catalog);

    let (engine, _state) = Engine::new(catalog.clone(), [7; 32]);
    let mut defender = vec![minion(&catalog, 100, "reborner")];
    defender.extend((1..7).map(|index| minion(&catalog, 100 + index, "filler")));
    let log = simulate_with(
        &engine,
        &[minion(&catalog, 1, "killer")],
        &defender,
        0,
        1,
        0,
        &mut rng(),
        Default::default(),
    );

    assert_eq!(
        log.actions
            .iter()
            .filter(|action| matches!(action, CombatAction::Summon { seat: 1, .. }))
            .count(),
        1,
        "死亡空出的位置先由召唤占用"
    );
    assert!(
        !log.actions
            .iter()
            .any(|action| matches!(action, CombatAction::Reborn { minion: 100 })),
        "召唤占满后复生作废"
    );
}

#[test]
fn combat_copy_to_board_fizzles_when_board_is_full() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "killer", 1000, 1000, &[]);
    add_card(&mut catalog, "copier", 1, 1, &[]);
    add_card(&mut catalog, "filler", 0, 10000, &[]);
    add_effect_skill(
        &mut catalog,
        "copier",
        "copier-combat-copy",
        "",
        "OnCombatStart|Copy|friendly:leftmost|to=board",
    );

    let (engine, _state) = Engine::new(catalog.clone(), [7; 32]);
    let mut defender = vec![minion(&catalog, 100, "copier")];
    defender.extend((1..7).map(|index| minion(&catalog, 100 + index, "filler")));
    let log = simulate_with(
        &engine,
        &[minion(&catalog, 1, "killer")],
        &defender,
        0,
        1,
        0,
        &mut rng(),
        Default::default(),
    );

    assert!(
        !log.actions
            .iter()
            .any(|action| matches!(action, CombatAction::Summon { .. })),
        "满场时复制到棋盘作废，不产生召唤动作"
    );
    assert_eq!(log.summon_count, 0);
}
