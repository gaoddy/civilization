//! 逐座位事件过滤与战斗回放测试。
//!
//! 覆盖三件事：私有事件只发给本人、公开事件发给所有人、战斗报告按接收座位
//! 映射为固定动作类型的 `CombatResult`。

use std::collections::BTreeMap;
use std::sync::Arc;

use protocol::GameEventKind;
use protocol::intents::Intent as ProtocolIntent;
use room_service::clock::{Clock, FixedClock};
use room_service::envelope::SeatChannel;
use room_service::filter::{combat_result, for_seat};
use room_service::game::{GameTask, RoomCommand, UpdateKind};
use room_service::room::PlayerSlot;
use room_service::view::SeatInfo;
use rule_engine::combat::{self, CombatAction};
use rule_engine::data::Catalog;
use rule_engine::events::GameEvent;
use rule_engine::settle::SeatCombatReport;
use rule_engine::state::{MinionInstance, Seat};

const CARDS_JSON: &str = include_str!("../../../data/v0.1/cards.json");
const SKILLS_JSON: &str = include_str!("../../../data/v0.1/skills.json");
const TOKENS_JSON: &str = include_str!("../../../data/v0.1/tokens.json");
const CONFIG_JSON: &str = include_str!("../../../data/v0.1/config.json");

/// 8 个座位各用一张不同的英雄卡。
const HEROES: [&str; 8] = [
    "adam-smith",
    "darwin",
    "davinci",
    "edison",
    "einstein",
    "faraday",
    "galileo",
    "knuth",
];

fn catalog() -> Catalog {
    Catalog::from_json(CARDS_JSON, SKILLS_JSON, TOKENS_JSON, CONFIG_JSON).expect("卡表应能加载")
}

fn seats() -> [SeatInfo; 8] {
    std::array::from_fn(|index| SeatInfo {
        player_id: format!("p{index}"),
        display_name: format!("玩家{index}"),
    })
}

fn minion(instance_id: u64) -> MinionInstance {
    let mut minion = MinionInstance::new(&catalog(), instance_id, "baiyi", false);
    minion.attacks_left = 1;
    minion
}

fn snapshot(instance_id: u64, attack: i32, health: i32, alive: bool) -> combat::MinionSnapshot {
    combat::MinionSnapshot {
        instance_id,
        attack,
        health,
        alive,
    }
}

/// 真实战斗报告：双方各两个随从，含被护盾挡下的攻击、死亡、复生、传染与一次召唤。
fn real_report(seat: Seat) -> SeatCombatReport {
    SeatCombatReport {
        seat,
        opponent_seat: Some(seat + 1),
        mirror: false,
        own_board: vec![minion(101), minion(102)],
        opponent_board: vec![minion(201), minion(202)],
        actions: vec![
            CombatAction::Attack {
                attacker: 101,
                target: 201,
                damage: 2,
                blocked: false,
            },
            CombatAction::Attack {
                attacker: 202,
                target: 102,
                damage: 1,
                blocked: true,
            },
            CombatAction::Death { minion: 201 },
            CombatAction::Reborn { minion: 102 },
            CombatAction::ContagionApplied { target: 102 },
            CombatAction::Summon {
                minion: 301,
                card_id: "baiyi".to_string(),
                golden: false,
                seat: seat + 1,
                position: 2,
            },
        ],
        states: vec![
            vec![snapshot(101, 2, 2, true), snapshot(201, 1, 0, false)],
            vec![snapshot(202, 1, 1, true), snapshot(102, 1, 1, true)],
            vec![snapshot(201, 1, 0, false)],
            vec![snapshot(102, 1, 1, true)],
            vec![snapshot(102, 1, 1, true)],
            vec![snapshot(301, 1, 1, true)],
        ],
        damage: 3,
        winner: Some(seat),
    }
}

#[test]
fn purchase_events_only_reach_owning_seat() {
    let event = GameEvent::MinionBought {
        seat: 2,
        slot: 1,
        card_id: "baiyi".to_string(),
    };

    assert!(
        for_seat(&event, 0, &seats()).is_none(),
        "买随从是私有信息，不得发给其他座位"
    );

    let mapped = for_seat(&event, 2, &seats()).expect("操作者应收到自己的购买事件");
    assert_eq!(mapped.kind, GameEventKind::MinionBought);
    assert_eq!(mapped.card_id.as_deref(), Some("baiyi"));
    assert_eq!(mapped.seat, Some(2));
    assert_eq!(
        mapped.player_id.as_deref(),
        Some("p2"),
        "私有事件的 playerId 必须显式填本人"
    );
}

