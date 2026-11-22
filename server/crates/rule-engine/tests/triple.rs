mod common;

use common::{minion, minion_id, set_shop, test_catalog, turn_started};
use rule_engine::data::{CardType, Era, card_in_pool};
use rule_engine::effects::{TargetRef, Zone};
use rule_engine::events::GameEvent;
use rule_engine::hash::state_hash;
use rule_engine::intent::{Intent, RejectCode};
use rule_engine::state::{GameState, HandCard, InstanceId, MinionInstance, PendingChoice, Seat};

use rule_engine::triple::REWARD_CARD_ID;

fn hand_card(instance_id: InstanceId, card_id: &str) -> HandCard {
    HandCard::Minion(minion(&test_catalog(), instance_id, card_id))
}

fn pool_count(state: &GameState, card_id: &str) -> u16 {
    state.pool.get(card_id).copied().unwrap_or(0)
}

fn effect_card_id() -> String {
    test_catalog()
        .cards
        .values()
        .find(|card| card.card_type == CardType::Effect && card.id != REWARD_CARD_ID)
        .expect("数据中应存在效果牌")
        .id
        .clone()
}

fn triple_event(card_id: &str, golden: InstanceId, sources: &[InstanceId]) -> GameEvent {
    GameEvent::TripleFormed {
        seat: 0,
        card_id: card_id.to_string(),
        golden_instance_id: golden,
        sources: sources.to_vec(),
    }
}

fn hand_minions(state: &GameState, seat: Seat) -> Vec<&MinionInstance> {
    state.players[usize::from(seat)]
        .hand
        .iter()
        .filter_map(HandCard::minion)
        .collect()
}

#[test]
fn three_same_minions_become_golden_in_hand() {
    let (engine, mut state) = turn_started(1);
    let card_id = minion_id(1);
    state.players[0].gold = 30;

    // 棋盘一张带成长、繁衍与护盾，手牌一张同名；买入的第三张进手牌后立即做三连。
    let mut base = minion(&engine.catalog, 10, &card_id);
    base.stats.attack.add_growth(7);
    base.stats.health.add_growth(7);
    base.breed = 3;
    base.shield_used = true;
    state.players[0].board.push(base);
    state.players[0].hand.push(hand_card(11, &card_id));
    let shop_instance = state.next_instance_id;

    set_shop(&mut state, 0, &[Some(card_id.clone())]);
    let outcome = engine
        .apply(&mut state, 0, Intent::BuyMinion { slot: 0 })
        .expect("第三张购买应成功");

    let golden_id = state.next_instance_id - 1;
    assert_eq!(
        outcome.events,
        vec![
            GameEvent::MinionBought {
                seat: 0,
                slot: 0,
                card_id: card_id.clone(),
            },
            triple_event(&card_id, golden_id, &[10, 11, shop_instance]),
        ],
        "买入后立即三连，事件带金色编号与三张来源"
    );
    assert!(state.players[0].board.is_empty(), "棋盘副本全部离场");
    assert_eq!(state.players[0].hand.len(), 1, "金色进入手牌");
    let golden = hand_minions(&state, 0)[0];
    assert!(golden.golden);
    assert_eq!(golden.card_id, card_id);
    assert_eq!(golden.instance_id, golden_id, "金色是新实例");
    assert_eq!(golden.stats.attack.growth, 7, "保留最早副本的成长");
    assert_eq!(golden.stats.health.growth, 7);
    let base_card = test_catalog()
        .cards
        .get(&card_id)
        .cloned()
        .expect("卡牌应存在");
    assert_eq!(
        golden.stats.attack.initial,
        base_card.attack.unwrap_or(0).saturating_mul(2),
        "金色基础攻击按卡面 ×2"
    );
    assert_eq!(
        golden.stats.health.initial,
        base_card.health.unwrap_or(0).saturating_mul(2),
        "金色基础生命按卡面 ×2"
    );
    assert_eq!(golden.breed, 3, "保留最早副本的繁衍");
    assert!(golden.shield_used, "保留最早副本的护盾状态");
    assert!(state.pending.is_empty(), "三连不再立即产生发现待选");
}

