mod common;

use std::collections::BTreeSet;

use common::{effects_of, minion, test_catalog, test_playstyles};
use rule_engine::combat;
use rule_engine::data::{Card, CardType, Catalog, Era, Playstyle, Skill};
use rule_engine::engine::Engine;
use rule_engine::events::GameEvent;
use rule_engine::hash::state_hash;
use rule_engine::rng::RngStreams;
use rule_engine::settle::{Pairing, damage_formula, ghost_board, pair_round, rank_of};
use rule_engine::state::{Phase, Seat};

/// 往目录里插入一张合成随从卡，避免测试依赖真实卡面数值。
fn add_minion_card(catalog: &mut Catalog, id: &str, level: u8, attack: i32, health: i32) {
    catalog.cards.insert(
        id.to_string(),
        Card {
            id: id.to_string(),
            card_type: CardType::Minion,
            playstyle: Playstyle::Mammal,
            taxon: "hominidae".to_string(),
            specialty: Vec::new(),
            subtype: None,
            level,
            era: None,
            attack: Some(attack),
            health: Some(health),
            skills: Vec::new(),
            is_scholar: false,
            is_hero: false,
            cost: Some(1),
        },
    );
}

/// 插入一张带单个关键词的合成随从卡。
fn add_keyword_card(
    catalog: &mut Catalog,
    id: &str,
    level: u8,
    attack: i32,
    health: i32,
    keyword: &str,
) {
    let skill_id = format!("{id}-keyword");
    catalog.skills.insert(
        skill_id.clone(),
        Skill {
            id: skill_id.clone(),
            keyword: Some(keyword.to_string()),
            effects: effects_of("[]"),
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
            level,
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

fn covered_seats(pairings: &[Pairing]) -> Vec<Seat> {
    let mut seats: Vec<Seat> = pairings
        .iter()
        .flat_map(|pairing| [pairing.a, pairing.b])
        .collect();
    seats.sort_unstable();
    seats.dedup();
    seats
}

#[test]
fn pairing_avoids_previous_opponent_when_possible() {
    let (_, mut state) = Engine::new(test_catalog(), [1; 32], test_playstyles());
    state.previous_opponent.insert(0, Some(1));
    state.previous_opponent.insert(1, Some(0));

    for n in 0..32u8 {
        state.rng = RngStreams::from_seed([n; 32]);
        let pairings = pair_round(&mut state);
        assert_eq!(pairings.len(), 4);
        assert!(pairings.iter().all(|pairing| pairing.ghost.is_none()));
        assert!(
            !pairings.iter().any(|pairing| {
                (pairing.a == 0 && pairing.b == 1) || (pairing.a == 1 && pairing.b == 0)
            }),
            "重复的上一场对手不应再次配对：{pairings:?}"
        );
        assert_eq!(covered_seats(&pairings), (0..8).collect::<Vec<Seat>>());
    }
}

#[test]
fn pairing_retries_are_bounded_and_deterministic() {
    // 座位 0 与 1、2、3 都交过手，4 人局无法完全排除上一场对手。
    let (_, base) = Engine::new(test_catalog(), [2; 32], test_playstyles());
    let mut base = base;
    for seat in 4..8 {
        base.players[seat].eliminated = true;
    }
    for seat in 1..4 {
        base.previous_opponent.insert(seat, Some(0));
    }

    let mut first = base.clone();
    let mut second = base.clone();
    first.rng = RngStreams::from_seed([9; 32]);
    second.rng = RngStreams::from_seed([9; 32]);

    let first_pairings = pair_round(&mut first);
    let second_pairings = pair_round(&mut second);

    assert_eq!(first_pairings, second_pairings);
    assert_eq!(first_pairings.len(), 2);
    assert_eq!(covered_seats(&first_pairings), vec![0, 1, 2, 3]);
}

#[test]
fn odd_player_fights_random_eliminated_ghost() {
    let (_, mut state) = Engine::new(test_catalog(), [3; 32], test_playstyles());
    // 座位 7 生命归零但尚未标记出局，同样不参与配对；座位 2、5 已出局。
    state.players[7].hp = 0;
    state.players[2].eliminated = true;
    state.players[5].eliminated = true;
    state.ranking = vec![5, 2];

    let pairings = pair_round(&mut state);
    assert_eq!(pairings.len(), 3);
    assert_eq!(covered_seats(&pairings), vec![0, 1, 3, 4, 6]);

    let ghosts: Vec<&Pairing> = pairings.iter().filter(|p| p.ghost.is_some()).collect();
    assert_eq!(ghosts.len(), 1);
    assert!(
        matches!(ghosts[0].ghost, Some(2) | Some(5)),
        "幽灵必须是某名出局者"
    );
    assert_eq!(ghosts[0].a, ghosts[0].b);
}

#[test]
fn odd_player_without_eliminated_still_fights_empty_ghost() {
    let (_, mut state) = Engine::new(test_catalog(), [3; 32], test_playstyles());
    state.players[7].hp = 0;

    let pairings = pair_round(&mut state);
    assert_eq!(pairings.len(), 4);
    let lone: Vec<&Pairing> = pairings.iter().filter(|p| p.a == p.b).collect();
    assert_eq!(lone.len(), 1, "落单座位仍必须产生配对");
    assert_eq!(lone[0].ghost, None, "没有出局者时幽灵为空");
}

#[test]
fn ghost_seat_draw_is_deterministic_per_seed() {
    let (_, mut base) = Engine::new(test_catalog(), [3; 32], test_playstyles());
    base.players[7].hp = 0;
    base.players[2].eliminated = true;
    base.players[5].eliminated = true;
    base.ranking = vec![5, 2];

    let ghost_of = |pairings: &[Pairing]| {
        pairings
            .iter()
            .find(|pairing| pairing.a == pairing.b)
            .and_then(|pairing| pairing.ghost)
    };

    let mut first = base.clone();
    first.rng = RngStreams::from_seed([17; 32]);
    let mut second = first.clone();
    let first_ghost = ghost_of(&pair_round(&mut first));
    let second_ghost = ghost_of(&pair_round(&mut second));
    assert_eq!(first_ghost, second_ghost, "同一颗种子必须抽到同一名幽灵");
    assert!(matches!(first_ghost, Some(2) | Some(5)));

    let mut seen = BTreeSet::new();
    for seed in 0..64u8 {
        let mut state = base.clone();
        state.rng = RngStreams::from_seed([seed; 32]);
        let ghost = ghost_of(&pair_round(&mut state)).expect("有出局者时必须抽到幽灵");
        seen.insert(ghost);
    }
    assert_eq!(seen, BTreeSet::from([2, 5]), "不同种子应能抽到全部出局者");
}

#[test]
fn damage_formula_uses_era_and_star_sum() {
    let config = &test_catalog().config;
    // 3 + ceil(7 / 2) = 3 + 4 = 7
    assert_eq!(damage_formula(config, Era::Classical, 7), 7);
    // 2 + ceil(3 / 2) = 2 + 2 = 4
    assert_eq!(damage_formula(config, Era::Primitive, 3), 4);
}

#[test]
fn damage_is_capped_and_floored() {
    let config = &test_catalog().config;
    assert_eq!(damage_formula(config, Era::Future, 40), 15);
    assert_eq!(damage_formula(config, Era::Prehistoric, 0), 1);
}

#[test]
fn finish_recruit_pairs_alive_players_and_enters_combat() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [5; 32], test_playstyles());
    state.phase = Phase::Recruit;
    state.players[6].hp = 0;
    state.players[7].eliminated = true;

    let events = engine.finish_recruit(&mut state).unwrap();

    assert_eq!(state.phase, Phase::Combat);
    assert_eq!(state.pairings.len(), 3);
    assert_eq!(
        covered_seats(&state.pairings),
        (0..6).collect::<Vec<Seat>>()
    );
    assert_eq!(
        events,
        vec![GameEvent::PhaseChanged {
            phase: Phase::Combat,
            turn: state.turn,
        }]
    );
}

#[test]
fn finish_recruit_after_timeout_enters_combat() {
    // 超时路径：未全员 EndRecruit，阶段仍为 Recruit，finish_recruit 生成配对并推进。
    let (mut engine, mut state) = Engine::new(test_catalog(), [6; 32], test_playstyles());
    state.phase = Phase::Recruit;

    let events = engine.finish_recruit(&mut state).unwrap();

    assert_eq!(state.phase, Phase::Combat);
    assert_eq!(state.pairings.len(), 4);
    assert_eq!(
        events,
        vec![GameEvent::PhaseChanged {
            phase: Phase::Combat,
            turn: state.turn,
        }]
    );
}

#[test]
fn finish_recruit_is_idempotent_after_all_ready() {
    // 全员就绪进入 Combat 后连续调用：配对与状态哈希都不变。
    let (mut engine, mut state) = Engine::new(test_catalog(), [8; 32], test_playstyles());
    state.phase = Phase::Recruit;
    let _ = engine.finish_recruit(&mut state).unwrap();
    assert_eq!(state.phase, Phase::Combat);
    assert_eq!(state.pairings.len(), 4);
    let pairings = state.pairings.clone();
    let before = state_hash(&state);

    let events = engine.finish_recruit(&mut state).unwrap();
    let second = engine.finish_recruit(&mut state).unwrap();

    assert!(events.is_empty());
    assert!(second.is_empty());
    assert_eq!(state.pairings, pairings, "重复调用不重新配对");
    assert_eq!(state_hash(&state), before, "重复调用不改变状态");
}

#[test]
fn finish_recruit_pairs_without_phase_event_after_all_ready() {
    // 全员 EndRecruit 后阶段已进入 Combat：finish_recruit 只生成配对，不重复发阶段事件。
    let (mut engine, mut state) = Engine::new(test_catalog(), [7; 32], test_playstyles());
    state.phase = Phase::Recruit;
    for player in &mut state.players {
        player.recruit_done = true;
    }
    state.phase = Phase::Combat;
    state.pairings.clear();
    assert!(state.pairings.is_empty());

    let events = engine.finish_recruit(&mut state).unwrap();

    assert_eq!(state.phase, Phase::Combat);
    assert_eq!(state.pairings.len(), 4, "Combat 且无配对时补齐一次");
    assert!(events.is_empty(), "阶段事件已在全员结束时发出");
}

#[test]
fn combat_resolution_applies_damage_and_history() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [4; 32], test_playstyles());
    add_minion_card(&mut engine.catalog, "big", 3, 10, 10);
    add_minion_card(&mut engine.catalog, "small", 1, 1, 1);

    state.phase = Phase::Combat;
    state.first_seat = 0;
    state.last_winner = None;
    state.players[1].era = Era::Primitive;
    state.players[0].board = vec![minion(&engine.catalog, 10, "big")];
    state.players[1].board = vec![minion(&engine.catalog, 20, "small")];
    state.pairings = vec![Pairing {
        a: 0,
        b: 1,
        ghost: None,
    }];

    let events = engine.resolve_combat(&mut state).unwrap().events;

    // 胜方 1 张 3 星随从：ceil(3 / 2) = 2；败者时代 2，合计 4。
    assert_eq!(state.pending_settle.get(&1), Some(&4));
    assert_eq!(state.players[0].damage_dealt, 4);
    assert_eq!(state.players[1].damage_dealt, 0);
    assert_eq!(state.last_winner, Some(0));
    assert!(state.last_won.contains(&0));
    assert_eq!(state.previous_opponent.get(&0), Some(&Some(1)));
    assert_eq!(state.previous_opponent.get(&1), Some(&Some(0)));
    assert_eq!(state.phase, Phase::Settle);
    assert_eq!(
        events,
        vec![
            GameEvent::CombatResolved {
                winner: 0,
                loser: 1,
                damage: 4,
            },
            GameEvent::PhaseChanged {
                phase: Phase::Settle,
                turn: state.turn,
            },
        ]
    );
}

