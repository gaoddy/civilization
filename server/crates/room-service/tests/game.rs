//! 对局驱动测试：开局、意图去重、拒绝、计时推进、逐座位视图与序号。
//!
//! 全部使用 [`FixedClock`] 与进程内引擎，不启网络。商店由种子确定，因此同一
//! 测试内多次构造的初始状态一致。

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use protocol::ErrorCode;
use protocol::intents::Intent as ProtocolIntent;
use room_service::clock::FixedClock;
use room_service::envelope::SeatChannel;
use room_service::game::{GameTask, RoomCommand, SeatUpdate, UpdateKind};
use room_service::room::PlayerSlot;
use rule_engine::data::{CardType, Catalog, Skill};
use rule_engine::state::Seat;

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

/// 对局引擎种子；与测试断言的具体商店无关，只要稳定即可。
const SEED: u64 = 7;

fn catalog() -> Catalog {
    Catalog::from_json(CARDS_JSON, SKILLS_JSON, TOKENS_JSON, CONFIG_JSON).expect("卡表应能加载")
}

fn heroes() -> BTreeMap<Seat, String> {
    HEROES
        .iter()
        .enumerate()
        .map(|(seat, hero)| (seat as Seat, (*hero).to_string()))
        .collect()
}

/// 按座位号升序的 8 个玩家；下标即座位号。
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

fn task_with(catalog: Catalog) -> (GameTask, FixedClock) {
    let clock = FixedClock::new(1_000_000);
    let clock_arc: Arc<dyn room_service::clock::Clock> = Arc::new(clock.clone());
    let task = GameTask::new(catalog, SEED, heroes(), players(), clock_arc).expect("开局应当成功");
    (task, clock)
}

fn task() -> (GameTask, FixedClock) {
    task_with(catalog())
}

fn command(player: &str, client_intent_id: &str, intent: ProtocolIntent) -> RoomCommand {
    RoomCommand::Intent {
        player_id: player.to_string(),
        client_intent_id: client_intent_id.to_string(),
        intent,
    }
}

fn end_recruit(player: &str, client_intent_id: &str) -> RoomCommand {
    command(player, client_intent_id, ProtocolIntent::EndRecruit)
}

fn buy(slot: i64) -> ProtocolIntent {
    ProtocolIntent::BuyMinion {
        slot,
        target_minion_id: None,
    }
}

/// 第一个非空商店槽位。
fn first_nonempty_slot(task: &GameTask, seat: Seat) -> i64 {
    let view = task.player_view(seat).expect("视图应可用");
    view.self_
        .shop
        .iter()
        .find(|slot| slot.card_id.is_some())
        .map(|slot| slot.index)
        .expect("商店至少应有一个非空槽位")
}

/// 把更新编码成信封，返回每条更新的序号。
fn envelope_seqs(task: &mut GameTask, updates: &[SeatUpdate]) -> Vec<u64> {
    updates
        .iter()
        .map(|update| {
            task.envelope_for(update)
                .expect("更新应能编码成信封")
                .seq
                .get()
        })
        .collect()
}

fn error_codes(updates: &[SeatUpdate]) -> Vec<ErrorCode> {
    updates
        .iter()
        .filter_map(|update| match &update.kind {
            UpdateKind::Error(error) => Some(error.code),
            _ => None,
        })
        .collect()
}

#[test]
fn game_advances_to_recruit_with_shop_and_gold() {
    let (task, _clock) = task();

    let view = task.player_view(0).expect("座位 0 视图应可用");
    let expected_gold = catalog()
        .config
        .gold_curve
        .first()
        .copied()
        .expect("金币曲线非空");

    assert_eq!(view.phase, protocol::view::Phase::Recruit);
    assert_eq!(view.turn.get(), 1);
    assert_eq!(view.self_.shop.len(), 6, "商店固定 6 个槽位");
    assert_eq!(view.self_.gold, u64::from(expected_gold));
    assert_eq!(view.players.len(), 8);
    assert_eq!(view.self_.player_id, "p0");
}

