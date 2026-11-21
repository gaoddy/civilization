//! 商店与经济类效果：刷新商店、增加商店槽位、获得随从。
//!
//! 对应关键词机制 §8 的 `RefreshShop` / `AddShopSlot` / `GainMinion`。
//! 追加槽位与获得随从都取「当前星级排序第一张」合格随从（卡牌目录 `id` 顺序），
//! 选择与遍历顺序确定，不额外消耗随机流；没有合格卡牌时静默跳过。
//! `GainMinion` 的来源与目标区域见 [`Engine::apply_gain_minion`]。

use std::collections::{BTreeMap, BTreeSet};

use crate::data::{CardType, Catalog, Era, Playstyle, card_in_pool};
use crate::effects::{Effect, EffectCtx, Params, Trigger, Zone};
use crate::engine::Engine;
use crate::events::GameEvent;
use crate::recruit::new_minion;
use crate::rng::Stream;
use crate::state::{GameState, HandCard, ShopSlot};

/// 商店槽位硬上限；追加槽位不得越过该值。
const MAX_SHOP_SLOTS: usize = 7;

impl Engine {
    /// `RefreshShop{free}`：立即重抽当前座位商店。
    ///
    /// `free = true` 不扣金币、不消耗本回合免费刷新额度；无论免费与否，
    /// 都通过与手动刷新相同的每回合闸门触发 `OnFirstRefresh`。
    pub(crate) fn apply_refresh_shop(
        &self,
        state: &mut GameState,
        effect: &Effect,
        ctx: &EffectCtx,
    ) -> Vec<GameEvent> {
        if state.players.get(usize::from(ctx.seat)).is_none() {
            return Vec::new();
        }
        let Params::RefreshShop(params) = &effect.params else {
            return Vec::new();
        };
        if params.free.unwrap_or(false) {
            crate::recruit::roll_player(self, state, ctx.seat, false);
            let mut events = vec![GameEvent::ShopRolled { seat: ctx.seat }];
            events.extend(crate::recruit::maybe_fire_first_refresh(
                self, state, ctx.seat,
            ));
            events
        } else {
            match crate::recruit::reroll(self, state, ctx.seat) {
                Ok(outcome) => outcome.events,
                // 效果执行不能拒绝：金币不足时静默不刷新。
                Err(_) => Vec::new(),
            }
        }
    }

    /// `AddShopSlot{era, count}`：按当前时代星级追加 `count` 个槽位。
    ///
    /// 每个槽位放入卡牌目录顺序中第一张合格随从，并从共享池扣 1 张；
    /// 达到 7 槽或没有合格卡牌时停止追加。
    pub(crate) fn apply_add_shop_slot(
        &self,
        state: &mut GameState,
        effect: &Effect,
        ctx: &EffectCtx,
    ) -> Vec<GameEvent> {
        let Some(star) = state
            .players
            .get(usize::from(ctx.seat))
            .map(|player| player.era as u8)
        else {
            return Vec::new();
        };
        let Params::AddShopSlot(params) = &effect.params else {
            return Vec::new();
        };
        let count = usize::try_from(params.count).unwrap_or(0);

        for _ in 0..count {
            let full = state
                .players
                .get(usize::from(ctx.seat))
                .is_none_or(|player| player.shop.len() >= MAX_SHOP_SLOTS);
            if full {
                break;
            }
            let Some(card_id) =
                first_eligible(&self.catalog, &state.pool, &state.enabled_playstyles, star)
            else {
                break;
            };
            if let Some(pool) = state.pool.get_mut(&card_id) {
                *pool = pool.saturating_sub(1);
            }
            let instance_id = state.next_instance_id;
            state.next_instance_id = instance_id.saturating_add(1);
            if let Some(player) = state.players.get_mut(usize::from(ctx.seat)) {
                player.shop.push(Some(ShopSlot {
                    instance_id,
                    card_id,
                    frozen: false,
                }));
            }
        }
        Vec::new()
    }