#[test]
fn ghost_loss_damages_like_real_combat() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [4; 32], test_playstyles());
    add_minion_card(&mut engine.catalog, "strong", 2, 10, 10);
    add_minion_card(&mut engine.catalog, "weak", 1, 1, 1);

    state.phase = Phase::Combat;
    state.players[0].board = vec![minion(&engine.catalog, 1, "weak")];
    state.players[3].eliminated = true;
    state.players[3].board = vec![minion(&engine.catalog, 2, "strong")];
    state.ranking = vec![3];
    state.pairings = vec![Pairing {
        a: 0,
        b: 0,
        ghost: Some(3),
    }];

    let outcome = engine.resolve_combat(&mut state).unwrap();

    let damage = *state
        .pending_settle
        .get(&0)
        .expect("幽灵获胜后落单座位应被扣血");
    assert!(damage > 0);
    assert!(!state.last_won.contains(&0), "输给幽灵不应计入胜者");
    let report = outcome.reports.get(&0).expect("幽灵战应有该座位的报告");
    assert!(report.mirror);
    assert_eq!(report.opponent_seat, Some(3));
    assert_eq!(report.damage, damage);
    assert_eq!(report.winner, Some(3), "幽灵获胜时战报胜者是幽灵归属座位");
}

#[test]
fn ghost_win_writes_last_won() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [4; 32], test_playstyles());
    add_minion_card(&mut engine.catalog, "strong", 2, 10, 10);
    add_minion_card(&mut engine.catalog, "weak", 1, 1, 1);

    state.phase = Phase::Combat;
    state.players[0].board = vec![minion(&engine.catalog, 1, "strong")];
    state.players[3].eliminated = true;
    state.players[3].board = vec![minion(&engine.catalog, 2, "weak")];
    state.ranking = vec![3];
    state.pairings = vec![Pairing {
        a: 0,
        b: 0,
        ghost: Some(3),
    }];

    let outcome = engine.resolve_combat(&mut state).unwrap();

    assert!(state.last_won.contains(&0));
    assert_eq!(state.last_winner, Some(0));
    assert!(state.pending_settle.is_empty());
    let report = outcome.reports.get(&0).expect("幽灵战应有该座位的报告");
    assert_eq!(report.damage, 0);
    assert_eq!(report.winner, Some(0));
}