#[test]
fn elimination_is_visible_to_all() {
    let event = GameEvent::PlayerEliminated { seat: 3, place: 5 };

    for seat in 0..8 {
        let mapped = for_seat(&event, seat, &seats()).expect("淘汰是公开信息，应发给所有座位");
        assert_eq!(mapped.kind, GameEventKind::PlayerEliminated);
        assert_eq!(mapped.seat, Some(3), "事件应标明被淘汰的座位");
        assert_eq!(
            mapped.player_id.as_deref(),
            Some("p3"),
            "公开事件必须给出被淘汰座位的 playerId，不能省略成「自己」"
        );
        assert_eq!(
            mapped.place,
            std::num::NonZeroU64::new(5),
            "淘汰事件应带名次"
        );
    }
}

#[test]
fn combat_resolved_is_never_sent() {
    let event = GameEvent::CombatResolved {
        winner: 1,
        loser: 2,
        damage: 4,
    };

    for seat in 0..8 {
        assert!(
            for_seat(&event, seat, &seats()).is_none(),
            "战斗结果只走 CombatResult，不得出现在 StateUpdate.events"
        );
    }
}

#[test]
fn combat_drawn_is_never_sent() {
    let event = GameEvent::CombatDrawn { a: 1, b: 2 };

    for seat in 0..8 {
        assert!(
            for_seat(&event, seat, &seats()).is_none(),
            "互清平局只供观测，不得出现在 StateUpdate.events"
        );
    }
}

#[test]
fn public_turn_events_reach_all_seats() {
    let events = [
        GameEvent::TurnStarted { turn: 3 },
        GameEvent::PhaseChanged {
            phase: rule_engine::state::Phase::Recruit,
            turn: 3,
        },
        GameEvent::GameOver {
            ranking: vec![0, 1, 2, 3, 4, 5, 6, 7],
        },
    ];

    for seat in 0..8 {
        let mapped: Vec<_> = events
            .iter()
            .map(|event| for_seat(event, seat, &seats()).expect("回合与阶段变化应发给所有座位"))
            .collect();
        assert_eq!(mapped[0].kind, GameEventKind::TurnStarted);
        assert_eq!(mapped[0].turn, std::num::NonZeroU64::new(3));
        assert_eq!(mapped[1].kind, GameEventKind::PhaseChanged);
        assert_eq!(mapped[1].turn, std::num::NonZeroU64::new(3));
        assert_eq!(mapped[2].kind, GameEventKind::GameOver);
        assert_eq!(mapped[2].ranking, vec![0, 1, 2, 3, 4, 5, 6, 7]);
        for mapped in &mapped {
            assert_eq!(mapped.player_id, None, "回合/阶段/终局事件不指向单个玩家");
        }
    }
}

#[test]
fn combat_result_maps_action_indices() {
    let report = real_report(2);
    let result = combat_result(&catalog(), &report, &seats());

    assert_eq!(result.own_board.len(), 2);
    assert_eq!(result.opponent_board.len(), 2);
    assert_eq!(result.mirror, Some(false));
    assert_eq!(result.winner_id.as_deref(), Some("p2"));

    // 己方 0 号攻击对手 0 号。
    let first = &result.actions[0];
    assert_eq!(first.kind, "attack");
    assert_eq!(first.source_index, Some(0));
    assert_eq!(first.target_index, Some(0));
    assert_eq!(
        first.source_side,
        Some(protocol::CombatResultActionsItemSourceSide::Own)
    );
    assert_eq!(
        first.target_side,
        Some(protocol::CombatResultActionsItemTargetSide::Opponent)
    );
    assert_eq!(first.damage, Some(2));

    // 反击被护盾挡下：来源是对手棋盘 1 号，目标是己方 1 号，伤害为 0。
    let blocked = &result.actions[1];
    assert_eq!(blocked.kind, "attack");
    assert_eq!(blocked.source_index, Some(1));
    assert_eq!(
        blocked.source_side,
        Some(protocol::CombatResultActionsItemSourceSide::Opponent)
    );
    assert_eq!(blocked.target_index, Some(1));
    assert_eq!(
        blocked.target_side,
        Some(protocol::CombatResultActionsItemTargetSide::Own)
    );
    assert_eq!(blocked.damage, Some(0), "护盾挡伤时 damage 为 0");
}

