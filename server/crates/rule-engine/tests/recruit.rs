mod common;

use common::{heroes, test_catalog};
use rule_engine::data::{CardType, Era};
use rule_engine::engine::Engine;
use rule_engine::events::GameEvent;
use rule_engine::hash::state_hash;
use rule_engine::intent::{Intent, RejectCode};
use rule_engine::state::{GameState, HandCard, InstanceId, MinionInstance, Phase, Seat, ShopSlot};

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

fn effect_card() -> (String, u8) {
    test_catalog()
        .cards
        .values()
        .find(|card| card.card_type == CardType::Effect)
        .map(|card| (card.id.clone(), card.cost.unwrap_or(3)))
        .expect("数据中应存在效果牌")
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

fn dummy(instance_id: InstanceId, card_id: &str) -> MinionInstance {
    MinionInstance {
        instance_id,
        card_id: card_id.to_string(),
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
        attacks_left: 0,
        cannot_attack: false,
    }
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
        .map(|(index, card_id)| dummy(1_000 + index as InstanceId, card_id))
        .collect();
}

fn fill_hand(state: &mut GameState, seat: Seat, count: usize) {
    let card_id = minion_id(1);
    let player = &mut state.players[usize::from(seat)];
    player.hand = (0..count)
        .map(|_| HandCard {
            card_id: card_id.clone(),
            golden: false,
        })
        .collect();
}

fn pool_count(state: &GameState, card_id: &str) -> u16 {
    state.pool.get(card_id).copied().unwrap_or(0)
}

#[test]
fn buy_deducts_gold_and_pool() {
    let (mut engine, mut state) = setup(1);
    let card_id = minion_id(1);
    set_shop(&mut state, 0, &[Some(card_id.clone())]);
    let before_pool = pool_count(&state, &card_id);

    let outcome = engine
        .apply(
            &mut state,
            0,
            Intent::BuyMinion {
                slot: 0,
                target: None,
            },
        )
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
    assert_eq!(state.players[0].board.len(), 1);
    assert_eq!(state.players[0].shop[0], None);
    let instance = &state.players[0].board[0];
    assert_eq!(instance.card_id, card_id);
    assert_eq!(instance.instance_id, state.next_instance_id - 1);
    assert!(!instance.golden);
    assert_eq!(instance.growth, 0);
    assert_eq!(instance.growth_health_only, 0);
    assert_eq!(instance.breed, 0);
    assert_eq!(instance.contagion, 0);
    assert!(instance.granted.is_empty());
    assert!(instance.mods.is_empty());
    assert!(!instance.shield_used);
    assert!(!instance.reborn_used);
    assert_eq!(instance.attacks_left, 0);
    assert!(!instance.cannot_attack);
}

#[test]
fn buy_effect_card_goes_to_hand_and_costs_card_cost() {
    let (mut engine, mut state) = setup(2);
    let (card_id, cost) = effect_card();
    set_shop(&mut state, 0, &[Some(card_id.clone())]);

    let outcome = engine
        .apply(
            &mut state,
            0,
            Intent::BuyMinion {
                slot: 0,
                target: None,
            },
        )
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
    assert_eq!(state.players[0].hand[0].card_id, card_id);
    assert!(!state.players[0].hand[0].golden);
    assert!(state.players[0].board.is_empty());
}

#[test]
fn buy_without_gold_rejects_without_mutating() {
    let (mut engine, mut state) = setup(3);
    let card_id = minion_id(1);
    set_shop(&mut state, 0, &[Some(card_id)]);
    state.players[0].gold = 0;
    let before = state_hash(&state);

    let result = engine.apply(
        &mut state,
        0,
        Intent::BuyMinion {
            slot: 0,
            target: None,
        },
    );

    assert_eq!(result.unwrap_err().code, RejectCode::NotEnoughGold);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn buy_empty_slot_rejects() {
    let (mut engine, mut state) = setup(4);
    set_shop(&mut state, 0, &[None, None]);
    let before = state_hash(&state);

    let result = engine.apply(
        &mut state,
        0,
        Intent::BuyMinion {
            slot: 0,
            target: None,
        },
    );

    assert_eq!(result.unwrap_err().code, RejectCode::ShopSlotEmpty);
    assert_eq!(before, state_hash(&state));

    let out_of_range = engine.apply(
        &mut state,
        0,
        Intent::BuyMinion {
            slot: 99,
            target: None,
        },
    );
    assert_eq!(out_of_range.unwrap_err().code, RejectCode::ShopSlotEmpty);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn buy_when_board_and_hand_full_rejects() {
    let (mut engine, mut state) = setup(5);
    let card_id = minion_id(1);
    set_shop(&mut state, 0, &[Some(card_id)]);
    fill_board(&mut state, 0, 7);
    fill_hand(&mut state, 0, 10);
    let before = state_hash(&state);

    let result = engine.apply(
        &mut state,
        0,
        Intent::BuyMinion {
            slot: 0,
            target: None,
        },
    );

    assert_eq!(result.unwrap_err().code, RejectCode::NoSpace);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn buy_routes_to_hand_when_board_is_full() {
    let (mut engine, mut state) = setup(6);
    let card_id = minion_id(1);
    set_shop(&mut state, 0, &[Some(card_id.clone())]);
    fill_board(&mut state, 0, 7);

    engine
        .apply(
            &mut state,
            0,
            Intent::BuyMinion {
                slot: 0,
                target: None,
            },
        )
        .expect("棋盘满时应入手牌");

    assert_eq!(state.players[0].board.len(), 7);
    assert_eq!(state.players[0].hand.len(), 1);
    assert_eq!(state.players[0].hand[0].card_id, card_id);
}

#[test]
fn sell_returns_gold_and_pool() {
    let (mut engine, mut state) = setup(7);
    let card_id = minion_id(1);
    state.players[0].board.push(dummy(42, &card_id));
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
    let (mut engine, mut state) = setup(8);
    let card_id = minion_id(2);
    let before_pool = pool_count(&state, &card_id);
    let mut golden = dummy(43, &card_id);
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
    let (mut engine, mut state) = setup(9);
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::SellMinion { minion: 777 });

    assert_eq!(result.unwrap_err().code, RejectCode::NoSuchMinion);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn reroll_first_is_free_then_costs_one() {
    let (mut engine, mut state) = setup(10);
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
    let (mut engine, mut state) = setup(11);
    state.players[0].gold = 0;
    state.players[0].first_refresh_used = true;
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::RerollShop);

    assert_eq!(result.unwrap_err().code, RejectCode::NotEnoughGold);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn upgrade_era_costs_and_rerolls() {
    let (mut engine, mut state) = setup(12);
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
    let (mut engine, mut state) = setup(13);
    state.players[0].gold = 4;
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::UpgradeEra);

    assert_eq!(result.unwrap_err().code, RejectCode::NotEnoughGold);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn upgrade_at_future_rejects() {
    let (mut engine, mut state) = setup(14);
    state.players[0].era = Era::Future;
    state.players[0].gold = 200;
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::UpgradeEra);

    assert_eq!(result.unwrap_err().code, RejectCode::AlreadyMaxEra);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn play_minion_from_hand_moves_to_board() {
    let (mut engine, mut state) = setup(15);
    let card_id = minion_id(1);
    state.players[0].hand.push(HandCard {
        card_id: card_id.clone(),
        golden: false,
    });
    let before_pool = pool_count(&state, &card_id);

    let outcome = engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
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
    let (mut engine, mut state) = setup(16);
    let (card_id, _) = effect_card();
    state.players[0].hand.push(HandCard {
        card_id,
        golden: false,
    });

    let outcome = engine
        .apply(
            &mut state,
            0,
            Intent::PlayCard {
                hand_index: 0,
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
    let (mut engine, mut state) = setup(17);
    let before = state_hash(&state);

    let result = engine.apply(
        &mut state,
        0,
        Intent::PlayCard {
            hand_index: 3,
            target: None,
        },
    );

    assert_eq!(result.unwrap_err().code, RejectCode::CardNotInHand);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn play_minion_with_full_board_rejects() {
    let (mut engine, mut state) = setup(18);
    fill_board(&mut state, 0, 7);
    let card_id = minion_id(1);
    state.players[0].hand.push(HandCard {
        card_id,
        golden: false,
    });
    let before = state_hash(&state);

    let result = engine.apply(
        &mut state,
        0,
        Intent::PlayCard {
            hand_index: 0,
            target: None,
        },
    );

    assert_eq!(result.unwrap_err().code, RejectCode::NoSpace);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn rearrange_moves_minion_to_target_index() {
    let (mut engine, mut state) = setup(19);
    state.players[0].board = vec![dummy(1, "a"), dummy(2, "b"), dummy(3, "c")];

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
    let (mut engine, mut state) = setup(20);
    state.players[0].board = vec![dummy(1, "a"), dummy(2, "b")];
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
    let (mut engine, mut state) = setup(21);
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
    let (mut engine, mut state) = setup(22);
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
    let (mut engine, mut state) = setup(23);
    set_shop(&mut state, 0, &[None]);
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::FreezeShop { slot: Some(0) });

    assert_eq!(result.unwrap_err().code, RejectCode::ShopSlotEmpty);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn use_hero_power_respects_cost_and_uses() {
    let (mut engine, mut state) = setup(24);
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
    let (mut engine, mut state) = setup(25);

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
    let (mut engine, mut state) = setup(26);
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
    let (mut engine, mut state) = setup(27);
    state.phase = Phase::Combat;
    let before = state_hash(&state);

    for intent in [
        Intent::RerollShop,
        Intent::BuyMinion {
            slot: 0,
            target: None,
        },
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
    let (mut engine, mut state) = setup(28);

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
    let (mut engine, mut state) = setup(33);
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
    let (mut engine, mut state) = setup(34);
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
    let (mut engine, mut state) = setup(29);
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
fn hero_cards_never_appear_in_shop() {
    let (mut engine, mut state) = setup(30);
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
    let (mut engine, mut state) = setup(31);
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
    let (mut engine, mut state) = setup(32);
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
    let (mut engine, mut state) = setup(33);
    let before = state_hash(&state);

    let result = engine.apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 });

    assert_eq!(result.unwrap_err().code, RejectCode::InvalidStage);
    assert_eq!(before, state_hash(&state));
}

#[test]
fn choose_discover_out_of_range_rejects_invalid_choice() {
    let (mut engine, mut state) = setup(34);
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
    let (mut engine, mut state) = setup(35);
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
    assert_eq!(state.players[0].hand[0].card_id, chosen);
    assert!(!state.players[0].hand[0].golden);
    assert!(state.pending.is_empty(), "消费后待选被移除");
    assert_eq!(pool_count(&state, &chosen), before_pool - 1);
}

#[test]
fn buy_ignores_target_instance_id() {
    let (mut engine, mut state) = setup(36);
    let card_id = minion_id(1);
    set_shop(&mut state, 0, &[Some(card_id.clone())]);

    engine
        .apply(
            &mut state,
            0,
            Intent::BuyMinion {
                slot: 0,
                target: Some(9999),
            },
        )
        .expect("本任务的目标字段应被忽略");

    assert_eq!(state.players[0].board.len(), 1);
    assert_eq!(state.players[0].board[0].card_id, card_id);
    assert_eq!(state.players[0].gold, 0);
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
