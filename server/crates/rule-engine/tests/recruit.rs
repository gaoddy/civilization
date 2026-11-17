mod common;

use common::{heroes, minion, minion_id, set_shop, test_catalog, turn_started};
use rule_engine::data::{CardType, Era};
use rule_engine::engine::Engine;
use rule_engine::events::GameEvent;
use rule_engine::hash::state_hash;
use rule_engine::intent::{Intent, RejectCode};
use rule_engine::state::{GameState, HandCard, InstanceId, PendingChoice, Phase, Seat};

fn effect_card() -> (String, u8) {
    test_catalog()
        .cards
        .values()
        .find(|card| card.card_type == CardType::Effect)
        .map(|card| (card.id.clone(), card.cost.unwrap_or(3)))
        .expect("数据中应存在效果牌")
}

fn fill_board(state: &mut GameState, seat: Seat, count: usize) {
    let catalog = test_catalog();
    let ids: Vec<String> = catalog
        .cards
        .values()
        .filter(|card| card.card_type == CardType::Minion && !card.is_hero)
        .map(|card| card.id.clone())
        .take(count)
        .collect();
    let player = &mut state.players[usize::from(seat)];
    player.board = ids
        .iter()
        .enumerate()
        .map(|(index, card_id)| minion(&test_catalog(), 1_000 + index as InstanceId, card_id))
        .collect();
}

fn fill_hand(state: &mut GameState, seat: Seat, count: usize) {
    let card_id = minion_id(1);
    let player = &mut state.players[usize::from(seat)];
    player.hand = (0..count)
        .map(|_| HandCard::effect(9_000, &card_id.clone()))
        .collect();
}

fn pool_count(state: &GameState, card_id: &str) -> u16 {
    state.pool.get(card_id).copied().unwrap_or(0)
}

#[test]
fn buy_deducts_gold_and_pool_and_goes_to_hand() {
    let (mut engine, mut state) = turn_started(1);
    let card_id = minion_id(1);
    set_shop(&mut state, 0, &[Some(card_id.clone())]);
    let before_pool = pool_count(&state, &card_id);
    let before_instance = state.next_instance_id;

    let outcome = engine
        .apply(&mut state, 0, Intent::BuyMinion { slot: 0 })
        .expect("购买应成功");

    assert_eq!(
        outcome.events,
        vec![GameEvent::MinionBought {
            seat: 0,
            slot: 0,
            card_id: card_id.clone(),
        }]
    );
    assert_eq!(state.players[0].gold, 0, "随从统一 3 金");
    assert_eq!(pool_count(&state, &card_id), before_pool - 1);
    assert_eq!(state.players[0].shop[0], None);
    assert!(
        state.players[0].board.is_empty(),
        "买入不入场，须另提交 PlayCard"
    );
    assert_eq!(
        state.next_instance_id, before_instance,
        "买入不分配实例编号"
    );
    assert_eq!(state.players[0].hand.len(), 1);
    assert_eq!(state.players[0].hand[0].card_id(), card_id);
    assert!(!state.players[0].hand[0].golden());
}

#[test]
fn buy_effect_card_goes_to_hand_and_costs_card_cost() {
    let (mut engine, mut state) = turn_started(2);
    let (card_id, cost) = effect_card();
    set_shop(&mut state, 0, &[Some(card_id.clone())]);

    let outcome = engine
        .apply(&mut state, 0, Intent::BuyMinion { slot: 0 })
        .expect("购买效果牌应成功");

    assert_eq!(
        outcome.events,
        vec![GameEvent::MinionBought {
            seat: 0,
            slot: 0,
            card_id: card_id.clone(),
        }]
    );
    assert_eq!(state.players[0].gold, 3 - cost);
    assert_eq!(state.players[0].hand.len(), 1);
    assert_eq!(state.players[0].hand[0].card_id(), card_id);
    assert!(!state.players[0].hand[0].golden());
    assert!(state.players[0].board.is_empty());
}

#[test]
fn buy_without_gold_rejects_without_mutating() {
    let (mut engine, mut state) = turn_started(3);
    let card_id = minion_id(1);
    set_shop(&mut state, 0, &[Some(card_id)]);
    state.players[0].gold = 0;
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::BuyMinion { slot: 0 });

    assert_eq!(result.unwrap_err().code, RejectCode::NotEnoughGold);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn buy_empty_slot_rejects() {
    let (mut engine, mut state) = turn_started(4);
    set_shop(&mut state, 0, &[None, None]);
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::BuyMinion { slot: 0 });

    assert_eq!(result.unwrap_err().code, RejectCode::ShopSlotEmpty);
    assert_eq!(before, state_hash(&state));

    let out_of_range = engine.apply(&mut state, 0, Intent::BuyMinion { slot: 99 });
    assert_eq!(out_of_range.unwrap_err().code, RejectCode::ShopSlotEmpty);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn buy_when_hand_full_rejects_even_with_board_room() {
    let (mut engine, mut state) = turn_started(5);
    let card_id = minion_id(1);
    set_shop(&mut state, 0, &[Some(card_id)]);
    fill_hand(&mut state, 0, 10);
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::BuyMinion { slot: 0 });

    assert_eq!(result.unwrap_err().code, RejectCode::NoSpace);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn buy_enters_hand_even_when_board_has_room() {
    let (mut engine, mut state) = turn_started(6);
    let card_id = minion_id(1);
    set_shop(&mut state, 0, &[Some(card_id.clone())]);

    engine
        .apply(&mut state, 0, Intent::BuyMinion { slot: 0 })
        .expect("棋盘有空位时也应入手牌");

    assert!(state.players[0].board.is_empty());
    assert_eq!(state.players[0].hand.len(), 1);
    assert_eq!(state.players[0].hand[0].card_id(), card_id);
}

