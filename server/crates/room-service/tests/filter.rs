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
use rule_engine::combat::CombatAction;
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
    MinionInstance {
        instance_id,
        card_id: "baiyi".to_string(),
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
        attacks_left: 1,
        cannot_attack: false,
    }
}

/// 真实战斗报告：双方各两个随从，含一条被护盾挡下的攻击。
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
        for_seat(&event, 0).is_none(),
        "买随从是私有信息，不得发给其他座位"
    );

    let mapped = for_seat(&event, 2).expect("操作者应收到自己的购买事件");
    assert_eq!(mapped.kind, GameEventKind::MinionBought);
    assert_eq!(mapped.card_id.as_deref(), Some("baiyi"));
    assert_eq!(mapped.seat, Some(2));
}

#[test]
fn elimination_is_visible_to_all() {
    let event = GameEvent::PlayerEliminated { seat: 3, place: 5 };

    for seat in 0..8 {
        let mapped = for_seat(&event, seat).expect("淘汰是公开信息，应发给所有座位");
        assert_eq!(mapped.kind, GameEventKind::PlayerEliminated);
        assert_eq!(mapped.seat, Some(3), "事件应标明被淘汰的座位");
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
            for_seat(&event, seat).is_none(),
            "战斗结果只走 CombatResult，不得出现在 StateUpdate.events"
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
            .map(|event| for_seat(event, seat).expect("回合与阶段变化应发给所有座位"))
            .collect();
        assert_eq!(mapped[0].kind, GameEventKind::TurnStarted);
        assert_eq!(mapped[0].turn, std::num::NonZeroU64::new(3));
        assert_eq!(mapped[1].kind, GameEventKind::PhaseChanged);
        assert_eq!(mapped[1].turn, std::num::NonZeroU64::new(3));
        assert_eq!(mapped[2].kind, GameEventKind::GameOver);
        assert_eq!(mapped[2].ranking, vec![0, 1, 2, 3, 4, 5, 6, 7]);
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
    let allowed = ["attack", "death", "reborn", "contagion"];

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
        vec!["attack", "attack", "death", "reborn", "contagion"]
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
            connected: true,
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
        intent: ProtocolIntent::BuyMinion {
            slot,
            target_minion_id: None,
        },
    });

    assert_eq!(updates.len(), 1, "购买是私有操作，只应产生操作者的一条更新");
    assert_eq!(updates[0].player_id, "p0");
    let UpdateKind::View { events } = &updates[0].kind else {
        panic!("购买应产生 View 更新");
    };
    assert!(
        events
            .iter()
            .any(|event| event.kind == GameEventKind::MinionBought),
        "操作者的 StateUpdate.events 应含 MinionBought"
    );

    // 经信封编码后再解析 JSON，确认下发载荷里确实带上了事件。
    let envelope = task.envelope_for(&updates[0]).expect("更新应能编码");
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

    // 其他座位既没有收到这条更新，快照事件的载荷也不含它。
    for seat in 1..8 {
        assert!(
            !updates
                .iter()
                .any(|update| update.player_id == format!("p{seat}")),
            "座位 {seat} 不应收到购买者的私有更新"
        );
        let state_update = task.state_update(seat).expect("状态应可用");
        assert!(
            state_update.events.is_empty(),
            "座位 {seat} 的 StateUpdate.events 不得包含其他座位的购买事件"
        );
    }
}
