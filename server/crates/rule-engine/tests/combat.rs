mod common;

use common::{
    categories_for, effects_of, minion, reborn_count, test_catalog, test_playstyles, token_card,
};
use rule_engine::combat::{
    CombatAction, CombatLog, effective_attack, effective_attack_with, effective_health,
    effective_health_with, keyword_set, simulate, simulate_with,
};
use rule_engine::data::{Card, CardType, Catalog, Playstyle, Skill};
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
    playstyle: Playstyle,
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
                keyword: Some((*keyword).to_string()),
                effects: effects_of("[]"),
                cost: None,
                uses_per_turn: None,
            },
        );
        skill_ids.push(skill_id);
    }
    catalog.cards.insert(
        id.to_string(),
        Card {
            id: id.to_string(),
            card_type: CardType::Minion,
            in_shop_pool: true,
            taxon: "hominidae".to_string(),
            categories: categories_for(playstyle),
            subtype: None,
            level: 1,
            era: None,
            attack: Some(attack),
            health: Some(health),
            skills: skill_ids,
            cost: Some(1),
        },
    );
}

fn add_card(catalog: &mut Catalog, id: &str, attack: i32, health: i32, keyword_list: &[&str]) {
    add_card_with(catalog, id, Playstyle::Mammal, attack, health, keyword_list);
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
            keyword: Some(keyword.to_string()),
            effects: effects_of(effects),
            cost: None,
            uses_per_turn: None,
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
    catalog
        .cards
        .insert("seedling".to_string(), token_card("seedling", 1, 1));
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

fn contagion_count(log: &CombatLog) -> usize {
    log.actions
        .iter()
        .filter(|action| matches!(action, CombatAction::ContagionApplied { .. }))
        .count()
}

#[test]
fn dying_target_still_counterattacks() {
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

    // A 打 B 3 点（B 死于 0 血）；目标仍反击 2 点。
    assert_eq!(attacks(&log), vec![(1, 2, 3, false)], "只记录主动攻击");
    assert_eq!(death_order(&log), vec![2]);
    assert_eq!(log.attacker_survivors, vec![1]);
    assert!(log.defender_survivors.is_empty());
    assert_eq!(log.winner, Some(0));
    assert_eq!(log.action_count, 1);

    let hit = log
        .actions
        .iter()
        .position(|action| matches!(action, CombatAction::Attack { attacker: 1, .. }))
        .expect("应有一次主动攻击");
    let attacker = log.states[hit]
        .iter()
        .find(|snapshot| snapshot.instance_id == 1)
        .expect("攻击方快照");
    assert_eq!(attacker.health, 1, "攻击方承受 2 点反击");
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
        vec![(1, 2, 2, false)],
        "只记录主动攻击；反击伤害并入该条动作"
    );
    let hit_index = log
        .actions
        .iter()
        .position(|action| matches!(action, CombatAction::Attack { attacker: 1, .. }))
        .expect("应有一次主动攻击");
    let attacker_state = log.states[hit_index]
        .iter()
        .find(|snapshot| snapshot.instance_id == 1)
        .expect("攻击方快照");
    assert_eq!(
        attacker_state.health, 0,
        "反击 3 点击杀攻击方，血量写入主动攻击快照"
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
    add_card(&mut catalog, "taunt", 1, 10, &["taunt"]);
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
    add_card(&mut catalog, "a", 2, 10, &["windfury"]);
    add_card(&mut catalog, "b", 0, 5, &["divine_shield"]);

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
    add_card(&mut catalog, "a1", 0, 10, &["windfury"]);
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
    assert_eq!(
        attacker_ids.iter().copied().take(3).collect::<Vec<_>>(),
        vec![1, 1, 2],
        "连击随从连续攻击两次后指针才右移"
    );
}

#[test]
fn ignore_taunt_can_target_non_taunt() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 3, 20, &["ignore_taunt"]);
    add_card(&mut catalog, "taunt", 1, 10, &["taunt"]);
    add_card(&mut catalog, "weak", 1, 2, &[]);

    let saw_non_taunt = (0..32u8).any(|seed| {
        let log = simulate(
            &catalog,
            &[minion(&catalog, 1, "a")],
            &[minion(&catalog, 2, "taunt"), minion(&catalog, 3, "weak")],
            0,
            1,
            0,
            &mut RngStreams::from_seed([seed; 32]),
        );
        attacks(&log).first().is_some_and(|event| event.1 == 3)
    });
    assert!(saw_non_taunt, "带无视嘲讽时应能选中非嘲讽目标");
}