#[test]
fn sell_returns_gold_and_pool() {
    let (mut engine, mut state) = turn_started(7);
    let card_id = minion_id(1);
    state.players[0]
        .board
        .push(minion(&engine.catalog, 42, &card_id));
    state.players[0].gold = 0;
    let before_pool = pool_count(&state, &card_id);

    let outcome = engine
        .apply(&mut state, 0, Intent::SellMinion { minion: 42 })
        .expect("出售应成功");

    assert_eq!(
        outcome.events,
        vec![GameEvent::MinionSold {
            seat: 0,
            minion: 42,
        }]
    );
    assert_eq!(state.players[0].gold, 1);
    assert!(state.players[0].board.is_empty());
    assert_eq!(pool_count(&state, &card_id), before_pool + 1);
}

#[test]
fn sell_golden_returns_three() {
    let (mut engine, mut state) = turn_started(8);
    let card_id = minion_id(2);
    let before_pool = pool_count(&state, &card_id);
    let mut golden = minion(&engine.catalog, 43, &card_id);
    golden.golden = true;
    state.players[0].board.push(golden);
    state.players[0].gold = 0;

    engine
        .apply(&mut state, 0, Intent::SellMinion { minion: 43 })
        .expect("出售金色随从应成功");

    assert_eq!(state.players[0].gold, 3);
    assert_eq!(
        pool_count(&state, &card_id),
        before_pool + 1,
        "金色随从也只归还 1 份池"
    );
}

#[test]
fn sell_missing_minion_rejects() {
    let (mut engine, mut state) = turn_started(9);
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::SellMinion { minion: 777 });

    assert_eq!(result.unwrap_err().code, RejectCode::NoSuchMinion);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn reroll_first_is_free_then_costs_one() {
    let (mut engine, mut state) = turn_started(10);
    let card_id = minion_id(1);
    let before_pool = pool_count(&state, &card_id);
    assert_eq!(state.players[0].gold, 3);
    assert!(!state.players[0].first_refresh_used);

    let outcome = engine
        .apply(&mut state, 0, Intent::RerollShop)
        .expect("首次刷新免费");
    assert_eq!(outcome.events, vec![GameEvent::ShopRolled { seat: 0 }]);
    assert_eq!(state.players[0].gold, 3);
    assert!(state.players[0].first_refresh_used);

    engine
        .apply(&mut state, 0, Intent::RerollShop)
        .expect("第二次刷新应扣 1 金");
    assert_eq!(state.players[0].gold, 2);
    assert_eq!(pool_count(&state, &card_id), before_pool, "刷新不扣池");
}

#[test]
fn reroll_without_gold_rejects() {
    let (mut engine, mut state) = turn_started(11);
    state.players[0].gold = 0;
    state.players[0].first_refresh_used = true;
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::RerollShop);

    assert_eq!(result.unwrap_err().code, RejectCode::NotEnoughGold);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn upgrade_era_costs_and_rerolls() {
    let (mut engine, mut state) = turn_started(12);
    state.players[0].gold = 10;
    let frozen_card = minion_id(2);
    set_shop(&mut state, 0, &[Some(frozen_card.clone())]);
    engine
        .apply(&mut state, 0, Intent::FreezeShop { slot: Some(0) })
        .expect("冻结应成功");

    let outcome = engine
        .apply(&mut state, 0, Intent::UpgradeEra)
        .expect("升时代应成功");

    assert_eq!(state.players[0].era, Era::Primitive);
    assert_eq!(state.players[0].gold, 5);
    assert_eq!(state.players[0].turns_in_era, 0);
    assert!(
        !state.players[0].first_refresh_used,
        "免费重抽不占本回合免费刷新"
    );
    assert_eq!(
        outcome.events[0],
        GameEvent::EraUpgraded {
            seat: 0,
            era: Era::Primitive,
        }
    );
    assert!(outcome.events.contains(&GameEvent::ShopRolled { seat: 0 }));
    assert_eq!(
        state.players[0].shop[0].as_ref().map(|s| s.frozen),
        Some(false)
    );

    engine
        .apply(&mut state, 0, Intent::RerollShop)
        .expect("升时代后的首次手动刷新仍应免费");
    assert_eq!(state.players[0].gold, 5);
}

#[test]
fn upgrade_without_gold_rejects() {
    let (mut engine, mut state) = turn_started(13);
    state.players[0].gold = 4;
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::UpgradeEra);

    assert_eq!(result.unwrap_err().code, RejectCode::NotEnoughGold);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn upgrade_at_future_rejects() {
    let (mut engine, mut state) = turn_started(14);
    state.players[0].era = Era::Future;
    state.players[0].gold = 200;
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::UpgradeEra);

    assert_eq!(result.unwrap_err().code, RejectCode::AlreadyMaxEra);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn play_minion_from_hand_moves_to_board() {
    let (mut engine, mut state) = turn_started(15);
    let card_id = minion_id(1);
    state.players[0]
        .hand
        .push(HandCard::Minion(minion(&engine.catalog, 9_000, &card_id)));
    let before_pool = pool_count(&state, &card_id);

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
        .expect("打出随从应成功");

    assert!(outcome.events.is_empty(), "本任务不执行战吼");
    assert!(state.players[0].hand.is_empty());
    assert_eq!(state.players[0].board.len(), 1);
    assert_eq!(state.players[0].board[0].card_id, card_id);
    assert_eq!(state.players[0].gold, 3, "打出不再收费");
    assert_eq!(pool_count(&state, &card_id), before_pool, "打出不扣池");
}

#[test]
fn play_effect_card_leaves_hand_without_events() {
    let (mut engine, mut state) = turn_started(16);
    let (card_id, _) = effect_card();
    state.players[0]
        .hand
        .push(HandCard::effect(9_000, &card_id));

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
        .expect("打出效果牌应成功");

    assert!(outcome.events.is_empty());
    assert!(state.players[0].hand.is_empty());
    assert!(state.players[0].board.is_empty());
}