#[test]
fn ghost_board_keeps_permanent_stats() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [4; 32], test_playstyles());
    add_minion_card(&mut engine.catalog, "plain", 1, 2, 3);
    let mut ghost = minion(&engine.catalog, 7, "plain");
    ghost.stats.attack.add_permanent(3);
    ghost.stats.health.add_permanent(3);
    state.players[3].eliminated = true;
    state.players[3].board = vec![ghost];

    let board = ghost_board(&engine, &state, Some(3));

    assert_eq!(board.len(), 1);
    assert_eq!(board[0].stats.attack.permanent, 3);
    assert_eq!(board[0].stats.health.permanent, 3);
}

#[test]
fn ghost_board_folds_own_aura() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [4; 32], test_playstyles());
    state.players[3].eliminated = true;
    state.players[3].board = vec![
        minion(&engine.catalog, 1, "qinghao"),
        minion(&engine.catalog, 2, "juzi"),
    ];
    engine.recompute_auras(&mut state);
    assert!(
        !state.auras.contains_key(&1),
        "出局者实例不在共享光环缓存中"
    );

    let board = ghost_board(&engine, &state, Some(3));

    let juzi = board
        .iter()
        .find(|minion| minion.instance_id == 2)
        .expect("幽灵应包含出局者棋盘");
    assert_eq!(
        combat::effective_attack(&engine.catalog, juzi),
        2,
        "受惠随从应叠加出局者自己的 +1 攻击光环"
    );
    assert_eq!(
        combat::effective_health(&engine.catalog, juzi),
        4,
        "受惠随从应叠加出局者自己的 +1 生命光环"
    );
    assert!(state.auras.is_empty(), "幽灵光环不得写回 state.auras");
}