#[test]
fn combat_result_actions_use_fixed_kinds() {
    let result = combat_result(&catalog(), &real_report(2), &seats());
    let allowed = ["attack", "death", "reborn", "contagion", "summon"];

    for action in &result.actions {
        assert!(
            allowed.contains(&action.kind.as_str()),
            "动作类型 {} 不在协议固定取值内",
            action.kind
        );
    }

    let kinds: Vec<&str> = result
        .actions
        .iter()
        .map(|action| action.kind.as_str())
        .collect();
    assert_eq!(
        kinds,
        vec!["attack", "attack", "death", "reborn", "contagion", "summon"]
    );
}

#[test]
fn mirror_combat_result_marked_mirror() {
    let catalog = catalog();
    let won = SeatCombatReport {
        seat: 1,
        opponent_seat: None,
        mirror: true,
        own_board: vec![minion(101)],
        opponent_board: vec![minion(101)],
        actions: vec![CombatAction::Attack {
            attacker: 101,
            target: 101,
            damage: 1,
            blocked: false,
        }],
        states: vec![vec![snapshot(101, 1, 1, true), snapshot(101, 1, 1, true)]],
        damage: 0,
        winner: Some(1),
    };

    let result = combat_result(&catalog, &won, &seats());
    assert_eq!(result.mirror, Some(true), "镜像战必须标记 mirror");
    assert_eq!(result.damage, 0, "镜像战不造成伤害");
    assert_eq!(result.winner_id.as_deref(), Some("p1"));

    let lost = SeatCombatReport {
        seat: 1,
        winner: None,
        ..won
    };
    let result = combat_result(&catalog, &lost, &seats());
    assert_eq!(result.mirror, Some(true));
    assert_eq!(result.damage, 0);
    assert_eq!(result.winner_id, None, "未获胜的镜像战 winnerId 为 null");
}

// ---------------------------------------------------------------------------
// 端到端：过滤后的事件随 StateUpdate 下发
// ---------------------------------------------------------------------------

fn players() -> Vec<PlayerSlot> {
    (0..8)
        .map(|seat| PlayerSlot {
            player_id: format!("p{seat}"),
            display_name: format!("玩家{seat}"),
            ready: true,
            hero_card_id: Some(HEROES[seat].to_string()),
            channel: SeatChannel::new(),
            hero_candidates: Vec::new(),
            candidate_segment: None,
            connected: true,
            disconnected_at_ms: None,
        })
        .collect()
}

fn task() -> (GameTask, FixedClock) {
    let clock = FixedClock::new(1_000_000);
    let clock_arc: Arc<dyn Clock> = Arc::new(clock.clone());
    let heroes: BTreeMap<Seat, String> = HEROES
        .iter()
        .enumerate()
        .map(|(seat, hero)| (seat as Seat, (*hero).to_string()))
        .collect();
    let task = GameTask::new(catalog(), 7, heroes, players(), clock_arc).expect("开局应当成功");
    (task, clock)
}

fn first_nonempty_slot(task: &GameTask, seat: Seat) -> i64 {
    task.player_view(seat)
        .expect("视图应可用")
        .self_
        .shop
        .iter()
        .find(|slot| slot.card_id.is_some())
        .map(|slot| slot.index)
        .expect("商店至少应有一个非空槽位")
}

