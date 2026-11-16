//! 招募阶段：商店刷新、购买、出售、刷新、升时代、打出、站位、冻结与英雄技能。
//!
//! 所有函数都先校验再修改状态，拒绝时 `GameState` 保持逐字节一致。

use std::collections::{BTreeMap, BTreeSet};

use crate::data::{Card, CardType, Catalog, Era, Tribe};
use crate::effects::{EffectCtx, Trigger};
use crate::engine::Engine;
use crate::events::GameEvent;
use crate::intent::{Intent, Outcome, Reject, RejectCode};
use crate::rng::{RngStreams, Stream};
use crate::state::{
    GameState, HandCard, InstanceId, MinionInstance, PendingChoice, Phase, PlayerState, Seat,
    ShopSlot,
};

/// 空星级最多重抽的次数；用尽后槽位留空。
const EMPTY_STAR_RETRIES: u32 = 5;

/// 按意图分派；调用方已用 [`crate::engine::Engine::apply`] 指定座位。
pub(crate) fn apply(
    engine: &mut Engine,
    state: &mut GameState,
    seat: Seat,
    intent: Intent,
) -> Result<Outcome, Reject> {
    if state.phase != Phase::Recruit {
        return Err(Reject::new(
            RejectCode::InvalidStage,
            format!("当前阶段为 {:?}，不接受招募意图", state.phase),
        ));
    }

    // `EndRecruit` 表示「本玩家结束招募」：已结束座位的重复提交由 [`end_recruit`] 幂等成功，
    // 其余招募意图在已结束座位上一律拒绝（见下方守卫）。
    if intent == Intent::EndRecruit {
        return end_recruit(state, seat);
    }
    if recruit_done(state, seat) {
        return Err(Reject::new(
            RejectCode::InvalidStage,
            format!("座位 {seat} 已结束本回合招募"),
        ));
    }

    match intent {
        Intent::BuyMinion { slot } => buy(engine, state, seat, slot),
        Intent::SellMinion { minion } => sell(state, seat, minion),
        Intent::RerollShop => reroll(engine, state, seat),
        Intent::UpgradeEra => upgrade(engine, state, seat),
        Intent::PlayCard { hand_index, target } => {
            play_card(engine, state, seat, hand_index, target)
        }
        Intent::RearrangeBoard { from, to } => rearrange(state, seat, from, to),
        Intent::FreezeShop { slot } => freeze(state, seat, slot),
        Intent::UseHeroPower { skill_id, target } => {
            use_hero_power(engine, state, seat, &skill_id, target)
        }
        Intent::EndRecruit => unreachable!("EndRecruit 在分派前处理"),
        Intent::ChooseDiscover { choice } => choose_discover(engine, state, seat, choice),
    }
}

/// `BuyMinion` 的只读预检：只校验该意图自身的条件，不修改状态、不消耗随机流。
///
/// **不校验** `Recruit` 阶段、`recruit_done` 与 `eliminated`；调用方（视图层）
/// 必须自行门控阶段与玩家状态，不能仅凭返回 `Ok` 认定该意图可操作。
///
/// 买入的牌一律进入手牌，因此只校验手牌是否有空位，与棋盘无关。
pub(crate) fn check_buy(
    engine: &Engine,
    state: &GameState,
    seat: Seat,
    slot: u8,
) -> Result<(), Reject> {
    let card_id = player(state, seat)?
        .shop
        .get(usize::from(slot))
        .and_then(Option::as_ref)
        .map(|shop_slot| shop_slot.card_id.clone())
        .ok_or_else(|| Reject::new(RejectCode::ShopSlotEmpty, format!("商店槽位 {slot} 为空")))?;

    let card = engine.catalog.cards.get(&card_id).ok_or_else(|| {
        Reject::new(
            RejectCode::ShopSlotEmpty,
            format!("商店卡牌 {card_id} 不在目录中"),
        )
    })?;
    if card.is_hero {
        return Err(Reject::new(
            RejectCode::ShopSlotEmpty,
            format!("英雄卡 {card_id} 不能购买"),
        ));
    }

    let card_type = card.card_type;
    let cost = match card_type {
        CardType::Minion => 3,
        CardType::Effect => card.cost.unwrap_or(3),
    };
    let config = &engine.catalog.config;
    let current = player(state, seat)?;
    if current.hand.len() >= config.hand_slots {
        return Err(Reject::new(RejectCode::NoSpace, "手牌已满"));
    }
    current
        .gold
        .checked_sub(cost)
        .ok_or_else(|| Reject::new(RejectCode::NotEnoughGold, format!("需要 {cost} 金币")))?;

    if card_type == CardType::Minion && state.pool.get(&card_id).copied().unwrap_or(0) == 0 {
        return Err(Reject::new(
            RejectCode::ShopSlotEmpty,
            format!("卡牌 {card_id} 的共享池已空"),
        ));
    }
    Ok(())
}

/// 购买：扣金币、清空槽位、从共享池扣 1 张，牌一律进入手牌。
///
/// 买入不入场，因此不触发战吼、不登记进化，也不需要目标与实例编号；
/// 随从要上棋盘须另提交 `PlayCard`。随从买入后仍立即做三连判定（手牌参与三连）。
fn buy(
    engine: &mut Engine,
    state: &mut GameState,
    seat: Seat,
    slot: u8,
) -> Result<Outcome, Reject> {
    check_buy(engine, state, seat, slot)?;

    let card_id = player(state, seat)
        .expect("check_buy 已保证座位存在")
        .shop
        .get(usize::from(slot))
        .and_then(Option::as_ref)
        .expect("check_buy 已保证槽位非空")
        .card_id
        .clone();
    let card = engine
        .catalog
        .cards
        .get(&card_id)
        .expect("check_buy 已保证卡牌在目录中");
    let card_type = card.card_type;
    let cost = match card_type {
        CardType::Minion => 3,
        CardType::Effect => card.cost.unwrap_or(3),
    };
    let current = player(state, seat).expect("check_buy 已保证座位存在");
    let remaining_gold = current
        .gold
        .checked_sub(cost)
        .expect("check_buy 已保证金币足够");

    let player = state
        .players
        .get_mut(usize::from(seat))
        .expect("check_buy 已保证座位存在");
    player.gold = remaining_gold;
    if let Some(shop_slot) = player.shop.get_mut(usize::from(slot)) {
        *shop_slot = None;
    }
    if card_type == CardType::Minion
        && let Some(count) = state.pool.get_mut(&card_id)
    {
        *count = count.saturating_sub(1);
    }
    player.hand.push(HandCard {
        card_id: card_id.clone(),
        golden: false,
    });

    let mut events = vec![GameEvent::MinionBought {
        seat,
        slot,
        card_id,
    }];
    if card_type == CardType::Minion {
        events.extend(engine.check_triple(state, seat));
    }
    Ok(Outcome::with(events))
}