#[test]
fn ghost_board_of_none_is_empty() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [4; 32], test_playstyles());
    add_minion_card(&mut engine.catalog, "plain", 1, 2, 3);
    // 棋盘上有随从也不能让「没有幽灵」凭空产出阵容。
    state.players[0].board = vec![minion(&engine.catalog, 1, "plain")];

    assert!(
        ghost_board(&engine, &state, None).is_empty(),
        "ghost 为 None 时幽灵阵容必须为空"
    );
}

#[test]
fn ghost_fight_with_empty_ghost_still_resolves() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [4; 32], test_playstyles());
    add_minion_card(&mut engine.catalog, "strong", 1, 10, 10);

    state.phase = Phase::Combat;
    state.players[0].board = vec![minion(&engine.catalog, 1, "strong")];
    state.pairings = vec![Pairing {
        a: 0,
        b: 0,
        ghost: None,
    }];

    let outcome = engine.resolve_combat(&mut state).unwrap();

    let report = outcome.reports.get(&0).expect("空幽灵同样应有报告");
    assert!(report.mirror);
    assert!(report.opponent_board.is_empty());
    assert!(state.last_won.contains(&0));
    assert!(state.pending_settle.is_empty());
}

#[test]
fn ghost_report_renumbers_disjoint_ids() {
    // 幽灵与己方实例编号重叠时，报告里的动作只能靠重新编号区分侧别。
    let (mut engine, mut state) = Engine::new(test_catalog(), [3; 32], test_playstyles());
    add_minion_card(&mut engine.catalog, "strong", 1, 10, 10);
    add_minion_card(&mut engine.catalog, "weak", 1, 1, 1);

    state.phase = Phase::Combat;
    // 己方禁攻，保证幽灵一定会主动出手；否则己方先手被反击致死后，
    // 幽灵没有主动攻击动作可断言。
    let mut own_minion = minion(&engine.catalog, 1, "weak");
    own_minion.cannot_attack = true;
    state.players[0].board = vec![own_minion];
    state.players[3].eliminated = true;
    state.players[3].board = vec![minion(&engine.catalog, 1, "strong")];
    state.ranking = vec![3];
    state.pairings = vec![Pairing {
        a: 0,
        b: 0,
        ghost: Some(3),
    }];

    let report = engine
        .resolve_combat(&mut state)
        .unwrap()
        .reports
        .remove(&0)
        .expect("幽灵座位应有一份报告");

    let own: BTreeSet<u64> = report
        .own_board
        .iter()
        .map(|minion| minion.instance_id)
        .collect();
    let ghost: BTreeSet<u64> = report
        .opponent_board
        .iter()
        .map(|minion| minion.instance_id)
        .collect();
    assert!(
        own.is_disjoint(&ghost),
        "己方与幽灵的实例编号不能重叠：己方 {own:?}，幽灵 {ghost:?}"
    );
    assert_eq!(report.own_board[0].instance_id, 1, "己方编号保持原值");

    // 幽灵方 10/10 攻击己方 1/1：这条动作的来源必须在幽灵棋盘上，
    // 否则重号会让「幽灵攻击」被当成己方随从的动作。
    let ghost_attack = report
        .actions
        .iter()
        .find_map(|action| match action {
            combat::CombatAction::Attack { attacker, .. } if ghost.contains(attacker) => {
                Some(*attacker)
            }
            _ => None,
        })
        .expect("幽灵应至少攻击一次");
    assert!(
        !own.contains(&ghost_attack),
        "幽灵攻击的来源 {ghost_attack} 不应落在己方棋盘"
    );
}