#[test]
fn duplicate_client_intent_id_executes_once() {
    let (mut task, _clock) = task();
    let slot = first_nonempty_slot(&task, 0);
    let gold_before = task.player_view(0).expect("视图应可用").self_.gold;

    let first = task.handle(command("p0", "buy-1", buy(slot)));
    assert!(error_codes(&first).is_empty(), "首次购买应成功");

    let gold_after_first = task.player_view(0).expect("视图应可用").self_.gold;
    assert!(gold_after_first < gold_before, "购买应扣金币");

    // 同一 (playerId, clientIntentId) 重发：只回当前视图，不再执行。
    let second = task.handle(command("p0", "buy-1", buy(slot)));
    assert!(error_codes(&second).is_empty(), "重发不应报错");
    assert_eq!(second.len(), 1, "重发只回操作者自己的视图");
    assert_eq!(second[0].player_id, "p0");
    assert!(matches!(second[0].kind, UpdateKind::View));

    let gold_after_second = task.player_view(0).expect("视图应可用").self_.gold;
    assert_eq!(gold_after_second, gold_after_first, "金币只能扣一次");
    assert!(
        task.state_update(0).expect("状态应可用").events.is_empty(),
        "重复 Applied 的 StateUpdate 事件为空"
    );
}

#[test]
fn rejected_intent_does_not_change_state() {
    let (mut task, _clock) = task();

    // 先花光 3 金币，再升级时代必然金币不足。
    let slot = first_nonempty_slot(&task, 0);
    task.handle(command("p0", "spend", buy(slot)));
    let before =
        serde_json::to_string(&task.state_update(0).expect("状态应可用")).expect("状态应可序列化");

    let rejected = task.handle(command("p0", "upgrade", ProtocolIntent::UpgradeEra));
    assert_eq!(error_codes(&rejected), vec![ErrorCode::NotEnoughGold]);

    let after =
        serde_json::to_string(&task.state_update(0).expect("状态应可用")).expect("状态应可序列化");
    assert_eq!(before, after, "拒绝不得改变状态");

    // 同一 clientIntentId 重发返回同一条 Error。
    let repeated = task.handle(command("p0", "upgrade", ProtocolIntent::UpgradeEra));
    assert_eq!(error_codes(&repeated), vec![ErrorCode::NotEnoughGold]);
    match (&rejected[0].kind, &repeated[0].kind) {
        (UpdateKind::Error(left), UpdateKind::Error(right)) => {
            assert_eq!(left.code, right.code);
            assert_eq!(left.message, right.message);
            assert_eq!(left.client_intent_id, right.client_intent_id);
        }
        _ => panic!("重发应返回 Error"),
    }
}

#[test]
fn all_seats_end_recruit_advances_to_combat_then_settle() {
    let (mut task, _clock) = task();

    let mut combat_seats = BTreeSet::new();
    for seat in 0..8 {
        let updates = task.handle(end_recruit(&format!("p{seat}"), &format!("end-{seat}")));
        assert!(
            error_codes(&updates).is_empty(),
            "座位 {seat} 结束招募应成功"
        );
        for update in updates {
            if matches!(update.kind, UpdateKind::CombatResult(_)) {
                combat_seats.insert(update.player_id);
            }
        }
    }

    assert_eq!(combat_seats.len(), 8, "每个座位都应收到自己的 CombatResult");
    let view = task.player_view(0).expect("视图应可用");
    assert_eq!(
        view.phase,
        protocol::view::Phase::Recruit,
        "战斗与结算后进入下一回合招募"
    );
    assert_eq!(view.turn.get(), 2);
}

#[test]
fn recruit_timer_fires_at_ninety_seconds() {
    let (mut task, clock) = task();

    clock.advance(90_000);
    let updates = task.tick();
    assert!(!updates.is_empty(), "到点应推进阶段");
    assert!(
        updates
            .iter()
            .any(|update| matches!(update.kind, UpdateKind::CombatResult(_))),
        "推进应下发 CombatResult"
    );

    let view = task.player_view(0).expect("视图应可用");
    assert_eq!(view.phase, protocol::view::Phase::Recruit);
    assert_eq!(view.turn.get(), 2);

    // 同一毫秒重复 tick 幂等。
    assert!(task.tick().is_empty(), "重复 tick 不得再次推进");
}

#[test]
fn unspent_pending_choice_does_not_block_timeout() {
    // 给所有 1 级随从追加「打出时发现」技能，购买任意商店随从都会留下待选。
    let mut catalog = catalog();
    catalog.skills.insert(
        "test-discover-on-play".to_string(),
        Skill {
            id: "test-discover-on-play".to_string(),
            name: None,
            keyword: None,
            text: "发现一张随从".to_string(),
            effects: "OnPlay|Discover|self|pool=tribe,choose=1".to_string(),
            cost: None,
            uses_per_turn: None,
            match_text: None,
        },
    );
    let token_ids: BTreeSet<String> = catalog.tokens.keys().cloned().collect();
    for card in catalog.cards.values_mut() {
        if card.card_type == CardType::Minion
            && card.level == 1
            && !card.is_hero
            && !token_ids.contains(&card.id)
        {
            card.skills.push("test-discover-on-play".to_string());
        }
    }

    let (mut task, clock) = task_with(catalog);
    let slot = first_nonempty_slot(&task, 0);
    let updates = task.handle(command("p0", "discover-buy", buy(slot)));
    assert!(error_codes(&updates).is_empty(), "购买应成功");

    let view = task.player_view(0).expect("视图应可用");
    assert!(
        !view.self_.pending_choices.is_empty(),
        "购买应留下未消费的发现待选"
    );

    clock.advance(90_000);
    let updates = task.tick();
    assert!(!updates.is_empty(), "待选未消费不应阻塞超时推进");
    assert_eq!(task.player_view(0).expect("视图应可用").turn.get(), 2);
}

