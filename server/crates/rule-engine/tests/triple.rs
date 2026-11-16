mod common;

use std::collections::BTreeSet;

use common::{heroes, test_catalog};
use rule_engine::data::{CardType, Catalog, Tribe};
use rule_engine::engine::Engine;
use rule_engine::events::GameEvent;
use rule_engine::hash::state_hash;
use rule_engine::intent::{Intent, RejectCode};
use rule_engine::state::{
    GameState, HandCard, InstanceId, MinionInstance, PendingChoice, Seat, ShopSlot,
};

fn setup(seed: u8) -> (Engine, GameState) {
    let (mut engine, mut state) = Engine::new(test_catalog(), [seed; 32]);
    engine
        .start_game(&mut state, &heroes())
        .expect("开局应成功");
    engine.start_turn(&mut state).expect("开始回合应成功");
    (engine, state)
}

fn minion_id(level: u8) -> String {
    test_catalog()
        .cards
        .values()
        .find(|card| card.card_type == CardType::Minion && !card.is_hero && card.level == level)
        .expect("数据中应存在该星级随从")
        .id
        .clone()
}

fn set_shop(state: &mut GameState, seat: Seat, ids: &[Option<String>]) {
    let len = state.players[usize::from(seat)].shop.len().max(ids.len());
    let mut shop: Vec<Option<ShopSlot>> = ids
        .iter()
        .cloned()
        .map(|card_id| {
            card_id.map(|card_id| ShopSlot {
                card_id,
                frozen: false,
            })
        })
        .collect();
    while shop.len() < len {
        shop.push(None);
    }
    state.players[usize::from(seat)].shop = shop;
}

fn dummy(catalog: &Catalog, instance_id: InstanceId, card_id: &str) -> MinionInstance {
    MinionInstance::new(catalog, instance_id, card_id, false)
}

fn hand_card(card_id: &str) -> HandCard {
    HandCard {
        card_id: card_id.to_string(),
        golden: false,
    }
}

fn pool_count(state: &GameState, card_id: &str) -> u16 {
    state.pool.get(card_id).copied().unwrap_or(0)
}

/// 用互不相同、且排除指定卡牌的随从填满棋盘，避免填充本身触发三连。
fn fill_board_distinct(state: &mut GameState, seat: Seat, count: usize, exclude: &str) {
    let ids: Vec<String> = test_catalog()
        .cards
        .values()
        .filter(|card| card.card_type == CardType::Minion && !card.is_hero && card.id != exclude)
        .map(|card| card.id.clone())
        .take(count)
        .collect();
    assert_eq!(ids.len(), count, "数据中应有足够的随从填满棋盘");
    state.players[usize::from(seat)].board = ids
        .iter()
        .enumerate()
        .map(|(index, card_id)| dummy(&test_catalog(), 500 + index as InstanceId, card_id))
        .collect();
}

fn effect_card_id() -> String {
    test_catalog()
        .cards
        .values()
        .find(|card| card.card_type == CardType::Effect)
        .expect("数据中应存在效果牌")
        .id
        .clone()
}

#[test]
fn three_same_minions_become_golden() {
    let (mut engine, mut state) = setup(1);
    let card_id = minion_id(1);
    state.players[0].gold = 30;

    // 棋盘一张带成长与增殖，手牌一张同名；买入的第二张同样进手牌后立即做三连。
    let mut base = dummy(&engine.catalog, 900, &card_id);
    base.stats.attack.add_growth(7);
    base.stats.health.add_growth(7);
    base.breed = 3;
    base.shield_used = true;
    state.players[0].board.push(base);
    state.players[0].hand.push(hand_card(&card_id));
    let base_id = state.players[0].board[0].instance_id;

    set_shop(&mut state, 0, &[Some(card_id.clone())]);
    let outcome = engine
        .apply(&mut state, 0, Intent::BuyMinion { slot: 0 })
        .expect("第三张购买应成功");

    assert!(
        outcome.events.contains(&GameEvent::TripleFormed {
            seat: 0,
            card_id: card_id.clone(),
        }),
        "第三张同名随从应触发三连"
    );
    assert_eq!(state.players[0].board.len(), 1, "金色随从占 1 格");
    assert!(state.players[0].hand.is_empty());
    let golden = &state.players[0].board[0];
    assert!(golden.golden);
    assert_eq!(golden.card_id, card_id);
    assert_eq!(golden.instance_id, base_id, "最早副本的实例编号保留");
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
    assert_eq!(golden.breed, 3, "保留最早副本的增殖");
    assert!(golden.shield_used, "保留最早副本的护盾状态");
    assert_eq!(golden.attacks_left, 0);
    assert!(!golden.cannot_attack);
    // MinionInstance 不存基础攻防：金色标志由展示/战斗层按卡面 ×2 计算。
    let card = test_catalog()
        .cards
        .get(&card_id)
        .cloned()
        .expect("卡牌应存在");
    assert!(
        card.attack.is_some() && card.health.is_some(),
        "随从应有卡面攻防"
    );
    assert_eq!(state.pending.len(), 1, "三连应产生一个发现待选");
}