#[test]
fn play_card_out_of_range_rejects() {
    let (mut engine, mut state) = turn_started(17);
    let before = state_hash(&state);

    let result = engine.apply(
        &mut state,
        0,
        Intent::PlayCard {
            hand_index: 3,
            position: None,
            target: None,
        },
    );

    assert_eq!(result.unwrap_err().code, RejectCode::CardNotInHand);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn play_minion_with_full_board_rejects() {
    let (mut engine, mut state) = turn_started(18);
    fill_board(&mut state, 0, 7);
    let card_id = minion_id(1);
    state.players[0]
        .hand
        .push(HandCard::Minion(minion(&engine.catalog, 9_000, &card_id)));
    let before = state_hash(&state);

    let result = engine.apply(
        &mut state,
        0,
        Intent::PlayCard {
            hand_index: 0,
            position: None,
            target: None,
        },
    );

    assert_eq!(result.unwrap_err().code, RejectCode::NoSpace);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn rearrange_moves_minion_to_target_index() {
    let (mut engine, mut state) = turn_started(19);
    state.players[0].board = vec![
        minion(&engine.catalog, 1, "a"),
        minion(&engine.catalog, 2, "b"),
        minion(&engine.catalog, 3, "c"),
    ];

    let outcome = engine
        .apply(&mut state, 0, Intent::RearrangeBoard { from: 0, to: 2 })
        .expect("换位应成功");
    assert!(outcome.events.is_empty());
    let order: Vec<InstanceId> = state.players[0]
        .board
        .iter()
        .map(|m| m.instance_id)
        .collect();
    assert_eq!(order, vec![2, 3, 1], "移除后在目标下标重新插入");

    engine
        .apply(&mut state, 0, Intent::RearrangeBoard { from: 1, to: 1 })
        .expect("同位置应为空操作");
    let order: Vec<InstanceId> = state.players[0]
        .board
        .iter()
        .map(|m| m.instance_id)
        .collect();
    assert_eq!(order, vec![2, 3, 1]);
}

#[test]
fn rearrange_out_of_range_rejects() {
    let (mut engine, mut state) = turn_started(20);
    state.players[0].board = vec![
        minion(&engine.catalog, 1, "a"),
        minion(&engine.catalog, 2, "b"),
    ];
    let before = state_hash(&state);

    for (from, to) in [(0u8, 2u8), (2, 0), (5, 5)] {
        let result = engine.apply(&mut state, 0, Intent::RearrangeBoard { from, to });
        assert_eq!(
            result.unwrap_err().code,
            RejectCode::NoSuchMinion,
            "from={from} to={to}"
        );
        assert_eq!(before, state_hash(&state));
    }
}

#[test]
fn freeze_persists_next_turn_and_reroll_clears() {
    let (mut engine, mut state) = turn_started(21);
    let frozen_id = minion_id(2);
    let other_id = minion_id(3);
    set_shop(
        &mut state,
        0,
        &[Some(frozen_id.clone()), Some(other_id.clone())],
    );

    engine
        .apply(&mut state, 0, Intent::FreezeShop { slot: Some(0) })
        .expect("冻结应成功");
    assert!(state.players[0].shop[0].as_ref().unwrap().frozen);
    assert!(!state.players[0].shop[1].as_ref().unwrap().frozen);

    let turn_before = state.turn;
    engine.start_turn(&mut state).expect("开始下一回合");
    assert_eq!(state.turn, turn_before + 1);
    let kept = state.players[0].shop[0].as_ref().expect("冻结槽位应保留");
    assert_eq!(kept.card_id, frozen_id);
    assert!(kept.frozen, "冻结标记跨回合保留");

    engine
        .apply(&mut state, 0, Intent::RerollShop)
        .expect("手动刷新应成功");
    assert!(
        state.players[0]
            .shop
            .iter()
            .flatten()
            .all(|slot| !slot.frozen),
        "手动刷新清除所有冻结"
    );
}

#[test]
fn freeze_with_none_freezes_all_slots() {
    let (mut engine, mut state) = turn_started(22);
    set_shop(&mut state, 0, &[Some(minion_id(1)), Some(minion_id(2))]);

    engine
        .apply(&mut state, 0, Intent::FreezeShop { slot: None })
        .expect("冻结全部应成功");

    assert!(
        state.players[0]
            .shop
            .iter()
            .flatten()
            .all(|slot| slot.frozen)
    );
}

#[test]
fn freeze_empty_slot_rejects() {
    let (mut engine, mut state) = turn_started(23);
    set_shop(&mut state, 0, &[None]);
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::FreezeShop { slot: Some(0) });

    assert_eq!(result.unwrap_err().code, RejectCode::ShopSlotEmpty);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn use_hero_power_respects_cost_and_uses() {
    let (mut engine, mut state) = turn_started(24);
    assert_eq!(state.players[0].hero.as_deref(), Some("adam-smith"));
    assert_eq!(state.players[0].gold, 3);

    engine
        .apply(
            &mut state,
            0,
            Intent::UseHeroPower {
                skill_id: "division-of-labor".to_string(),
                target: None,
            },
        )
        .expect("主动技能应成功");
    assert_eq!(state.players[0].gold, 2);
    assert_eq!(
        state.players[0].hero_uses.get("division-of-labor").copied(),
        Some(1)
    );

    let before = state_hash(&state);
    let exhausted = engine.apply(
        &mut state,
        0,
        Intent::UseHeroPower {
            skill_id: "division-of-labor".to_string(),
            target: None,
        },
    );
    assert_eq!(exhausted.unwrap_err().code, RejectCode::HeroPowerLimit);
    assert_eq!(before, state_hash(&state));

    engine.start_turn(&mut state).expect("开始下一回合");
    assert!(
        state.players[0].hero_uses.is_empty(),
        "回合开始重置技能次数"
    );
    assert_eq!(state.players[0].gold, 4);
    engine
        .apply(
            &mut state,
            0,
            Intent::UseHeroPower {
                skill_id: "division-of-labor".to_string(),
                target: None,
            },
        )
        .expect("新回合技能次数已重置");
    assert_eq!(state.players[0].gold, 3);
}

#[test]
fn use_passive_or_unknown_hero_power_rejects() {
    let (mut engine, mut state) = turn_started(25);

    let passive = engine.apply(
        &mut state,
        0,
        Intent::UseHeroPower {
            skill_id: "observation".to_string(),
            target: None,
        },
    );
    assert_eq!(passive.unwrap_err().code, RejectCode::HeroPowerLimit);

    let unknown = engine.apply(
        &mut state,
        0,
        Intent::UseHeroPower {
            skill_id: "not-a-skill".to_string(),
            target: None,
        },
    );
    assert_eq!(unknown.unwrap_err().code, RejectCode::HeroPowerLimit);

    state.players[0].hero = Some("galileo".to_string());
    let hero_without_skill = engine.apply(
        &mut state,
        0,
        Intent::UseHeroPower {
            skill_id: "observation".to_string(),
            target: None,
        },
    );
    assert_eq!(
        hero_without_skill.unwrap_err().code,
        RejectCode::HeroPowerLimit
    );
}

#[test]
fn use_hero_power_without_gold_rejects() {
    let (mut engine, mut state) = turn_started(26);
    state.players[0].gold = 0;
    let before = state_hash(&state);

    let result = engine.apply(
        &mut state,
        0,
        Intent::UseHeroPower {
            skill_id: "division-of-labor".to_string(),
            target: None,
        },
    );

    assert_eq!(result.unwrap_err().code, RejectCode::NotEnoughGold);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn intent_outside_recruit_rejects() {
    let (mut engine, mut state) = turn_started(27);
    state.phase = Phase::Combat;
    let before = state_hash(&state);

    for intent in [
        Intent::RerollShop,
        Intent::BuyMinion { slot: 0 },
        Intent::ChooseDiscover { choice: 0 },
        Intent::EndRecruit,
    ] {
        let result = engine.apply(&mut state, 0, intent);
        assert_eq!(result.unwrap_err().code, RejectCode::InvalidStage);
    }
    assert_eq!(before, state_hash(&state));
}

#[test]
fn end_recruit_marks_seat_until_all_ready() {
    // 核心规则 §2/§3：EndRecruit 只表示「本玩家结束招募」，全员结束后才进入 Combat。
    let (mut engine, mut state) = turn_started(28);

    let outcome = engine
        .apply(&mut state, 0, Intent::EndRecruit)
        .expect("结束招募应成功");

    assert_eq!(state.phase, Phase::Recruit, "单个座位结束不推进阶段");
    assert!(outcome.events.is_empty(), "未全员结束时不发 PhaseChanged");
    assert!(state.players[0].recruit_done, "该座位标记为已结束");

    // 其余座位仍可执行招募意图，已结束的座位不可再执行。
    state.players[1].gold = 10;
    let shop_before = state.players[1].shop.clone();
    engine
        .apply(&mut state, 1, Intent::RerollShop)
        .expect("其他座位仍可招募");
    assert_ne!(state.players[1].shop, shop_before, "其他座位刷新应生效");
    let before = state_hash(&state);
    let reject = engine.apply(&mut state, 0, Intent::RerollShop);
    assert_eq!(reject.unwrap_err().code, RejectCode::InvalidStage);
    assert_eq!(
        before,
        state_hash(&state),
        "已结束座位的意图被拒绝且不改状态"
    );

    // 未全员前重复 EndRecruit 仍然成功且无副作用（核心规则 §3：前置条件只有「处于
    // Recruit」，失败返回码为「—」）。
    let before = state_hash(&state);
    let again = engine
        .apply(&mut state, 0, Intent::EndRecruit)
        .expect("未全员前重复 EndRecruit 应成功");
    assert!(again.events.is_empty(), "重复提交不发事件");
    assert_eq!(before, state_hash(&state), "重复提交不改变状态");

    // 其余座位逐个结束，只有最后一个结束的座位带来 PhaseChanged → Combat。
    for seat in 1..7 {
        let outcome = engine
            .apply(&mut state, seat, Intent::EndRecruit)
            .expect("结束招募应成功");
        assert_eq!(state.phase, Phase::Recruit);
        assert!(outcome.events.is_empty());
    }
    let outcome = engine
        .apply(&mut state, 7, Intent::EndRecruit)
        .expect("最后一名结束招募应成功");
    assert_eq!(state.phase, Phase::Combat);
    assert_eq!(
        outcome.events,
        vec![GameEvent::PhaseChanged {
            phase: Phase::Combat,
            turn: state.turn,
        }]
    );

    // 全员就绪进入 Combat 后，重复提交不再处于 Recruit，返回 InvalidStage 且不改状态。
    let before = state_hash(&state);
    let reject = engine.apply(&mut state, 0, Intent::EndRecruit);
    assert_eq!(reject.unwrap_err().code, RejectCode::InvalidStage);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn end_recruit_ignores_eliminated_seats() {
    let (mut engine, mut state) = turn_started(33);
    state.players[7].eliminated = true;
    for seat in 0..7 {
        engine
            .apply(&mut state, seat, Intent::EndRecruit)
            .expect("结束招募应成功");
    }
    assert_eq!(state.phase, Phase::Combat, "出局座位不计入就绪");
}

#[test]
fn recruit_done_is_hashed_and_reset_each_turn() {
    let (mut engine, mut state) = turn_started(34);
    let before = state_hash(&state);
    state.players[0].recruit_done = true;
    assert_ne!(before, state_hash(&state), "招募就绪标记计入状态哈希");
    state.players[0].recruit_done = false;

    engine
        .apply(&mut state, 0, Intent::EndRecruit)
        .expect("结束招募应成功");
    engine.start_turn(&mut state).expect("进入下一回合");
    assert!(
        state.players.iter().all(|player| !player.recruit_done),
        "新回合重置全员就绪标记"
    );
}

#[test]
fn reroll_with_empty_star_retries_then_leaves_slot_empty() {
    let (mut engine, mut state) = turn_started(29);
    // 史前时代只给 7 星权重，而 7 星只有随从；池清空后候选必为空。
    engine.catalog.config.shop_weights[0] = [0, 0, 0, 0, 0, 0, 100];
    for count in state.pool.values_mut() {
        *count = 0;
    }

    let outcome = engine
        .apply(&mut state, 0, Intent::RerollShop)
        .expect("空层重试后不应失败");
    assert_eq!(outcome.events, vec![GameEvent::ShopRolled { seat: 0 }]);
    assert!(
        state.players[0].shop.iter().all(Option::is_none),
        "重试耗尽后槽位为空"
    );
}

#[test]
fn reroll_with_exhausted_pool_leaves_empty_slots_and_buy_rejects() {
    let (mut engine, mut state) = turn_started(29);
    // 全池耗尽：清空共享池后，1 星权重的候选只剩不耗池的效果牌。
    // 把 1 星效果牌 aq-jingshen、dishengxiyu、erfenfa（不耗池）从本次目录中移除，
    // 抽卡候选于是全部为空。
    engine.catalog.config.shop_weights[0] = [100, 0, 0, 0, 0, 0, 0];
    engine.catalog.cards.remove("aq-jingshen");
    engine.catalog.cards.remove("erfenfa");
    engine.catalog.cards.remove("dishengxiyu");
    for count in state.pool.values_mut() {
        *count = 0;
    }
    // 先清空槽位，确保本次刷新全部重新抽卡（不保留原有上架卡）。
    for slot in state.players[0].shop.iter_mut() {
        *slot = None;
    }

    let outcome = engine
        .apply(&mut state, 0, Intent::RerollShop)
        .expect("空池重试后不应失败");
    assert_eq!(outcome.events, vec![GameEvent::ShopRolled { seat: 0 }]);
    let shop = &state.players[0].shop;
    assert_eq!(shop.len(), 6, "商店仍保留基础槽位数");
    assert!(shop.iter().all(Option::is_none), "全池耗尽后槽位留空");
    for slot in 0..u8::try_from(shop.len()).expect("槽位数可放入 u8") {
        let reject = engine
            .apply(&mut state, 0, Intent::BuyMinion { slot })
            .expect_err("空槽不可购买");
        assert_eq!(reject.code, RejectCode::ShopSlotEmpty, "槽位 {slot}");
    }
}

#[test]
fn hero_cards_never_appear_in_shop() {
    let (mut engine, mut state) = turn_started(30);
    // 只刷 6 星：该星级同时有英雄卡 knuth 与普通随从。
    engine.catalog.config.shop_weights[0] = [0, 0, 0, 0, 0, 100, 0];
    let catalog = test_catalog();

    for _ in 0..40 {
        engine.roll_shop(&mut state, 0);
        for slot in state.players[0].shop.iter().flatten() {
            let card = catalog
                .cards
                .get(&slot.card_id)
                .expect("商店卡牌应存在于目录");
            assert!(!card.is_hero, "英雄卡 {} 不应出现在商店", card.id);
            assert_eq!(card.level, 6);
        }
    }
}

#[test]
fn start_turn_rolls_shops_and_skips_eliminated() {
    let (mut engine, mut state) = turn_started(31);
    state.players[7].eliminated = true;
    set_shop(&mut state, 7, &[Some(minion_id(1))]);
    let sentinel = state.players[7].shop.clone();

    let events = engine.start_turn(&mut state).expect("开始下一回合");

    let rolled: Vec<Seat> = events
        .iter()
        .filter_map(|event| match event {
            GameEvent::ShopRolled { seat } => Some(*seat),
            _ => None,
        })
        .collect();
    assert_eq!(rolled, vec![0, 1, 2, 3, 4, 5, 6]);
    for seat in 0..7 {
        assert_eq!(state.players[seat].shop.len(), 6);
        assert!(
            state.players[seat].shop.iter().all(Option::is_some),
            "座位 {seat} 的商店应被填满"
        );
    }
    assert_eq!(state.players[7].shop, sentinel, "淘汰玩家不刷新商店");
}

#[test]
fn start_turn_increments_turns_in_era_and_resets_hero_uses() {
    let (mut engine, mut state) = turn_started(32);
    state.players[0].turns_in_era = 2;
    state.players[0]
        .hero_uses
        .insert("division-of-labor".to_string(), 1);

    engine.start_turn(&mut state).expect("开始下一回合");

    assert_eq!(state.players[0].turns_in_era, 3);
    assert!(state.players[0].hero_uses.is_empty());
}

#[test]
fn choose_discover_without_pending_rejects_invalid_stage() {
    let (mut engine, mut state) = turn_started(33);
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 });

    assert_eq!(result.unwrap_err().code, RejectCode::InvalidStage);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn choose_discover_out_of_range_rejects_invalid_choice() {
    let (mut engine, mut state) = turn_started(34);
    state
        .pending
        .push_back(rule_engine::state::PendingChoice::Discover {
            seat: 0,
            candidates: vec!["a".to_string(), "b".to_string()],
            source: 1,
        });
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::ChooseDiscover { choice: 5 });

    assert_eq!(result.unwrap_err().code, RejectCode::InvalidChoice);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn choose_discover_valid_choice_consumes_and_adds_to_hand() {
    let (mut engine, mut state) = turn_started(35);
    let chosen = minion_id(2);
    let before_pool = pool_count(&state, &chosen);
    state
        .pending
        .push_back(rule_engine::state::PendingChoice::Discover {
            seat: 0,
            candidates: vec![chosen.clone(), minion_id(3)],
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
    assert_eq!(state.players[0].hand.len(), 1);
    assert_eq!(state.players[0].hand[0].card_id(), chosen);
    assert!(!state.players[0].hand[0].golden());
    assert!(state.pending.is_empty(), "消费后待选被移除");
    assert_eq!(pool_count(&state, &chosen), before_pool - 1);
}

#[test]
fn buy_fires_no_battlecry_until_played() {
    let (mut engine, mut state) = turn_started(36);
    put_champion(&engine, &mut state, 0, 9_000, "adam-smith");
    set_shop(&mut state, 0, &[Some(BATTLECRY_TARGET.to_string())]);

    engine
        .apply(&mut state, 0, Intent::BuyMinion { slot: 0 })
        .expect("买入带战吼的随从不应报错");

    assert_eq!(state.players[0].hand.len(), 1, "买入只入手牌");
    let champion = state.players[0]
        .board
        .iter()
        .find(|minion| minion.instance_id == 9_000)
        .expect("原有随从仍在棋盘");
    assert_eq!(champion.stats.attack.growth, 0, "买入不入场，不结算战吼");
}

#[test]
fn start_game_fills_turn_one_shops() {
    let (mut engine, mut state) = Engine::new(test_catalog(), [36; 32]);
    let catalog = test_catalog();

    let events = engine
        .start_game(&mut state, &heroes())
        .expect("开局应成功");
    let rolled = events
        .iter()
        .filter(|event| matches!(event, GameEvent::ShopRolled { .. }))
        .count();
    assert_eq!(rolled, 8, "开局为每名玩家刷新一次商店");

    for seat in 0..8 {
        let shop = &state.players[seat].shop;
        assert_eq!(shop.len(), catalog.config.shop_slots);
        assert!(
            shop.iter().any(Option::is_some),
            "座位 {seat} 的第 1 回合商店应有卡"
        );
        for slot in shop.iter().flatten() {
            let card = catalog
                .cards
                .get(&slot.card_id)
                .expect("商店卡牌应存在于目录");
            assert!(!card.is_hero, "第 1 回合商店不应出现英雄卡 {}", card.id);
        }
    }

    let events = engine.start_turn(&mut state).expect("开始下一回合");
    let rolled = events
        .iter()
        .filter(|event| matches!(event, GameEvent::ShopRolled { .. }))
        .count();
    assert_eq!(rolled, 8, "start_turn 仍为每名玩家刷新一次");
}

// ---- 目标校验：需要单一显式目标的效果 ----
//
// 口径：存在合法目标但未指定或指定不合法 → `NO_VALID_TARGET`，整条意图被拒绝且
// 不产生任何状态变化；池为空 → 允许执行、效果落空。

/// 战吼「使一个友方随从获得成长」，需要目标。
const BATTLECRY_TARGET: &str = "bisheng";
/// 战吼「使一个友方机械获得成长」，己方无机械时池为空。
const BATTLECRY_TRIBE_TARGET: &str = "archimedes";
/// 效果牌「使一个友方随从获得 +1/+1」，需要目标。
const EFFECT_CARD_TARGET: &str = "chengbang-faling";

fn put_champion(
    engine: &Engine,
    state: &mut GameState,
    seat: Seat,
    instance_id: InstanceId,
    card_id: &str,
) {
    state.players[usize::from(seat)]
        .board
        .push(minion(&engine.catalog, instance_id, card_id));
}

#[test]
fn play_battlecry_without_target_enters_board_then_pends() {
    let (mut engine, mut state) = turn_started(40);
    put_champion(&engine, &mut state, 0, 9_000, "adam-smith");
    state.players[0]
        .hand
        .push(HandCard::effect(9_000, BATTLECRY_TARGET));

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
        .expect("战吼需要目标不再阻塞出牌");

    assert!(state.players[0].hand.is_empty(), "牌已离开手牌");
    assert_eq!(state.players[0].board.len(), 2, "随从先入场");
    assert!(
        target_pending(&state, 0).is_some(),
        "入场后产生目标待选，战吼尚未结算"
    );
}

#[test]
fn play_battlecry_with_unknown_target_pends_then_choose_rejects() {
    let (mut engine, mut state) = turn_started(41);
    put_champion(&engine, &mut state, 0, 9_000, "adam-smith");
    state.players[0]
        .hand
        .push(HandCard::effect(9_000, BATTLECRY_TARGET));

    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: Some(9_999),
            },
        )
        .expect("出牌本身成功，目标由待选重新指定");

    let before = state_hash(&state);
    let result = engine.apply(&mut state, 0, Intent::ChooseTarget { minion: 9_999 });

    assert_eq!(result.unwrap_err().code, RejectCode::NoValidTarget);
    assert_eq!(before, state_hash(&state), "拒绝后状态不变");
}

#[test]
fn play_battlecry_with_target_applies_effect() {
    let (mut engine, mut state) = turn_started(42);
    put_champion(&engine, &mut state, 0, 9_000, "adam-smith");
    state.players[0]
        .hand
        .push(HandCard::effect(9_000, BATTLECRY_TARGET));

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
        .expect("打出随从应成功");
    engine
        .apply(&mut state, 0, Intent::ChooseTarget { minion: 9_000 })
        .expect("选定目标应成功");

    let target = state.players[0]
        .board
        .iter()
        .find(|minion| minion.instance_id == 9_000)
        .expect("目标随从仍在棋盘");
    assert_eq!(target.stats.attack.growth, 1, "战吼成长命中目标");
    assert_eq!(state.players[0].board.len(), 2, "打出的随从进入棋盘");
    assert!(state.players[0].hand.is_empty());
}

#[test]
fn play_battlecry_with_empty_pool_is_allowed() {
    let (mut engine, mut state) = turn_started(43);
    // 目标筛「机械」，己方只有学者与即将入场的学者，池为空。
    put_champion(&engine, &mut state, 0, 9_000, "gutenberg");
    state.players[0]
        .hand
        .push(HandCard::effect(9_000, BATTLECRY_TRIBE_TARGET));

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
        .expect("池为空时按落空处理，不拒绝");

    assert_eq!(state.players[0].board.len(), 2);
}

#[test]
fn play_effect_card_without_target_rejects_and_keeps_hand() {
    let (mut engine, mut state) = turn_started(44);
    put_champion(&engine, &mut state, 0, 9_000, "adam-smith");
    state.players[0]
        .hand
        .push(HandCard::effect(9_000, EFFECT_CARD_TARGET));
    let before = state_hash(&state);

    let result = engine.apply(
        &mut state,
        0,
        Intent::PlayCard {
            hand_index: 0,
            position: None,
            target: None,
        },
    );

    assert_eq!(result.unwrap_err().code, RejectCode::NoValidTarget);
    assert_eq!(before, state_hash(&state), "拒绝后手牌与状态不变");
}

#[test]
fn play_effect_card_with_empty_pool_is_allowed_and_whiffs() {
    let (mut engine, mut state) = turn_started(45);
    state.players[0]
        .hand
        .push(HandCard::effect(9_000, EFFECT_CARD_TARGET));

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
        .expect("池为空时允许打出，效果落空");

    assert!(state.players[0].hand.is_empty(), "效果牌已离开手牌");
}

#[test]
fn hero_power_without_target_rejects_and_keeps_uses() {
    let (mut engine, mut state) = turn_started(46);
    // 亚当·斯密：condition 要求己方至少 3 个人类，池非空才进入目标校验。
    put_champion(&engine, &mut state, 0, 9_000, "chengbang-gongmin");
    put_champion(&engine, &mut state, 0, 9_001, "dongxuehuajia");
    put_champion(&engine, &mut state, 0, 9_002, "mengmalieren");
    let before = state_hash(&state);

    let result = engine.apply(
        &mut state,
        0,
        Intent::UseHeroPower {
            skill_id: "division-of-labor".to_string(),
            target: None,
        },
    );

    assert_eq!(result.unwrap_err().code, RejectCode::NoValidTarget);
    assert_eq!(before, state_hash(&state), "拒绝后金币与技能次数不变");
}

#[test]
fn hero_power_with_empty_pool_rejects() {
    // 卡牌文档：场上没有合法目标时技能不可用（返回 NO_VALID_TARGET）。
    let (mut engine, mut state) = turn_started(47);
    state.players[0].hero = Some("luxun".to_string());
    let before = state_hash(&state);

    let result = engine.apply(
        &mut state,
        0,
        Intent::UseHeroPower {
            skill_id: "shout".to_string(),
            target: None,
        },
    );

    assert_eq!(result.unwrap_err().code, RejectCode::NoValidTarget);
    assert_eq!(before, state_hash(&state), "拒绝后金币与技能次数不变");
}

#[test]
fn play_minion_at_len_appends() {
    let (mut engine, mut state) = turn_started(48);
    let card_id = minion_id(1);
    state.players[0].board = vec![
        minion(&engine.catalog, 1, "a"),
        minion(&engine.catalog, 2, "b"),
    ];
    let new_instance = 9_000;
    state.players[0].hand.push(HandCard::Minion(minion(
        &engine.catalog,
        new_instance,
        &card_id,
    )));

    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: Some(2),
                target: None,
            },
        )
        .expect("落点等于随从数应追加到末尾");

    let order: Vec<InstanceId> = state.players[0]
        .board
        .iter()
        .map(|m| m.instance_id)
        .collect();
    assert_eq!(order, vec![1, 2, new_instance]);
    assert_eq!(state.players[0].board[2].card_id, card_id);
}

#[test]
fn play_minion_at_position_inserts() {
    let (mut engine, mut state) = turn_started(49);
    let card_id = minion_id(1);
    state.players[0].board = vec![
        minion(&engine.catalog, 1, "a"),
        minion(&engine.catalog, 2, "b"),
    ];
    let new_instance = 9_000;
    state.players[0].hand.push(HandCard::Minion(minion(
        &engine.catalog,
        new_instance,
        &card_id,
    )));

    engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: Some(1),
                target: None,
            },
        )
        .expect("落点 1 应插入中间");

    let order: Vec<InstanceId> = state.players[0]
        .board
        .iter()
        .map(|m| m.instance_id)
        .collect();
    assert_eq!(order, vec![1, new_instance, 2], "原下标及其右侧依次右移");
}