#[test]
fn contagion_applies_after_damage() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 2, 10, &["contagion"]);
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
    assert_eq!(contagion_count(&log), 2, "每次造成实际伤害都叠一层传染");
}

#[test]
fn contagion_skipped_when_blocked() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 2, 10, &["contagion", "windfury"]);
    add_card(&mut catalog, "b", 0, 5, &["divine_shield"]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "b")],
        0,
        1,
        0,
        &mut rng(),
    );

    assert_eq!(contagion_count(&log), 2, "只有造成实际伤害的攻击叠传染");
    assert_eq!(attacks(&log)[0], (1, 2, 0, true));
    assert_eq!(
        attacks(&log)[1],
        (1, 2, 2, false),
        "连击的第二次攻击造成实际伤害"
    );
}

#[test]
fn reborn_returns_once_with_one_health() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 3, 10, &["windfury"]);
    add_card(&mut catalog, "b", 0, 2, &["reborn"]);

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
fn mutual_death_when_counter_is_lethal() {
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

    // A 的主伤害击杀 B；B 仍反击 2 点，击杀 2 血的 A。
    assert_eq!(death_order(&log), vec![1, 2], "攻守双方同时死亡");
    assert!(log.attacker_survivors.is_empty());
    assert!(log.defender_survivors.is_empty());
    assert_eq!(log.winner, None);
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
    add_card(&mut catalog, "d2", 0, 1, &["reborn"]);

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
    add_card(&mut catalog, "a", 0, 1, &["windfury"]);
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
    // 行动数到上限而双方仍有存活者：交锋层不判胜，交给结算层按
    // 生命 → 累计伤害 → 座位判胜。
    assert_eq!(log.winner, None);
    assert_eq!(log.attacker_survivors.len(), 100);
    assert_eq!(log.defender_survivors, vec![500]);
}

#[test]
fn zero_attack_does_not_counterattack() {
    let mut catalog = test_catalog();
    add_card_with(&mut catalog, "zero", Playstyle::Neutral, 0, 10, &[]);
    add_card(&mut catalog, "b", 2, 3, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "zero")],
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
    assert!(
        attacks(&log)
            .iter()
            .filter(|event| event.0 == 1)
            .all(|event| event.2 == 0),
        "0 攻随从主动攻击与反击都造成 0 伤害"
    );
    assert_eq!(log.winner, Some(1), "0 攻随从被反击致死");

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
fn zero_attack_does_not_consume_shield() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "shielded", 10, 10, &["divine_shield"]);
    add_card_with(&mut catalog, "zero", Playstyle::Neutral, 0, 1, &["taunt"]);
    add_card(&mut catalog, "brute", 5, 5, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "shielded")],
        &[minion(&catalog, 2, "zero"), minion(&catalog, 3, "brute")],
        0,
        1,
        0,
        &mut rng(),
    );

    // 0 攻击反击不消耗护盾；随后 5 攻的 brute 仍被护盾挡下。
    assert!(death_order(&log).contains(&2), "嘲讽的 0 攻随从被击杀");
    assert_eq!(
        log.attacker_final[0].stats.health.current, 10,
        "brute 的攻击应被未被消耗的护盾挡下"
    );
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
fn zero_attack_minion_granted_windfury_attacks_twice() {
    let mut catalog = test_catalog();
    add_card_with(&mut catalog, "zero", Playstyle::Neutral, 0, 10, &[]);
    add_card(&mut catalog, "b", 0, 20, &[]);

    let mut zero = minion(&catalog, 1, "zero");
    zero.granted
        .push(("windfury".to_string(), Duration::Combat));
    let log = simulate(
        &catalog,
        &[zero],
        &[minion(&catalog, 2, "b")],
        0,
        1,
        0,
        &mut rng(),
    );

    let ids: Vec<InstanceId> = attacks(&log).iter().map(|event| event.0).take(3).collect();
    assert_eq!(ids, vec![1, 1, 2], "0 攻随从被授予连击时同样连续攻击两次");
}