#[test]
fn golden_sells_for_three_and_cannot_triple_again() {
    let (mut engine, mut state) = setup(2);
    let card_id = minion_id(2);
    state.players[0].gold = 0;
    state.players[0].board = vec![
        dummy(&engine.catalog, 10, &card_id),
        dummy(&engine.catalog, 11, &card_id),
        dummy(&engine.catalog, 12, &card_id),
    ];

    let events = engine.check_triple(&mut state, 0);
    assert_eq!(
        events,
        vec![GameEvent::TripleFormed {
            seat: 0,
            card_id: card_id.clone(),
        }]
    );
    assert_eq!(state.players[0].board.len(), 1);
    assert!(state.players[0].board[0].golden);

    state.players[0]
        .board
        .push(dummy(&engine.catalog, 20, &card_id));
    state.players[0]
        .board
        .push(dummy(&engine.catalog, 21, &card_id));
    let before = state_hash(&state);
    let events = engine.check_triple(&mut state, 0);
    assert!(events.is_empty(), "金色随从不再参与三连");
    assert_eq!(state.players[0].board.len(), 3, "金色 + 两张非金色不合成");
    assert_eq!(before, state_hash(&state));

    engine
        .apply(&mut state, 0, Intent::SellMinion { minion: 10 })
        .expect("出售金色随从应成功");
    assert_eq!(state.players[0].gold, 3, "金色随从出售 +3");
    assert_eq!(state.players[0].board.len(), 2);
}

#[test]
fn triple_only_looks_at_board_and_hand() {
    let (engine, mut state) = setup(3);
    let card_id = minion_id(2);
    state.players[0].board = vec![
        dummy(&engine.catalog, 10, &card_id),
        dummy(&engine.catalog, 11, &card_id),
    ];
    assert!(pool_count(&state, &card_id) > 0, "共享池仍有同名卡");

    let events = engine.check_triple(&mut state, 0);

    assert!(events.is_empty(), "池中的同名卡不参与三连");
    assert_eq!(state.players[0].board.len(), 2);
    assert!(state.pending.is_empty());
}

#[test]
fn discover_offers_three_candidates_of_next_level() {
    let (engine, mut state) = setup(4);
    let card_id = minion_id(1);
    state.players[0].board = vec![
        dummy(&engine.catalog, 10, &card_id),
        dummy(&engine.catalog, 11, &card_id),
        dummy(&engine.catalog, 12, &card_id),
    ];

    let events = engine.check_triple(&mut state, 0);
    assert_eq!(
        events,
        vec![GameEvent::TripleFormed {
            seat: 0,
            card_id: card_id.clone(),
        }]
    );

    let pending = state.pending.front().expect("三连应写入发现待选");
    let PendingChoice::Discover {
        seat,
        candidates,
        source,
    } = pending
    else {
        panic!("三连只登记发现待选");
    };
    assert_eq!(*seat, 0);
    assert_eq!(*source, 10, "来源为金色随从的实例编号");
    assert_eq!(candidates.len(), 3, "候选最多三张");

    let catalog = test_catalog();
    let mut seen = BTreeSet::new();
    for candidate in candidates {
        let card = catalog.cards.get(candidate).expect("候选应存在于目录");
        assert_eq!(card.level, 2, "发现星级为原卡 +1");
        assert_eq!(card.card_type, CardType::Minion);
        assert!(!card.is_hero);
        assert!(
            card.tribe == Tribe::General || state.enabled_tribes.contains(&card.tribe),
            "候选流派需本局启用或为通用"
        );
        assert!(pool_count(&state, candidate) > 0, "候选池需有剩余");
        assert!(seen.insert(candidate.clone()), "候选不得重复");
    }
}

