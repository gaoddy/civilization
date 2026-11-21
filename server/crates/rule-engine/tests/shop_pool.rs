//! 商店抽取占用共享牌池：抽取即扣池、刷新归还、冻结占用与出局归还。

mod common;

use common::{minion, minion_id, setup_with_gold};
use rule_engine::data::{CardType, card_in_pool};
use rule_engine::effects::EffectCtx;
use rule_engine::engine::Engine;
use rule_engine::intent::Intent;
use rule_engine::state::{GameState, HandCard, Seat};

/// 只保留一个 1 星随从可抽，并把它的公共池份数设为 `copies`。
///
/// 移除全部效果牌、清空其它随从份数、把各时代商店权重固定为 1 星，
/// 使商店能抽到的随从只有这一张，便于断言份数守恒。
fn prepare_single_minion(engine: &mut Engine, state: &mut GameState, copies: u16) -> String {
    engine
        .catalog
        .cards
        .retain(|_, card| card.card_type != CardType::Effect);
    let card_id = engine
        .catalog
        .cards
        .values()
        .find(|card| {
            card.card_type == CardType::Minion
                && card.in_shop_pool
                && card.level == 1
                && card_in_pool(card, &state.enabled_playstyles)
        })
        .map(|card| card.id.clone())
        .expect("应存在启用的 1 星随从");
    for row in &mut engine.catalog.config.shop_weights {
        *row = [1, 0, 0, 0, 0, 0, 0];
    }
    for count in state.pool.values_mut() {
        *count = 0;
    }
    state.pool.insert(card_id.clone(), copies);
    card_id
}

/// 所有玩家商店中指定卡的总数。
fn in_shops(state: &GameState, card_id: &str) -> u16 {
    state
        .players
        .iter()
        .flat_map(|player| player.shop.iter().flatten())
        .filter(|slot| slot.card_id == card_id)
        .count() as u16
}

/// 找到指定座位商店中第一张指定卡的下标。
fn shop_slot_of(state: &GameState, seat: Seat, card_id: &str) -> Option<u8> {
    state.players[usize::from(seat)]
        .shop
        .iter()
        .position(|slot| slot.as_ref().is_some_and(|slot| slot.card_id == card_id))
        .and_then(|index| u8::try_from(index).ok())
}

#[test]
fn shop_draw_reserves_copy_from_pool() {
    let (mut engine, mut state) = setup_with_gold(1);
    let card_id = prepare_single_minion(&mut engine, &mut state, 1);

    engine.roll_shop(&mut state, 0);
    assert_eq!(
        state.pool.get(&card_id).copied().unwrap_or(0),
        0,
        "抽取即扣池"
    );
    assert_eq!(in_shops(&state, &card_id), 1);

    engine.roll_shop(&mut state, 1);
    assert_eq!(
        in_shops(&state, &card_id),
        1,
        "唯一的份不能同时出现在两名玩家的商店"
    );
}

#[test]
fn reroll_returns_unpicked_copies_before_drawing() {
    let (mut engine, mut state) = setup_with_gold(2);
    let card_id = prepare_single_minion(&mut engine, &mut state, 2);

    engine.roll_shop(&mut state, 0);
    assert_eq!(state.pool.get(&card_id).copied().unwrap_or(0), 0);
    assert_eq!(in_shops(&state, &card_id), 2);

    engine.roll_shop(&mut state, 0);
    assert_eq!(
        state.pool.get(&card_id).copied().unwrap_or(0) + in_shops(&state, &card_id),
        2,
        "刷新后份数仍在池与商店之间守恒"
    );
}

#[test]
fn frozen_slot_keeps_reservation_across_turn_start() {
    let (mut engine, mut state) = setup_with_gold(3);
    let card_id = prepare_single_minion(&mut engine, &mut state, 1);

    engine.roll_shop(&mut state, 0);
    let slot = shop_slot_of(&state, 0, &card_id).expect("座位 0 应刷到该卡");
    state.players[0].shop[usize::from(slot)]
        .as_mut()
        .expect("槽位非空")
        .frozen = true;

    engine.start_turn(&mut state).expect("开始回合应成功");
    assert_eq!(
        state.players[0].shop[usize::from(slot)]
            .as_ref()
            .map(|slot| slot.card_id.as_str()),
        Some(card_id.as_str()),
        "冻结槽位跨回合保留"
    );
    assert_eq!(
        state.pool.get(&card_id).copied().unwrap_or(0),
        0,
        "冻结牌继续占用份数"
    );
    assert_eq!(in_shops(&state, &card_id), 1);
}