    /// `GrantMinion{level, to}`：不经过发现与共享池，直接发放一张随从到手牌。
    ///
    /// - `level = next` 取 `level_override`（三连奖励在发牌时已算好来源星级 +1），
    ///   缺省用玩家当前时代的下一级，6 星封顶；`current` 与具体星级同 `GainMinion`。
    /// - 候选为进商店卡池、命中本局启用玩法的随从；随机只走 [`Stream::Effects`]。
    /// - 不扣共享池、不登记发现待选；手牌满或没有候选时不发放，效果牌照常消耗。
    /// - 发放后立即执行三连判定。
    pub(crate) fn apply_grant_minion(
        &self,
        state: &mut GameState,
        effect: &Effect,
        ctx: &EffectCtx,
    ) -> Vec<GameEvent> {
        let Some(player) = state.players.get(usize::from(ctx.seat)) else {
            return Vec::new();
        };
        if player.hand.len() >= self.catalog.config.hand_slots {
            return Vec::new();
        }
        let Params::GrantMinion(params) = &effect.params else {
            return Vec::new();
        };
        let star = match params.level.as_str() {
            "next" => ctx.level_override.unwrap_or(player.era as u8 + 1).min(6),
            "current" => ctx.level_override.unwrap_or(player.era as u8).min(6),
            other => match other.parse::<u8>() {
                Ok(star) => star,
                Err(_) => return Vec::new(),
            },
        };
        let candidates: Vec<&str> = self
            .catalog
            .cards
            .values()
            .filter(|card| {
                card.card_type == CardType::Minion
                    && card.in_shop_pool
                    && card.level == star
                    && card_in_pool(card, &state.enabled_playstyles)
            })
            .map(|card| card.id.as_str())
            .collect();
        if candidates.is_empty() {
            return Vec::new();
        }
        let index = state.rng.below(Stream::Effects, candidates.len() as u64) as usize;
        let card_id = candidates[index].to_string();
        let instance_id = state.next_instance_id;
        state.next_instance_id = instance_id.saturating_add(1);
        state.players[usize::from(ctx.seat)]
            .hand
            .push(HandCard::Minion(new_minion(
                &self.catalog,
                instance_id,
                &card_id,
                false,
            )));
        self.check_triple(state, ctx.seat)
    }