/// `SellMinion` 的只读预检：只校验座位与随从存在，不修改状态、不消耗随机流。
///
/// **不校验** `Recruit` 阶段、`recruit_done`、`eliminated` 与 `target`；调用方（视图层）
/// 必须自行门控阶段与玩家状态，不能仅凭返回 `Ok` 认定该意图可操作。
pub(crate) fn check_sell(state: &GameState, seat: Seat, minion: InstanceId) -> Result<(), Reject> {
    player(state, seat)?
        .board
        .iter()
        .position(|candidate| candidate.instance_id == minion)
        .ok_or_else(|| {
            Reject::new(
                RejectCode::NoSuchMinion,
                format!("棋盘上不存在随从 {minion}"),
            )
        })?;
    Ok(())
}

fn sell(state: &mut GameState, seat: Seat, minion: InstanceId) -> Result<Outcome, Reject> {
    check_sell(state, seat, minion)?;

    let index = player(state, seat)
        .expect("check_sell 已保证座位存在")
        .board
        .iter()
        .position(|candidate| candidate.instance_id == minion)
        .expect("check_sell 已保证随从存在");
    let player = player(state, seat).expect("check_sell 已保证座位存在");
    let card_id = player.board[index].card_id.clone();
    let refund = if player.board[index].golden { 3 } else { 1 };

    let player = state
        .players
        .get_mut(usize::from(seat))
        .expect("check_sell 已保证座位存在");
    player.board.remove(index);
    player.gold = player.gold.saturating_add(refund);
    let count = state.pool.entry(card_id).or_insert(0);
    *count = count.saturating_add(1);

    Ok(Outcome::with(vec![GameEvent::MinionSold { seat, minion }]))
}

/// `RerollShop` 的只读预检：首次刷新免费、之后每次 1 金的费用校验，不修改状态、
/// 不消耗随机流。
///
/// **不校验** `Recruit` 阶段、`recruit_done`、`eliminated` 与 `target`；调用方（视图层）
/// 必须自行门控阶段与玩家状态，不能仅凭返回 `Ok` 认定该意图可操作。
pub(crate) fn check_reroll(state: &GameState, seat: Seat) -> Result<(), Reject> {
    let player = player(state, seat)?;
    let cost = u8::from(player.first_refresh_used);
    player
        .gold
        .checked_sub(cost)
        .ok_or_else(|| Reject::new(RejectCode::NotEnoughGold, "刷新商店需要 1 金币"))?;
    Ok(())
}

pub(crate) fn reroll(
    engine: &mut Engine,
    state: &mut GameState,
    seat: Seat,
) -> Result<Outcome, Reject> {
    check_reroll(state, seat)?;

    let player = player(state, seat).expect("check_reroll 已保证座位存在");
    let cost = u8::from(player.first_refresh_used);

    let player = state
        .players
        .get_mut(usize::from(seat))
        .expect("check_reroll 已保证座位存在");
    if cost == 0 {
        player.first_refresh_used = true;
    } else {
        player.gold -= cost;
    }

    // 手动刷新取消所有冻结并重抽全部槽位。
    roll_player(engine, state, seat, false);

    let mut events = vec![GameEvent::ShopRolled { seat }];
    events.extend(maybe_fire_first_refresh(engine, state, seat));
    Ok(Outcome::with(events))
}

/// 若本回合尚未触发首次刷新，则触发一次并清除标记；否则返回空事件。
///
/// 手动 `RerollShop` 与效果驱动的 `RefreshShop` 共用这一闸门；升时代重抽与
/// 回合开始自动刷新不调用本函数。
pub(crate) fn maybe_fire_first_refresh(
    engine: &mut Engine,
    state: &mut GameState,
    seat: Seat,
) -> Vec<GameEvent> {
    let pending = state
        .players
        .get(usize::from(seat))
        .is_some_and(|player| player.first_refresh_pending);
    if !pending {
        return Vec::new();
    }
    if let Some(player) = state.players.get_mut(usize::from(seat)) {
        player.first_refresh_pending = false;
    }
    fire_first_refresh(engine, state, seat)
}

/// 触发本座位本回合的首次刷新效果：先英雄被动，再棋盘随从按站位从左到右。
///
/// 未携带 `OnFirstRefresh` 效果的卡牌由执行器收集为空；金色随从由执行器触发两次。
pub(crate) fn fire_first_refresh(
    engine: &mut Engine,
    state: &mut GameState,
    seat: Seat,
) -> Vec<GameEvent> {
    let Some(player) = state.players.get(usize::from(seat)) else {
        return Vec::new();
    };
    let hero = player.hero.clone();
    let owners: Vec<(InstanceId, String, bool)> = player
        .board
        .iter()
        .map(|minion| (minion.instance_id, minion.card_id.clone(), minion.golden))
        .collect();

    let mut events = Vec::new();
    if let Some(hero_id) = hero {
        events.extend(engine.fire_card(
            state,
            &hero_id,
            Trigger::OnFirstRefresh,
            EffectCtx {
                seat,
                source: None,
                target: None,
                event_target: None,
                golden: false,
            },
        ));
    }
    for (instance_id, card_id, golden) in owners {
        events.extend(engine.fire_card(
            state,
            &card_id,
            Trigger::OnFirstRefresh,
            EffectCtx {
                seat,
                source: Some(instance_id),
                target: None,
                event_target: None,
                golden,
            },
        ));
    }
    events
}

/// `UpgradeEra` 的只读预检：未到未来时代且金币足够，不修改状态、不消耗随机流。
///
/// **不校验** `Recruit` 阶段、`recruit_done`、`eliminated` 与 `target`；调用方（视图层）
/// 必须自行门控阶段与玩家状态，不能仅凭返回 `Ok` 认定该意图可操作。
pub(crate) fn check_upgrade(engine: &Engine, state: &GameState, seat: Seat) -> Result<(), Reject> {
    let player = player(state, seat)?;
    next_era(player.era).ok_or_else(|| Reject::new(RejectCode::AlreadyMaxEra, "已处于未来时代"))?;
    let cost = engine
        .catalog
        .config
        .era_costs
        .get(era_index(player.era))
        .copied()
        .ok_or_else(|| Reject::new(RejectCode::AlreadyMaxEra, "已处于未来时代"))?;
    let cost = u8::try_from(cost).unwrap_or(u8::MAX);
    player
        .gold
        .checked_sub(cost)
        .ok_or_else(|| Reject::new(RejectCode::NotEnoughGold, format!("升级需要 {cost} 金币")))?;
    Ok(())
}

