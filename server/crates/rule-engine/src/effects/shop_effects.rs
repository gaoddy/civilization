//! 商店与经济类效果：刷新商店、增加商店槽位、获得随从。
//!
//! 对应关键词机制 §8 的 `RefreshShop` / `AddShopSlot` / `GainMinion`。
//! 追加槽位与获得随从都取「当前星级排序第一张」合格随从（卡牌目录 `id` 顺序），
//! 选择与遍历顺序确定，不额外消耗随机流；没有合格卡牌时静默跳过。

use std::collections::{BTreeMap, BTreeSet};

use crate::data::{CardType, Catalog, Tribe};
use crate::effects::{Effect, EffectCtx, Zone};
use crate::engine::Engine;
use crate::events::GameEvent;
use crate::state::{GameState, HandCard, ShopSlot};

/// 商店槽位硬上限；追加槽位不得越过该值。
const MAX_SHOP_SLOTS: usize = 7;

impl Engine {
    /// `RefreshShop{free}`：立即重抽当前座位商店。
    ///
    /// `free = true` 不扣金币、不消耗本回合免费刷新额度，也不触发
    /// `OnFirstRefresh`；否则走普通刷新路径（金币与首次刷新触发一致）。
    pub(crate) fn apply_refresh_shop(
        &mut self,
        state: &mut GameState,
        effect: &Effect,
        ctx: &EffectCtx,
    ) -> Vec<GameEvent> {
        if effect.params.free.unwrap_or(false) {
            crate::recruit::roll_player(self, state, ctx.seat, false);
            vec![GameEvent::ShopRolled { seat: ctx.seat }]
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
                continue;
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

    /// `GainMinion{from, level, to}`：把当前星级排序第一张合格随从放入手牌。
    ///
    /// `level = current` 表示玩家当前时代；手牌已满时不做任何修改。`from = shop`
    /// 仅是说明性参数：共享池只扣一次，也不移除任何商店槽位。
    pub(crate) fn apply_gain_minion(
        &mut self,
        state: &mut GameState,
        effect: &Effect,
        ctx: &EffectCtx,
    ) -> Vec<GameEvent> {
        if effect.params.to != Some(Zone::Hand) {
            return Vec::new();
        }
        let Some(player) = state.players.get(usize::from(ctx.seat)) else {
            return Vec::new();
        };
        let star = match effect.params.level.as_deref() {
            Some("current") => player.era as u8,
            Some(raw) => match raw.parse::<u8>() {
                Ok(level) => level,
                Err(_) => return Vec::new(),
            },
            None => return Vec::new(),
        };
        if player.hand.len() >= self.catalog.config.hand_slots {
            return Vec::new();
        }

        let Some(card_id) = first_eligible(&self.catalog, &state.pool, &state.enabled_tribes, star)
        else {
            return Vec::new();
        };
        if let Some(pool) = state.pool.get_mut(&card_id) {
            *pool = pool.saturating_sub(1);
        }
        if let Some(player) = state.players.get_mut(usize::from(ctx.seat)) {
            player.hand.push(HandCard {
                card_id,
                golden: false,
            });
        }
        Vec::new()
    }
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