#[test]
fn discover_level_six_stays_at_six() {
    let (engine, mut state) = setup(5);
    let card_id = minion_id(6);
    state.players[0].board = vec![
        dummy(&engine.catalog, 10, &card_id),
        dummy(&engine.catalog, 11, &card_id),
        dummy(&engine.catalog, 12, &card_id),
    ];

    let events = engine.check_triple(&mut state, 0);
    assert_eq!(
        events,
        vec![GameEvent::TripleFormed {
            seat: 0,
            card_id: card_id.clone(),
        }]
    );

    let pending = state.pending.front().expect("应写入发现待选");
    let PendingChoice::Discover { candidates, .. } = pending else {
        panic!("三连只登记发现待选");
    };
    assert!(!candidates.is_empty());
    let catalog = test_catalog();
    for candidate in candidates {
        assert_eq!(
            catalog.cards.get(candidate).expect("候选应存在").level,
            6,
            "6 星三连的发现仍为 6 星"
        );
    }
}

#[test]
fn no_candidates_skips_pending() {
    let (engine, mut state) = setup(6);
    let card_id = minion_id(6);
    for card in test_catalog().cards.values() {
        if card.card_type == CardType::Minion && !card.is_hero && card.level == 6 {
            state.pool.insert(card.id.clone(), 0);
        }
    }
    state.players[0].board = vec![
        dummy(&engine.catalog, 10, &card_id),
        dummy(&engine.catalog, 11, &card_id),
        dummy(&engine.catalog, 12, &card_id),
    ];

    let events = engine.check_triple(&mut state, 0);

    assert_eq!(
        events,
        vec![GameEvent::TripleFormed {
            seat: 0,
            card_id: card_id.clone(),
        }],
        "无候选时仍合成金色，但只记录三连事件"
    );
    assert!(state.pending.is_empty(), "无候选时不写入待选");
}

#[test]
fn choose_discover_out_of_range_rejects() {
    let (mut engine, mut state) = setup(7);
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
    let (mut engine, mut state) = setup(8);
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
    assert_eq!(state.players[0].hand[0].card_id, chosen);
    assert!(!state.players[0].hand[0].golden);
    assert_eq!(
        pool_count(&state, &chosen),
        before_pool - 1,
        "发现的卡扣 1 份池"
    );
}

#[test]
fn discovered_third_copy_can_form_triple() {
    let (mut engine, mut state) = setup(9);
    let card_id = minion_id(1);
    state.players[0].hand = vec![hand_card(&card_id), hand_card(&card_id)];
    state.pending.push_back(PendingChoice::Discover {
        seat: 0,
        candidates: vec![card_id.clone()],
        source: 1,
    });

    let outcome = engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect("发现应成功");

    assert_eq!(
        outcome.events,
        vec![
            GameEvent::CardDiscovered {
                seat: 0,
                card_id: card_id.clone(),
            },
            GameEvent::TripleFormed {
                seat: 0,
                card_id: card_id.clone(),
            },
        ],
        "发现的第三张也能触发三连"
    );
    assert!(state.players[0].hand.is_empty());
    assert_eq!(state.players[0].board.len(), 1);
    assert!(state.players[0].board[0].golden);
    assert_eq!(state.players[0].board[0].card_id, card_id);
    assert_eq!(state.pending.len(), 1, "新三连又产生一次发现待选");
}

#[test]
fn hand_triple_goes_to_board_when_board_has_room() {
    let (engine, mut state) = setup(10);
    let card_id = minion_id(2);
    state.players[0].hand = vec![
        hand_card(&card_id),
        hand_card(&card_id),
        hand_card(&card_id),
    ];

    let events = engine.check_triple(&mut state, 0);

    assert_eq!(
        events,
        vec![GameEvent::TripleFormed {
            seat: 0,
            card_id: card_id.clone(),
        }]
    );
    assert!(state.players[0].hand.is_empty());
    assert_eq!(state.players[0].board.len(), 1);
    assert!(state.players[0].board[0].golden);
    assert_eq!(state.players[0].board[0].card_id, card_id);
    assert_eq!(
        state.players[0].board[0].instance_id,
        state.next_instance_id - 1,
        "入手牌三连的金色随从分配新实例编号"
    );
}

#[test]
fn hand_triple_stays_in_hand_when_board_full() {
    let (engine, mut state) = setup(11);
    let card_id = minion_id(2);
    fill_board_distinct(&mut state, 0, engine.catalog.config.board_slots, &card_id);
    let board_before: Vec<InstanceId> = state.players[0]
        .board
        .iter()
        .map(|minion| minion.instance_id)
        .collect();
    state.players[0].hand = vec![
        hand_card(&card_id),
        hand_card(&card_id),
        hand_card(&card_id),
    ];

    let events = engine.check_triple(&mut state, 0);

    assert_eq!(
        events,
        vec![GameEvent::TripleFormed {
            seat: 0,
            card_id: card_id.clone(),
        }]
    );
    let board_after: Vec<InstanceId> = state.players[0]
        .board
        .iter()
        .map(|minion| minion.instance_id)
        .collect();
    assert_eq!(board_before, board_after, "棋盘已满时不改动棋盘");
    assert_eq!(state.players[0].hand.len(), 1, "三张手牌合成一张金色手牌");
    assert_eq!(state.players[0].hand[0].card_id, card_id);
    assert!(state.players[0].hand[0].golden);
}