fn upgrade(engine: &Engine, state: &mut GameState, seat: Seat) -> Result<Outcome, Reject> {
    check_upgrade(engine, state, seat)?;

    let player = player(state, seat).expect("check_upgrade 已保证座位存在");
    let next = next_era(player.era).expect("check_upgrade 已保证可以升级");
    let cost = engine
        .catalog
        .config
        .era_costs
        .get(era_index(player.era))
        .copied()
        .expect("check_upgrade 已保证时代费用存在");
    let cost = u8::try_from(cost).unwrap_or(u8::MAX);
    let remaining_gold = player
        .gold
        .checked_sub(cost)
        .expect("check_upgrade 已保证金币足够");

    let player = state
        .players
        .get_mut(usize::from(seat))
        .expect("check_upgrade 已保证座位存在");
    player.gold = remaining_gold;
    player.era = next;
    player.turns_in_era = 0;

    // 升时代附带的免费重抽取消冻结、也不消耗本回合的免费刷新（core-rules.md:61）。
    roll_player(engine, state, seat, false);
    Ok(Outcome::with(vec![
        GameEvent::EraUpgraded { seat, era: next },
        GameEvent::ShopRolled { seat },
    ]))
}

/// `PlayCard` 的只读预检：手牌下标合法；随从还需要棋盘有空位与可用实例编号。
/// 不修改状态、不消耗随机流。
///
/// **不校验** `Recruit` 阶段、`recruit_done`、`eliminated` 与 `target`；调用方（视图层）
/// 必须自行门控阶段与玩家状态，不能仅凭返回 `Ok` 认定该意图可操作。
///
/// `target` 只影响效果结算，不参与意图能否提交的判定，因此不在参数中。
pub(crate) fn check_play_card(
    engine: &Engine,
    state: &GameState,
    seat: Seat,
    hand_index: u8,
) -> Result<(), Reject> {
    let index = usize::from(hand_index);
    let hand_card = player(state, seat)?
        .hand
        .get(index)
        .cloned()
        .ok_or_else(|| {
            Reject::new(
                RejectCode::CardNotInHand,
                format!("手牌下标 {hand_index} 不存在"),
            )
        })?;
    let card_type = engine
        .catalog
        .cards
        .get(&hand_card.card_id)
        .map(|card| card.card_type)
        .ok_or_else(|| {
            Reject::new(
                RejectCode::CardNotInHand,
                format!("手牌卡牌 {} 不在目录中", hand_card.card_id),
            )
        })?;

    if card_type == CardType::Minion {
        if player(state, seat)?.board.len() >= engine.catalog.config.board_slots {
            return Err(Reject::new(RejectCode::NoSpace, "棋盘已满"));
        }
        state
            .next_instance_id
            .checked_add(1)
            .ok_or_else(|| Reject::new(RejectCode::NoSpace, "随从实例编号已耗尽"))?;
    }
    Ok(())
}

fn play_card(
    engine: &mut Engine,
    state: &mut GameState,
    seat: Seat,
    hand_index: u8,
    target: Option<InstanceId>,
) -> Result<Outcome, Reject> {
    check_play_card(engine, state, seat, hand_index)?;

    let index = usize::from(hand_index);
    let hand_card = player(state, seat)
        .expect("check_play_card 已保证座位存在")
        .hand
        .get(index)
        .cloned()
        .expect("check_play_card 已保证手牌下标存在");
    let card_type = engine
        .catalog
        .cards
        .get(&hand_card.card_id)
        .expect("check_play_card 已保证卡牌在目录中")
        .card_type;

    match card_type {
        CardType::Effect => {
            let ctx = EffectCtx {
                seat,
                source: None,
                target,
                event_target: None,
                golden: false,
            };
            engine.check_targets(
                state,
                &engine.effects_of(&hand_card.card_id, Trigger::OnPlayEffect),
                &ctx,
                false,
            )?;
            engine.check_on_use_targets(state, &ctx)?;

            let player = state
                .players
                .get_mut(usize::from(seat))
                .expect("check_play_card 已保证座位存在");
            player.hand.remove(index);

            let mut events =
                engine.fire_card(state, &hand_card.card_id, Trigger::OnPlayEffect, ctx);
            events.extend(engine.fire(state, Trigger::OnUse, ctx));
            Ok(Outcome::with(events))
        }
        CardType::Minion => {
            let instance_id = state.next_instance_id;
            let ctx = EffectCtx {
                seat,
                source: Some(instance_id),
                target,
                event_target: None,
                golden: hand_card.golden,
            };
            engine.check_targets(
                state,
                &engine.effects_of(&hand_card.card_id, Trigger::OnPlay),
                &ctx,
                false,
            )?;

            let following_instance_id = state
                .next_instance_id
                .checked_add(1)
                .expect("check_play_card 已保证实例编号未耗尽");
            let player = state
                .players
                .get_mut(usize::from(seat))
                .expect("check_play_card 已保证座位存在");
            player.hand.remove(index);
            player.board.push(new_minion(
                &engine.catalog,
                instance_id,
                &hand_card.card_id,
                hand_card.golden,
            ));
            state.next_instance_id = following_instance_id;

            // 先登记进化、再结算战吼（关键词机制 §2.2）。登记了进化的来源等到
            // 玩家选择结算完再触发 OnAllyEvolve；战吼带出的效果牌与响应在本次
            // 意图内同步结算，不跨意图排队。
            engine.register_evolve(state, seat, instance_id);
            let mut events = engine.fire(
                state,
                Trigger::OnPlay,
                EffectCtx {
                    seat,
                    source: Some(instance_id),
                    target,
                    event_target: None,
                    golden: hand_card.golden,
                },
            );
            events.extend(engine.check_triple(state, seat));
            engine.fire_evolve_after_enter(state, seat, instance_id);
            Ok(Outcome::with(events))
        }
    }
}