#[test]
fn buy_does_not_deduct_pool_twice() {
    let (mut engine, mut state) = setup_with_gold(4);
    let card_id = prepare_single_minion(&mut engine, &mut state, 2);
    // 只留 1 个槽位：抽走 1 份后池里还剩 1 份，购买不能再扣。
    engine.catalog.config.shop_slots = 1;

    engine.roll_shop(&mut state, 0);
    let slot = shop_slot_of(&state, 0, &card_id).expect("座位 0 应刷到该卡");
    assert_eq!(state.pool.get(&card_id).copied().unwrap_or(0), 1);

    engine
        .apply(&mut state, 0, Intent::BuyMinion { slot })
        .expect("购买应成功");

    assert_eq!(
        state.pool.get(&card_id).copied().unwrap_or(0),
        1,
        "购买不再二次扣池"
    );
    assert_eq!(state.players[0].hand.len(), 1);
    assert_eq!(state.players[0].hand[0].card_id(), card_id);
}

#[test]
fn eliminated_player_shop_returns_copies_to_pool() {
    let (mut engine, mut state) = setup_with_gold(5);
    let card_id = prepare_single_minion(&mut engine, &mut state, 1);

    engine.roll_shop(&mut state, 0);
    assert_eq!(in_shops(&state, &card_id), 1);

    state.players[0].hp = 0;
    rule_engine::settle::forfeit(&mut state, 0);

    assert!(
        state.players[0].shop.iter().all(Option::is_none),
        "出局后商店清空"
    );
    assert_eq!(
        state.pool.get(&card_id).copied().unwrap_or(0),
        1,
        "未购买的份归还公共池"
    );
}

#[test]
fn sell_returns_only_charged_pool_copies() {
    let (engine, mut state) = setup_with_gold(6);
    let card_id = minion_id(1);
    let before_pool = state.pool.get(&card_id).copied().unwrap_or(0);

    let mut charged = minion(&engine.catalog, 100, &card_id);
    charged.pool_copies = 1;
    let uncharged = minion(&engine.catalog, 101, &card_id);
    state.players[0].board = vec![charged, uncharged];

    engine
        .apply(&mut state, 0, Intent::SellMinion { minion: 100 })
        .expect("出售应成功");
    assert_eq!(
        state.pool.get(&card_id).copied().unwrap_or(0),
        before_pool + 1,
        "扣过池的随从归还 1 份"
    );

    engine
        .apply(&mut state, 0, Intent::SellMinion { minion: 101 })
        .expect("出售应成功");
    assert_eq!(
        state.pool.get(&card_id).copied().unwrap_or(0),
        before_pool + 1,
        "未扣池的副本不归还份数"
    );
}

/// 把 3 张手牌随从合成金色并把金色移到棋盘，返回金色实例。
fn form_and_board_golden(
    engine: &Engine,
    state: &mut GameState,
    card_id: &str,
    pool_copies: [u8; 3],
) -> u64 {
    let hand = pool_copies
        .iter()
        .enumerate()
        .map(|(index, copies)| {
            let mut minion = minion(&engine.catalog, 200 + index as u64, card_id);
            minion.pool_copies = *copies;
            HandCard::Minion(minion)
        })
        .collect();
    state.players[0].hand = hand;

    engine.check_triple(state, 0);
    let golden = state.players[0]
        .hand
        .iter()
        .find_map(|card| match card {
            HandCard::Minion(minion) if minion.golden => Some(minion.clone()),
            _ => None,
        })
        .expect("应合成金色");
    state.players[0].hand.clear();
    state.players[0].board = vec![golden.clone()];
    golden.pool_copies as u64
}

#[test]
fn golden_keeps_charged_copies_of_its_materials() {
    let (engine, mut state) = setup_with_gold(7);
    let card_id = minion_id(1);

    let charged = form_and_board_golden(&engine, &mut state, &card_id, [1, 1, 1]);
    assert_eq!(charged, 3, "三张扣池材料合计 3 份");

    let mixed = form_and_board_golden(&engine, &mut state, &card_id, [1, 1, 0]);
    assert_eq!(mixed, 2, "两张扣池材料合计 2 份");

    let uncharged = form_and_board_golden(&engine, &mut state, &card_id, [0, 0, 0]);
    assert_eq!(uncharged, 0, "未扣池材料合计 0 份");
}