#[test]
fn mutual_wipe_in_real_combat_is_a_draw() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [4; 32], test_playstyles());
    state.phase = Phase::Combat;
    state.players[0].board = vec![];
    state.players[1].board = vec![];
    state.last_winner = Some(1);
    state.pairings = vec![Pairing {
        a: 0,
        b: 1,
        ghost: None,
    }];

    let outcome = engine.resolve_combat(&mut state).unwrap();

    assert!(state.pending_settle.is_empty(), "互清不应扣任何一方的血");
    assert!(state.last_won.is_empty(), "互清不产生胜者");
    assert_eq!(state.last_winner, Some(1), "平局不改变先手");
    for seat in [0u8, 1u8] {
        let report = outcome.reports.get(&seat).expect("双方都应收到报告");
        assert_eq!(report.winner, None);
        assert_eq!(report.damage, 0);
    }
    assert!(
        outcome
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::CombatDrawn { a: 0, b: 1 }))
    );
}

#[test]
fn ghost_mutual_wipe_is_a_draw() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [4; 32], test_playstyles());
    // 落单座位与幽灵都为空阵容 → 双方同回合清场。
    state.phase = Phase::Combat;
    state.players[0].board = vec![];
    state.pairings = vec![Pairing {
        a: 0,
        b: 0,
        ghost: None,
    }];

    let outcome = engine.resolve_combat(&mut state).unwrap();

    assert!(state.pending_settle.is_empty(), "平局不应扣血");
    assert!(!state.last_won.contains(&0), "平局不应计入胜者");
    let report = outcome.reports.get(&0).expect("幽灵平局同样产生报告");
    assert_eq!(report.winner, None);
    assert_eq!(report.damage, 0);
    assert!(
        outcome
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::CombatDrawn { a: 0, b: 0 }))
    );
}