/// `RearrangeBoard` 的只读预检：两个下标都必须落在棋盘范围内，不修改状态、
/// 不消耗随机流。
///
/// **不校验** `Recruit` 阶段、`recruit_done`、`eliminated` 与 `target`；调用方（视图层）
/// 必须自行门控阶段与玩家状态，不能仅凭返回 `Ok` 认定该意图可操作。
pub(crate) fn check_rearrange(
    state: &GameState,
    seat: Seat,
    from: u8,
    to: u8,
) -> Result<(), Reject> {
    let from = usize::from(from);
    let to = usize::from(to);
    let len = player(state, seat)?.board.len();
    if from >= len || to >= len {
        return Err(Reject::new(
            RejectCode::NoSuchMinion,
            format!("站位下标越界：from={from} to={to} len={len}"),
        ));
    }
    Ok(())
}

fn rearrange(state: &mut GameState, seat: Seat, from: u8, to: u8) -> Result<Outcome, Reject> {
    check_rearrange(state, seat, from, to)?;

    let from = usize::from(from);
    let to = usize::from(to);
    if from == to {
        return Ok(Outcome::empty());
    }

    let player = state
        .players
        .get_mut(usize::from(seat))
        .expect("check_rearrange 已保证座位存在");
    let minion = player.board.remove(from);
    player.board.insert(to, minion);
    Ok(Outcome::empty())
}

/// `FreezeShop` 的只读预检：指定槽位时必须非空；冻结全部槽位无需额外条件。
/// 不修改状态、不消耗随机流。
///
/// **不校验** `Recruit` 阶段、`recruit_done`、`eliminated` 与 `target`；调用方（视图层）
/// 必须自行门控阶段与玩家状态，不能仅凭返回 `Ok` 认定该意图可操作。
pub(crate) fn check_freeze(state: &GameState, seat: Seat, slot: Option<u8>) -> Result<(), Reject> {
    let player = player(state, seat)?;
    if let Some(slot) = slot {
        let exists = player
            .shop
            .get(usize::from(slot))
            .and_then(Option::as_ref)
            .is_some();
        if !exists {
            return Err(Reject::new(
                RejectCode::ShopSlotEmpty,
                format!("商店槽位 {slot} 为空"),
            ));
        }
    }
    Ok(())
}

fn freeze(state: &mut GameState, seat: Seat, slot: Option<u8>) -> Result<Outcome, Reject> {
    check_freeze(state, seat, slot)?;

    match slot {
        Some(slot) => {
            let player = state
                .players
                .get_mut(usize::from(seat))
                .expect("check_freeze 已保证座位存在");
            if let Some(Some(shop_slot)) = player.shop.get_mut(usize::from(slot)) {
                shop_slot.frozen = true;
            }
        }
        None => {
            let player = state
                .players
                .get_mut(usize::from(seat))
                .expect("check_freeze 已保证座位存在");
            for shop_slot in player.shop.iter_mut().flatten() {
                shop_slot.frozen = true;
            }
        }
    }
    Ok(Outcome::empty())
}

/// `UseHeroPower` 的只读预检：技能属于当前英雄、是主动技能、次数未用尽且金币足够。
/// 不修改状态、不消耗随机流。
///
/// **不校验** `Recruit` 阶段、`recruit_done`、`eliminated` 与 `target`；调用方（视图层）
/// 必须自行门控阶段与玩家状态，不能仅凭返回 `Ok` 认定该意图可操作。
///
/// `target` 只影响效果结算，不参与意图能否提交的判定，因此不在参数中。
pub(crate) fn check_hero_power(
    engine: &Engine,
    state: &GameState,
    seat: Seat,
    skill_id: &str,
) -> Result<(), Reject> {
    let player = player(state, seat)?;
    let hero_id = player
        .hero
        .as_deref()
        .ok_or_else(|| Reject::new(RejectCode::HeroPowerLimit, "该玩家没有选择英雄"))?;
    let hero = engine.catalog.cards.get(hero_id).ok_or_else(|| {
        Reject::new(
            RejectCode::HeroPowerLimit,
            format!("英雄卡 {hero_id} 不在目录中"),
        )
    })?;
    if !hero.skills.iter().any(|known| known == skill_id) {
        return Err(Reject::new(
            RejectCode::HeroPowerLimit,
            format!("技能 {skill_id} 不属于英雄 {hero_id}"),
        ));
    }
    let skill = engine.catalog.skills.get(skill_id).ok_or_else(|| {
        Reject::new(
            RejectCode::HeroPowerLimit,
            format!("技能 {skill_id} 不在目录中"),
        )
    })?;
    let cost = skill.cost.ok_or_else(|| {
        Reject::new(
            RejectCode::HeroPowerLimit,
            format!("技能 {skill_id} 是被动技能，不能主动使用"),
        )
    })?;
    let used = player.hero_uses.get(skill_id).copied().unwrap_or(0);
    let limit = skill.uses_per_turn.unwrap_or(1);
    if used >= limit {
        return Err(Reject::new(
            RejectCode::HeroPowerLimit,
            format!("技能 {skill_id} 本回合已用尽（{limit} 次）"),
        ));
    }
    player
        .gold
        .checked_sub(cost)
        .ok_or_else(|| Reject::new(RejectCode::NotEnoughGold, format!("技能需要 {cost} 金币")))?;
    Ok(())
}

fn use_hero_power(
    engine: &mut Engine,
    state: &mut GameState,
    seat: Seat,
    skill_id: &str,
    target: Option<InstanceId>,
) -> Result<Outcome, Reject> {
    check_hero_power(engine, state, seat, skill_id)?;

    let hero_id = player(state, seat)
        .expect("check_hero_power 已保证座位存在")
        .hero
        .clone()
        .expect("check_hero_power 已保证英雄存在");
    let ctx = EffectCtx {
        seat,
        source: None,
        target,
        event_target: None,
        golden: false,
    };
    // 主动技能的效果来自英雄卡：预检口径与 `fire(OnActivate)` 一致。
    engine.check_targets(
        state,
        &engine.effects_of(&hero_id, Trigger::OnActivate),
        &ctx,
        true,
    )?;

    let player = player(state, seat).expect("check_hero_power 已保证座位存在");
    let cost = engine
        .catalog
        .skills
        .get(skill_id)
        .expect("check_hero_power 已保证技能在目录中")
        .cost
        .expect("check_hero_power 已保证技能是主动技能");
    let remaining_gold = player
        .gold
        .checked_sub(cost)
        .expect("check_hero_power 已保证金币足够");

    let player = state
        .players
        .get_mut(usize::from(seat))
        .expect("check_hero_power 已保证座位存在");
    player.gold = remaining_gold;
    let uses = player.hero_uses.entry(skill_id.to_string()).or_insert(0);
    *uses = uses.saturating_add(1);

    let events = engine.fire(
        state,
        Trigger::OnActivate,
        EffectCtx {
            seat,
            source: None,
            target,
            event_target: None,
            golden: false,
        },
    );
    Ok(Outcome::with(events))
}