#[test]
fn reborn_attacker_keeps_its_spent_token() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 2, 2, &["reborn"]);
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
    add_card_with(&mut catalog, "zero", Playstyle::Neutral, 0, 4, &[]);

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

    let mut zero = minion(&catalog, 2, "zero");
    zero.stats.attack.add_growth(3);
    zero.stats.health.add_growth(3);
    zero.stats.attack.add_permanent(7);
    assert_eq!(
        effective_attack(&catalog, &zero),
        10,
        "攻击力 0 的随从按普通成长累计攻击"
    );
    assert_eq!(effective_health(&catalog, &zero), 7);

    let mut floored = minion(&catalog, 3, "a");
    floored.stats.attack.add_turn(-10);
    floored.stats.health.add_turn(-10);
    assert_eq!(effective_attack(&catalog, &floored), 0);
    assert_eq!(effective_health(&catalog, &floored), 0);

    let missing = minion(&catalog, 4, "missing-card");
    assert_eq!(effective_attack(&catalog, &missing), 0);
    assert_eq!(effective_health(&catalog, &missing), 0);
    assert!(keyword_set(&catalog, &missing).is_empty());
}

#[test]
fn keywords_merge_card_and_active_grants() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 1, 1, &["taunt"]);

    let mut subject = minion(&catalog, 1, "a");
    subject.granted = vec![
        ("divine_shield".to_string(), Duration::Permanent),
        ("windfury".to_string(), Duration::Combat),
        ("reborn".to_string(), Duration::Instant),
        ("未知关键词".to_string(), Duration::Permanent),
    ];

    let expected = ["divine_shield", "taunt", "windfury", "未知关键词"]
        .map(str::to_string)
        .to_vec();
    assert_eq!(keyword_set(&catalog, &subject).names(), expected);
}

/// 构造一场真实配对战斗：座位 0、1 存活，其余标淘汰。
fn real_combat_state() -> (Engine, rule_engine::state::GameState) {
    let (mut engine, mut state) = Engine::new(test_catalog(), [4; 32], test_playstyles());
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
            keywords: rule_engine::keyword::KeywordSet::from_names(["divine_shield"]),
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
    let (engine, mut state) = real_combat_state();

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
    assert!(keyword_set(&engine.catalog, folded_a).contains_name("divine_shield"));
    // 座位 1 视角的 opponent_board 看到的是同一份折叠后数值。
    let folded_b_view = &report_b.opponent_board[0];
    assert_eq!(effective_attack(&engine.catalog, folded_b_view), 7);
    assert_eq!(effective_health(&engine.catalog, folded_b_view), 6);
    assert!(keyword_set(&engine.catalog, folded_b_view).contains_name("divine_shield"));
    // 持久棋盘保持原始数值：折叠只发生在报告副本上。
    assert_eq!(
        effective_attack(&engine.catalog, &state.players[0].board[0]),
        5
    );
    assert_eq!(
        effective_health(&engine.catalog, &state.players[0].board[0]),
        5
    );
    assert!(
        !keyword_set(&engine.catalog, &state.players[0].board[0]).contains_name("divine_shield")
    );
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
    let (mut engine, mut state) = Engine::new(test_catalog(), [3; 32], test_playstyles());
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
    let (engine, mut state) = real_combat_state();
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
    let (engine, mut state) = real_combat_state();
    engine.resolve_combat(&mut state).unwrap();
    let discarded = state_hash(&state);
    assert_eq!(with_reports, discarded);
}