    /// `GainMinion{from, level, to}`：按 `level` 从商店获得一张随从，放到 `to`。
    ///
    /// - `level = current` 为玩家当前时代，其余为具体星级。
    /// - `from` 只接受 `shop`（省略同样按 `shop`）；其它取值在加载期已被拒绝
    ///   （`effects::parse`），这里不再做运行期兜底。
    /// - `from = shop`：在有效商店槽位中按卡牌目录顺序选第一张合格随从，清空该槽位，
    ///   共享池扣 1。没有合格槽位时不获得（不凭空从共享池取卡）。
    /// - `to = hand`：进入手牌，手牌满时不获得；`to = board`：进入战场最右侧，
    ///   棋盘满时不获得；`to = shop` 不是合法目标，不获得。
    ///
    /// 获得后与购买/发现一致立即执行三连判定。
    pub(crate) fn apply_gain_minion(
        &self,
        state: &mut GameState,
        effect: &Effect,
        ctx: &EffectCtx,
    ) -> Vec<GameEvent> {
        let Params::GainMinion(params) = &effect.params else {
            return Vec::new();
        };
        let Some(star) = state
            .players
            .get(usize::from(ctx.seat))
            .and_then(|player| level_star(&params.level, player.era))
        else {
            return Vec::new();
        };
        let to = match params.to {
            Zone::Hand => Zone::Hand,
            Zone::Board => Zone::Board,
            Zone::Shop => return Vec::new(),
        };
        let (hand_slots, board_slots) = (
            self.catalog.config.hand_slots,
            self.catalog.config.board_slots,
        );
        let Some(player) = state.players.get(usize::from(ctx.seat)) else {
            return Vec::new();
        };
        let full = match to {
            Zone::Hand => player.hand.len() >= hand_slots,
            Zone::Board => player.board.len() >= board_slots,
            Zone::Shop => true,
        };
        if full {
            return Vec::new();
        }

        // 加载期已保证 `from` 只能是 shop（或省略）。
        let Some((card_id, slot)) =
            first_shop_slot(&self.catalog, &state.enabled_playstyles, star, &player.shop)
        else {
            return Vec::new();
        };

        if let Some(count) = state.pool.get_mut(&card_id) {
            *count = count.saturating_sub(1);
        }
        let instance_id = state.next_instance_id;
        let card_instance_id = state.players[usize::from(ctx.seat)]
            .shop
            .get(slot)
            .and_then(Option::as_ref)
            .map(|slot| slot.instance_id)
            .unwrap_or(instance_id);
        let player = &mut state.players[usize::from(ctx.seat)];
        if let Some(shop_slot) = player.shop.get_mut(slot) {
            *shop_slot = None;
        }
        match to {
            Zone::Hand => player.hand.push(HandCard::Minion(new_minion(
                &self.catalog,
                card_instance_id,
                &card_id,
                false,
            ))),
            Zone::Board => {
                state.next_instance_id = instance_id.saturating_add(1);
                // 新随从进入战场：先登记进化，再结算它自己的战吼与友方入场触发。
                state.players[usize::from(ctx.seat)].board.push(new_minion(
                    &self.catalog,
                    instance_id,
                    &card_id,
                    false,
                ));
                self.register_evolve(state, ctx.seat, instance_id);
                let _ = self.fire(
                    state,
                    Trigger::OnPlay,
                    EffectCtx {
                        seat: ctx.seat,
                        source: Some(instance_id),
                        target: None,
                        event_target: None,
                        golden: false,
                        level_override: None,
                    },
                );
                self.fire_ally_summon_recruit(state, ctx.seat, instance_id);
                let events = self.check_triple(state, ctx.seat);
                self.fire_evolve_after_enter(state, ctx.seat, instance_id);
                return events;
            }
            Zone::Shop => unreachable!(),
        }
        // 获得随从后与购买/打出/发现一致，立即执行三连判定。
        self.check_triple(state, ctx.seat)
    }
}

/// 解析 `level` 参数：`current` 取玩家当前时代对应的星级。
fn level_star(level: &str, era: Era) -> Option<u8> {
    if level == "current" {
        return Some(era as u8);
    }
    level.parse::<u8>().ok()
}

/// 商店中按卡牌目录顺序第一张合格槽位：返回 `(卡牌 id, 槽位下标)`。
fn first_shop_slot(
    catalog: &Catalog,
    playstyles: &BTreeSet<Playstyle>,
    star: u8,
    shop: &[Option<ShopSlot>],
) -> Option<(String, usize)> {
    catalog
        .cards
        .values()
        .filter(|card| is_level_minion(catalog, &card.id, star, playstyles))
        .find_map(|card| {
            shop.iter()
                .position(|slot| slot.as_ref().is_some_and(|slot| slot.card_id == card.id))
                .map(|index| (card.id.clone(), index))
        })
}

/// 指定星级的合格随从：池内随从、该星级、[`card_in_pool`] 通过。
///
/// `neutral` 只保留给 Token 与内部哨兵，入池完全由 `play:` 决定，这里不再额外排除。
fn is_level_minion(
    catalog: &Catalog,
    card_id: &str,
    star: u8,
    playstyles: &BTreeSet<Playstyle>,
) -> bool {
    catalog.cards.get(card_id).is_some_and(|card| {
        card.card_type == CardType::Minion
            && card.in_shop_pool
            && card.level == star
            && card_in_pool(card, playstyles)
    })
}

/// 指定星级的「排序第一张」合格随从：池内随从、池中有货、[`card_in_pool`] 通过。
fn first_eligible(
    catalog: &Catalog,
    pool: &BTreeMap<String, u16>,
    playstyles: &BTreeSet<Playstyle>,
    star: u8,
) -> Option<String> {
    catalog
        .cards
        .values()
        .find(|card| {
            card.card_type == CardType::Minion
                && card.in_shop_pool
                && card.level == star
                && pool.get(&card.id).copied().unwrap_or(0) > 0
                && card_in_pool(card, playstyles)
        })
        .map(|card| card.id.clone())
}