#[test]
fn play_minion_position_beyond_len_rejects() {
    let (mut engine, mut state) = turn_started(50);
    state.players[0].board = vec![
        minion(&engine.catalog, 1, "a"),
        minion(&engine.catalog, 2, "b"),
    ];
    state.players[0]
        .hand
        .push(HandCard::effect(9_000, &minion_id(1)));
    let before = state_hash(&state);

    let result = engine.apply(
        &mut state,
        0,
        Intent::PlayCard {
            hand_index: 0,
            position: Some(5),
            target: None,
        },
    );

    let reject = result.unwrap_err();
    assert_eq!(reject.code, RejectCode::InvalidChoice);
    assert_eq!(reject.message, "落点 5 超出当前随从数 2");
    assert_eq!(before, state_hash(&state));
}

#[test]
fn play_minion_without_position_appends() {
    let (mut engine, mut state) = turn_started(51);
    state.players[0].board = vec![
        minion(&engine.catalog, 1, "a"),
        minion(&engine.catalog, 2, "b"),
    ];
    let new_instance = 9_000;
    state.players[0].hand.push(HandCard::Minion(minion(
        &engine.catalog,
        new_instance,
        &minion_id(1),
    )));

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
        .expect("缺省落点应追加");

    let order: Vec<InstanceId> = state.players[0]
        .board
        .iter()
        .map(|m| m.instance_id)
        .collect();
    assert_eq!(order, vec![1, 2, new_instance]);
}

