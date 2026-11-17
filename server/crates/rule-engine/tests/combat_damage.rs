//! T03：战斗伤害路径统一。
//!
//! 覆盖攻击主伤害、反击、护盾、`canKill=false` 下限、复生、战后写回与幽灵快照，
//! 所有断言都读取 `MinionStats` 的当前生命，确保战斗中不再存在平行生命数组。

mod common;

use common::{effects_of, minion, test_catalog, test_playstyles};
use rule_engine::combat::{CombatAction, CombatSetup, simulate, simulate_with};
use rule_engine::data::{Card, CardType, Catalog, Playstyle, Skill};
use rule_engine::engine::Engine;
use rule_engine::rng::RngStreams;
use rule_engine::settle::Pairing;
use rule_engine::state::{InstanceId, Phase};

fn add_card(catalog: &mut Catalog, id: &str, attack: i32, health: i32, keyword_list: &[&str]) {
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

fn add_effect_card(catalog: &mut Catalog, id: &str, attack: i32, health: i32, effects: &str) {
    let skill_id = format!("{id}-skill");
    catalog.skills.insert(
        skill_id.clone(),
        Skill {
            id: skill_id.clone(),
            keyword: None,
            effects: effects_of(effects),
            cost: None,
            uses_per_turn: None,
        },
    );
    catalog.cards.insert(
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
            skills: vec![skill_id],
            is_scholar: false,
            is_hero: false,
            cost: Some(1),
        },
    );
}

fn rng() -> RngStreams {
    RngStreams::from_seed([19u8; 32])
}

fn attacks(log: &rule_engine::combat::CombatLog) -> Vec<(InstanceId, InstanceId, i32, bool)> {
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

/// 1) 攻击主伤害写入 `stats.health.current`，层和不变，`damage()` 等于累计已受伤害（本例仅一次命中）。
#[test]
fn attack_damage_updates_current_only() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "attacker", 3, 3, &[]);
    add_card(&mut catalog, "defender", 0, 5, &[]);

    let attacker = minion(&catalog, 1, "attacker");
    let mut defender = minion(&catalog, 2, "defender");
    // 只让攻击方出手，隔离一次主伤害。
    defender.cannot_attack = true;

    let log = simulate(&catalog, &[attacker], &[defender], 0, 1, 0, &mut rng());

    // 多轮行动下读取第一次攻击后的现场快照，隔离单次主伤害。
    let hit_index = log
        .actions
        .iter()
        .position(|action| matches!(action, CombatAction::Attack { attacker: 1, .. }))
        .expect("应有一次攻击");
    let hit = log.states[hit_index]
        .iter()
        .find(|snapshot| snapshot.instance_id == 2)
        .expect("目标快照");
    assert_eq!(hit.health, 2, "生命从 5 降到 2");
    assert!(hit.alive);
    assert_eq!(
        log.defender_final[0].stats.health.layers(),
        5,
        "伤害不改层和"
    );
    assert_eq!(log.defender_final[0].stats.health.growth, 0);
    assert_eq!(attacks(&log)[0], (1, 2, 3, false), "记录一次主伤害");
}

/// 2) 反击伤害同样写入受击方的 `stats.health.current`。
#[test]
fn counter_damage_updates_current() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "attacker", 1, 5, &[]);
    add_card(&mut catalog, "defender", 2, 5, &[]);

    let attacker = minion(&catalog, 1, "attacker");
    let mut defender = minion(&catalog, 2, "defender");
    // 攻击方先手造成 1 点主伤害；防守方反击 2 点后不再出手。
    defender.cannot_attack = true;

    let log = simulate(&catalog, &[attacker], &[defender], 0, 1, 0, &mut rng());

    // 主动攻击动作的快照在反击结算之后抓取，用第一次交换隔离单次反击伤害。
    let hit_index = log
        .actions
        .iter()
        .position(|action| matches!(action, CombatAction::Attack { attacker: 1, .. }))
        .expect("应有一次主动攻击");
    let attacker_state = log.states[hit_index]
        .iter()
        .find(|snapshot| snapshot.instance_id == 1)
        .expect("攻击方快照");
    assert_eq!(attacker_state.health, 3, "攻击方承受 2 点反击");
    assert!(attacker_state.alive);
    assert_eq!(log.attacker_final[0].stats.health.layers(), 5);
    assert_eq!(attacks(&log)[0], (1, 2, 1, false), "主伤害记录为主动攻击");
    assert!(
        attacks(&log)
            .iter()
            .all(|(attacker, _, _, _)| *attacker == 1),
        "反击不再占用动作"
    );
}