#[test]
fn base_is_the_earliest_instance_by_id() {
    let (engine, mut state) = turn_started(2);
    let card_id = minion_id(2);
    let mut early_board = minion(&engine.catalog, 40, &card_id);
    early_board.breed = 5;
    early_board.stats.attack.add_growth(3);
    // 手牌副本编号更小：三张都在时手牌副本才是基底，棋盘副本一并离场。
    state.players[0].board = vec![early_board, minion(&engine.catalog, 50, "baiyi")];
    state.players[0].hand = vec![
        hand_card(20, &card_id),
        HandCard::Minion({
            let mut late = minion(&engine.catalog, 30, &card_id);
            late.contagion = 1;
            late
        }),
    ];

    let events = engine.check_triple(&mut state, 0);

    let golden_id = state.next_instance_id - 1;
    assert_eq!(
        events,
        vec![triple_event(&card_id, golden_id, &[20, 30, 40])],
        "取编号最小的三张，基底为手牌里的 20"
    );
    let golden = hand_minions(&state, 0)[0];
    assert_eq!(golden.instance_id, golden_id);
    assert_eq!(
        golden.breed, 0,
        "基底是卡面无状态的手牌副本，不继承其它副本的繁衍"
    );
    assert_eq!(golden.stats.attack.growth, 0);
    assert_eq!(golden.contagion, 0);
    assert_eq!(state.players[0].board.len(), 1, "只剩无关的棋盘随从");
}

#[test]
fn golden_sells_for_three_and_cannot_triple_again() {
    let (engine, mut state) = turn_started(3);
    let card_id = minion_id(2);
    state.players[0].gold = 0;
    state.players[0].board = vec![
        minion(&engine.catalog, 10, &card_id),
        minion(&engine.catalog, 11, &card_id),
        minion(&engine.catalog, 12, &card_id),
    ];

    let events = engine.check_triple(&mut state, 0);
    let golden_id = state.next_instance_id - 1;
    assert_eq!(
        events,
        vec![triple_event(&card_id, golden_id, &[10, 11, 12])]
    );
    assert!(state.players[0].board.is_empty(), "三张棋盘副本都离场");

    // 打出金色后再攒两张同名：金色不参与，不会合成。
    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect("打出金色应成功");
    assert!(state.players[0].board[0].golden);
    state.players[0]
        .board
        .push(minion(&engine.catalog, 20, &card_id));
    state.players[0]
        .board
        .push(minion(&engine.catalog, 21, &card_id));
    let before = state_hash(&state);
    let events = engine.check_triple(&mut state, 0);
    assert!(events.is_empty(), "金色随从不再参与三连");
    assert_eq!(state.players[0].board.len(), 3, "金色 + 两张非金色不合成");
    assert_eq!(before, state_hash(&state));

    engine
        .apply(&mut state, 0, Intent::SellMinion { minion: golden_id })
        .expect("出售金色随从应成功");
    assert_eq!(state.players[0].gold, 3, "金色随从出售 +3");
    assert_eq!(state.players[0].board.len(), 2);
}

#[test]
fn triple_only_looks_at_board_and_hand() {
    let (engine, mut state) = turn_started(4);
    let card_id = minion_id(2);
    state.players[0].board = vec![
        minion(&engine.catalog, 10, &card_id),
        minion(&engine.catalog, 11, &card_id),
    ];
    assert!(pool_count(&state, &card_id) > 0, "共享池仍有同名卡");

    let events = engine.check_triple(&mut state, 0);

    assert!(events.is_empty(), "池中的同名卡不参与三连");
    assert_eq!(state.players[0].board.len(), 2);
    assert!(state.pending.is_empty());
}

#[test]
fn discovered_third_copy_forms_triple_in_hand() {
    let (engine, mut state) = turn_started(5);
    let card_id = minion_id(1);
    state.players[0].hand = vec![hand_card(40, &card_id), hand_card(41, &card_id)];
    state.pending.push_back(PendingChoice::Discover {
        seat: 0,
        candidates: vec![card_id.clone()],
        source: 1,
    });

    let outcome = engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect("发现应成功");

    let discovered_id = state.next_instance_id - 2;
    let golden_id = state.next_instance_id - 1;
    assert_eq!(
        outcome.events,
        vec![
            GameEvent::CardDiscovered {
                seat: 0,
                card_id: card_id.clone(),
            },
            triple_event(&card_id, golden_id, &[40, 41, discovered_id]),
        ],
        "发现的第三张也能触发三连；选中的随从与金色各占一个编号"
    );
    assert!(state.players[0].board.is_empty());
    assert_eq!(state.players[0].hand.len(), 1);
    let golden = hand_minions(&state, 0)[0];
    assert!(golden.golden);
    assert_eq!(golden.card_id, card_id);
    assert!(state.pending.is_empty(), "新三连没有再产生发现待选");
}