#[test]
fn combat_reports_do_not_mutate_state_fields() {
    let (engine, mut state) = real_combat_state();
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
    // 随从属性改为分层模型后哈希编码变化，金标准同步更新；英雄卡从 13 张扩到 32 张
    // 后共享池条目随之增加、配对新增幽灵座位，两处改动合并后金标准再次同步；
    // i18n 改造后关键词标识由中文改为英文 id，哈希编码随之变化，金标准再次同步；
    // 部落改为玩法后，哈希写入 5 个玩法的序号，金标准再次同步。
    // 随从新增潜行与复苏状态位后，哈希编码增加两个布尔值，金标准再次同步。
    // 新增 latent_used 字段（潜伏爆发每场一次）后哈希编码再增加一个布尔值，金标准同步。
    // 鱼纲、真菌界与原生生物界卡牌加入共享池后池条目变化，金标准再次同步。
    // 玩法扩充新增 106 张卡牌（池条目 346 张）后共享池再次变化，金标准同步；
    // 该场景只用自造卡 `combat-big` / `combat-small`，战斗行为本身未变。
    // 新增 delayed_battlecry_used 字段（延迟战吼每实例只结算一次）后哈希编码
    // 再增加一个布尔值，金标准同步。
    // 新增 froze_this_turn 字段（解冻只在冻结的当回合可用）后玩家哈希再增加
    // 一个布尔值，金标准同步。
    // 移除 is_scholar 字段后随从哈希少写一个布尔值，金标准同步；战斗行为不变。
    // 移除仅参与哈希的 turns_in_era 后玩家哈希少写一个字段，金标准同步；规则行为不变。
    // 行为不变，任何 GameState 字段或随机流消耗顺序变化仍会使该值改变。
    let (engine, mut state) = real_combat_state();
    let outcome = engine.resolve_combat(&mut state).unwrap();
    assert_eq!(outcome.reports.len(), 2);
    assert_eq!(
        format!("{:016x}", state_hash(&state)),
        "710e73f7570d6ce3",
        "固定场景的 state_hash 金标准"
    );
}

#[test]
fn mixed_real_and_ghost_reports_cover_exactly_three_seats() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [5; 32], test_playstyles());
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
    add_card(&mut catalog, "sower", 1, 1, &["sow"]);
    catalog
        .cards
        .insert("seedling".to_string(), token_card("seedling", 1, 1));

    let (engine, _state) = Engine::new(catalog.clone(), [7; 32], test_playstyles());
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
    add_card(&mut catalog, "summoner", 1, 1, &["taunt"]);
    add_card(&mut catalog, "filler", 0, 10000, &[]);
    add_effect_skill(
        &mut catalog,
        "summoner",
        "summoner-death-summon",
        "deathrattle",
        "OnDeath|Summon|self|token=seedling,count=2",
    );
    add_seedling_token(&mut catalog);

    let (engine, _state) = Engine::new(catalog.clone(), [7; 32], test_playstyles());
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
    add_card(&mut catalog, "reborner", 1, 1, &["reborn", "taunt"]);
    add_card(&mut catalog, "filler", 0, 10000, &[]);
    add_effect_skill(
        &mut catalog,
        "reborner",
        "reborner-death-summon",
        "deathrattle",
        "OnDeath|Summon|self|token=seedling,count=1",
    );
    add_seedling_token(&mut catalog);

    let (engine, _state) = Engine::new(catalog.clone(), [7; 32], test_playstyles());
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

    let (engine, _state) = Engine::new(catalog.clone(), [7; 32], test_playstyles());
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

#[test]
fn rounds_reset_when_a_side_finishes() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a1", 0, 100, &[]);
    add_card(&mut catalog, "a2", 0, 100, &[]);
    add_card(&mut catalog, "b", 0, 100, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a1"), minion(&catalog, 2, "a2")],
        &[minion(&catalog, 3, "b")],
        0,
        1,
        0,
        &mut rng(),
    );

    let ids: Vec<InstanceId> = attacks(&log).iter().map(|event| event.0).take(6).collect();
    assert_eq!(
        ids,
        vec![1, 3, 2, 3, 1, 3],
        "先手侧走完后回到最左，后手侧只有一名随从时每拍都出手"
    );
}