/// 3) 护盾挡下主伤害：当前生命不变，`shield_used` 置位。
#[test]
fn shield_blocks_without_touching_health() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "attacker", 3, 3, &[]);
    add_card(&mut catalog, "shielded", 0, 5, &["divine_shield"]);

    let attacker = minion(&catalog, 1, "attacker");
    let mut defender = minion(&catalog, 2, "shielded");
    defender.cannot_attack = true;

    let log = simulate(&catalog, &[attacker], &[defender], 0, 1, 0, &mut rng());

    // 读取第一次攻击后的现场快照：护盾挡下主伤害。
    let hit_index = log
        .actions
        .iter()
        .position(|action| matches!(action, CombatAction::Attack { attacker: 1, .. }))
        .expect("应有一次攻击");
    let target_state = log.states[hit_index]
        .iter()
        .find(|snapshot| snapshot.instance_id == 2)
        .expect("目标快照");
    assert!(log.defender_final[0].shield_used, "护盾被消耗");
    assert_eq!(target_state.health, 5, "护盾分支不扣生命");
    assert_eq!(attacks(&log)[0], (1, 2, 0, true), "记录为被挡下");
}

/// 4) `canKill = false` 的效果伤害最低只把生命降到 1。
#[test]
fn cannot_kill_damage_floors_at_one() {
    let mut catalog = test_catalog();
    add_effect_card(
        &mut catalog,
        "burner",
        0,
        10,
        "OnCombatStart|DealDamage|enemy:all|amount=10,canKill=false",
    );
    add_card(&mut catalog, "victim", 0, 4, &[]);

    let (engine, _) = Engine::new(catalog, [23u8; 32], test_playstyles());

    let attacker = minion(&engine.catalog, 1, "burner");
    let victim = minion(&engine.catalog, 2, "victim");

    let log = simulate_with(
        &engine,
        &[attacker],
        &[victim],
        0,
        1,
        0,
        &mut rng(),
        CombatSetup::default(),
    );

    let target = &log.defender_final[0].stats.health;
    assert_eq!(target.current, 1, "非致死伤害夹在 1");
    assert_eq!(target.layers(), 4);
    assert_eq!(target.damage(), 3, "实际造成 3 点伤害");
}

/// 5) 复生把当前生命直接置为 1，允许大于层和（`damage()` 为负）。
#[test]
fn reborn_revives_current_to_one_beyond_layers() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "reborn_zero", 0, 0, &["reborn"]);

    let reborn = minion(&catalog, 1, "reborn_zero");
    let log = simulate(&catalog, &[reborn], &[], 0, 1, 0, &mut rng());

    let health = &log.attacker_final[0].stats.health;
    assert_eq!(health.current, 1, "复生后当前生命为 1");
    assert_eq!(health.layers(), 0, "层和仍为 0");
    assert_eq!(health.damage(), -1, "current 大于层和时 damage() 为负");
    assert_eq!(
        log.actions
            .iter()
            .filter(|action| matches!(action, CombatAction::Reborn { .. }))
            .count(),
        1,
        "只复生一次"
    );
}