#[test]
fn hand_triple_stays_in_hand() {
    let (engine, mut state) = turn_started(6);
    let card_id = minion_id(2);
    state.players[0].hand = vec![
        hand_card(100, &card_id),
        hand_card(101, &card_id),
        hand_card(102, &card_id),
    ];

    let events = engine.check_triple(&mut state, 0);

    let golden_id = state.next_instance_id - 1;
    assert_eq!(
        events,
        vec![triple_event(&card_id, golden_id, &[100, 101, 102])]
    );
    assert!(
        state.players[0].board.is_empty(),
        "三张都在手牌时不新增棋盘"
    );
    assert_eq!(state.players[0].hand.len(), 1);
    let golden = hand_minions(&state, 0)[0];
    assert!(golden.golden);
    assert_eq!(golden.instance_id, golden_id);
    assert_eq!(
        golden.stats.attack.growth, 0,
        "三张都是卡面副本，金色没有额外成长层"
    );
}

#[test]
fn all_board_triple_with_full_hand_keeps_golden_on_board() {
    let (engine, mut state) = turn_started(7);
    let catalog = test_catalog();
    let hand_slots = catalog.config.hand_slots;
    let card_id = minion_id(2);
    // 手牌满且与目标卡同名；三张副本全在棋盘（编号 300 最早），棋盘第 1 位是别家随从。
    let filler: Vec<String> = catalog
        .cards
        .values()
        .filter(|card| card.card_type == CardType::Minion && card.id != card_id)
        .take(hand_slots)
        .map(|card| card.id.clone())
        .collect();
    state.players[0].hand = filler
        .iter()
        .enumerate()
        .map(|(index, filler_id)| hand_card(2_000 + index as InstanceId, filler_id))
        .collect();
    state.players[0].board = vec![
        minion(&catalog, 100, "baiyi"),
        minion(&catalog, 300, &card_id),
        minion(&catalog, 301, &card_id),
        minion(&catalog, 302, &card_id),
    ];

    let events = engine.check_triple(&mut state, 0);

    let golden_id = state.next_instance_id - 1;
    assert_eq!(
        events,
        vec![triple_event(&card_id, golden_id, &[300, 301, 302])]
    );
    assert_eq!(
        state.players[0].hand.len(),
        hand_slots,
        "手牌已满时金色不入入手牌"
    );
    let golden = state.players[0]
        .board
        .iter()
        .find(|minion| minion.instance_id == golden_id)
        .expect("金色留在棋盘");
    assert!(golden.golden);
    assert_eq!(
        state.players[0].board[1].instance_id, golden_id,
        "金色留在基底原来的站位"
    );
    assert_eq!(
        state.players[0].board.len(),
        2,
        "三张合成一张，剩金色与无关随从"
    );
}

#[test]
fn hand_triple_with_full_hand_still_goes_to_hand() {
    let (engine, mut state) = turn_started(20);
    let catalog = test_catalog();
    let hand_slots = catalog.config.hand_slots;
    let card_id = minion_id(2);
    let filler: Vec<String> = catalog
        .cards
        .values()
        .filter(|card| card.card_type == CardType::Minion && card.id != card_id)
        .take(hand_slots - 2)
        .map(|card| card.id.clone())
        .collect();
    state.players[0].hand = filler
        .iter()
        .enumerate()
        .map(|(index, filler_id)| hand_card(2_000 + index as InstanceId, filler_id))
        .chain([hand_card(2_100, &card_id), hand_card(2_101, &card_id)])
        .collect();
    assert_eq!(state.players[0].hand.len(), hand_slots, "手牌满");
    state.players[0]
        .board
        .push(minion(&catalog, 2_102, &card_id));
    state.players[0].board.push(minion(&catalog, 100, "baiyi"));

    let events = engine.check_triple(&mut state, 0);

    let golden_id = state.next_instance_id - 1;
    assert_eq!(
        events,
        vec![triple_event(&card_id, golden_id, &[2_100, 2_101, 2_102])]
    );
    assert_eq!(
        state.players[0].hand.len(),
        hand_slots - 1,
        "两张手牌副本离场，金色补回一张"
    );
    let golden = hand_minions(&state, 0)
        .into_iter()
        .find(|minion| minion.golden)
        .expect("金色进入手牌");
    assert_eq!(golden.instance_id, golden_id);
    assert_eq!(state.players[0].board.len(), 1, "只剩无关的棋盘随从");
}