#[test]
fn unequal_side_sizes_advance_independently() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 0, 100, &[]);
    add_card(&mut catalog, "b1", 0, 100, &[]);
    add_card(&mut catalog, "b2", 0, 100, &[]);
    add_card(&mut catalog, "b3", 0, 100, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[
            minion(&catalog, 2, "b1"),
            minion(&catalog, 3, "b2"),
            minion(&catalog, 4, "b3"),
        ],
        0,
        1,
        0,
        &mut rng(),
    );

    let ids: Vec<InstanceId> = attacks(&log).iter().map(|event| event.0).take(8).collect();
    assert_eq!(
        ids,
        vec![1, 2, 1, 3, 1, 4, 1, 2],
        "两侧轮次各自独立：先手每拍出手，后手按从左到右循环"
    );
}

#[test]
fn windfury_retargets_after_kill() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 10, 10, &["windfury"]);
    add_card(&mut catalog, "b1", 0, 1, &[]);
    add_card(&mut catalog, "b2", 0, 1, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "b1"), minion(&catalog, 3, "b2")],
        0,
        1,
        0,
        &mut rng(),
    );

    let hits: Vec<InstanceId> = attacks(&log)
        .into_iter()
        .filter(|event| event.0 == 1)
        .map(|event| event.1)
        .take(2)
        .collect();
    assert_eq!(hits.len(), 2, "连击第二次攻击不因目标阵亡而取消");
    assert_ne!(hits[0], hits[1], "第一次击杀后第二次重新选择目标");
}

#[test]
fn windfury_stops_when_attacker_dies() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "a", 1, 1, &["windfury"]);
    add_card(&mut catalog, "b", 10, 10, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "a")],
        &[minion(&catalog, 2, "b")],
        0,
        1,
        0,
        &mut rng(),
    );

    let hits = attacks(&log)
        .into_iter()
        .filter(|event| event.0 == 1)
        .count();
    assert_eq!(hits, 1, "第一次攻击后攻击者阵亡，连击第二次不进行");
    assert_eq!(log.winner, Some(1));
}

#[test]
fn reborn_minion_acts_later_in_the_same_round() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "weak", 0, 100, &[]);
    add_card(&mut catalog, "killer", 10, 100, &[]);
    add_card(&mut catalog, "front", 0, 100, &[]);
    add_card(&mut catalog, "reborner", 0, 1, &["reborn", "taunt"]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "weak"), minion(&catalog, 2, "killer")],
        &[
            minion(&catalog, 3, "front"),
            minion(&catalog, 4, "reborner"),
        ],
        0,
        1,
        0,
        &mut rng(),
    );

    assert!(
        attacks(&log).iter().any(|event| event.0 == 4),
        "本轮未行动过的复生随从在指针走到它时仍会出手"
    );
}

#[test]
fn reborn_minion_does_not_act_again_after_acting() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "front", 0, 100, &[]);
    add_card(&mut catalog, "striker", 10, 100, &[]);
    add_card(&mut catalog, "reborner", 1, 1, &["reborn", "taunt"]);
    add_card(&mut catalog, "back", 0, 100, &[]);

    let log = simulate(
        &catalog,
        &[minion(&catalog, 1, "front"), minion(&catalog, 2, "striker")],
        &[minion(&catalog, 3, "reborner"), minion(&catalog, 4, "back")],
        0,
        1,
        0,
        &mut rng(),
    );

    let reborn_index = log
        .actions
        .iter()
        .position(|action| matches!(action, CombatAction::Reborn { minion } if *minion == 3))
        .expect("复生随从应复活");
    let next_attacker = log.actions[reborn_index..]
        .iter()
        .find_map(|action| match action {
            CombatAction::Attack { attacker, .. } => Some(*attacker),
            _ => None,
        });
    assert_ne!(
        next_attacker,
        Some(3),
        "本轮已行动过的随从复生后不会立即再次行动"
    );
}

// ---- 潜行与俯冲 ----