#[test]
fn filtered_events_reach_only_own_seat_via_state_update() {
    let (mut task, _clock) = task();
    let slot = first_nonempty_slot(&task, 0);

    let updates = task.handle(RoomCommand::Intent {
        player_id: "p0".to_string(),
        client_intent_id: "buy-e2e".to_string(),
        intent: ProtocolIntent::BuyMinion { slot },
    });

    assert_eq!(updates.len(), 8, "购买改变公开摘要，应广播给全部 8 个座位");

    let owner = updates
        .iter()
        .find(|update| update.player_id == "p0")
        .expect("操作者应收到更新");
    let UpdateKind::View { events } = &owner.kind else {
        panic!("购买应产生 View 更新");
    };
    assert!(
        events
            .iter()
            .any(|event| event.kind == GameEventKind::MinionBought),
        "操作者的 StateUpdate.events 应含 MinionBought"
    );

    // 经信封编码后再解析 JSON，确认下发载荷里确实带上了事件。
    let owner_index = updates
        .iter()
        .position(|update| update.player_id == "p0")
        .expect("操作者更新应存在");
    let envelope = task
        .envelope_for(&updates[owner_index])
        .expect("更新应能编码");
    let payload = serde_json::to_value(&envelope.payload).expect("载荷应可序列化");
    let kinds: Vec<&str> = payload["events"]
        .as_array()
        .expect("events 应为数组")
        .iter()
        .map(|event| event["kind"].as_str().expect("kind 应为字符串"))
        .collect();
    assert!(
        kinds.contains(&"MinionBought"),
        "座位 0 的下行事件应含 MinionBought，实际为 {kinds:?}"
    );

    // 其他座位也收到视图，但事件为空：私有信息不随广播泄漏。
    for seat in 1..8 {
        let update = updates
            .iter()
            .find(|update| update.player_id == format!("p{seat}"))
            .unwrap_or_else(|| panic!("座位 {seat} 也应收到视图更新"));
        let UpdateKind::View { events } = &update.kind else {
            panic!("座位 {seat} 的更新应为 View");
        };
        assert!(
            events.is_empty(),
            "座位 {seat} 的 StateUpdate.events 不得包含其他座位的购买事件"
        );
        assert!(
            task.state_update(seat)
                .expect("状态应可用")
                .events
                .is_empty(),
            "座位 {seat} 的快照事件也不得包含私有购买事件"
        );
    }
}

#[test]
fn private_intent_broadcasts_updated_public_summary_to_all_seats() {
    let (mut task, _clock) = task();
    let slot = first_nonempty_slot(&task, 0);

    // 买入只入手牌，公开摘要不变；打上棋盘后座位 B 才能看到棋盘数变化。
    task.handle(RoomCommand::Intent {
        player_id: "p0".to_string(),
        client_intent_id: "buy-broadcast".to_string(),
        intent: ProtocolIntent::BuyMinion { slot },
    });

    // 打牌前：座位 B 看到的座位 A 棋盘数为 0。
    let before = task
        .state_update(1)
        .expect("座位 1 状态应可用")
        .view
        .players[0]
        .board_count;
    assert_eq!(before, 0, "买入不入场，座位 0 的棋盘仍为空");

    let updates = task.handle(RoomCommand::Intent {
        player_id: "p0".to_string(),
        client_intent_id: "play-broadcast".to_string(),
        intent: ProtocolIntent::PlayCard {
            hand_index: 0,
            position: None,
            target_minion_id: None,
        },
    });

    // 座位 B 立即收到视图，且视图里对手棋盘数已变化。
    let viewer = updates
        .iter()
        .find(|update| update.player_id == "p1")
        .expect("座位 B 应立即收到视图更新");
    let UpdateKind::View { events } = &viewer.kind else {
        panic!("座位 B 的更新应为 View");
    };
    assert!(events.is_empty(), "座位 B 不应收到座位 A 的私有事件");

    // 经信封编码后按 JSON 断言，确认下发载荷本身已带上新的 boardCount。
    let index = updates
        .iter()
        .position(|update| update.player_id == "p1")
        .expect("座位 B 更新应存在");
    let envelope = task.envelope_for(&updates[index]).expect("更新应能编码");
    let payload = serde_json::to_value(&envelope.payload).expect("载荷应可序列化");
    let board_count = payload["view"]["players"][0]["boardCount"]
        .as_i64()
        .expect("boardCount 应为整数");
    assert_eq!(board_count, 1, "座位 B 应立刻看到座位 A 的棋盘数为 1");
    assert_eq!(payload["events"].as_array().map(Vec::len), Some(0));
}

/// 从 `StateUpdate` JSON 载荷里取出事件 kind 列表。
fn event_kinds(events: &[serde_json::Value]) -> Vec<String> {
    events
        .iter()
        .map(|event| {
            event["kind"]
                .as_str()
                .expect("事件 kind 应为字符串")
                .to_string()
        })
        .collect()
}

