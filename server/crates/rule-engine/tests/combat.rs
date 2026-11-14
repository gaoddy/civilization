mod common;

use std::collections::BTreeSet;

use common::test_catalog;
use rule_engine::combat::{
    CombatAction, CombatLog, effective_attack, effective_attack_with, effective_health,
    effective_health_with, keywords, simulate,
};
use rule_engine::data::{Card, CardType, Catalog, Skill, Tribe};
use rule_engine::effects::Duration;
use rule_engine::rng::RngStreams;
use rule_engine::state::{InstanceId, MinionInstance, Modifier};

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
        &[minion(1, "a")],
        &[minion(2, "b")],
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
        &[minion(1, "a")],
        &[minion(2, "b")],
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
        &[minion(1, "a")],
        &[minion(2, "plain"), minion(3, "taunt")],
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
        &[minion(1, "a")],
        &[minion(2, "b")],
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
        &[minion(1, "a1"), minion(2, "a2")],
        &[minion(3, "b")],
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
        &[minion(1, "a")],
        &[minion(2, "taunt"), minion(3, "weak")],
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
        &[minion(1, "a")],
        &[minion(2, "taunt"), minion(4, "sturdy")],
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
        &[minion(1, "a")],
        &[minion(2, "b")],
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
        &[minion(1, "a")],
        &[minion(2, "b")],
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
        &[minion(1, "a")],
        &[minion(2, "b")],
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
        &[minion(1, "a")],
        &[minion(2, "b")],
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

fn zero_health(instance_id: InstanceId, card_id: &str) -> MinionInstance {
    let mut subject = minion(instance_id, card_id);
    subject.mods.push(Modifier {
        attack: 0,
        health: -5,
        duration: Duration::Permanent,
    });
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
        &[zero_health(1, "corpse"), minion(2, "fighter")],
        &[zero_health(3, "d1"), zero_health(4, "d2")],
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
    let attacker_board: Vec<MinionInstance> = (1..=100).map(|index| minion(index, "a")).collect();

    let log = simulate(
        &catalog,
        &attacker_board,
        &[minion(500, "b")],
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
        &[minion(1, "scholar")],
        &[minion(2, "b")],
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
        &[minion(1, "a")],
        &[minion(2, "defender")],
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

    let mut bound = minion(1, "bound");
    bound.cannot_attack = true;
    // 防守方也禁攻，避免反击事件干扰「谁发起攻击」的断言。
    let mut defender = minion(3, "b");
    defender.cannot_attack = true;
    let log = simulate(
        &catalog,
        &[bound, minion(2, "free")],
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
        &[minion(1, "a")],
        &[minion(2, "b")],
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

    let mut scholar = minion(1, "scholar");
    scholar.granted.push(("连击".to_string(), Duration::Combat));
    let log = simulate(&catalog, &[scholar], &[minion(2, "b")], 0, 1, 0, &mut rng());

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
        &[minion(1, "a")],
        &[minion(2, "b")],
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

    let mut monster = minion(1, "a");
    monster.golden = true;
    monster.growth = 2;
    monster.growth_health_only = 1;
    monster.mods = vec![
        Modifier {
            attack: 1,
            health: 1,
            duration: Duration::Permanent,
        },
        Modifier {
            attack: 2,
            health: 3,
            duration: Duration::Combat,
        },
    ];
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

    let mut scholar = minion(2, "scholar");
    scholar.growth = 3;
    scholar.mods = vec![Modifier {
        attack: 7,
        health: 0,
        duration: Duration::Permanent,
    }];
    assert_eq!(effective_attack(&catalog, &scholar), 0, "学者攻击力强制 0");
    assert_eq!(effective_health(&catalog, &scholar), 7);

    let mut floored = minion(3, "a");
    floored.mods = vec![Modifier {
        attack: -10,
        health: -10,
        duration: Duration::Combat,
    }];
    assert_eq!(effective_attack(&catalog, &floored), 0);
    assert_eq!(effective_health(&catalog, &floored), 0);

    let missing = minion(4, "missing-card");
    assert_eq!(effective_attack(&catalog, &missing), 0);
    assert_eq!(effective_health(&catalog, &missing), 0);
    assert!(keywords(&catalog, &missing).is_empty());
}

#[test]
fn keywords_merge_card_and_active_grants() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 1, 1, &["嘲讽"]);

    let mut subject = minion(1, "a");
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