#[test]
fn reward_is_granted_only_when_golden_is_played() {
    let (engine, mut state) = turn_started(8);
    let card_id = minion_id(1);
    state.players[0].board = vec![
        minion(&engine.catalog, 10, &card_id),
        minion(&engine.catalog, 11, &card_id),
        minion(&engine.catalog, 12, &card_id),
    ];
    engine.check_triple(&mut state, 0);
    let golden = hand_minions(&state, 0)[0].clone();
    assert!(
        state.players[0]
            .hand
            .iter()
            .all(|card| card.reward_level().is_none()),
        "合成本身不发放奖励牌"
    );

    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect("打出金色应成功");

    let rewards: Vec<&HandCard> = state.players[0]
        .hand
        .iter()
        .filter(|card| card.reward_level().is_some())
        .collect();
    assert_eq!(rewards.len(), 1, "打出金色后手牌多出一张奖励牌");
    assert_eq!(rewards[0].card_id(), REWARD_CARD_ID);
    assert_eq!(
        rewards[0].reward_level(),
        Some(2),
        "获得奖励牌时时代 1 +1 为 2 星"
    );
    let board_card = test_catalog().cards.get(&card_id).cloned().unwrap();
    assert_eq!(board_card.level, 1, "夹具使用 1 星随从");
    assert_eq!(state.players[0].board[0].instance_id, golden.instance_id);
}

#[test]
fn reward_level_caps_at_six() {
    let (engine, mut state) = turn_started(9);
    // 时代 5 +1 应为 6 星；7 星时代也封顶 6。
    state.players[0].era = Era::Industrial;
    let card_id = minion_id(7);
    state.players[0].board = vec![
        minion(&engine.catalog, 10, &card_id),
        minion(&engine.catalog, 11, &card_id),
        minion(&engine.catalog, 12, &card_id),
    ];
    engine.check_triple(&mut state, 0);
    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect("打出金色应成功");

    assert!(state.players[0].hand.is_empty(), "战吼待选期间不发放奖励牌");

    let source = state.players[0].board[0].instance_id;
    engine
        .apply(
            &mut state,
            0,
            Intent::ChooseTarget {
                target: TargetRef {
                    zone: Zone::Board,
                    id: source,
                },
            },
        )
        .expect("选定目标应成功");

    let reward = state.players[0]
        .hand
        .iter()
        .find(|card| card.reward_level().is_some())
        .expect("应发放奖励牌");
    assert_eq!(
        reward.reward_level(),
        Some(6),
        "时代 5 的奖励牌为 6 星且封顶"
    );
}

#[test]
fn reward_level_is_locked_when_granted() {
    let (engine, mut state) = turn_started(13);
    state.players[0].era = Era::Primitive;
    let card_id = minion_id(1);
    state.players[0].board = vec![
        minion(&engine.catalog, 10, &card_id),
        minion(&engine.catalog, 11, &card_id),
        minion(&engine.catalog, 12, &card_id),
    ];

    engine.check_triple(&mut state, 0);
    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect("打出金色应成功");

    let reward = state.players[0]
        .hand
        .iter()
        .find(|card| card.reward_level().is_some())
        .expect("应发放奖励牌");
    assert_eq!(reward.reward_level(), Some(3), "时代 2 的奖励牌为 3 星");

    // 获得奖励牌后再升时代：已锁定的星级不变，出牌仍按 3 星发放。
    state.players[0].era = Era::Future;
    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect("打出奖励牌应成功");

    let granted_id = state.players[0].hand[0].card_id();
    let card = test_catalog()
        .cards
        .get(granted_id)
        .cloned()
        .expect("获得的随从应在目录中");
    assert_eq!(card.level, 3, "奖励星级在获得时锁定，升时代不追溯");
}

