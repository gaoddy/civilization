//! 招募阶段：商店刷新、购买、出售、刷新、升时代、打出、站位、冻结与英雄技能。
//!
//! 所有函数都先校验再修改状态，拒绝时 `GameState` 保持逐字节一致。

use std::collections::{BTreeMap, BTreeSet};

use crate::data::{Card, CardType, Catalog, Era, Playstyle, card_in_pool};
use crate::effects::{EffectCtx, TargetRef, Trigger, target_in_other_zone, target_in_own_zone};
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
    engine: &Engine,
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

    // 有待选时：目标待选只允许 `ChooseTarget` 与 `AbandonTarget`，其余招募意图一律拒绝；
    // 发现/进化待选只允许 `ChooseDiscover`，且不能手动 `EndRecruit`，其他招募意图照常执行。
    // 超时路径不经过本函数，由 `finish_recruit` 把未消费待选视为放弃并清空。
    if let Some(index) = pending_index(state, seat) {
        match (&state.pending[index], &intent) {
            (PendingChoice::Target { .. }, Intent::ChooseTarget { .. } | Intent::AbandonTarget) => {
            }
            (
                PendingChoice::Discover { .. } | PendingChoice::Evolve { .. },
                Intent::ChooseDiscover { .. },
            ) => {}
            (PendingChoice::Target { .. }, Intent::ChooseDiscover { .. }) => {
                return Err(Reject::new(
                    RejectCode::InvalidChoice,
                    format!("座位 {seat} 有待选目标，不能选择发现"),
                ));
            }
            (
                PendingChoice::Discover { .. } | PendingChoice::Evolve { .. },
                Intent::ChooseTarget { .. },
            ) => {
                return Err(Reject::new(
                    RejectCode::InvalidChoice,
                    format!("座位 {seat} 最靠前的待选不是目标选择"),
                ));
            }
            (PendingChoice::Target { .. }, _) => {
                return Err(Reject::new(
                    RejectCode::InvalidStage,
                    format!("座位 {seat} 有待选目标，不能执行其它招募意图"),
                ));
            }
            (PendingChoice::Discover { .. } | PendingChoice::Evolve { .. }, Intent::EndRecruit) => {
                return Err(Reject::new(
                    RejectCode::InvalidStage,
                    format!("座位 {seat} 有待选未消费，不能手动结束招募"),
                ));
            }
            (PendingChoice::Discover { .. } | PendingChoice::Evolve { .. }, _) => {}
        }
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
        Intent::PlayCard {
            hand_index,
            position,
            target,
        } => play_card(engine, state, seat, hand_index, position, target),
        Intent::RearrangeBoard { from, to } => rearrange(state, seat, from, to),
        Intent::FreezeShop { slot } => freeze(state, seat, slot),
        Intent::UnfreezeShop { slot } => unfreeze(state, seat, slot),
        Intent::UseHeroPower { skill_id, target } => {
            use_hero_power(engine, state, seat, &skill_id, target)
        }
        Intent::EndRecruit => unreachable!("EndRecruit 在分派前处理"),
        Intent::ChooseDiscover { choice } => choose_discover(engine, state, seat, choice),
        Intent::ChooseTarget { target } => choose_target(engine, state, seat, target),
        Intent::AbandonTarget => abandon_target(state, seat),
    }
}

