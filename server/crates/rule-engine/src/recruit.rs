//! 招募阶段：商店刷新、购买、出售、刷新、升时代、打出、站位、冻结与英雄技能。
//!
//! 所有函数都先校验再修改状态，拒绝时 `GameState` 保持逐字节一致。

use std::collections::{BTreeMap, BTreeSet};

use crate::data::{Card, CardType, Era, Tribe};
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
        Intent::BuyMinion { slot, target } => buy(engine, state, seat, slot, target),
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

fn buy(
    engine: &mut Engine,
    state: &mut GameState,
    seat: Seat,
    slot: u8,
    target: Option<InstanceId>,
) -> Result<Outcome, Reject> {
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
    let board_room = current.board.len() < config.board_slots;
    let hand_room = current.hand.len() < config.hand_slots;
    let to_board = match card_type {
        CardType::Minion => {
            if !board_room && !hand_room {
                return Err(Reject::new(RejectCode::NoSpace, "棋盘与手牌均已满"));
            }
            board_room
        }
        CardType::Effect => {
            if !hand_room {
                return Err(Reject::new(RejectCode::NoSpace, "手牌已满"));
            }
            false
        }
    };
    let remaining_gold = current
        .gold
        .checked_sub(cost)
        .ok_or_else(|| Reject::new(RejectCode::NotEnoughGold, format!("需要 {cost} 金币")))?;

    if card_type == CardType::Minion && state.pool.get(&card_id).copied().unwrap_or(0) == 0 {
        return Err(Reject::new(
            RejectCode::ShopSlotEmpty,
            format!("卡牌 {card_id} 的共享池已空"),
        ));
    }

    let instance_id = state.next_instance_id;
    let following_instance_id = if to_board {
        instance_id
            .checked_add(1)
            .ok_or_else(|| Reject::new(RejectCode::NoSpace, "随从实例编号已耗尽"))?
    } else {
        instance_id
    };

    let player = state
        .players
        .get_mut(usize::from(seat))
        .ok_or_else(|| Reject::new(RejectCode::InvalidStage, format!("座位 {seat} 不存在")))?;
    player.gold = remaining_gold;
    if let Some(shop_slot) = player.shop.get_mut(usize::from(slot)) {
        *shop_slot = None;
    }
    if card_type == CardType::Minion
        && let Some(count) = state.pool.get_mut(&card_id)
    {
        *count = count.saturating_sub(1);
    }
    if to_board {
        state.next_instance_id = following_instance_id;
        player.board.push(new_minion(instance_id, &card_id, false));
    } else {
        player.hand.push(HandCard {
            card_id: card_id.clone(),
            golden: false,
        });
    }

    let mut events = vec![GameEvent::MinionBought {
        seat,
        slot,
        card_id,
    }];
    if to_board {
        engine.register_evolve(state, seat, instance_id);
        events.extend(engine.fire(
            state,
            Trigger::OnPlay,
            EffectCtx {
                seat,
                source: Some(instance_id),
                target,
                event_target: None,
                golden: false,
            },
        ));
        engine.fire_evolve_after_enter(state, seat, instance_id);
    }
    if card_type == CardType::Minion {
        events.extend(engine.check_triple(state, seat));
    }
    Ok(Outcome::with(events))
}

fn sell(state: &mut GameState, seat: Seat, minion: InstanceId) -> Result<Outcome, Reject> {
    let player = player(state, seat)?;
    let index = player
        .board
        .iter()
        .position(|candidate| candidate.instance_id == minion)
        .ok_or_else(|| {
            Reject::new(
                RejectCode::NoSuchMinion,
                format!("棋盘上不存在随从 {minion}"),
            )
        })?;
    let card_id = player.board[index].card_id.clone();
    let refund = if player.board[index].golden { 3 } else { 1 };

    let player = state
        .players
        .get_mut(usize::from(seat))
        .ok_or_else(|| Reject::new(RejectCode::InvalidStage, format!("座位 {seat} 不存在")))?;
    player.board.remove(index);
    player.gold = player.gold.saturating_add(refund);
    let count = state.pool.entry(card_id).or_insert(0);
    *count = count.saturating_add(1);

    Ok(Outcome::with(vec![GameEvent::MinionSold { seat, minion }]))
}