#[test]
fn selling_golden_returns_its_charged_copies() {
    let (engine, mut state) = setup_with_gold(8);
    let card_id = minion_id(1);
    let before_pool = state.pool.get(&card_id).copied().unwrap_or(0);
    state.players[0].gold = 0;

    form_and_board_golden(&engine, &mut state, &card_id, [1, 1, 0]);
    let instance_id = state.players[0].board[0].instance_id;

    engine
        .apply(
            &mut state,
            0,
            Intent::SellMinion {
                minion: instance_id,
            },
        )
        .expect("出售金色应成功");

    assert_eq!(state.players[0].gold, 3, "金色出售返 3 金");
    assert_eq!(
        state.pool.get(&card_id).copied().unwrap_or(0),
        before_pool + 2,
        "归还合成材料的实际扣池份数"
    );
}

#[test]
fn gain_minion_from_shop_does_not_deduct_pool_twice() {
    let (mut engine, mut state) = setup_with_gold(9);
    let card_id = prepare_single_minion(&mut engine, &mut state, 2);
    engine.catalog.config.shop_slots = 1;
    engine.roll_shop(&mut state, 0);
    assert_eq!(state.pool.get(&card_id).copied().unwrap_or(0), 1);

    let effects = common::effects_of("OnActivate|GainMinion|self|from=shop,level=1,to=hand");
    let effect = &effects[0];
    let ctx = EffectCtx {
        seat: 0,
        source: None,
        target: None,
        event_target: None,
        golden: false,
        level_override: None,
    };
    engine.apply_op(&mut state, effect, &ctx, None);

    assert_eq!(
        state.pool.get(&card_id).copied().unwrap_or(0),
        1,
        "从商店取得不再二次扣池"
    );
    assert_eq!(state.players[0].hand.len(), 1);
    assert_eq!(state.players[0].hand[0].minion().unwrap().pool_copies, 1);
}

/// 效果执行上下文：无来源、无目标。
fn ctx(seat: Seat) -> EffectCtx {
    EffectCtx {
        seat,
        source: None,
        target: None,
        event_target: None,
        golden: false,
        level_override: None,
    }
}

#[test]
fn discover_candidates_exclude_shop_reserved_copies() {
    let (mut engine, mut state) = setup_with_gold(10);
    let card_id = prepare_single_minion(&mut engine, &mut state, 1);
    engine.roll_shop(&mut state, 1);
    assert_eq!(in_shops(&state, &card_id), 1);
    assert_eq!(state.pool.get(&card_id).copied().unwrap_or(0), 0);

    let effects = common::effects_of("OnActivate|Discover|self|pool=playstyle,choose=1");
    engine.apply_op(&mut state, &effects[0], &ctx(0), None);

    assert!(
        state.pending.is_empty(),
        "被商店占用的唯一份不能作为发现候选"
    );
}

#[test]
fn chosen_discover_records_charged_copy() {
    let (mut engine, mut state) = setup_with_gold(11);
    let card_id = prepare_single_minion(&mut engine, &mut state, 2);
    let effects = common::effects_of("OnActivate|Discover|self|pool=playstyle,choose=1");
    engine.apply_op(&mut state, &effects[0], &ctx(0), None);
    assert!(!state.pending.is_empty(), "池中仍有份，应登记候选");
    let before_pool = state.pool.get(&card_id).copied().unwrap_or(0);

    engine
        .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
        .expect("选择发现应成功");

    assert_eq!(
        state.pool.get(&card_id).copied().unwrap_or(0),
        before_pool - 1,
        "选择发现扣 1 份"
    );
    assert_eq!(
        state.players[0].hand[0].minion().unwrap().pool_copies,
        1,
        "发现得到的随从出售时归还 1 份"
    );
}

#[test]
fn summon_card_with_empty_pool_does_not_summon() {
    let (engine, mut state) = setup_with_gold(12);
    let card_id = minion_id(1);
    state.pool.insert(card_id.clone(), 0);
    let effects = common::effects_of(&format!("OnActivate|Summon|self|card={card_id},count=1"));

    engine.apply_op(&mut state, &effects[0], &ctx(0), None);
    assert!(state.players[0].board.is_empty(), "池中无份时不召唤");

    state.pool.insert(card_id, 1);
    engine.apply_op(&mut state, &effects[0], &ctx(0), None);
    assert_eq!(state.players[0].board.len(), 1);
    assert_eq!(state.players[0].board[0].pool_copies, 1);
    assert_eq!(state.pool.get(&minion_id(1)).copied().unwrap_or(0), 0);
}