#[test]
fn playing_reward_grants_next_level_minion() {
    let (engine, mut state) = turn_started(10);
    let reward_id = state.next_instance_id;
    state.next_instance_id += 1;
    state.players[0].hand.push(HandCard::Effect {
        instance_id: reward_id,
        card_id: REWARD_CARD_ID.to_string(),
        reward_level: Some(3),
    });
    let pool_before = state.pool.clone();

    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect("打出奖励牌应成功");

    assert!(state.pending.is_empty(), "奖励牌不再登记发现待选");
    assert_eq!(state.players[0].hand.len(), 1, "奖励换成一张手牌随从");
    let granted_id = state.players[0].hand[0].card_id();
    let catalog = test_catalog();
    let card = catalog.cards.get(granted_id).expect("获得的随从应在目录中");
    assert_eq!(card.level, 3, "奖励牌锁定星级即发放星级");
    assert_eq!(card.card_type, CardType::Minion);
    assert!(card.in_shop_pool, "奖励只会发出池内随从");
    assert!(
        card_in_pool(card, &state.enabled_playstyles),
        "候选玩法需本局启用"
    );
    assert_eq!(state.pool, pool_before, "直接发牌不扣共享池");
}

#[test]
fn reward_card_without_candidates_is_consumed() {
    let (engine, mut state) = turn_started(11);
    state.enabled_playstyles.clear();
    state.players[0].hand.push(HandCard::Effect {
        instance_id: 9_000,
        card_id: REWARD_CARD_ID.to_string(),
        reward_level: Some(6),
    });

    let outcome = engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect("打出奖励牌应成功");

    assert!(outcome.events.is_empty());
    assert!(state.players[0].hand.is_empty(), "奖励牌照常消耗");
    assert!(state.pending.is_empty(), "无候选时不登记待选");
}

#[test]
fn reward_card_never_appears_in_shop_or_discover() {
    let (engine, mut state) = turn_started(12);
    let catalog = test_catalog();
    let reward = catalog
        .cards
        .get(REWARD_CARD_ID)
        .expect("奖励牌应在卡牌目录中");
    assert_eq!(
        reward.card_type,
        CardType::Effect,
        "奖励牌是效果牌，不可能是随从候选"
    );

    // 反复重抽商店：任何槽位都不能出现奖励牌。
    for _ in 0..40 {
        let events = engine.roll_shop(&mut state, 0);
        assert!(!events.is_empty());
        for slot in state.players[0].shop.iter().flatten() {
            assert_ne!(slot.card_id, REWARD_CARD_ID);
        }
    }

    // 奖励牌在任意锁定星级下抽到的候选都不含奖励牌。
    for level in 1..=6u8 {
        let mut state = state.clone();
        state.players[0].hand.clear();
        state.pending.clear();
        state.players[0].hand.push(HandCard::Effect {
            instance_id: 9_000 + u64::from(level),
            card_id: REWARD_CARD_ID.to_string(),
            reward_level: Some(level),
        });
        engine
            .apply(
                &mut state,
                0,
                Intent::PlayCard {
                    hand_index: 0,
                    position: None,
                    target: None,
                },
            )
            .expect("打出奖励牌应成功");
        if let Some(PendingChoice::Discover { candidates, .. }) = state.pending.front() {
            assert!(candidates.iter().all(|id| id != REWARD_CARD_ID));
        }
    }
}

#[test]
fn hand_effect_cards_never_form_triple() {
    let (engine, mut state) = turn_started(13);
    let effect_id = effect_card_id();
    let card_id = minion_id(2);
    state.players[0].hand = vec![
        HandCard::effect(10, &effect_id),
        HandCard::effect(11, &effect_id),
        HandCard::effect(12, &effect_id),
    ];
    state.players[0].board = vec![minion(&engine.catalog, 13, &card_id)];

    let events = engine.check_triple(&mut state, 0);

    assert!(events.is_empty(), "效果牌不参与三连");
    assert!(state.pending.is_empty(), "效果牌不产生发现待选");
    assert_eq!(state.players[0].board.len(), 1, "棋盘不增加随从");
    assert_eq!(state.players[0].board[0].card_id, card_id);
    assert_eq!(state.players[0].hand.len(), 3, "三张效果牌都保留");
    assert!(
        state.players[0]
            .hand
            .iter()
            .all(|card| card.card_id() == effect_id && !card.golden())
    );
}