#[test]
fn elimination_order_defines_ranking() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [7; 32], test_playstyles());
    state.phase = Phase::Settle;
    state.turn = 5;
    for seat in 4..8 {
        state.players[seat].eliminated = true;
    }
    state.ranking = vec![7, 6, 5, 4];
    state.pending_settle.insert(0, 50);
    state.pending_settle.insert(2, 50);

    let events = engine.resolve_settle(&mut state).unwrap();

    assert_eq!(state.players[0].hp, -10);
    assert_eq!(state.players[2].hp, -10);
    assert!(state.players[0].eliminated);
    assert!(state.players[2].eliminated);
    assert_eq!(state.ranking, vec![2, 0, 7, 6, 5, 4]);
    assert_eq!(state.phase, Phase::Settle);
    assert!(state.pending_settle.is_empty());
    assert!(state.pairings.is_empty());

    let eliminated: Vec<(Seat, u8)> = events
        .iter()
        .filter_map(|event| match event {
            GameEvent::PlayerEliminated { seat, place } => Some((*seat, *place)),
            _ => None,
        })
        .collect();
    assert_eq!(eliminated, vec![(0, 4), (2, 3)]);

    let damaged: Vec<(Seat, i32)> = events
        .iter()
        .filter_map(|event| match event {
            GameEvent::DamageTaken { seat, amount } => Some((*seat, *amount)),
            _ => None,
        })
        .collect();
    assert_eq!(damaged, vec![(0, 50), (2, 50)]);
}

#[test]
fn turn_20_forces_ranking() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [8; 32], test_playstyles());
    state.phase = Phase::Settle;
    state.turn = 20;
    for seat in 6..8 {
        state.players[seat].eliminated = true;
    }
    state.ranking = vec![7, 6];

    state.players[0].hp = 30;
    state.players[1].hp = 30;
    state.players[2].hp = 20;
    state.players[3].hp = 20;
    state.players[4].hp = 10;
    state.players[5].hp = 10;
    state.last_won.insert(1);
    state.players[2].damage_dealt = 5;
    state.players[3].damage_dealt = 5;
    state.players[4].damage_dealt = 9;
    state.players[5].damage_dealt = 1;

    let events = engine.resolve_settle(&mut state).unwrap();

    assert_eq!(state.phase, Phase::GameOver);
    assert_eq!(state.ranking, vec![1, 0, 2, 3, 4, 5, 7, 6]);
    let mut sorted = state.ranking.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, (0..8).collect::<Vec<Seat>>());
    assert_eq!(
        events.last(),
        Some(&GameEvent::GameOver {
            ranking: vec![1, 0, 2, 3, 4, 5, 7, 6],
        })
    );
}

#[test]
fn last_survivor_wins_immediately() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [9; 32], test_playstyles());
    state.phase = Phase::Settle;
    state.turn = 5;
    for seat in 0..6 {
        state.players[seat].eliminated = true;
    }
    state.ranking = vec![5, 4, 3, 2, 1, 0];
    state.pending_settle.insert(6, 50);

    let events = engine.resolve_settle(&mut state).unwrap();

    assert_eq!(state.phase, Phase::GameOver);
    assert!(state.players[6].eliminated);
    assert_eq!(state.ranking, vec![7, 6, 5, 4, 3, 2, 1, 0]);
    assert!(events.contains(&GameEvent::PlayerEliminated { seat: 6, place: 2 }));
    assert_eq!(
        events.last(),
        Some(&GameEvent::GameOver {
            ranking: vec![7, 6, 5, 4, 3, 2, 1, 0],
        })
    );
}

#[test]
fn wrong_phase_calls_are_rejected_without_mutation() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [6; 32], test_playstyles());
    let before = state_hash(&state);
    assert!(engine.finish_recruit(&mut state).is_err());
    assert_eq!(before, state_hash(&state));

    state.phase = Phase::Recruit;
    let before = state_hash(&state);
    assert!(engine.resolve_combat(&mut state).is_err());
    assert_eq!(before, state_hash(&state));

    state.phase = Phase::Combat;
    let before = state_hash(&state);
    assert!(engine.resolve_settle(&mut state).is_err());
    assert_eq!(before, state_hash(&state));

    state.phase = Phase::Settle;
    assert!(engine.resolve_settle(&mut state).is_ok());
}

#[test]
fn start_turn_accepts_settle_and_returns_to_recruit() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [10; 32], test_playstyles());
    state.phase = Phase::Settle;
    state.turn = 3;

    let events = engine.start_turn(&mut state).unwrap();

    assert_eq!(state.phase, Phase::Recruit);
    assert_eq!(state.turn, 4);
    assert!(events.contains(&GameEvent::PhaseChanged {
        phase: Phase::Recruit,
        turn: 4,
    }));
}