#[test]
fn all_shops_never_exceed_available_copies() {
    let (mut engine, mut state) = setup_with_gold(15);
    let card_id = prepare_single_minion(&mut engine, &mut state, 3);

    for seat in 0..state.players.len() as Seat {
        engine.roll_shop(&mut state, seat);
    }

    assert_eq!(
        in_shops(&state, &card_id),
        3,
        "八名玩家全部刷新后，商店中的总份数不超过公共池份数"
    );
    assert_eq!(state.pool.get(&card_id).copied().unwrap_or(0), 0);
}

#[test]
fn shops_only_show_remaining_copies() {
    let (mut engine, mut state) = setup_with_gold(16);
    let card_id = prepare_single_minion(&mut engine, &mut state, 2);
    // 每人只开 1 个槽位，便于观察份数在玩家之间分配。
    engine.catalog.config.shop_slots = 1;

    engine.roll_shop(&mut state, 0);
    assert_eq!(in_shops(&state, &card_id), 1);
    assert_eq!(state.pool.get(&card_id).copied().unwrap_or(0), 1);

    engine.roll_shop(&mut state, 1);
    assert_eq!(
        in_shops(&state, &card_id),
        2,
        "剩余份只能再进入一名玩家的商店"
    );
    assert_eq!(state.pool.get(&card_id).copied().unwrap_or(0), 0);

    engine.roll_shop(&mut state, 2);
    assert_eq!(
        in_shops(&state, &card_id),
        2,
        "份数用完后其他玩家的商店不再出现该卡"
    );
}

#[test]
fn upgrade_era_returns_frozen_slot_copy_before_reroll() {
    let (mut engine, mut state) = setup_with_gold(14);
    let card_id = prepare_single_minion(&mut engine, &mut state, 2);

    engine.roll_shop(&mut state, 0);
    for slot in state.players[0].shop.iter_mut().flatten() {
        slot.frozen = true;
    }
    let frozen_before = in_shops(&state, &card_id);
    assert_eq!(frozen_before, 2, "两份都被刷进商店并冻结");

    engine
        .apply(&mut state, 0, Intent::UpgradeEra)
        .expect("金币足够时升时代应成功");

    assert!(
        state.players[0]
            .shop
            .iter()
            .flatten()
            .all(|slot| !slot.frozen),
        "升时代重抽取消冻结"
    );
    assert_eq!(
        state.pool.get(&card_id).copied().unwrap_or(0) + in_shops(&state, &card_id),
        2,
        "冻结份先归还再重抽，份数守恒"
    );
}

#[test]
fn copied_minion_does_not_return_pool_copies() {
    let (engine, mut state) = setup_with_gold(13);
    let card_id = minion_id(1);
    let before_pool = state.pool.get(&card_id).copied().unwrap_or(0);
    let mut source = minion(&engine.catalog, 300, &card_id);
    source.pool_copies = 1;
    state.players[0].board.push(source);
    let mut source_ctx = ctx(0);
    source_ctx.source = Some(300);

    let effects = common::effects_of("OnActivate|Copy|self|state=current,to=hand");
    engine.apply_op(&mut state, &effects[0], &source_ctx, None);

    assert_eq!(state.players[0].hand.len(), 1, "完整复制来源随从");
    let copied = state.players[0].hand[0].minion().unwrap();
    assert_eq!(copied.pool_copies, 0, "复制品未从公共池扣除份数");
    let copied_id = copied.instance_id;
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
        .expect("复制品应能打出");

    engine
        .apply(&mut state, 0, Intent::SellMinion { minion: copied_id })
        .expect("出售复制品应成功");
    assert_eq!(
        state.pool.get(&card_id).copied().unwrap_or(0),
        before_pool,
        "复制品出售不归还份数"
    );
}

#[test]
fn minion_id_helper_still_works() {
    // 防止 helpers 在重构中失效；同时确保测试目录里有 1 星随从。
    assert!(!minion_id(1).is_empty());
}