// ---- 战吼目标待选：先入场再选目标 ----

/// 战吼「使一个友方随从获得成长」的随从，入场后需指定目标。
const BATTLECRY_GROWTH_MINION: &str = "fangzhiji";

fn play_growth_minion(engine: &mut Engine, state: &mut GameState) -> InstanceId {
    let instance_id = 9_000;
    state.players[0].hand.push(HandCard::Minion(minion(
        &engine.catalog,
        instance_id,
        BATTLECRY_GROWTH_MINION,
    )));
    engine
        .apply(
            state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect("打出带目标战吼的随从应成功");
    instance_id
}

fn target_pending(state: &GameState, seat: Seat) -> Option<(InstanceId, String, bool)> {
    state.pending.iter().find_map(|pending| match pending {
        PendingChoice::Target {
            seat: pending_seat,
            source,
            card_id,
            golden,
        } if *pending_seat == seat => Some((*source, card_id.clone(), *golden)),
        _ => None,
    })
}

#[test]
fn target_pending_created_after_minion_plays() {
    let (mut engine, mut state) = turn_started(52);
    let instance_id = play_growth_minion(&mut engine, &mut state);

    assert_eq!(state.players[0].board.len(), 1, "随从先入场");
    assert_eq!(
        target_pending(&state, 0),
        Some((instance_id, BATTLECRY_GROWTH_MINION.to_string(), false)),
        "入场后推入目标待选"
    );
    assert_eq!(
        state.players[0].board[0].stats.attack.growth, 0,
        "战吼尚未结算"
    );
}

#[test]
fn target_pending_pool_includes_self() {
    let (mut engine, mut state) = turn_started(53);
    let instance_id = play_growth_minion(&mut engine, &mut state);

    engine
        .apply(
            &mut state,
            0,
            Intent::ChooseTarget {
                minion: instance_id,
            },
        )
        .expect("选定刚入场的随从应成功");

    assert!(state.pending.is_empty(), "待选被消费");
    assert_eq!(
        state.players[0].board[0].stats.attack.growth, 1,
        "战吼命中自身"
    );
}

#[test]
fn target_pending_skipped_when_filtered_pool_empty() {
    let (mut engine, mut state) = turn_started(54);
    // 战吼筛「机械」，己方棋盘没有机械，入场后池仍为空。
    state.players[0].hand.push(HandCard::Minion(minion(
        &engine.catalog,
        9_000,
        "archimedes",
    )));

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
        .expect("池为空时按落空处理");

    assert!(target_pending(&state, 0).is_none(), "不产生待选");
    assert_eq!(state.players[0].board.len(), 1);
    assert_eq!(state.players[0].board[0].stats.attack.growth, 0, "战吼落空");
}

#[test]
fn target_pending_blocks_recruit_intents() {
    let (mut engine, mut state) = turn_started(55);
    set_shop(&mut state, 0, &[Some(minion_id(1))]);
    let instance_id = play_growth_minion(&mut engine, &mut state);
    state.players[0].gold = 10;

    let intents = vec![
        Intent::BuyMinion { slot: 0 },
        Intent::RerollShop,
        Intent::UpgradeEra,
        Intent::EndRecruit,
        Intent::SellMinion {
            minion: instance_id,
        },
        Intent::RearrangeBoard { from: 0, to: 0 },
    ];
    for intent in intents {
        let before = state_hash(&state);
        let label = format!("{intent:?}");
        let reject = engine
            .apply(&mut state, 0, intent)
            .expect_err("待选期间招募意图必须被拒");
        assert_eq!(reject.code, RejectCode::InvalidStage, "{label}");
        assert_eq!(before, state_hash(&state), "{label}");
    }

    let before = state_hash(&state);
    let reject = engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
                position: None,
                target: None,
            },
        )
        .expect_err("待选期间不能继续出牌");
    assert_eq!(reject.code, RejectCode::InvalidStage);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn target_pending_rejects_choose_discover() {
    let (mut engine, mut state) = turn_started(56);
    play_growth_minion(&mut engine, &mut state);

    let reject = engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect_err("头部是目标待选时不能选择发现");

    assert_eq!(reject.code, RejectCode::InvalidChoice);
    assert!(target_pending(&state, 0).is_some(), "待选仍在");
}