/// 该座位是否已在本回合提交 `EndRecruit`（不存在的座位视为未结束，由调用方报错）。
fn recruit_done(state: &GameState, seat: Seat) -> bool {
    state
        .players
        .get(usize::from(seat))
        .is_some_and(|player| player.recruit_done)
}

/// `EndRecruit` 的只读预检：只校验座位存在，不修改状态、不消耗随机流。
///
/// **不校验** `Recruit` 阶段、`recruit_done`、`eliminated` 与 `target`；调用方（视图层）
/// 必须自行门控阶段与玩家状态，不能仅凭返回 `Ok` 认定该意图可操作。
///
/// `Recruit` 阶段由 [`apply`] 顶层保证；已结束座位的重复提交是幂等成功，不是拒绝。
pub(crate) fn check_end_recruit(state: &GameState, seat: Seat) -> Result<(), Reject> {
    if state.players.get(usize::from(seat)).is_none() {
        return Err(Reject::new(
            RejectCode::InvalidStage,
            format!("座位 {seat} 不存在"),
        ));
    }
    Ok(())
}

/// 本玩家结束招募：只标记该座位。所有未出局玩家都结束后才进入 `Combat`，
/// 并在该次结果中发出唯一一条 `PhaseChanged { Combat }`；出局座位不计入就绪。
/// 已结束座位的重复提交成功且无副作用（核心规则 §3：`EndRecruit` 前置条件只是「处于
/// `Recruit`」，失败返回码为「—」，重复提交时仍处于 `Recruit`）；不在 `Recruit` 阶段时
/// 由 [`apply`] 顶层检查返回 `InvalidStage`。
fn end_recruit(state: &mut GameState, seat: Seat) -> Result<Outcome, Reject> {
    check_end_recruit(state, seat)?;

    let player = state
        .players
        .get_mut(usize::from(seat))
        .expect("check_end_recruit 已保证座位存在");
    if player.recruit_done {
        return Ok(Outcome::empty());
    }
    player.recruit_done = true;
    let all_done = state
        .players
        .iter()
        .all(|player| player.eliminated || player.recruit_done);
    if !all_done {
        return Ok(Outcome::empty());
    }
    state.phase = Phase::Combat;
    Ok(Outcome::with(vec![GameEvent::PhaseChanged {
        phase: Phase::Combat,
        turn: state.turn,
    }]))
}

/// 该座位在队列中最靠前的待选下标（发现与进化共用）。
fn pending_index(state: &GameState, seat: Seat) -> Option<usize> {
    state.pending.iter().position(|pending| match pending {
        PendingChoice::Discover {
            seat: pending_seat, ..
        }
        | PendingChoice::Evolve {
            seat: pending_seat, ..
        } => *pending_seat == seat,
    })
}

/// `ChooseDiscover` 的只读预检：本座位有待选、下标合法；发现还需要手牌有空位。
/// 不修改状态、不消耗随机流。
///
/// **不校验** `Recruit` 阶段、`recruit_done`、`eliminated` 与 `target`；调用方（视图层）
/// 必须自行门控阶段与玩家状态，不能仅凭返回 `Ok` 认定该意图可操作。
///
/// 越界优先于手牌已满；进化不做手牌检查。
pub(crate) fn check_choose_discover(
    engine: &Engine,
    state: &GameState,
    seat: Seat,
    choice: u8,
) -> Result<(), Reject> {
    let index = pending_index(state, seat).ok_or_else(|| {
        Reject::new(
            RejectCode::InvalidStage,
            format!("座位 {seat} 没有待处理的发现选择"),
        )
    })?;

    match state.pending.get(index) {
        Some(PendingChoice::Discover { candidates, .. }) => {
            candidates.get(usize::from(choice)).ok_or_else(|| {
                Reject::new(
                    RejectCode::InvalidChoice,
                    format!("发现候选下标 {choice} 越界"),
                )
            })?;
            let hand_slots = engine.catalog.config.hand_slots;
            if player(state, seat)?.hand.len() >= hand_slots {
                return Err(Reject::new(RejectCode::NoSpace, "手牌已满"));
            }
        }
        Some(PendingChoice::Evolve { .. }) => {
            crate::effects::EVOLVE_CHOICES
                .get(usize::from(choice))
                .ok_or_else(|| {
                    Reject::new(
                        RejectCode::InvalidChoice,
                        format!("进化选项下标 {choice} 越界"),
                    )
                })?;
        }
        None => {
            return Err(Reject::new(
                RejectCode::InvalidStage,
                format!("座位 {seat} 没有待处理的发现选择"),
            ));
        }
    }
    Ok(())
}

/// `ChooseDiscover`：消费该座位在队列中最靠前的待选，其他座位的待选不受影响。
///
/// 发现待选与进化待选共用该意图：`choice` 对发现是候选下标，对进化是选项下标
/// （`0` = +2/+2，`1` = 护盾，`2` = 复生，见 `effects::evolve`）。
fn choose_discover(
    engine: &mut Engine,
    state: &mut GameState,
    seat: Seat,
    choice: u8,
) -> Result<Outcome, Reject> {
    check_choose_discover(engine, state, seat, choice)?;

    let index = pending_index(state, seat).expect("check_choose_discover 已保证有待选");

    let card_id = match state.pending.get(index) {
        Some(PendingChoice::Discover { candidates, .. }) => candidates
            .get(usize::from(choice))
            .cloned()
            .expect("check_choose_discover 已保证候选下标合法"),
        Some(PendingChoice::Evolve { .. }) => {
            engine
                .resolve_evolve_choice(state, index, choice)
                .expect("check_choose_discover 已保证进化选项合法");
            return Ok(Outcome::empty());
        }
        None => unreachable!("check_choose_discover 已保证待选存在"),
    };

    state.pending.remove(index);
    if let Some(count) = state.pool.get_mut(&card_id) {
        *count = count.saturating_sub(1);
    }
    let player = state
        .players
        .get_mut(usize::from(seat))
        .expect("check_choose_discover 已保证座位存在");
    player.hand.push(HandCard {
        card_id: card_id.clone(),
        golden: false,
    });

    let mut events = vec![GameEvent::CardDiscovered { seat, card_id }];
    events.extend(engine.check_triple(state, seat));
    Ok(Outcome::with(events))
}