/// 6) 战后持久棋盘：存活随从满血写在 `current`，战斗内临时层不写回。
#[test]
fn combat_write_back_restores_full_health() {
    let mut catalog = test_catalog();
    // 战斗开始给自身叠加永久与回合层，验证这些战斗内临时层不会写回；
    // 同时让幸存者在战斗中真实掉 2 点生命，确保战后满血不是「本来就没掉血」。
    add_effect_card(
        &mut catalog,
        "survivor",
        5,
        5,
        "OnCombatStart|ModifyStats|self|attack=5,health=5,duration=permanent;\
         OnCombatStart|ModifyStats|self|attack=3,health=3,duration=combat;\
         OnCombatStart|DealDamage|self|amount=2",
    );
    add_card(&mut catalog, "weakling", 0, 1, &[]);

    let (mut engine, mut state) = Engine::new(catalog, [29u8; 32], test_playstyles());
    state.phase = Phase::Combat;
    state.turn = 2;
    state.first_seat = 0;
    state.last_winner = None;
    state.pairings = vec![Pairing {
        a: 0,
        b: 1,
        ghost: None,
    }];
    state.players[0].hp = 30;
    state.players[1].hp = 30;
    state.players[0]
        .board
        .push(minion(&engine.catalog, 1, "survivor"));
    state.players[1]
        .board
        .push(minion(&engine.catalog, 2, "weakling"));

    // 战前已有成长层：战后应按 `initial + growth` 满血。
    state.players[0].board[0].stats.attack.add_growth(1);
    state.players[0].board[0].stats.health.add_growth(2);
    // 战斗只作用于克隆，持久实例不会因战斗扣血；这里先扣 2 点，让写回的
    // 「按层和重算当前值（满血进入下一回合）」契约成为可观测行为。
    state.players[0].board[0].stats.health.apply_damage(2);
    assert_eq!(state.players[0].board[0].stats.health.current, 5);
    assert_eq!(state.players[0].board[0].stats.health.layers(), 7);

    engine.resolve_combat(&mut state).expect("战斗结算应成功");

    let health = &state.players[0].board[0].stats.health;
    assert_eq!(health.initial, 5);
    assert_eq!(health.growth, 2);
    assert_eq!(health.current, 7, "存活随从战后满血 = initial + growth");
    assert_eq!(health.damage(), 0, "战斗伤害不写回");
    assert_eq!(health.turn, 0, "战斗内回合层不写回");
    assert_eq!(health.permanent, 0, "战斗内永久层不写回");
    assert_eq!(health.aura, 0, "光环折叠不写回");
    let attack = &state.players[0].board[0].stats.attack;
    assert_eq!(attack.turn, 0);
    assert_eq!(attack.permanent, 0);
    assert_eq!(attack.aura, 0);
}

/// 7) 幽灵阵容原样保留出局者棋盘的成长/永久层；战斗临时层的清理由上游负责。
#[test]
fn ghost_board_keeps_growth_and_permanent() {
    let mut catalog = test_catalog();
    add_card(&mut catalog, "plain", 3, 4, &[]);
    let mut original = minion(&catalog, 1, "plain");
    original.stats.attack.add_growth(2);
    original.stats.attack.add_permanent(5);
    original.stats.attack.add_turn(7);
    original.stats.attack.set_aura(3);
    original.stats.health.add_growth(1);
    original.stats.health.add_permanent(-2);
    original.stats.health.add_turn(4);
    original.stats.health.set_aura(6);

    // `ghost_board` 不做剥离：`turn` 层与战斗光环在玩家出局的那次结算里
    // 已由 `clear_combat_duration` 清掉，能进入死后棋盘的临时层本就为 0。
    // 这里直接构造未经结算的棋盘，用来确认 `ghost_board` 不额外改写任何层。
    let (engine, mut state) = Engine::new(catalog, [29u8; 32], test_playstyles());
    state.players[3].eliminated = true;
    state.players[3].board = vec![original];

    let ghost = &rule_engine::settle::ghost_board(&engine, &state, Some(3))[0];

    assert_eq!(ghost.stats.attack.permanent, 5);
    assert_eq!(ghost.stats.health.permanent, -2);
    assert_eq!(ghost.stats.attack.growth, 2);
    assert_eq!(ghost.stats.health.growth, 1);
    // 没有光环来源时 `fold_auras` 不覆盖 aura 层，临时层与战斗光环保持输入值。
    assert_eq!(ghost.stats.attack.turn, 7);
    assert_eq!(ghost.stats.health.turn, 4);
    assert_eq!(ghost.stats.attack.aura, 3);
    assert_eq!(ghost.stats.health.aura, 6);
}