#[test]
fn one_check_forms_multiple_triples() {
    let (engine, mut state) = turn_started(14);
    let card_id = minion_id(2);
    state.players[0].board = vec![
        minion(&engine.catalog, 10, &card_id),
        minion(&engine.catalog, 11, &card_id),
        minion(&engine.catalog, 12, &card_id),
    ];
    state.players[0].hand = vec![
        hand_card(13, &card_id),
        hand_card(14, &card_id),
        hand_card(15, &card_id),
    ];

    let events = engine.check_triple(&mut state, 0);

    let first_id = state.next_instance_id - 2;
    let second_id = state.next_instance_id - 1;
    assert_eq!(
        events,
        vec![
            triple_event(&card_id, first_id, &[10, 11, 12]),
            triple_event(&card_id, second_id, &[13, 14, 15]),
        ],
        "六张同名副本一次调用合成两次"
    );
    assert_eq!(state.players[0].hand.len(), 2, "两张金色都在手牌");
    assert!(state.players[0].board.is_empty());
    assert!(
        state.players[0]
            .hand
            .iter()
            .map(|card| card.minion().expect("金色是随从牌"))
            .all(|minion| minion.golden && minion.card_id == card_id)
    );
}

#[test]
fn choose_discover_out_of_range_rejects() {
    let (engine, mut state) = turn_started(15);
    state.pending.push_back(PendingChoice::Discover {
        seat: 0,
        candidates: vec!["a".to_string(), "b".to_string()],
        source: 1,
    });
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::ChooseDiscover { choice: 5 });

    assert_eq!(result.unwrap_err().code, RejectCode::InvalidChoice);
    assert_eq!(before, state_hash(&state), "拒绝不改变状态");
}

#[test]
fn choose_discover_consumes_pending_and_adds_to_hand() {
    let (engine, mut state) = turn_started(16);
    let chosen = minion_id(2);
    let other = minion_id(3);
    let before_pool = pool_count(&state, &chosen);
    state.pending.push_back(PendingChoice::Discover {
        seat: 0,
        candidates: vec![chosen.clone(), other],
        source: 1,
    });

    let outcome = engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect("合法的发现选择应被消费");

    assert_eq!(
        outcome.events,
        vec![GameEvent::CardDiscovered {
            seat: 0,
            card_id: chosen.clone(),
        }]
    );
    assert!(state.pending.is_empty(), "消费后待选被移除");
    assert_eq!(state.players[0].hand.len(), 1);
    assert_eq!(state.players[0].hand[0].card_id(), chosen);
    assert!(!state.players[0].hand[0].golden());
    assert_eq!(
        pool_count(&state, &chosen),
        before_pool - 1,
        "发现的卡扣 1 份池"
    );
}

#[test]
fn choose_discover_with_full_hand_rejects_without_mutating() {
    let (engine, mut state) = turn_started(17);
    let chosen = minion_id(2);
    state.players[0].hand = (0..engine.catalog.config.hand_slots)
        .map(|index| hand_card(2_000 + index as InstanceId, &minion_id(1)))
        .collect();
    state.pending.push_back(PendingChoice::Discover {
        seat: 0,
        candidates: vec![chosen],
        source: 1,
    });
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 });

    assert_eq!(result.unwrap_err().code, RejectCode::NoSpace);
    assert_eq!(before, state_hash(&state));
    assert_eq!(state.pending.len(), 1, "手牌满时待选保留");
}