/// 重抽一名玩家的全部非冻结槽位。
///
/// `preserve_frozen == true` 时保留冻结槽位（回合开始）；`false` 时先清除所有冻结
/// 标记再重抽（手动刷新、升时代）。
pub(crate) fn roll_player(
    engine: &Engine,
    state: &mut GameState,
    seat: Seat,
    preserve_frozen: bool,
) {
    let Some(current) = state.players.get(usize::from(seat)) else {
        return;
    };
    let weights = engine
        .catalog
        .config
        .shop_weights
        .get(era_index(current.era))
        .copied();
    let old_shop = current.shop.clone();
    // 商店恢复为基础槽位数；追加槽位只在本次刷新结果中保留。
    let slot_count = engine.catalog.config.shop_slots;

    let (rng, pool, tribes) = (&mut state.rng, &state.pool, &state.enabled_tribes);
    let mut new_shop = Vec::with_capacity(slot_count);
    for index in 0..slot_count {
        let slot = old_shop.get(index);
        if preserve_frozen
            && let Some(Some(existing)) = slot
            && existing.frozen
        {
            new_shop.push(Some(existing.clone()));
            continue;
        }
        let card_id = weights
            .as_ref()
            .and_then(|weights| draw_shop_card(engine, rng, pool, tribes, weights));
        new_shop.push(card_id.map(|card_id| ShopSlot {
            card_id,
            frozen: false,
        }));
    }

    if let Some(player) = state.players.get_mut(usize::from(seat)) {
        player.shop = new_shop;
    }
}

/// 抽一张商店卡：先按整数权重选星级，再在合格候选中均匀抽取。
fn draw_shop_card(
    engine: &Engine,
    rng: &mut RngStreams,
    pool: &BTreeMap<String, u16>,
    tribes: &BTreeSet<Tribe>,
    weights: &[u16; 7],
) -> Option<String> {
    for _ in 0..EMPTY_STAR_RETRIES {
        let star = pick_star(rng, weights)?;
        let candidates: Vec<&Card> = engine
            .catalog
            .cards
            .values()
            .filter(|card| card.level == star && is_eligible(card, pool, tribes))
            .collect();
        if candidates.is_empty() {
            continue;
        }
        let index = rng.below(Stream::Shop, candidates.len() as u64) as usize;
        return candidates.get(index).map(|card| card.id.clone());
    }
    None
}

/// 按整数权重选择一个 1 起的星级；全零权重返回 `None`。
fn pick_star(rng: &mut RngStreams, weights: &[u16; 7]) -> Option<u8> {
    let total: u32 = weights.iter().map(|&weight| u32::from(weight)).sum();
    if total == 0 {
        return None;
    }
    let mut roll = rng.below(Stream::Shop, u64::from(total)) as u32;
    for (index, &weight) in weights.iter().enumerate() {
        let weight = u32::from(weight);
        if roll < weight {
            return u8::try_from(index + 1).ok();
        }
        roll -= weight;
    }
    Some(7)
}

/// 英雄卡不进商店；随从需要池中有副本；流派需本局启用或为通用。
fn is_eligible(card: &Card, pool: &BTreeMap<String, u16>, tribes: &BTreeSet<Tribe>) -> bool {
    if card.is_hero {
        return false;
    }
    if card.card_type == CardType::Minion && pool.get(&card.id).copied().unwrap_or(0) == 0 {
        return false;
    }
    card.tribe == Tribe::General || tribes.contains(&card.tribe)
}

pub(crate) fn new_minion(
    catalog: &Catalog,
    instance_id: InstanceId,
    card_id: &str,
    golden: bool,
) -> MinionInstance {
    MinionInstance::new(catalog, instance_id, card_id, golden)
}

fn player(state: &GameState, seat: Seat) -> Result<&PlayerState, Reject> {
    state
        .players
        .get(usize::from(seat))
        .ok_or_else(|| Reject::new(RejectCode::InvalidStage, format!("座位 {seat} 不存在")))
}

fn era_index(era: Era) -> usize {
    match era {
        Era::Prehistoric => 0,
        Era::Primitive => 1,
        Era::Classical => 2,
        Era::Feudal => 3,
        Era::Industrial => 4,
        Era::Modern => 5,
        Era::Future => 6,
    }
}