#[test]
fn four_pairs_emit_events_in_pairing_order_and_record_opponents() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [14; 32], test_playstyles());
    add_minion_card(&mut engine.catalog, "strong", 1, 10, 10);
    add_minion_card(&mut engine.catalog, "weak", 1, 1, 1);

    state.phase = Phase::Combat;
    for seat in 0..8u64 {
        let card_id = if seat % 2 == 0 { "strong" } else { "weak" };
        state.players[seat as usize].board = vec![minion(&engine.catalog, seat + 100, card_id)];
    }
    let expected = [(0, 1), (2, 3), (4, 5), (6, 7)];
    state.pairings = expected
        .iter()
        .map(|&(a, b)| Pairing { a, b, ghost: None })
        .collect();

    let events = engine.resolve_combat(&mut state).unwrap().events;

    let resolved: Vec<&GameEvent> = events
        .iter()
        .filter(|event| matches!(event, GameEvent::CombatResolved { .. }))
        .collect();
    assert_eq!(resolved.len(), 4);
    for (index, &(a, b)) in expected.iter().enumerate() {
        match resolved[index] {
            GameEvent::CombatResolved {
                winner,
                loser,
                damage,
            } => {
                assert_eq!((*winner, *loser), (a, b));
                assert!(*damage >= 1);
            }
            _ => panic!("第 {index} 个事件应为 CombatResolved"),
        }
        assert_eq!(state.previous_opponent.get(&a), Some(&Some(b)));
        assert_eq!(state.previous_opponent.get(&b), Some(&Some(a)));
    }
    assert_eq!(
        events.last(),
        Some(&GameEvent::PhaseChanged {
            phase: Phase::Settle,
            turn: state.turn,
        })
    );
}

#[test]
fn action_cap_draw_uses_settle_seat_tiebreak() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [12; 32], test_playstyles());
    add_keyword_card(&mut engine.catalog, "wind", 1, 0, 1, "windfury");
    add_minion_card(&mut engine.catalog, "wall", 1, 0, 100);

    // 100 个 0 攻连击随从共 200 个令牌，把行动数推到上限而双方仍存活。
    state.phase = Phase::Combat;
    state.first_seat = 1;
    state.players[0].board = vec![minion(&engine.catalog, 500, "wall")];
    state.players[1].board = (1..=100)
        .map(|index| minion(&engine.catalog, index, "wind"))
        .collect();
    state.pairings = vec![Pairing {
        a: 1,
        b: 0,
        ghost: None,
    }];

    let events = engine.resolve_combat(&mut state).unwrap().events;

    // 生命与累计伤害相同，按座位号小者判胜：座位 0。
    assert_eq!(state.last_winner, Some(0));
    assert!(state.last_won.contains(&0));
    assert_eq!(
        events.first(),
        Some(&GameEvent::CombatResolved {
            winner: 0,
            loser: 1,
            damage: 2,
        })
    );
}

#[test]
fn last_won_resets_each_round_and_feeds_turn_20_ranking() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [15; 32], test_playstyles());
    add_minion_card(&mut engine.catalog, "strong", 1, 10, 10);
    add_minion_card(&mut engine.catalog, "weak", 1, 1, 1);

    // 第 1 回合：座位 0 击败座位 1。
    state.phase = Phase::Combat;
    state.players[0].board = vec![minion(&engine.catalog, 1, "strong")];
    state.players[1].board = vec![minion(&engine.catalog, 2, "weak")];
    for seat in 2..8u64 {
        state.players[seat as usize].board = vec![minion(&engine.catalog, seat + 10, "weak")];
    }
    state.pairings = [(0, 1), (2, 3), (4, 5), (6, 7)]
        .iter()
        .map(|&(a, b)| Pairing { a, b, ghost: None })
        .collect();
    engine.resolve_combat(&mut state).unwrap();
    assert!(state.last_won.contains(&0));

    // 第 2 回合：座位 0 输给座位 2，座位 1 获胜；last_won 只反映本回合。
    state.phase = Phase::Combat;
    state.players[0].board = vec![minion(&engine.catalog, 101, "weak")];
    state.players[1].board = vec![minion(&engine.catalog, 102, "strong")];
    state.players[2].board = vec![minion(&engine.catalog, 103, "strong")];
    state.players[3].board = vec![minion(&engine.catalog, 104, "weak")];
    state.players[4].board = vec![minion(&engine.catalog, 105, "strong")];
    state.players[5].board = vec![minion(&engine.catalog, 106, "weak")];
    state.players[6].board = vec![minion(&engine.catalog, 107, "strong")];
    state.players[7].board = vec![minion(&engine.catalog, 108, "weak")];
    state.pairings = [(0, 2), (1, 3), (4, 5), (6, 7)]
        .iter()
        .map(|&(a, b)| Pairing { a, b, ghost: None })
        .collect();
    engine.resolve_combat(&mut state).unwrap();
    assert!(
        !state.last_won.contains(&0),
        "上一回合的胜者不应残留在 last_won"
    );
    assert!(state.last_won.contains(&1));

    // 第 20 回合：生命与累计伤害相同，只有上一场胜者能区分名次。
    state.turn = 20;
    state.phase = Phase::Settle;
    for player in &mut state.players {
        player.hp = 20;
        player.damage_dealt = 0;
    }
    state.pending_settle.clear();
    engine.resolve_settle(&mut state).unwrap();

    assert_eq!(state.phase, Phase::GameOver);
    assert_eq!(state.ranking, vec![1, 2, 4, 6, 0, 3, 5, 7]);
    let index_of = |seat: Seat| {
        state
            .ranking
            .iter()
            .position(|candidate| *candidate == seat)
            .expect("名次应包含全部座位")
    };
    assert!(index_of(1) < index_of(0), "上一场胜者应排在败者之前");
}

