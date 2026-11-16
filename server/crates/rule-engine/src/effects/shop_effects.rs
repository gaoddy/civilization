//! 商店与经济类效果：刷新商店、增加商店槽位、获得随从。
//!
//! 对应关键词机制 §8 的 `RefreshShop` / `AddShopSlot` / `GainMinion`。
//! 追加槽位与获得随从都取「当前星级排序第一张」合格随从（卡牌目录 `id` 顺序），
//! 选择与遍历顺序确定，不额外消耗随机流；没有合格卡牌时静默跳过。
//! `GainMinion` 的来源与目标区域见 [`Engine::apply_gain_minion`]。

use std::collections::{BTreeMap, BTreeSet};

use crate::data::{CardType, Catalog, Era, Tribe};
use crate::effects::{Effect, EffectCtx, Trigger, Zone};
use crate::engine::Engine;
use crate::events::GameEvent;
use crate::recruit::new_minion;
use crate::state::{GameState, HandCard, ShopSlot};

/// 商店槽位硬上限；追加槽位不得越过该值。
const MAX_SHOP_SLOTS: usize = 7;

impl Engine {
    /// `RefreshShop{free}`：立即重抽当前座位商店。
    ///
    /// `free = true` 不扣金币、不消耗本回合免费刷新额度；无论免费与否，
    /// 都通过与手动刷新相同的每回合闸门触发 `OnFirstRefresh`。
    pub(crate) fn apply_refresh_shop(
        &mut self,
        state: &mut GameState,
        effect: &Effect,
        ctx: &EffectCtx,
    ) -> Vec<GameEvent> {
        if state.players.get(usize::from(ctx.seat)).is_none() {
            return Vec::new();
        }
        if effect.params.free.unwrap_or(false) {
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
        &mut self,
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
        let count = usize::try_from(effect.params.count.unwrap_or(1)).unwrap_or(0);

        for _ in 0..count {
            let full = state
                .players
                .get(usize::from(ctx.seat))
                .is_none_or(|player| player.shop.len() >= MAX_SHOP_SLOTS);
            if full {
                break;
            }
            let Some(card_id) =
                first_eligible(&self.catalog, &state.pool, &state.enabled_tribes, star)
            else {
                break;
            };
            if let Some(pool) = state.pool.get_mut(&card_id) {
                *pool = pool.saturating_sub(1);
            }
            if let Some(player) = state.players.get_mut(usize::from(ctx.seat)) {
                player.shop.push(Some(ShopSlot {
                    card_id,
                    frozen: false,
                }));
            }
        }
        Vec::new()
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
        &mut self,
        state: &mut GameState,
        effect: &Effect,
        ctx: &EffectCtx,
    ) -> Vec<GameEvent> {
        let Some(star) = effect.params.level.as_deref().and_then(|level| {
            let player = state.players.get(usize::from(ctx.seat))?;
            level_star(level, player.era)
        }) else {
            return Vec::new();
        };
        let to = match effect.params.to {
            Some(Zone::Hand) => Zone::Hand,
            Some(Zone::Board) => Zone::Board,
            Some(Zone::Shop) | None => return Vec::new(),
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
            first_shop_slot(&self.catalog, &state.enabled_tribes, star, &player.shop)
        else {
            return Vec::new();
        };

        if let Some(count) = state.pool.get_mut(&card_id) {
            *count = count.saturating_sub(1);
        }
        let instance_id = state.next_instance_id;
        let player = &mut state.players[usize::from(ctx.seat)];
        if let Some(shop_slot) = player.shop.get_mut(slot) {
            *shop_slot = None;
        }
        match to {
            Zone::Hand => player.hand.push(HandCard {
                card_id,
                golden: false,
            }),
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
    tribes: &BTreeSet<Tribe>,
    star: u8,
    shop: &[Option<ShopSlot>],
) -> Option<(String, usize)> {
    catalog
        .cards
        .values()
        .filter(|card| is_level_minion(catalog, &card.id, star, tribes))
        .find_map(|card| {
            shop.iter()
                .position(|slot| slot.as_ref().is_some_and(|slot| slot.card_id == card.id))
                .map(|index| (card.id.clone(), index))
        })
}

/// 指定星级的合格随从：随从、非英雄、该星级、流派启用或通用。
///
/// 学者是英雄专属流派（没有可购买的学者随从），获得时也排除。
fn is_level_minion(catalog: &Catalog, card_id: &str, star: u8, tribes: &BTreeSet<Tribe>) -> bool {
    catalog.cards.get(card_id).is_some_and(|card| {
        card.card_type == CardType::Minion
            && !card.is_hero
            && card.level == star
            && card.tribe != Tribe::Scholar
            && (card.tribe == Tribe::General || tribes.contains(&card.tribe))
    })
}

/// 指定星级的「排序第一张」合格随从：随从、非英雄、池中有货、流派启用或通用。
fn first_eligible(
    catalog: &Catalog,
    pool: &BTreeMap<String, u16>,
    tribes: &BTreeSet<Tribe>,
    star: u8,
) -> Option<String> {
    catalog
        .cards
        .values()
        .find(|card| {
            card.card_type == CardType::Minion
                && !card.is_hero
                && card.level == star
                && pool.get(&card.id).copied().unwrap_or(0) > 0
                && (card.tribe == Tribe::General || tribes.contains(&card.tribe))
        })
        .map(|card| card.id.clone())
}