fn next_era(era: Era) -> Option<Era> {
    match era {
        Era::Prehistoric => Some(Era::Primitive),
        Era::Primitive => Some(Era::Classical),
        Era::Classical => Some(Era::Feudal),
        Era::Feudal => Some(Era::Industrial),
        Era::Industrial => Some(Era::Modern),
        Era::Modern => Some(Era::Future),
        Era::Future => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::hash::state_hash;

    const CARDS_JSON: &str = include_str!("../../../data/v0.1/cards.json");
    const SKILLS_JSON: &str = include_str!("../../../data/v0.1/skills.json");
    const TOKENS_JSON: &str = include_str!("../../../data/v0.1/tokens.json");
    const CONFIG_JSON: &str = include_str!("../../../data/v0.1/config.json");

    fn test_catalog() -> Catalog {
        Catalog::from_json(CARDS_JSON, SKILLS_JSON, TOKENS_JSON, CONFIG_JSON)
            .expect("已提交的 v0.1 数据应能完整加载")
    }

    fn heroes() -> BTreeMap<Seat, String> {
        test_catalog()
            .cards
            .values()
            .filter(|card| card.is_hero)
            .take(8)
            .enumerate()
            .map(|(index, card)| (index as Seat, card.id.clone()))
            .collect()
    }

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

    fn dummy(instance_id: InstanceId, card_id: &str) -> MinionInstance {
        MinionInstance::new(&test_catalog(), instance_id, card_id, false)
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

    type SetupFn = Box<dyn Fn() -> (Engine, GameState)>;
    type CheckFn = Box<dyn Fn(&Engine, &GameState) -> Result<(), Reject>>;

    struct Case {
        name: &'static str,
        setup: SetupFn,
        intent: Intent,
        check: CheckFn,
        expected: RejectCode,
        /// 逐字锁定拒绝消息（仅对需要锚定的用例填写）。
        expected_message: Option<&'static str>,
    }

    /// 每个拒绝场景同时走 `Engine::apply` 与对应 `check_*`：
    /// 断言两者拒绝码都等于显式 `expected`（三者相等），指定用例的拒绝消息逐字一致，
    /// 且 `check_*` 与 `apply` 都不改变状态哈希。
    ///
    /// 组合条件用例让夹具同时满足两个失败条件，从而锁住校验优先级。
    #[test]
    fn check_agrees_with_apply_on_rejects() {
        let cases = vec![
            Case {
                name: "金币不足买卡",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(101);
                    set_shop(&mut state, 0, &[Some(minion_id(1))]);
                    state.players[0].gold = 0;
                    (engine, state)
                }),
                intent: Intent::BuyMinion { slot: 0 },
                check: Box::new(|engine, state| check_buy(engine, state, 0, 0)),
                expected: RejectCode::NotEnoughGold,
                expected_message: None,
            },
            Case {
                name: "空槽位买卡",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(102);
                    set_shop(&mut state, 0, &[None, None]);
                    (engine, state)
                }),
                intent: Intent::BuyMinion { slot: 0 },
                check: Box::new(|engine, state| check_buy(engine, state, 0, 0)),
                expected: RejectCode::ShopSlotEmpty,
                expected_message: Some("商店槽位 0 为空"),
            },
            Case {
                name: "空槽位且金币为 0 优先报空槽位",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(118);
                    set_shop(&mut state, 0, &[None]);
                    state.players[0].gold = 0;
                    (engine, state)
                }),
                intent: Intent::BuyMinion { slot: 0 },
                check: Box::new(|engine, state| check_buy(engine, state, 0, 0)),
                expected: RejectCode::ShopSlotEmpty,
                expected_message: Some("商店槽位 0 为空"),
            },
            Case {
                name: "共享池为空买卡",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(103);
                    let card_id = minion_id(1);
                    set_shop(&mut state, 0, &[Some(card_id.clone())]);
                    state.pool.insert(card_id, 0);
                    (engine, state)
                }),
                intent: Intent::BuyMinion { slot: 0 },
                check: Box::new(|engine, state| check_buy(engine, state, 0, 0)),
                expected: RejectCode::ShopSlotEmpty,
                expected_message: None,
            },
            Case {
                name: "共享池空且金币为 0 优先报金币不足",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(119);
                    let card_id = minion_id(1);
                    set_shop(&mut state, 0, &[Some(card_id.clone())]);
                    state.pool.insert(card_id, 0);
                    state.players[0].gold = 0;
                    (engine, state)
                }),
                intent: Intent::BuyMinion { slot: 0 },
                check: Box::new(|engine, state| check_buy(engine, state, 0, 0)),
                expected: RejectCode::NotEnoughGold,
                expected_message: Some("需要 3 金币"),
            },
            Case {
                name: "手牌满时买卡",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(104);
                    set_shop(&mut state, 0, &[Some(minion_id(1))]);
                    fill_board(&mut state, 0, 7);
                    fill_hand(&mut state, 0, 10);
                    (engine, state)
                }),
                intent: Intent::BuyMinion { slot: 0 },
                check: Box::new(|engine, state| check_buy(engine, state, 0, 0)),
                expected: RejectCode::NoSpace,
                expected_message: Some("手牌已满"),
            },
            Case {
                name: "棋盘手牌满且金币为 0 优先报空间不足",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(120);
                    set_shop(&mut state, 0, &[Some(minion_id(1))]);
                    fill_board(&mut state, 0, 7);
                    fill_hand(&mut state, 0, 10);
                    state.players[0].gold = 0;
                    (engine, state)
                }),
                intent: Intent::BuyMinion { slot: 0 },
                check: Box::new(|engine, state| check_buy(engine, state, 0, 0)),
                expected: RejectCode::NoSpace,
                expected_message: None,
            },
            Case {
                name: "非法出售实例",
                setup: Box::new(|| setup(105)),
                intent: Intent::SellMinion { minion: 777 },
                check: Box::new(|_, state| check_sell(state, 0, 777)),
                expected: RejectCode::NoSuchMinion,
                expected_message: None,
            },
            Case {
                name: "金币不足刷新",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(106);
                    state.players[0].gold = 0;
                    state.players[0].first_refresh_used = true;
                    (engine, state)
                }),
                intent: Intent::RerollShop,
                check: Box::new(|_, state| check_reroll(state, 0)),
                expected: RejectCode::NotEnoughGold,
                expected_message: None,
            },
            Case {
                name: "金币不足升时代",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(107);
                    state.players[0].gold = 4;
                    (engine, state)
                }),
                intent: Intent::UpgradeEra,
                check: Box::new(|engine, state| check_upgrade(engine, state, 0)),
                expected: RejectCode::NotEnoughGold,
                expected_message: None,
            },
            Case {
                name: "满时代升级",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(108);
                    state.players[0].era = Era::Future;
                    state.players[0].gold = 200;
                    (engine, state)
                }),
                intent: Intent::UpgradeEra,
                check: Box::new(|engine, state| check_upgrade(engine, state, 0)),
                expected: RejectCode::AlreadyMaxEra,
                expected_message: None,
            },
            Case {
                name: "手牌下标越界",
                setup: Box::new(|| setup(109)),
                intent: Intent::PlayCard {
                    hand_index: 3,
                    target: None,
                },
                check: Box::new(|engine, state| check_play_card(engine, state, 0, 3)),
                expected: RejectCode::CardNotInHand,
                expected_message: None,
            },
            Case {
                name: "棋盘已满打出手牌随从",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(110);
                    fill_board(&mut state, 0, 7);
                    state.players[0].hand.push(HandCard {
                        card_id: minion_id(1),
                        golden: false,
                    });
                    (engine, state)
                }),
                intent: Intent::PlayCard {
                    hand_index: 0,
                    target: None,
                },
                check: Box::new(|engine, state| check_play_card(engine, state, 0, 0)),
                expected: RejectCode::NoSpace,
                expected_message: Some("棋盘已满"),
            },
            Case {
                name: "站位下标越界",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(111);
                    state.players[0].board = vec![dummy(1, "a"), dummy(2, "b")];
                    (engine, state)
                }),
                intent: Intent::RearrangeBoard { from: 0, to: 2 },
                check: Box::new(|_, state| check_rearrange(state, 0, 0, 2)),
                expected: RejectCode::NoSuchMinion,
                expected_message: None,
            },
            Case {
                name: "冻结空槽位",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(112);
                    set_shop(&mut state, 0, &[None]);
                    (engine, state)
                }),
                intent: Intent::FreezeShop { slot: Some(0) },
                check: Box::new(|_, state| check_freeze(state, 0, Some(0))),
                expected: RejectCode::ShopSlotEmpty,
                expected_message: None,
            },
            Case {
                name: "金币不足使用英雄技能",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(113);
                    state.players[0].gold = 0;
                    (engine, state)
                }),
                intent: Intent::UseHeroPower {
                    skill_id: "division-of-labor".to_string(),
                    target: None,
                },
                check: Box::new(|engine, state| {
                    check_hero_power(engine, state, 0, "division-of-labor")
                }),
                expected: RejectCode::NotEnoughGold,
                expected_message: None,
            },
            Case {
                name: "被动技能不能主动使用",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(114);
                    // 座位 0 默认是 adam-smith；改成 galileo 才能走到
                    // 「技能属于英雄但没有 cost」的被动分支。
                    state.players[0].hero = Some("galileo".to_string());
                    (engine, state)
                }),
                intent: Intent::UseHeroPower {
                    skill_id: "observation".to_string(),
                    target: None,
                },
                check: Box::new(|engine, state| check_hero_power(engine, state, 0, "observation")),
                expected: RejectCode::HeroPowerLimit,
                expected_message: Some("技能 observation 是被动技能，不能主动使用"),
            },
            Case {
                name: "无待选时选择发现",
                setup: Box::new(|| setup(115)),
                intent: Intent::ChooseDiscover { choice: 0 },
                check: Box::new(|engine, state| check_choose_discover(engine, state, 0, 0)),
                expected: RejectCode::InvalidStage,
                expected_message: None,
            },
            Case {
                name: "发现候选下标越界",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(116);
                    state.pending.push_back(PendingChoice::Discover {
                        seat: 0,
                        candidates: vec![minion_id(1), minion_id(2)],
                        source: 1,
                    });
                    (engine, state)
                }),
                intent: Intent::ChooseDiscover { choice: 5 },
                check: Box::new(|engine, state| check_choose_discover(engine, state, 0, 5)),
                expected: RejectCode::InvalidChoice,
                expected_message: Some("发现候选下标 5 越界"),
            },
            Case {
                name: "手牌已满时选择发现",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(117);
                    fill_hand(&mut state, 0, 10);
                    state.pending.push_back(PendingChoice::Discover {
                        seat: 0,
                        candidates: vec![minion_id(1), minion_id(2)],
                        source: 1,
                    });
                    (engine, state)
                }),
                intent: Intent::ChooseDiscover { choice: 0 },
                check: Box::new(|engine, state| check_choose_discover(engine, state, 0, 0)),
                expected: RejectCode::NoSpace,
                expected_message: None,
            },
            Case {
                name: "手牌满且发现下标越界优先报下标越界",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(121);
                    fill_hand(&mut state, 0, 10);
                    state.pending.push_back(PendingChoice::Discover {
                        seat: 0,
                        candidates: vec![minion_id(1), minion_id(2)],
                        source: 1,
                    });
                    (engine, state)
                }),
                intent: Intent::ChooseDiscover { choice: 5 },
                check: Box::new(|engine, state| check_choose_discover(engine, state, 0, 5)),
                expected: RejectCode::InvalidChoice,
                expected_message: None,
            },
        ];

        for case in cases {
            let (mut engine, mut state) = (case.setup)();
            let before = state_hash(&state);

            let checked = (case.check)(&engine, &state).unwrap_err();
            assert_eq!(
                before,
                state_hash(&state),
                "{}：check_* 不得修改状态",
                case.name
            );
            assert_eq!(
                checked.code, case.expected,
                "{}：check_* 的拒绝码应等于显式期望",
                case.name
            );
            if let Some(message) = case.expected_message {
                assert_eq!(
                    checked.message, message,
                    "{}：check_* 的拒绝消息应逐字保留",
                    case.name
                );
            }

            let applied = engine
                .apply(&mut state, 0, case.intent.clone())
                .unwrap_err();
            assert_eq!(
                applied.code, case.expected,
                "{}：apply 的拒绝码应等于显式期望",
                case.name
            );
            if let Some(message) = case.expected_message {
                assert_eq!(
                    applied.message, message,
                    "{}：apply 的拒绝消息应逐字保留",
                    case.name
                );
            }
            assert_eq!(
                before,
                state_hash(&state),
                "{}：apply 拒绝后不得修改状态",
                case.name
            );
        }
    }

    /// `check_choose_discover` 的 `Evolve` 分支：越界选项与 `apply` 一致拒绝。
    #[test]
    fn check_choose_discover_evolve_branch_rejects_out_of_range() {
        let (mut engine, mut state) = setup(122);
        state.players[0].board.push(dummy(1, &minion_id(1)));
        state.pending.push_back(PendingChoice::Evolve {
            seat: 0,
            source: 1,
            remaining: 1,
        });
        let before = state_hash(&state);

        let checked = check_choose_discover(&engine, &state, 0, 5).unwrap_err();
        assert_eq!(before, state_hash(&state), "check_* 不得修改状态");
        assert_eq!(checked.code, RejectCode::InvalidChoice);
        assert_eq!(checked.message, "进化选项下标 5 越界");

        let applied = engine
            .apply(&mut state, 0, Intent::ChooseDiscover { choice: 5 })
            .unwrap_err();
        assert_eq!(applied.code, RejectCode::InvalidChoice);
        assert_eq!(applied.message, checked.message, "两条路径的拒绝消息应一致");
        assert_eq!(before, state_hash(&state), "apply 拒绝后不得修改状态");
    }

    /// `check_choose_discover` 的 `Evolve` 分支：合法选项预检通过，`apply` 消费待选。
    #[test]
    fn check_choose_discover_evolve_choice_consumes_pending() {
        let (mut engine, mut state) = setup(123);
        state.players[0].board.push(dummy(1, &minion_id(1)));
        state.pending.push_back(PendingChoice::Evolve {
            seat: 0,
            source: 1,
            remaining: 1,
        });
        let before = state_hash(&state);

        check_choose_discover(&engine, &state, 0, 0).expect("合法进化选项应通过预检");
        assert_eq!(before, state_hash(&state), "check_* 不得修改状态");

        let pending_before = state.pending.len();
        engine
            .apply(&mut state, 0, Intent::ChooseDiscover { choice: 0 })
            .expect("合法进化选项应被消费");
        assert_eq!(
            state.pending.len(),
            pending_before - 1,
            "apply 应消费该进化待选"
        );
    }
}