#[test]
fn stealth_cannot_be_targeted_by_single_effects_but_aoe_hits() {
    let mut single_catalog = test_catalog();
    add_card(&mut single_catalog, "caster", 0, 100, &[]);
    add_effect_skill(
        &mut single_catalog,
        "caster",
        "caster-single",
        "trait",
        "OnCombatStart|DealDamage|enemy:leftmost|amount=3",
    );
    add_card(&mut single_catalog, "hidden", 0, 100, &["stealth"]);
    add_card(&mut single_catalog, "visible", 0, 100, &[]);

    let (engine, _) = Engine::new(single_catalog.clone(), [7; 32], test_playstyles());
    let log = simulate_with(
        &engine,
        &[minion(&single_catalog, 1, "caster")],
        &[
            minion(&single_catalog, 2, "hidden"),
            minion(&single_catalog, 3, "visible"),
        ],
        0,
        1,
        0,
        &mut rng(),
        Default::default(),
    );

    assert_eq!(
        log.defender_final[0].health(),
        100,
        "潜行随从不能被单体效果选中"
    );
    assert_eq!(
        log.defender_final[1].health(),
        97,
        "单体效果改选未潜行的最左目标"
    );

    let mut aoe_catalog = test_catalog();
    add_card(&mut aoe_catalog, "caster", 0, 100, &[]);
    add_effect_skill(
        &mut aoe_catalog,
        "caster",
        "caster-aoe",
        "trait",
        "OnCombatStart|DealDamage|enemy:all|amount=2",
    );
    add_card(&mut aoe_catalog, "hidden", 0, 100, &["stealth"]);
    add_card(&mut aoe_catalog, "visible", 0, 100, &[]);

    let (engine, _) = Engine::new(aoe_catalog.clone(), [7; 32], test_playstyles());
    let log = simulate_with(
        &engine,
        &[minion(&aoe_catalog, 1, "caster")],
        &[
            minion(&aoe_catalog, 2, "hidden"),
            minion(&aoe_catalog, 3, "visible"),
        ],
        0,
        1,
        0,
        &mut rng(),
        Default::default(),
    );

    assert_eq!(log.defender_final[0].health(), 98, "全体伤害仍命中潜行随从");
    assert_eq!(log.defender_final[1].health(), 98);
}

#[test]
fn all_enemies_stealthed_still_attack_leftmost_and_break_stealth() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "ambusher", 2, 100, &["stealth"]);
    add_card(&mut catalog, "guard-left", 0, 100, &["stealth"]);
    add_card(&mut catalog, "guard-right", 0, 100, &["stealth", "taunt"]);

    let log = combat_log(
        &catalog,
        &[minion(&catalog, 1, "ambusher")],
        &[
            minion(&catalog, 2, "guard-left"),
            minion(&catalog, 3, "guard-right"),
        ],
    );

    let first_attack = attacks(&log)
        .into_iter()
        .next()
        .expect("战斗必须推进并产生攻击");
    assert_eq!(first_attack.0, 1, "攻击方为潜行随从");
    assert_eq!(
        first_attack.1, 2,
        "所有防守方都潜行时攻击棋盘最左侧，避免死循环"
    );
    assert!(
        log.attacker_final[0].stealth_broken,
        "潜行随从攻击后打破潜行"
    );
}

#[test]
fn lowest_attack_pick_breaks_ties_leftmost() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "diver", 0, 100, &[]);
    add_effect_skill(
        &mut catalog,
        "diver",
        "diver-dive",
        "dive",
        "OnCombatStart|DealDamage|enemy:lowestAttack|amount=1",
    );
    add_card(&mut catalog, "low-left", 2, 100, &[]);
    add_card(&mut catalog, "low-right", 2, 100, &[]);
    add_card(&mut catalog, "high", 5, 100, &[]);

    let (engine, _) = Engine::new(catalog.clone(), [7; 32], test_playstyles());
    let log = simulate_with(
        &engine,
        &[minion(&catalog, 1, "diver")],
        &[
            minion(&catalog, 2, "low-left"),
            minion(&catalog, 3, "low-right"),
            minion(&catalog, 4, "high"),
        ],
        0,
        1,
        0,
        &mut rng(),
        Default::default(),
    );

    assert_eq!(
        log.defender_final[0].health(),
        99,
        "攻击力并列最低时俯冲命中棋盘最左侧"
    );
    assert_eq!(log.defender_final[1].health(), 100);
    assert_eq!(log.defender_final[2].health(), 100);
}