#[test]
fn multiple_pending_choices_consume_frontmost_for_seat() {
    let (engine, mut state) = turn_started(18);
    let first = minion_id(2);
    let second = minion_id(3);
    let third = minion_id(4);
    state.pending.push_back(PendingChoice::Discover {
        seat: 0,
        candidates: vec![first.clone()],
        source: 1,
    });
    state.pending.push_back(PendingChoice::Discover {
        seat: 1,
        candidates: vec![second.clone()],
        source: 2,
    });
    state.pending.push_back(PendingChoice::Discover {
        seat: 0,
        candidates: vec![third.clone()],
        source: 3,
    });

    engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect("座位 0 的第一次发现应成功");
    assert_eq!(state.players[0].hand[0].card_id(), first);
    assert_eq!(state.pending.len(), 2, "只消费队列中最靠前的同座位待选");
    assert!(matches!(
        state.pending.front(),
        Some(PendingChoice::Discover { seat: 1, .. })
    ));

    engine
        .apply(&mut state, 1, Intent::ChooseDiscover { choice: 0 })
        .expect("座位 1 的发现应成功");
    assert_eq!(state.players[1].hand[0].card_id(), second);
    assert_eq!(state.pending.len(), 1);
    assert!(matches!(
        state.pending.front(),
        Some(PendingChoice::Discover { seat: 0, candidates, .. }) if candidates == &vec![third]
    ));
}

#[test]
fn golden_played_from_hand_keeps_instance_and_state() {
    let (engine, mut state) = turn_started(19);
    let card_id = minion_id(2);
    let mut base = minion(&engine.catalog, 10, &card_id);
    base.stats.health.add_growth(4);
    base.contagion = 1;
    state.players[0].board = vec![
        base,
        minion(&engine.catalog, 11, &card_id),
        minion(&engine.catalog, 12, &card_id),
    ];

    engine.check_triple(&mut state, 0);
    let golden_id = state.next_instance_id - 1;
    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect("打出金色应成功");

    let played = &state.players[0].board[0];
    assert_eq!(played.instance_id, golden_id, "打出沿用金色实例编号");
    assert_eq!(played.stats.health.growth, 4, "成长随金色入场");
    assert_eq!(played.contagion, 1, "传染随金色入场");
    assert!(played.golden);
}

#[test]
fn copy_hand_minion_forming_third_copy_triples_immediately() {
    let (engine, mut state) = turn_started(7);
    let card_id = minion_id(1);
    state.players[0]
        .board
        .push(minion(&engine.catalog, 10, &card_id));
    state.players[0].hand.push(hand_card(11, &card_id));
    let effect_id = state.next_instance_id;
    state.next_instance_id = effect_id.saturating_add(1);
    state.players[0]
        .hand
        .push(HandCard::effect(effect_id, "yinshuashu"));

    let outcome = engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 1,
                position: None,
                target: Some(TargetRef {
                    zone: Zone::Hand,
                    id: 11,
                }),
            },
        )
        .expect("印刷术应能打出");

    assert!(
        outcome.events.iter().any(|event| matches!(
            event,
            GameEvent::TripleFormed { card_id: id, .. } if id == &card_id
        )),
        "复制形成第三张同名随从后应立即三连"
    );
    assert!(state.players[0].board.is_empty(), "三张副本离场");
    let golden = hand_minions(&state, 0)
        .into_iter()
        .find(|minion| minion.golden)
        .expect("金色进入手牌");
    assert_eq!(golden.card_id, card_id);
}

#[test]
fn copy_board_minion_forming_third_copy_triples_immediately() {
    let (engine, mut state) = turn_started(8);
    let card_id = minion_id(1);
    state.players[0]
        .board
        .push(minion(&engine.catalog, 10, &card_id));
    state.players[0].hand.push(hand_card(11, &card_id));
    let effect_id = state.next_instance_id;
    state.next_instance_id = effect_id.saturating_add(1);
    state.players[0]
        .hand
        .push(HandCard::effect(effect_id, "fuzhi"));

    let outcome = engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 1,
                position: None,
                target: Some(TargetRef {
                    zone: Zone::Board,
                    id: 10,
                }),
            },
        )
        .expect("复制应能打出");

    assert!(
        outcome.events.iter().any(|event| matches!(
            event,
            GameEvent::TripleFormed { card_id: id, .. } if id == &card_id
        )),
        "复制棋盘随从形成第三张后应立即三连"
    );
    assert!(state.players[0].board.is_empty(), "三张副本离场");
    let golden = hand_minions(&state, 0)
        .into_iter()
        .find(|minion| minion.golden)
        .expect("金色进入手牌");
    assert_eq!(golden.card_id, card_id);
}