/// 金色随从从手牌打出并成功入场：发放一张「三连奖励」效果牌。
///
/// 星级在合成来源卡的基础上 +1、封顶 6 星，与玩家时代无关。
/// 战吼带目标时由 [`choose_target`] 在目标结算后调用；放弃时不发放。
fn grant_triple_reward(engine: &Engine, state: &mut GameState, seat: Seat, hand_card: &HandCard) {
    if !hand_card.golden() {
        return;
    }
    let Some(level) = engine
        .catalog
        .cards
        .get(hand_card.card_id())
        .and_then(|card| card.level.checked_add(1))
        .map(|level| level.min(6))
    else {
        return;
    };
    let reward_id = state.next_instance_id;
    state.next_instance_id = reward_id.saturating_add(1);
    if let Some(player) = state.players.get_mut(usize::from(seat)) {
        player.hand.push(HandCard::Effect {
            instance_id: reward_id,
            card_id: crate::triple::REWARD_CARD_ID.to_string(),
            reward_level: Some(level),
        });
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

    Ok(())
}

/// 购买：扣金币、清空槽位，牌一律进入手牌。
///
/// 该份在商店抽取时已从共享池扣除，购买不再扣池，随从按 1 份扣池记录。
/// 买入不入场，因此不触发战吼、不登记进化，也不需要目标与实例编号；
/// 随从要上棋盘须另提交 `PlayCard`。随从买入后仍立即做三连判定（手牌参与三连）。
fn buy(engine: &Engine, state: &mut GameState, seat: Seat, slot: u8) -> Result<Outcome, Reject> {
    check_buy(engine, state, seat, slot)?;

    let shop_slot = player(state, seat)
        .expect("check_buy 已保证座位存在")
        .shop
        .get(usize::from(slot))
        .and_then(Option::as_ref)
        .expect("check_buy 已保证槽位非空");
    let card_id = shop_slot.card_id.clone();
    let instance_id = shop_slot.instance_id;
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
    match card_type {
        CardType::Minion => player.hand.push(HandCard::Minion(new_pool_minion(
            &engine.catalog,
            instance_id,
            &card_id,
            false,
        ))),
        CardType::Effect => player.hand.push(HandCard::effect(instance_id, &card_id)),
    }

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
    let pool_copies = player.board[index].pool_copies;

    let player = state
        .players
        .get_mut(usize::from(seat))
        .expect("check_sell 已保证座位存在");
    player.board.remove(index);
    player.gold = player.gold.saturating_add(refund);
    let count = state.pool.entry(card_id).or_insert(0);
    *count = count.saturating_add(u16::from(pool_copies));

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
    engine: &Engine,
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
    engine: &Engine,
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
    engine: &Engine,
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
    if hero.is_some() {
        // 英雄技能只走英雄池：无来源触发由执行器解析到英雄卡。
        events.extend(engine.fire(
            state,
            Trigger::OnFirstRefresh,
            EffectCtx {
                seat,
                source: None,
                target: None,
                event_target: None,
                golden: false,
                level_override: None,
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
                level_override: None,
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
        .get(hand_card.card_id())
        .map(|card| card.card_type)
        .ok_or_else(|| {
            Reject::new(
                RejectCode::CardNotInHand,
                format!("手牌卡牌 {} 不在目录中", hand_card.card_id()),
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

/// 落点转插入下标：`None` 取棋盘末尾；`Some(pos)` 必须 `pos <= board.len()`。
///
/// 越界返回 `InvalidChoice`。位置校验不放进 [`check_play_card`]：视图的
/// `playable` 与落点无关（视图预检时没有落点）。
pub(crate) fn check_play_position(
    state: &GameState,
    seat: Seat,
    position: Option<u8>,
) -> Result<usize, Reject> {
    let len = player(state, seat)?.board.len();
    match position {
        None => Ok(len),
        Some(pos) => {
            let index = usize::from(pos);
            if index > len {
                return Err(Reject::new(
                    RejectCode::InvalidChoice,
                    format!("落点 {pos} 超出当前随从数 {len}"),
                ));
            }
            Ok(index)
        }
    }
}

/// 意图提交的目标区域校验：目标必须落在该座位本人的对应区域内。
///
/// 目标属于其他座位时返回 `TARGET_NOT_ALLOWED`（与核心规则 §4.4/§4.5 对齐）；
/// 目标不存在或区域与实例实际所在区域不一致时返回 `NO_VALID_TARGET`。
fn check_target_zone(
    state: &GameState,
    seat: Seat,
    target: Option<TargetRef>,
) -> Result<(), Reject> {
    match target {
        Some(target) if target_in_own_zone(state, seat, target) => Ok(()),
        Some(target) if target_in_other_zone(state, seat, target) => Err(Reject::new(
            RejectCode::TargetNotAllowed,
            format!("目标 {:?}:{} 属于其他玩家", target.zone, target.id),
        )),
        Some(target) => Err(Reject::new(
            RejectCode::NoValidTarget,
            format!("目标 {:?}:{} 不在本座位的该区域内", target.zone, target.id),
        )),
        None => Ok(()),
    }
}

fn play_card(
    engine: &Engine,
    state: &mut GameState,
    seat: Seat,
    hand_index: u8,
    position: Option<u8>,
    target: Option<TargetRef>,
) -> Result<Outcome, Reject> {
    check_play_card(engine, state, seat, hand_index)?;
    check_target_zone(state, seat, target)?;
    let target = target.map(|target| target.id);

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
        .get(hand_card.card_id())
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
                level_override: hand_card.reward_level(),
            };
            engine.check_targets(
                state,
                &engine.effects_of(hand_card.card_id(), Trigger::OnPlayEffect),
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
                engine.fire_card(state, hand_card.card_id(), Trigger::OnPlayEffect, ctx);
            events.extend(engine.fire(
                state,
                Trigger::OnUse,
                EffectCtx {
                    level_override: None,
                    ..ctx
                },
            ));
            Ok(Outcome::with(events))
        }
        CardType::Minion => {
            let insert_index = check_play_position(state, seat, position)?;
            // 目录判定为随从牌、手牌却是效果牌（测试夹具可能如此）：按卡面补一张随从牌。
            let mut minion = match hand_card.clone() {
                HandCard::Minion(minion) => minion,
                other => new_minion(
                    &engine.catalog,
                    other.instance_id(),
                    other.card_id(),
                    other.golden(),
                ),
            };
            // 手牌实例原样入场：编号与成长、繁衍、护盾等状态都不重建。
            minion.attacks_left = 0;
            minion.cannot_attack = false;
            let instance_id = minion.instance_id;
            let golden = minion.golden;
            let ctx = EffectCtx {
                seat,
                source: Some(instance_id),
                target,
                event_target: None,
                golden: minion.golden,
                level_override: None,
            };
            let player = state
                .players
                .get_mut(usize::from(seat))
                .expect("check_play_card 已保证座位存在");
            player.hand.remove(index);
            player.board.insert(insert_index, minion);

            // 先登记进化、再结算战吼（关键词机制 §2.2）。登记了进化的来源等到
            // 玩家选择结算完再触发 OnAllyEvolve；战吼带出的效果牌与响应在本次
            // 意图内同步结算，不跨意图排队。
            engine.register_evolve(state, seat, instance_id);
            let evolve_registered = state.pending.iter().any(|pending| {
                matches!(
                    pending,
                    PendingChoice::Evolve { seat: pending_seat, source, .. }
                        if *pending_seat == seat && *source == instance_id
                )
            });

            // 战吼需要单一目标时改为先入场、再推入目标待选：合法池按入场后的棋盘
            // 判定，因此池内包含刚入场的这个随从本身；候选为空时战吼直接落空。
            let effects = engine.effects_of(hand_card.card_id(), Trigger::OnPlay);
            let needs_target = engine.check_targets(state, &effects, &ctx, false).is_err();
            let has_candidates = needs_target
                && !engine
                    .target_candidates(state, &effects, &ctx, false)
                    .is_empty();
            if has_candidates {
                // 待选期间的入场副作用（金色奖励、三连、入场观察者）推迟到选定
                // 目标之后结算；放弃时只需撤回入场本身，不留残余效果。
                state.pending.push_back(PendingChoice::Target {
                    seat,
                    source: instance_id,
                    hand_card,
                    hand_index,
                    fire_ally_evolve: !evolve_registered,
                });
                return Ok(Outcome::empty());
            }

            let mut events = engine.fire(
                state,
                Trigger::OnPlay,
                EffectCtx {
                    seat,
                    source: Some(instance_id),
                    target,
                    event_target: None,
                    golden,
                    level_override: None,
                },
            );
            grant_triple_reward(engine, state, seat, &hand_card);
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

/// `FreezeShop` 与 `UnfreezeShop` 共用的只读预检：指定槽位时必须非空；
/// 操作全部槽位无需额外条件。不修改状态、不消耗随机流。
///
/// **不校验** `Recruit` 阶段、`recruit_done`、`eliminated` 与 `target`；调用方（视图层）
/// 必须自行门控阶段与玩家状态，不能仅凭返回 `Ok` 认定该意图可操作。
fn check_shop_slot(state: &GameState, seat: Seat, slot: Option<u8>) -> Result<(), Reject> {
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

/// `FreezeShop` 的只读预检，规则见 [`check_shop_slot`]。
pub(crate) fn check_freeze(state: &GameState, seat: Seat, slot: Option<u8>) -> Result<(), Reject> {
    check_shop_slot(state, seat, slot)
}

/// `UnfreezeShop` 的只读预检：槽位规则同 [`check_shop_slot`]，且本回合必须冻结过商店。
///
/// 解冻只在冻结的当回合可用；跨回合保留的冻结状态不能直接解冻，只能靠刷新换新卡
/// 或买走。下一回合重新按「冻结」会再次取得解冻权。
pub(crate) fn check_unfreeze(
    state: &GameState,
    seat: Seat,
    slot: Option<u8>,
) -> Result<(), Reject> {
    check_shop_slot(state, seat, slot)?;
    let player = player(state, seat)?;
    if !player.froze_this_turn {
        return Err(Reject::new(
            RejectCode::InvalidStage,
            "本回合尚未冻结商店，不能解冻".to_string(),
        ));
    }
    Ok(())
}

/// 设置槽位的冻结标记：指定槽位只改该槽位，`None` 改全部非空槽位。
fn set_frozen(state: &mut GameState, seat: Seat, slot: Option<u8>, frozen: bool) {
    let player = state
        .players
        .get_mut(usize::from(seat))
        .expect("check_shop_slot 已保证座位存在");
    match slot {
        Some(slot) => {
            if let Some(Some(shop_slot)) = player.shop.get_mut(usize::from(slot)) {
                shop_slot.frozen = frozen;
            }
        }
        None => {
            for shop_slot in player.shop.iter_mut().flatten() {
                shop_slot.frozen = frozen;
            }
        }
    }
}

fn freeze(state: &mut GameState, seat: Seat, slot: Option<u8>) -> Result<Outcome, Reject> {
    check_freeze(state, seat, slot)?;
    set_frozen(state, seat, slot, true);
    let player = state
        .players
        .get_mut(usize::from(seat))
        .expect("check_freeze 已保证座位存在");
    player.froze_this_turn = true;
    Ok(Outcome::empty())
}

fn unfreeze(state: &mut GameState, seat: Seat, slot: Option<u8>) -> Result<Outcome, Reject> {
    check_unfreeze(state, seat, slot)?;
    set_frozen(state, seat, slot, false);
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
    let hero = engine.catalog.heroes.get(hero_id).ok_or_else(|| {
        Reject::new(
            RejectCode::HeroPowerLimit,
            format!("英雄卡 {hero_id} 不在英雄目录中"),
        )
    })?;
    let Some(skill_ref) = engine.catalog.skill_id(skill_id) else {
        return Err(Reject::new(
            RejectCode::HeroPowerLimit,
            format!("技能 {skill_id} 不在目录中"),
        ));
    };
    if !hero.skills.contains(&skill_ref) {
        return Err(Reject::new(
            RejectCode::HeroPowerLimit,
            format!("技能 {skill_id} 不属于英雄 {hero_id}"),
        ));
    }
    let skill = engine.catalog.skill(skill_ref);
    let cost = skill.cost.ok_or_else(|| {
        Reject::new(
            RejectCode::NotActiveSkill,
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
    engine: &Engine,
    state: &mut GameState,
    seat: Seat,
    skill_id: &str,
    target: Option<TargetRef>,
) -> Result<Outcome, Reject> {
    check_hero_power(engine, state, seat, skill_id)?;
    check_target_zone(state, seat, target)?;
    let target = target.map(|target| target.id);

    let ctx = EffectCtx {
        seat,
        source: None,
        target,
        event_target: None,
        golden: false,
        level_override: None,
    };
    // 主动技能只结算被选中的技能：多技能英雄不得一次放出全部技能。
    engine.check_targets(
        state,
        &engine.skill_effects_of(skill_id, Trigger::OnActivate),
        &ctx,
        true,
    )?;

    let player = player(state, seat).expect("check_hero_power 已保证座位存在");
    let cost = engine
        .catalog
        .skill_by_name(skill_id)
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

    let events = engine.fire_skill(
        state,
        skill_id,
        Trigger::OnActivate,
        EffectCtx {
            seat,
            source: None,
            target,
            event_target: None,
            golden: false,
            level_override: None,
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
        }
        | PendingChoice::Target {
            seat: pending_seat, ..
        } => *pending_seat == seat,
    })
}

/// `ChooseTarget`：为队首的目标待选指定一个本人区域内的目标，随后结算该战吼。
///
/// 选定后补做被推迟的入场副作用：金色奖励、三连与入场观察者。
fn choose_target(
    engine: &Engine,
    state: &mut GameState,
    seat: Seat,
    target: TargetRef,
) -> Result<Outcome, Reject> {
    let index = pending_index(state, seat).ok_or_else(|| {
        Reject::new(
            RejectCode::InvalidChoice,
            format!("座位 {seat} 没有待选目标"),
        )
    })?;
    let PendingChoice::Target {
        source,
        hand_card,
        fire_ally_evolve,
        ..
    } = state.pending[index].clone()
    else {
        return Err(Reject::new(
            RejectCode::InvalidChoice,
            format!("座位 {seat} 最靠前的待选不是目标选择"),
        ));
    };
    check_target_zone(state, seat, Some(target))?;

    let card_id = hand_card.card_id().to_string();
    let ctx = EffectCtx {
        seat,
        source: Some(source),
        target: Some(target.id),
        event_target: None,
        golden: hand_card.golden(),
        level_override: None,
    };
    // 与出牌时的预检同源：目标必须在效果的合法池内（core-rules §4 补充返回码
    // 「ChooseTarget 指定的随从不在合法池时同样返回该码」）。只校验棋盘会让
    // 选到池外随从时效果静默落空。
    engine.check_targets(
        state,
        &engine.effects_of(&card_id, Trigger::OnPlay),
        &ctx,
        false,
    )?;

    state.pending.remove(index);
    let mut events = engine.fire_card(state, &card_id, Trigger::OnPlay, ctx);
    grant_triple_reward(engine, state, seat, &hand_card);
    events.extend(engine.check_triple(state, seat));
    if fire_ally_evolve {
        engine.fire_evolve_after_enter(state, seat, source);
    }
    Ok(Outcome::with(events))
}

/// `AbandonTarget`：放弃队首目标待选，撤销本次出牌。
///
/// 来源随从离场，入场前的原手牌按原下标退回；战吼不结算，被推迟的金色奖励、
/// 三连与入场观察者都不发生。出牌本身不消耗金币，因此不涉及退款。
fn abandon_target(state: &mut GameState, seat: Seat) -> Result<Outcome, Reject> {
    let index = pending_index(state, seat).ok_or_else(|| {
        Reject::new(
            RejectCode::InvalidChoice,
            format!("座位 {seat} 没有待选目标"),
        )
    })?;
    let PendingChoice::Target {
        source,
        hand_card,
        hand_index,
        ..
    } = state.pending[index].clone()
    else {
        return Err(Reject::new(
            RejectCode::InvalidChoice,
            format!("座位 {seat} 最靠前的待选不是目标选择"),
        ));
    };
    let player = state
        .players
        .get_mut(usize::from(seat))
        .ok_or_else(|| Reject::new(RejectCode::NoSuchMinion, format!("座位 {seat} 不存在")))?;
    let position = player
        .board
        .iter()
        .position(|minion| minion.instance_id == source)
        .ok_or_else(|| {
            Reject::new(
                RejectCode::NoSuchMinion,
                format!("待选来源随从 {source} 不在棋盘上"),
            )
        })?;
    player.board.remove(position);
    let at = usize::from(hand_index).min(player.hand.len());
    player.hand.insert(at, hand_card);
    state.pending.remove(index);
    Ok(Outcome::empty())
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
        Some(PendingChoice::Target { .. }) => {
            return Err(Reject::new(
                RejectCode::InvalidChoice,
                format!("座位 {seat} 有待选目标，不能选择发现"),
            ));
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
    engine: &Engine,
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
        Some(PendingChoice::Target { .. }) => {
            unreachable!("check_choose_discover 已拒绝目标待选")
        }
        None => unreachable!("check_choose_discover 已保证待选存在"),
    };

    state.pending.remove(index);
    // 候选登记后池中份数可能已被其他操作消耗；只有实际扣到份才记为占用。
    let charged = match state.pool.get_mut(&card_id) {
        Some(count) if *count > 0 => {
            *count -= 1;
            true
        }
        _ => false,
    };
    let instance_id = state.next_instance_id;
    state.next_instance_id = instance_id.saturating_add(1);
    let player = state
        .players
        .get_mut(usize::from(seat))
        .expect("check_choose_discover 已保证座位存在");
    let minion = if charged {
        new_pool_minion(&engine.catalog, instance_id, &card_id, false)
    } else {
        new_minion(&engine.catalog, instance_id, &card_id, false)
    };
    player.hand.push(HandCard::Minion(minion));

    let mut events = vec![GameEvent::CardDiscovered { seat, card_id }];
    events.extend(engine.check_triple(state, seat));
    Ok(Outcome::with(events))
}

/// 重抽一名玩家的全部非冻结槽位。
///
/// `preserve_frozen == true` 时保留冻结槽位的卡牌、实例编号与冻结标记（回合开始）：
/// 不手动解冻就一直保留，只补足空槽。`false` 时先清除所有冻结标记再重抽（手动刷新、
/// 升时代），因此刷新后的新卡牌不带冻结状态。
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

    // 未保留的旧槽位先把份归还公共池，再抽取新牌。冻结且位于基础槽位内的卡
    // 跨回合保留，继续占用其份数。
    for (index, slot) in old_shop.iter().enumerate() {
        let Some(slot) = slot else { continue };
        let kept = preserve_frozen && index < slot_count && slot.frozen;
        if kept {
            continue;
        }
        if let Some(count) = state.pool.get_mut(&slot.card_id) {
            *count = count.saturating_add(1);
        }
    }

    let (rng, pool, playstyles) = (&mut state.rng, &mut state.pool, &state.enabled_playstyles);
    // 冻结槽位沿用原实例编号；其余槽位在确定卡牌后统一分配新编号。
    let mut drawn: Vec<Option<(Option<InstanceId>, String, bool)>> = Vec::with_capacity(slot_count);
    for index in 0..slot_count {
        let slot = old_shop.get(index);
        if preserve_frozen
            && let Some(Some(existing)) = slot
            && existing.frozen
        {
            // 冻结标记原样保留：不解冻就一直锁在商店里。
            drawn.push(Some((
                Some(existing.instance_id),
                existing.card_id.clone(),
                true,
            )));
            continue;
        }
        let card_id = weights
            .as_ref()
            .and_then(|weights| draw_shop_card(engine, rng, pool, playstyles, weights));
        drawn.push(card_id.map(|card_id| (None, card_id, false)));
    }

    let new_shop = drawn
        .into_iter()
        .map(|slot| {
            slot.map(|(instance_id, card_id, frozen)| {
                let instance_id = instance_id.unwrap_or_else(|| {
                    let id = state.next_instance_id;
                    state.next_instance_id = id.saturating_add(1);
                    id
                });
                ShopSlot {
                    instance_id,
                    card_id,
                    frozen,
                }
            })
        })
        .collect();

    if let Some(player) = state.players.get_mut(usize::from(seat)) {
        player.shop = new_shop;
    }
}

/// 出局时清空商店，并把未购买的随从份归还公共牌池。
///
/// 效果牌不在公共池中，`pool.get_mut` 返回 `None` 时自然跳过。
pub(crate) fn return_shop_to_pool(state: &mut GameState, seat: Seat) {
    let Some(player) = state.players.get_mut(usize::from(seat)) else {
        return;
    };
    for slot in &mut player.shop {
        let Some(existing) = slot.take() else {
            continue;
        };
        if let Some(count) = state.pool.get_mut(&existing.card_id) {
            *count = count.saturating_add(1);
        }
    }
}

/// 抽一张商店卡：先按整数权重选星级，再在合格候选中均匀抽取。
///
/// 抽到随从后立即从共享牌池扣除 1 份；效果牌不在池中，不扣。
fn draw_shop_card(
    engine: &Engine,
    rng: &mut RngStreams,
    pool: &mut BTreeMap<String, u16>,
    playstyles: &BTreeSet<Playstyle>,
    weights: &[u16; 7],
) -> Option<String> {
    for _ in 0..EMPTY_STAR_RETRIES {
        let star = pick_star(rng, weights)?;
        let candidates: Vec<&Card> = engine
            .catalog
            .cards
            .values()
            .filter(|card| card.level == star && is_eligible(card, pool, playstyles))
            .collect();
        if candidates.is_empty() {
            continue;
        }
        let index = rng.below(Stream::Shop, candidates.len() as u64) as usize;
        let card = candidates.get(index).copied()?;
        if let Some(count) = pool.get_mut(&card.id) {
            *count = count.saturating_sub(1);
        }
        return Some(card.id.clone());
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

/// 英雄卡不进商店；非池卡（衍生物、生成卡）不进商店；随从需要池中有副本；
/// 卡池判定统一走 [`card_in_pool`]。
fn is_eligible(
    card: &Card,
    pool: &BTreeMap<String, u16>,
    playstyles: &BTreeSet<Playstyle>,
) -> bool {
    if !card.in_shop_pool {
        return false;
    }
    if card.card_type == CardType::Minion && pool.get(&card.id).copied().unwrap_or(0) == 0 {
        return false;
    }
    card_in_pool(card, playstyles)
}

pub(crate) fn new_minion(
    catalog: &Catalog,
    instance_id: InstanceId,
    card_id: &str,
    golden: bool,
) -> MinionInstance {
    MinionInstance::new(catalog, instance_id, card_id, golden)
}

/// 从公共牌池取 1 份创建的随从：出售时归还 1 份。
pub(crate) fn new_pool_minion(
    catalog: &Catalog,
    instance_id: InstanceId,
    card_id: &str,
    golden: bool,
) -> MinionInstance {
    let mut minion = new_minion(catalog, instance_id, card_id, golden);
    minion.pool_copies = 1;
    minion
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
    const HEROES_JSON: &str = include_str!("../../../data/v0.1/heroes.json");
    const SKILLS_JSON: &str = include_str!("../../../data/v0.1/skills.json");
    const CONFIG_JSON: &str = include_str!("../../../data/v0.1/config.json");
    const TAXONOMY_JSON: &str = include_str!("../../../data/v0.1/taxonomy.json");

    fn test_catalog() -> Catalog {
        Catalog::from_json(
            CARDS_JSON,
            HEROES_JSON,
            SKILLS_JSON,
            CONFIG_JSON,
            TAXONOMY_JSON,
        )
        .expect("已提交的 v0.1 数据应能完整加载")
    }

    fn test_playstyles() -> [Playstyle; 5] {
        crate::playstyles::draw(1)
    }

    fn heroes() -> BTreeMap<Seat, String> {
        test_catalog()
            .heroes
            .values()
            .take(8)
            .enumerate()
            .map(|(index, hero)| (index as Seat, hero.id.clone()))
            .collect()
    }

    fn setup(seed: u8) -> (Engine, GameState) {
        let (engine, mut state) = Engine::new(test_catalog(), [seed; 32], test_playstyles());
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
            .find(|card| card.card_type == CardType::Minion && card.level == level)
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
                card_id.map(|card_id| {
                    let instance_id = state.next_instance_id;
                    state.next_instance_id = instance_id.saturating_add(1);
                    ShopSlot {
                        instance_id,
                        card_id,
                        frozen: false,
                    }
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
            .filter(|card| card.card_type == CardType::Minion)
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
            .map(|index| HandCard::Minion(dummy(2_000 + index as InstanceId, &card_id)))
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
                    position: None,
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
                    let card_id = minion_id(1);
                    state.players[0]
                        .hand
                        .push(HandCard::Minion(dummy(3_000, &card_id)));
                    (engine, state)
                }),
                intent: Intent::PlayCard {
                    hand_index: 0,
                    position: None,
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
                name: "解冻空槽位",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(123);
                    set_shop(&mut state, 0, &[None]);
                    (engine, state)
                }),
                intent: Intent::UnfreezeShop { slot: Some(0) },
                check: Box::new(|_, state| check_unfreeze(state, 0, Some(0))),
                expected: RejectCode::ShopSlotEmpty,
                expected_message: None,
            },
            Case {
                name: "本回合未冻结就解冻",
                setup: Box::new(|| {
                    let (engine, mut state) = setup(124);
                    set_shop(&mut state, 0, &[Some(minion_id(1))]);
                    if let Some(Some(slot)) = state.players[0].shop.first_mut() {
                        slot.frozen = true;
                    }
                    (engine, state)
                }),
                intent: Intent::UnfreezeShop { slot: Some(0) },
                check: Box::new(|_, state| check_unfreeze(state, 0, Some(0))),
                expected: RejectCode::InvalidStage,
                expected_message: Some("本回合尚未冻结商店，不能解冻"),
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
                expected: RejectCode::NotActiveSkill,
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
            let (engine, mut state) = (case.setup)();
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
        let (engine, mut state) = setup(122);
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
        let (engine, mut state) = setup(123);
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