#[test]
fn multiple_pending_choices_consume_frontmost_for_seat() {
    let (mut engine, mut state) = setup(12);
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
    assert_eq!(state.players[0].hand[0].card_id, first);
    assert_eq!(state.pending.len(), 2, "只消费队列中最靠前的同座位待选");
    assert!(matches!(
        state.pending.front(),
        Some(PendingChoice::Discover { seat: 1, .. })
    ));

    engine
        .apply(&mut state, 1, Intent::ChooseDiscover { choice: 0 })
        .expect("座位 1 的发现应成功");
    assert_eq!(state.players[1].hand[0].card_id, second);
    assert_eq!(state.pending.len(), 1);
    assert!(matches!(
        state.pending.front(),
        Some(PendingChoice::Discover { seat: 0, candidates, .. }) if candidates == &vec![third]
    ));
}

#[test]
fn choose_discover_with_full_hand_rejects_without_mutating() {
    let (mut engine, mut state) = setup(13);
    let chosen = minion_id(2);
    state.players[0].hand = (0..engine.catalog.config.hand_slots)
        .map(|_| hand_card(&minion_id(1)))
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
fn golden_battlecry_triggers_twice() {
    // Task 10 接入效果执行器后，在此断言战吼连续触发两次且中间不插入其他效果。
    // 本任务只验证金色合成结果，不执行任何效果。
    let (engine, mut state) = setup(14);
    let card_id = minion_id(1);
    state.players[0].board = vec![
        dummy(&engine.catalog, 10, &card_id),
        dummy(&engine.catalog, 11, &card_id),
        dummy(&engine.catalog, 12, &card_id),
    ];

    engine.check_triple(&mut state, 0);

    assert_eq!(state.players[0].board.len(), 1);
    assert!(state.players[0].board[0].golden);
}

#[test]
fn hand_effect_cards_never_form_triple() {
    let (engine, mut state) = setup(15);
    let effect_id = effect_card_id();
    let minion = minion_id(2);
    state.players[0].hand = vec![
        hand_card(&effect_id),
        hand_card(&effect_id),
        hand_card(&effect_id),
    ];
    state.players[0].board = vec![dummy(&engine.catalog, 10, &minion)];

    let events = engine.check_triple(&mut state, 0);

    assert!(events.is_empty(), "效果牌不参与三连");
    assert!(state.pending.is_empty(), "效果牌不产生发现待选");
    assert_eq!(state.players[0].board.len(), 1, "棋盘不增加非法随从");
    assert_eq!(state.players[0].board[0].card_id, minion);
    assert_eq!(state.players[0].hand.len(), 3, "三张效果牌都保留");
    assert!(
        state.players[0]
            .hand
            .iter()
            .all(|card| card.card_id == effect_id && !card.golden)
    );
}

#[test]
fn one_check_forms_multiple_triples() {
    let (engine, mut state) = setup(16);
    let card_id = minion_id(2);
    state.players[0].board = vec![
        dummy(&engine.catalog, 10, &card_id),
        dummy(&engine.catalog, 11, &card_id),
        dummy(&engine.catalog, 12, &card_id),
    ];
    state.players[0].hand = vec![
        hand_card(&card_id),
        hand_card(&card_id),
        hand_card(&card_id),
    ];

    let events = engine.check_triple(&mut state, 0);

    assert_eq!(
        events,
        vec![
            GameEvent::TripleFormed {
                seat: 0,
                card_id: card_id.clone(),
            },
            GameEvent::TripleFormed {
                seat: 0,
                card_id: card_id.clone(),
            },
        ],
        "六张同名副本一次调用合成两次"
    );
    assert!(state.players[0].hand.is_empty());
    assert_eq!(state.players[0].board.len(), 2);
    assert!(
        state.players[0]
            .board
            .iter()
            .all(|minion| minion.golden && minion.card_id == card_id)
    );
    assert_eq!(state.pending.len(), 2, "两次三连各产生一个发现待选");
    assert!(
        state
            .pending
            .iter()
            .all(|pending| matches!(pending, PendingChoice::Discover { seat: 0, .. }))
    );
}