#[test]
fn view_never_contains_other_seats_gold_or_shop() {
    let (mut task, _clock) = task();

    // 让座位 0 花光金币，两个座位的金币与商店都不同。
    let slot = first_nonempty_slot(&task, 0);
    task.handle(command("p0", "buy-for-privacy", buy(slot)));

    let seat0 = task.player_view(0).expect("座位 0 视图应可用");
    let seat1_update = task.state_update(1).expect("座位 1 状态应可用");
    let serialized = serde_json::to_string(&seat1_update).expect("状态应可序列化");

    let seat1_shop: BTreeSet<String> = seat1_update
        .view
        .self_
        .shop
        .iter()
        .filter_map(|slot| slot.card_id.clone())
        .collect();
    let seat0_only: Vec<String> = seat0
        .self_
        .shop
        .iter()
        .filter_map(|slot| slot.card_id.clone())
        .filter(|card_id| !seat1_shop.contains(card_id))
        .collect();
    assert!(!seat0_only.is_empty(), "夹具要求两个座位的商店不同");

    for card_id in &seat0_only {
        assert!(
            !serialized.contains(card_id),
            "座位 1 的视图不应包含座位 0 独有的商店卡 {card_id}"
        );
    }
    assert!(
        !serialized.contains("\"gold\":0"),
        "座位 1 的视图不应包含座位 0 花光后的金币值"
    );
    assert!(
        serialized.contains("\"gold\":3"),
        "座位 1 应只看到自己的金币"
    );
    assert_eq!(seat0.self_.gold, 0);
}

#[test]
fn seq_increases_per_seat_independently() {
    let (mut task, _clock) = task();
    let slot = first_nonempty_slot(&task, 0);

    // 座位 0 的私有操作只推进座位 0 的序号。
    let updates = task.handle(command("p0", "seq-buy", buy(slot)));
    let seqs = envelope_seqs(&mut task, &updates);
    assert_eq!(seqs, vec![1]);
    assert_eq!(task.channel_seq(0), 1);
    assert_eq!(task.channel_seq(1), 0);

    // 座位 1 的私有操作只推进座位 1 的序号。
    let updates = task.handle(command("p1", "seq-reroll", ProtocolIntent::RerollShop));
    let seqs = envelope_seqs(&mut task, &updates);
    assert_eq!(seqs, vec![1]);
    assert_eq!(task.channel_seq(1), 1);
    assert_eq!(task.channel_seq(0), 1);

    // 座位 0 再做一次私有操作，只有座位 0 继续递增。
    let updates = task.handle(command("p0", "seq-reroll", ProtocolIntent::RerollShop));
    let seqs = envelope_seqs(&mut task, &updates);
    assert_eq!(seqs, vec![2]);
    assert_eq!(task.channel_seq(0), 2);
    assert_eq!(task.channel_seq(1), 1);
}

#[test]
fn tick_before_deadline_returns_empty() {
    let (mut task, clock) = task();

    clock.advance(89_999);
    assert!(task.tick().is_empty(), "未到点不应推进");

    let view = task.player_view(0).expect("视图应可用");
    assert_eq!(view.phase, protocol::view::Phase::Recruit);
    assert_eq!(view.turn.get(), 1);
}

#[test]
fn intent_from_unknown_player_is_rejected() {
    let (mut task, _clock) = task();
    let before =
        serde_json::to_string(&task.state_update(0).expect("状态应可用")).expect("状态应可序列化");

    let updates = task.handle(end_recruit("p99", "ghost-intent"));
    assert_eq!(error_codes(&updates), vec![ErrorCode::NotInRoom]);
    assert_eq!(updates[0].player_id, "p99");

    let after =
        serde_json::to_string(&task.state_update(0).expect("状态应可用")).expect("状态应可序列化");
    assert_eq!(before, after, "未知玩家的意图不得改变状态");
}
