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

/// 给所有 1 级随从追加「打出时发现」技能：任何一次购买都会留下待选，
/// 用于在不改引擎的前提下确定性构造 `ChooseDiscover`。
fn catalog_with_discover_on_play() -> Catalog {
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
    catalog
}

/// 从当前招募回合空过到下一回合（超时推进，含战斗与结算）。
fn advance_turn(task: &mut GameTask, clock: &FixedClock) {
    clock.advance(90_000);
    let updates = task.tick();
    assert!(!updates.is_empty(), "到点应推进到下一回合");
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
    assert!(matches!(second[0].kind, UpdateKind::View { .. }));

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
    let (mut task, clock) = task_with(catalog_with_discover_on_play());
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

    // 广播给全部 8 个座位：每个座位各消耗一次自己的序号，互不影响。
    let updates = task.handle(command("p0", "seq-buy", buy(slot)));
    let seqs = envelope_seqs(&mut task, &updates);
    assert_eq!(seqs, vec![1; 8], "每个座位都应各推进到 1");
    for seat in 0..8 {
        assert_eq!(task.channel_seq(seat), 1, "座位 {seat} 序号应为 1");
    }

    // 同一条 (playerId, clientIntentId) 重发只回操作者自己的视图，
    // 不重新广播，因此只有座位 0 的序号继续递增。
    let repeat = task.handle(command("p0", "seq-buy", buy(slot)));
    let seqs = envelope_seqs(&mut task, &repeat);
    assert_eq!(seqs, vec![2]);
    assert_eq!(task.channel_seq(0), 2);
    for seat in 1..8 {
        assert_eq!(task.channel_seq(seat), 1, "座位 {seat} 序号不应变化");
    }
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

// ---------------------------------------------------------------------------
// 十种协议意图的服务层成功路径
//
// bot 只发送一部分意图，这些用例经 `GameTask::handle(RoomCommand::Intent)`
// 覆盖其余分支，断言可观察的视图变化。
// ---------------------------------------------------------------------------

#[test]
fn sell_minion_succeeds_and_refunds_gold() {
    let (mut task, _clock) = task();
    let slot = first_nonempty_slot(&task, 0);
    let bought = task.handle(command("p0", "sell-buy", buy(slot)));
    assert!(error_codes(&bought).is_empty(), "购买应成功");

    let view = task.player_view(0).expect("视图应可用");
    assert_eq!(view.self_.board.len(), 1, "购买后棋盘应有 1 个随从");
    let gold_after_buy = view.self_.gold;
    let minion_id = view.self_.board[0].minion_id.clone();

    let updates = task.handle(command(
        "p0",
        "sell-1",
        ProtocolIntent::SellMinion { minion_id },
    ));
    assert!(error_codes(&updates).is_empty(), "出售应成功");

    let after = task.player_view(0).expect("视图应可用");
    assert!(after.self_.board.is_empty(), "出售后棋盘随从数应下降为 0");
    assert_eq!(
        after.self_.gold,
        gold_after_buy + 1,
        "出售非金色随从应返还 1 金币"
    );
    assert_eq!(updates.len(), 8, "出售也应广播视图给所有座位");
}

#[test]
fn play_card_succeeds() {
    let (mut task, _clock) = task_with(catalog_with_discover_on_play());
    let slot = first_nonempty_slot(&task, 0);
    let bought = task.handle(command("p0", "play-buy", buy(slot)));
    assert!(error_codes(&bought).is_empty(), "购买应成功");

    // 购买触发了 OnPlay 发现：先消费待选，让卡进入手牌。
    let pending = task.player_view(0).expect("视图应可用");
    assert_eq!(
        pending.self_.pending_choices.len(),
        1,
        "购买应留下一条发现待选"
    );
    let chosen = task.handle(command(
        "p0",
        "play-choose",
        ProtocolIntent::ChooseDiscover { choice: 0 },
    ));
    assert!(error_codes(&chosen).is_empty(), "选择发现应成功");

    let before = task.player_view(0).expect("视图应可用");
    assert_eq!(before.self_.hand.len(), 1, "选择后卡应进入手牌");
    let board_before = before.self_.board.len();
    // 棋盘已有随从时，需要目标的效果必须指定目标。
    let target = before
        .self_
        .board
        .first()
        .map(|minion| minion.minion_id.clone());

    let played = task.handle(command(
        "p0",
        "play-1",
        ProtocolIntent::PlayCard {
            hand_index: 0,
            target_minion_id: target,
        },
    ));
    assert!(error_codes(&played).is_empty(), "打出手牌应成功");

    let after = task.player_view(0).expect("视图应可用");
    assert_eq!(after.self_.hand.len(), 0, "打出手牌后手牌数应下降");
    assert_eq!(
        after.self_.board.len(),
        board_before + 1,
        "打出的随从应进入棋盘"
    );
}

#[test]
fn rearrange_board_succeeds() {
    let (mut task, clock) = task();
    let slot = first_nonempty_slot(&task, 0);
    task.handle(command("p0", "rearrange-buy-1", buy(slot)));
    advance_turn(&mut task, &clock);
    let slot = first_nonempty_slot(&task, 0);
    // 棋盘已有随从时，带目标战吼的随从需要指定目标才能买入。
    let target = task.player_view(0).expect("视图应可用").self_.board[0]
        .minion_id
        .clone();
    task.handle(command(
        "p0",
        "rearrange-buy-2",
        ProtocolIntent::BuyMinion {
            slot,
            target_minion_id: Some(target),
        },
    ));

    let before = task.player_view(0).expect("视图应可用");
    assert_eq!(before.self_.board.len(), 2, "两个回合应买入 2 个随从");
    let order: Vec<String> = before
        .self_
        .board
        .iter()
        .map(|minion| minion.minion_id.clone())
        .collect();
    assert_ne!(order[0], order[1], "两个随从实例应不同");

    let updates = task.handle(command(
        "p0",
        "rearrange-1",
        ProtocolIntent::RearrangeBoard {
            from_index: 0,
            to_index: 1,
        },
    ));
    assert!(error_codes(&updates).is_empty(), "调整站位应成功");

    let after = task.player_view(0).expect("视图应可用");
    assert_eq!(
        after.self_.board[0].minion_id, order[1],
        "0 号位应换成原来的 1 号随从"
    );
    assert_eq!(
        after.self_.board[1].minion_id, order[0],
        "1 号位应换成原来的 0 号随从"
    );
}

#[test]
fn freeze_shop_succeeds() {
    let (mut task, _clock) = task();
    let slot = first_nonempty_slot(&task, 0);

    let updates = task.handle(command(
        "p0",
        "freeze-1",
        ProtocolIntent::FreezeShop { slot: Some(slot) },
    ));
    assert!(error_codes(&updates).is_empty(), "冻结应成功");

    let view = task.player_view(0).expect("视图应可用");
    let frozen = view
        .self_
        .shop
        .iter()
        .find(|shop_slot| shop_slot.index == slot)
        .expect("被冻结的槽位应仍在商店里");
    assert!(frozen.frozen, "冻结后该槽位应带冻结标记");
    for other in view.self_.shop.iter().filter(|s| s.index != slot) {
        assert!(!other.frozen, "未指定的槽位不应被冻结");
    }
}

#[test]
fn choose_discover_consumes_pending_choice() {
    let (mut task, _clock) = task_with(catalog_with_discover_on_play());
    let slot = first_nonempty_slot(&task, 0);
    task.handle(command("p0", "discover-buy", buy(slot)));

    let before = task.player_view(0).expect("视图应可用");
    assert_eq!(
        before.self_.pending_choices.len(),
        1,
        "购买应留下一条发现待选"
    );

    let updates = task.handle(command(
        "p0",
        "discover-choose",
        ProtocolIntent::ChooseDiscover { choice: 0 },
    ));
    assert!(error_codes(&updates).is_empty(), "选择发现应成功");

    let after = task.player_view(0).expect("视图应可用");
    assert!(after.self_.pending_choices.is_empty(), "选择后待选应被消费");
    assert_eq!(after.self_.hand.len(), 1, "选择的卡应进入手牌");
}

#[test]
fn upgrade_era_succeeds() {
    let (mut task, clock) = task();
    // 空过前三回合进入第 4 回合，金币曲线给到 5，正好够升时代。
    for _ in 0..3 {
        advance_turn(&mut task, &clock);
    }

    let before = task.player_view(0).expect("视图应可用");
    assert_eq!(before.turn.get(), 4, "应进入第 4 回合");
    assert_eq!(before.self_.era.get(), 1, "开局时代应为 1");
    assert_eq!(before.self_.gold, 5, "第 4 回合金币应为 5");

    let updates = task.handle(command("p0", "upgrade-era", ProtocolIntent::UpgradeEra));
    assert!(error_codes(&updates).is_empty(), "金币足够时升时代应成功");

    let after = task.player_view(0).expect("视图应可用");
    assert_eq!(after.self_.era.get(), 2, "升级后时代应增加");
    assert_eq!(after.self_.gold, 0, "升级应扣除 5 金币");
}

// ---------------------------------------------------------------------------
// GameOver 终局路径
// ---------------------------------------------------------------------------

/// 空阵容下逐回合超时推进，直到对局结束；返回最后一批更新。
fn reach_game_over(task: &mut GameTask, clock: &FixedClock) -> Vec<SeatUpdate> {
    for _ in 0..30 {
        clock.advance(90_000);
        let updates = task.tick();
        assert!(!updates.is_empty(), "每个回合到点都应推进");
        if task.player_view(0).expect("视图应可用").phase == protocol::view::Phase::GameOver {
            return updates;
        }
    }
    panic!("30 个回合内应结束对局");
}

#[test]
fn game_over_emits_ranking_and_seed() {
    let (mut task, clock) = task();
    let updates = reach_game_over(&mut task, &clock);

    assert_eq!(
        task.player_view(0).expect("视图应可用").phase,
        protocol::view::Phase::GameOver
    );

    let game_overs: Vec<&protocol::server::GameOver> = updates
        .iter()
        .filter_map(|update| match &update.kind {
            UpdateKind::GameOver(game_over) => Some(game_over),
            _ => None,
        })
        .collect();
    assert_eq!(game_overs.len(), 8, "8 个座位各收到一份 GameOver");

    let expected_seed = format!("{SEED:016x}");
    for game_over in &game_overs {
        assert_eq!(game_over.seed, expected_seed);
    }

    // 每份载荷都包含完整排名：名次 1..=8 互不重复，玩家覆盖 8 个座位。
    for game_over in &game_overs {
        let places: BTreeSet<u64> = game_over
            .ranking
            .iter()
            .map(|item| item.place.get())
            .collect();
        assert_eq!(places, (1..=8_u64).collect::<BTreeSet<_>>());
        let players: BTreeSet<&str> = game_over
            .ranking
            .iter()
            .map(|item| item.player_id.as_str())
            .collect();
        assert_eq!(players.len(), 8);
    }
}

#[test]
fn intent_after_game_over_is_rejected() {
    let (mut task, clock) = task();
    reach_game_over(&mut task, &clock);

    let updates = task.handle(end_recruit("p0", "late-intent"));
    assert_eq!(error_codes(&updates), vec![ErrorCode::InvalidStage]);
    assert_eq!(
        task.player_view(0).expect("视图应可用").phase,
        protocol::view::Phase::GameOver,
        "拒绝的意图不得改变阶段"
    );
}

#[test]
fn tick_after_game_over_returns_empty() {
    let (mut task, clock) = task();
    reach_game_over(&mut task, &clock);

    clock.advance(90_000);
    assert!(task.tick().is_empty(), "结束后不再推进");
}

// ---------------------------------------------------------------------------
// 战斗动作侧别与镜像标记
// ---------------------------------------------------------------------------

fn fixture_minion(instance_id: u64) -> rule_engine::state::MinionInstance {
    rule_engine::state::MinionInstance {
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

/// 双方各两个随从的真实战斗报告；含一条「攻防互换」的反击动作。
fn real_report(seat: Seat) -> rule_engine::settle::SeatCombatReport {
    use rule_engine::combat::CombatAction;
    rule_engine::settle::SeatCombatReport {
        seat,
        opponent_seat: Some(seat + 1),
        mirror: false,
        own_board: vec![fixture_minion(101), fixture_minion(102)],
        opponent_board: vec![fixture_minion(201), fixture_minion(202)],
        actions: vec![
            // 己方 0 号攻击对手 0 号。
            CombatAction::Attack {
                attacker: 101,
                target: 201,
                damage: 2,
                blocked: false,
            },
            // 反击：对手 1 号攻击己方 1 号。
            CombatAction::Attack {
                attacker: 202,
                target: 102,
                damage: 1,
                blocked: false,
            },
            CombatAction::Death { minion: 201 },
            CombatAction::ContagionApplied { target: 102 },
        ],
        damage: 3,
        winner: Some(seat),
    }
}

#[test]
fn combat_action_indices_carry_side() {
    use protocol::CombatResultActionsItemSourceSide as SourceSide;
    use protocol::CombatResultActionsItemTargetSide as TargetSide;

    let (task, _clock) = task();
    let mut reports = BTreeMap::new();
    reports.insert(2, real_report(2));

    let updates = task.combat_updates(&reports);
    assert_eq!(updates.len(), 1);
    let UpdateKind::CombatResult(result) = &updates[0].kind else {
        panic!("应生成 CombatResult");
    };

    let mut saw_own_source = false;
    let mut saw_opponent_source = false;
    for action in &result.actions {
        if let Some(index) = action.source_index {
            let side = action
                .source_side
                .expect("有 sourceIndex 就必须有 sourceSide");
            let board = match side {
                SourceSide::Own => &result.own_board,
                SourceSide::Opponent => &result.opponent_board,
            };
            assert!(
                (index as usize) < board.len(),
                "sourceIndex 必须能按侧别在对应棋盘取到随从"
            );
            match side {
                SourceSide::Own => saw_own_source = true,
                SourceSide::Opponent => saw_opponent_source = true,
            }
        }
        if let Some(index) = action.target_index {
            let side = action
                .target_side
                .expect("有 targetIndex 就必须有 targetSide");
            let board = match side {
                TargetSide::Own => &result.own_board,
                TargetSide::Opponent => &result.opponent_board,
            };
            assert!(
                (index as usize) < board.len(),
                "targetIndex 必须能按侧别在对应棋盘取到随从"
            );
        }
    }
    assert!(saw_own_source, "应至少有一条己方来源的动作");
    assert!(saw_opponent_source, "反击动作的来源必须标为对手棋盘");

    // 具体侧别与下标：己方 0 号 -> 对手 0 号；对手 1 号 -> 己方 1 号。
    assert_eq!(result.actions[0].source_side, Some(SourceSide::Own));
    assert_eq!(result.actions[0].source_index, Some(0));
    assert_eq!(result.actions[0].target_side, Some(TargetSide::Opponent));
    assert_eq!(result.actions[0].target_index, Some(0));
    assert_eq!(result.actions[1].source_side, Some(SourceSide::Opponent));
    assert_eq!(result.actions[1].source_index, Some(1));
    assert_eq!(result.actions[1].target_side, Some(TargetSide::Own));
    assert_eq!(result.actions[1].target_index, Some(1));
}

#[test]
fn mirror_combat_result_is_marked() {
    use rule_engine::combat::CombatAction;

    let (task, _clock) = task();
    let mirror = rule_engine::settle::SeatCombatReport {
        seat: 1,
        opponent_seat: None,
        mirror: true,
        own_board: vec![fixture_minion(101)],
        opponent_board: vec![fixture_minion(101)],
        actions: vec![CombatAction::Attack {
            attacker: 101,
            target: 101,
            damage: 1,
            blocked: false,
        }],
        damage: 0,
        winner: Some(1),
    };
    let mut reports = BTreeMap::new();
    reports.insert(1, mirror);

    let updates = task.combat_updates(&reports);
    assert_eq!(updates.len(), 1, "镜像战只产生 1 条 CombatResult");
    assert_eq!(updates[0].player_id, "p1");
    let UpdateKind::CombatResult(result) = &updates[0].kind else {
        panic!("应生成 CombatResult");
    };
    assert_eq!(result.mirror, Some(true));
}

#[test]
fn real_combat_result_is_not_marked() {
    let (task, _clock) = task();
    let mut reports = BTreeMap::new();
    reports.insert(2, real_report(2));

    let updates = task.combat_updates(&reports);
    assert_eq!(updates.len(), 1);
    let UpdateKind::CombatResult(result) = &updates[0].kind else {
        panic!("应生成 CombatResult");
    };
    assert_eq!(result.mirror, Some(false));
}