#[test]
fn choose_target_outside_board_rejects() {
    let (mut engine, mut state) = turn_started(57);
    play_growth_minion(&mut engine, &mut state);
    let before = state_hash(&state);

    let reject = engine
        .apply(&mut state, 0, Intent::ChooseTarget { minion: 9_999 })
        .expect_err("目标不在棋盘上必须拒绝");

    assert_eq!(reject.code, RejectCode::NoValidTarget);
    assert_eq!(before, state_hash(&state), "拒绝后状态不变");
    assert!(target_pending(&state, 0).is_some(), "待选仍在");
}

#[test]
fn choose_target_outside_filtered_pool_rejects() {
    // 战吼筛「机械」：棋盘上有机械（因此产生待选），但选一个学者必须被拒，
    // 而不是接受后静默落空（core-rules §4：指定随从不在合法池时返回该码）。
    let (mut engine, mut state) = turn_started(59);
    put_champion(&engine, &mut state, 0, 9_000, "fangzhiji");
    put_champion(&engine, &mut state, 0, 9_001, "gutenberg");
    state.players[0]
        .hand
        .push(HandCard::effect(9_000, BATTLECRY_TRIBE_TARGET));

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
        .expect("机械战吼入场后应产生目标待选");
    assert!(target_pending(&state, 0).is_some(), "池内有机械时产生待选");

    let before = state_hash(&state);
    let reject = engine
        .apply(&mut state, 0, Intent::ChooseTarget { minion: 9_001 })
        .expect_err("目标不在合法池内必须拒绝");

    assert_eq!(reject.code, RejectCode::NoValidTarget);
    assert_eq!(before, state_hash(&state), "拒绝后状态不变");
    assert!(target_pending(&state, 0).is_some(), "待选仍在");
}

#[test]
fn target_pending_cleared_on_finish_recruit() {
    let (mut engine, mut state) = turn_started(58);
    play_growth_minion(&mut engine, &mut state);
    state.pending.push_back(PendingChoice::Discover {
        seat: 0,
        candidates: vec![minion_id(1)],
        source: 0,
    });

    engine.finish_recruit(&mut state).expect("结束招募应成功");

    assert!(target_pending(&state, 0).is_none(), "目标待选被清空");
    assert_eq!(state.pending.len(), 1, "发现待选保留");
    assert_eq!(state.phase, Phase::Combat);
}