#[test]
fn new_settle_fields_participate_in_state_hash() {
    let (_, base) = Engine::new(test_catalog(), [11; 32], test_playstyles());
    let before = state_hash(&base);

    let mut with_pairing = base.clone();
    with_pairing.pairings.push(Pairing {
        a: 0,
        b: 1,
        ghost: None,
    });
    assert_ne!(before, state_hash(&with_pairing));

    let mut with_pending = base.clone();
    with_pending.pending_settle.insert(0, 3);
    assert_ne!(before, state_hash(&with_pending));

    let mut with_won = base.clone();
    with_won.last_won.insert(0);
    assert_ne!(before, state_hash(&with_won));
}

#[test]
fn rank_of_hides_eliminated_in_tail() {
    let (_, mut state) = Engine::new(test_catalog(), [5; 32], test_playstyles());
    state.players[3].eliminated = true;
    state.players[3].hp = 0;
    state.ranking = vec![3];

    assert_eq!(rank_of(&state, 3), Some(8));
    let mut alive_ranks: Vec<u8> = (0..8)
        .filter(|s| *s != 3)
        .filter_map(|s| rank_of(&state, s))
        .collect();
    alive_ranks.sort_unstable();
    assert_eq!(
        alive_ranks,
        vec![1, 2, 3, 4, 5, 6, 7],
        "存活者名次应唯一且完备（1..=7）"
    );
}

#[test]
fn rank_of_recently_eliminated_ranks_better() {
    let (_, mut state) = Engine::new(test_catalog(), [5; 32], test_playstyles());
    for seat in [3, 5] {
        state.players[seat].eliminated = true;
        state.players[seat].hp = 0;
    }
    // `ranking` 的约定是「最近出局在前」：下标 0 的名次紧挨存活者。
    state.ranking = vec![5, 3];

    assert_eq!(rank_of(&state, 5), Some(7));
    assert_eq!(rank_of(&state, 3), Some(8));
}

#[test]
fn rank_of_matches_compare_alive_order() {
    let (_, mut state) = Engine::new(test_catalog(), [6; 32], test_playstyles());
    state.players[0].damage_dealt = 5;
    state.players[1].damage_dealt = 0;
    assert_eq!(rank_of(&state, 0), Some(1), "同生命时累计伤害高者名次靠前");
    assert_eq!(rank_of(&state, 1), Some(2));

    // 同生命、同伤害，只有 `last_won` 不同：上战胜者靠前。
    let (_, mut state) = Engine::new(test_catalog(), [6; 32], test_playstyles());
    state.last_won.insert(1);
    assert_eq!(
        rank_of(&state, 1),
        Some(1),
        "同生命同伤害时上战胜者名次靠前"
    );
    assert_eq!(rank_of(&state, 0), Some(2));

    // 所有键相同：座位号小者靠前，名次按座位升序连续。
    let (_, state) = Engine::new(test_catalog(), [6; 32], test_playstyles());
    let ranks: Vec<Option<u8>> = (0..8).map(|seat| rank_of(&state, seat)).collect();
    assert_eq!(
        ranks,
        vec![
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            Some(5),
            Some(6),
            Some(7),
            Some(8)
        ],
        "所有比较键相同时按座位号升序"
    );
}