pub(crate) fn reroll(
    engine: &mut Engine,
    state: &mut GameState,
    seat: Seat,
) -> Result<Outcome, Reject> {
    let player = player(state, seat)?;
    let cost = u8::from(player.first_refresh_used);
    player
        .gold
        .checked_sub(cost)
        .ok_or_else(|| Reject::new(RejectCode::NotEnoughGold, "刷新商店需要 1 金币"))?;

    let player = state
        .players
        .get_mut(usize::from(seat))
        .ok_or_else(|| Reject::new(RejectCode::InvalidStage, format!("座位 {seat} 不存在")))?;
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

fn upgrade(engine: &Engine, state: &mut GameState, seat: Seat) -> Result<Outcome, Reject> {
    let player = player(state, seat)?;
    let next = next_era(player.era)
        .ok_or_else(|| Reject::new(RejectCode::AlreadyMaxEra, "已处于未来时代"))?;
    let cost = engine
        .catalog
        .config
        .era_costs
        .get(era_index(player.era))
        .copied()
        .ok_or_else(|| Reject::new(RejectCode::AlreadyMaxEra, "已处于未来时代"))?;
    let cost = u8::try_from(cost).unwrap_or(u8::MAX);
    let remaining_gold = player
        .gold
        .checked_sub(cost)
        .ok_or_else(|| Reject::new(RejectCode::NotEnoughGold, format!("升级需要 {cost} 金币")))?;

    let player = state
        .players
        .get_mut(usize::from(seat))
        .ok_or_else(|| Reject::new(RejectCode::InvalidStage, format!("座位 {seat} 不存在")))?;
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

fn play_card(
    engine: &mut Engine,
    state: &mut GameState,
    seat: Seat,
    hand_index: u8,
    target: Option<InstanceId>,
) -> Result<Outcome, Reject> {
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

    match card_type {
        CardType::Effect => {
            let player = state.players.get_mut(usize::from(seat)).ok_or_else(|| {
                Reject::new(RejectCode::InvalidStage, format!("座位 {seat} 不存在"))
            })?;
            player.hand.remove(index);

            let ctx = EffectCtx {
                seat,
                source: None,
                target,
                event_target: None,
                golden: false,
            };
            let mut events =
                engine.fire_card(state, &hand_card.card_id, Trigger::OnPlayEffect, ctx);
            events.extend(engine.fire(state, Trigger::OnUse, ctx));
            Ok(Outcome::with(events))
        }
        CardType::Minion => {
            if player(state, seat)?.board.len() >= engine.catalog.config.board_slots {
                return Err(Reject::new(RejectCode::NoSpace, "棋盘已满"));
            }
            let following_instance_id = state
                .next_instance_id
                .checked_add(1)
                .ok_or_else(|| Reject::new(RejectCode::NoSpace, "随从实例编号已耗尽"))?;
            let instance_id = state.next_instance_id;
            let player = state.players.get_mut(usize::from(seat)).ok_or_else(|| {
                Reject::new(RejectCode::InvalidStage, format!("座位 {seat} 不存在"))
            })?;
            player.hand.remove(index);
            player.board.push(new_minion(
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

fn rearrange(state: &mut GameState, seat: Seat, from: u8, to: u8) -> Result<Outcome, Reject> {
    let from = usize::from(from);
    let to = usize::from(to);
    let len = player(state, seat)?.board.len();
    if from >= len || to >= len {
        return Err(Reject::new(
            RejectCode::NoSuchMinion,
            format!("站位下标越界：from={from} to={to} len={len}"),
        ));
    }
    if from == to {
        return Ok(Outcome::empty());
    }

    let player = state
        .players
        .get_mut(usize::from(seat))
        .ok_or_else(|| Reject::new(RejectCode::InvalidStage, format!("座位 {seat} 不存在")))?;
    let minion = player.board.remove(from);
    player.board.insert(to, minion);
    Ok(Outcome::empty())
}

fn freeze(state: &mut GameState, seat: Seat, slot: Option<u8>) -> Result<Outcome, Reject> {
    let player = player(state, seat)?;
    match slot {
        Some(slot) => {
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
            let player = state.players.get_mut(usize::from(seat)).ok_or_else(|| {
                Reject::new(RejectCode::InvalidStage, format!("座位 {seat} 不存在"))
            })?;
            if let Some(Some(shop_slot)) = player.shop.get_mut(usize::from(slot)) {
                shop_slot.frozen = true;
            }
        }
        None => {
            let player = state.players.get_mut(usize::from(seat)).ok_or_else(|| {
                Reject::new(RejectCode::InvalidStage, format!("座位 {seat} 不存在"))
            })?;
            for shop_slot in player.shop.iter_mut().flatten() {
                shop_slot.frozen = true;
            }
        }
    }
    Ok(Outcome::empty())
}

fn use_hero_power(
    engine: &mut Engine,
    state: &mut GameState,
    seat: Seat,
    skill_id: &str,
    target: Option<InstanceId>,
) -> Result<Outcome, Reject> {
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
    let remaining_gold = player
        .gold
        .checked_sub(cost)
        .ok_or_else(|| Reject::new(RejectCode::NotEnoughGold, format!("技能需要 {cost} 金币")))?;

    let player = state
        .players
        .get_mut(usize::from(seat))
        .ok_or_else(|| Reject::new(RejectCode::InvalidStage, format!("座位 {seat} 不存在")))?;
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

/// 本玩家结束招募：只标记该座位。所有未出局玩家都结束后才进入 `Combat`，
/// 并在该次结果中发出唯一一条 `PhaseChanged { Combat }`；出局座位不计入就绪。
/// 已结束座位的重复提交成功且无副作用（核心规则 §3：`EndRecruit` 前置条件只是「处于
/// `Recruit`」，失败返回码为「—」，重复提交时仍处于 `Recruit`）；不在 `Recruit` 阶段时
/// 由 [`apply`] 顶层检查返回 `InvalidStage`。
fn end_recruit(state: &mut GameState, seat: Seat) -> Result<Outcome, Reject> {
    match state.players.get_mut(usize::from(seat)) {
        Some(player) if !player.recruit_done => player.recruit_done = true,
        Some(_) => return Ok(Outcome::empty()),
        None => {
            return Err(Reject::new(
                RejectCode::InvalidStage,
                format!("座位 {seat} 不存在"),
            ));
        }
    }
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
    let index = state
        .pending
        .iter()
        .position(|pending| match pending {
            PendingChoice::Discover {
                seat: pending_seat, ..
            }
            | PendingChoice::Evolve {
                seat: pending_seat, ..
            } => *pending_seat == seat,
        })
        .ok_or_else(|| {
            Reject::new(
                RejectCode::InvalidStage,
                format!("座位 {seat} 没有待处理的发现选择"),
            )
        })?;

    let card_id = match state.pending.get(index) {
        Some(PendingChoice::Discover { candidates, .. }) => candidates
            .get(usize::from(choice))
            .cloned()
            .ok_or_else(|| {
                Reject::new(
                    RejectCode::InvalidChoice,
                    format!("发现候选下标 {choice} 越界"),
                )
            })?,
        Some(PendingChoice::Evolve { .. }) => {
            return match engine.resolve_evolve_choice(state, index, choice) {
                Ok(()) => Ok(Outcome::empty()),
                Err(()) => Err(Reject::new(
                    RejectCode::InvalidChoice,
                    format!("进化选项下标 {choice} 越界"),
                )),
            };
        }
        None => {
            return Err(Reject::new(
                RejectCode::InvalidStage,
                format!("座位 {seat} 没有待处理的发现选择"),
            ));
        }
    };

    let hand_slots = engine.catalog.config.hand_slots;
    if player(state, seat)?.hand.len() >= hand_slots {
        return Err(Reject::new(RejectCode::NoSpace, "手牌已满"));
    }

    state.pending.remove(index);
    if let Some(count) = state.pool.get_mut(&card_id) {
        *count = count.saturating_sub(1);
    }
    let player = state
        .players
        .get_mut(usize::from(seat))
        .ok_or_else(|| Reject::new(RejectCode::InvalidStage, format!("座位 {seat} 不存在")))?;
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

pub(crate) fn new_minion(instance_id: InstanceId, card_id: &str, golden: bool) -> MinionInstance {
    MinionInstance {
        instance_id,
        card_id: card_id.to_string(),
        golden,
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