#[test]
fn combat_stage_attaches_filtered_public_events() {
    let (mut task, clock) = task();

    // 互清判平局后，空棋盘对战不再产生伤害：先让座位 0 上场一个随从，
    // 保证本回合至少有一场真实战分出胜负。
    let slot = first_nonempty_slot(&task, 0);
    task.handle(RoomCommand::Intent {
        player_id: "p0".to_string(),
        client_intent_id: "buy-for-combat".to_string(),
        intent: ProtocolIntent::BuyMinion { slot },
    });
    task.handle(RoomCommand::Intent {
        player_id: "p0".to_string(),
        client_intent_id: "play-for-combat".to_string(),
        intent: ProtocolIntent::PlayCard {
            hand_index: 0,
            position: None,
            target_minion_id: None,
        },
    });
    // 战吼需要目标的随从会留下目标待选，未结算时该座位无法结束招募。
    let view = task.player_view(0).expect("视图应可用");
    if matches!(
        view.self_.pending_choices.first(),
        Some(protocol::view::PendingChoice::Target { .. })
    ) {
        let minion_id = view.self_.board[0].minion_id.clone();
        task.handle(RoomCommand::Intent {
            player_id: "p0".to_string(),
            client_intent_id: "target-for-combat".to_string(),
            intent: ProtocolIntent::ChooseTarget { minion_id },
        });
    }

    // 全员结束招募：最后一条意图触发 finish_recruit → resolve_combat，阶段停在
    // Combat 演示；阶段事件应随每个座位的 StateUpdate 下发。
    let mut last = Vec::new();
    for seat in 0..8 {
        last = task.handle(RoomCommand::Intent {
            player_id: format!("p{seat}"),
            client_intent_id: format!("end-{seat}"),
            intent: ProtocolIntent::EndRecruit,
        });
    }

    let mut events_by_player: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for update in &last {
        if !matches!(update.kind, UpdateKind::View { .. }) {
            continue;
        }
        // 只经信封编码后按 JSON 断言，不读内部 Vec 绕过过滤。
        let envelope = task.envelope_for(update).expect("更新应能编码");
        let payload = serde_json::to_value(&envelope.payload).expect("载荷应可序列化");
        let events = payload["events"]
            .as_array()
            .expect("events 应为数组")
            .clone();
        events_by_player.insert(update.player_id.clone(), events);
    }

    assert_eq!(
        events_by_player.len(),
        8,
        "阶段推进后 8 个座位都应收到带事件的视图"
    );

    let seat0 = &events_by_player["p0"];
    let seat0_kinds = event_kinds(seat0);
    assert!(
        seat0_kinds.contains(&"PhaseChanged".to_string()),
        "座位 0 的 StateUpdate.events 应含 PhaseChanged，实际为 {seat0_kinds:?}"
    );
    assert_eq!(
        task.player_view(0).expect("视图应可用").phase,
        protocol::view::Phase::Combat,
        "演示阶段应停在 Combat"
    );

    // 演示到点结算：伤害事件随下一回合的视图下发。
    let demo_ms = last
        .iter()
        .filter_map(|update| match &update.kind {
            UpdateKind::CombatResult(result) => Some(result.actions.len()),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    clock.advance((demo_ms as u64) * 320 + 2_000);
    let settle = task.tick();
    let mut damage_events = Vec::new();
    for update in &settle {
        if !matches!(update.kind, UpdateKind::View { .. }) {
            continue;
        }
        let envelope = task.envelope_for(update).expect("更新应能编码");
        let payload = serde_json::to_value(&envelope.payload).expect("载荷应可序列化");
        let events = payload["events"].as_array().cloned().unwrap_or_default();
        damage_events.extend(events);
    }

    let kinds = event_kinds(&damage_events);
    assert!(
        kinds.contains(&"DamageTaken".to_string()),
        "战斗结算后应含 DamageTaken，实际为 {kinds:?}"
    );

    // 公开伤害事件必须标明被打的玩家，不能留空让客户端当成自己。
    for event in &damage_events {
        if event["kind"] == "DamageTaken" {
            assert!(event["playerId"].is_string(), "DamageTaken 必须带 playerId");
        }
    }

    for (player, events) in &events_by_player {
        let kinds = event_kinds(events);
        assert!(
            !kinds.iter().any(|kind| kind == "CombatResolved"),
            "座位 {player} 的事件流不得包含 CombatResolved，实际为 {kinds:?}"
        );
    }
}

#[test]
fn combat_attack_carries_minion_ids_and_states() {
    let catalog = catalog();
    let result = combat_result(&catalog, &real_report(0), &seats());

    let attack = &result.actions[0];
    assert_eq!(attack.kind, "attack");
    assert_eq!(attack.source_minion_id.as_deref(), Some("101"));
    assert_eq!(attack.target_minion_id.as_deref(), Some("201"));
    let source = attack.source.as_ref().expect("攻击应带攻方状态");
    assert_eq!(source.minion_id, "101");
    assert_eq!(source.attack, 2);
    assert_eq!(source.health, 2);
    assert!(source.alive);
    let target = attack.target.as_ref().expect("攻击应带受方状态");
    assert_eq!(target.health, 0, "受方动作后生命直接取服务端快照");
    assert!(!target.alive);

    let blocked = &result.actions[1];
    assert_eq!(blocked.damage, Some(0), "护盾抵消时伤害为 0");
    assert_eq!(
        blocked.target.as_ref().map(|state| state.health),
        Some(1),
        "被挡下的攻击不改生命"
    );
}

#[test]
fn combat_summon_maps_side_position_and_minion() {
    let catalog = catalog();
    let result = combat_result(&catalog, &real_report(0), &seats());

    let summon = result.actions[5]
        .summon
        .as_ref()
        .expect("召唤动作应带 summon");
    assert_eq!(
        summon.side,
        protocol::CombatResultActionsItemSummonSide::Opponent
    );
    assert_eq!(summon.position, 2);
    assert_eq!(summon.minion.index, 2, "minion.index 与插入下标一致");
    assert_eq!(summon.minion.minion_id, "301");
    assert_eq!(summon.minion.card_id, "baiyi");
    assert!(!summon.minion.sellable, "演示中的随从不可出售");

    let own_report = SeatCombatReport {
        seat: 0,
        own_board: vec![minion(101)],
        opponent_board: vec![minion(201)],
        actions: vec![CombatAction::Summon {
            minion: 301,
            card_id: "baiyi".to_string(),
            golden: false,
            seat: 0,
            position: 1,
        }],
        states: vec![vec![snapshot(301, 1, 1, true)]],
        ..real_report(0)
    };
    let own = combat_result(&catalog, &own_report, &seats());
    assert_eq!(
        own.actions[0].summon.as_ref().map(|summon| summon.side),
        Some(protocol::CombatResultActionsItemSummonSide::Own),
        "召唤方与接收座位相同时 side 为 own"
    );
}

#[test]
fn combat_summon_uses_state_snapshot_values() {
    // 召唤物可能带光环/战斗内加成：展示数值取动作的状态快照，而不是卡面基础值。
    let catalog = catalog();
    let report = SeatCombatReport {
        seat: 0,
        own_board: vec![minion(101)],
        opponent_board: vec![minion(201)],
        actions: vec![CombatAction::Summon {
            minion: 301,
            card_id: "baiyi".to_string(),
            golden: false,
            seat: 0,
            position: 1,
        }],
        // baiyi 卡面 1/2，这里给出加成后的 5/7。
        states: vec![vec![snapshot(301, 5, 7, true)]],
        ..real_report(0)
    };

    let result = combat_result(&catalog, &report, &seats());
    let summon = result.actions[0]
        .summon
        .as_ref()
        .expect("召唤动作应带 summon");
    assert_eq!(summon.minion.attack, 5, "召唤物攻击取状态快照");
    assert_eq!(summon.minion.health, 7, "召唤物生命取状态快照");
    assert_eq!(summon.minion.card_id, "baiyi", "卡面信息仍来自动作");
    assert_eq!(summon.position, 1, "位置原样取自动作");
    assert_eq!(
        summon.minion.index, 1,
        "召唤物的 index 与 position 同为存活序列下标"
    );
}

#[test]
fn combat_death_and_reborn_carry_target_state() {
    let catalog = catalog();
    let result = combat_result(&catalog, &real_report(0), &seats());

    let death = result.actions[2]
        .target
        .as_ref()
        .expect("死亡动作带 target 状态");
    assert_eq!(death.minion_id, "201");
    assert!(!death.alive, "死亡后存活为 false");
    assert_eq!(death.health, 0);

    let reborn = result.actions[3]
        .target
        .as_ref()
        .expect("复生动作带 target 状态");
    assert_eq!(reborn.minion_id, "102");
    assert!(reborn.alive);
    assert_eq!(reborn.health, 1, "复生后生命为 1");
}
