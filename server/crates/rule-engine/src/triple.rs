//! 三连合成与发现待选。
//!
//! 三连只统计棋盘与手牌中的同名非金色随从；效果牌、金色随从与共享池都不参与。
//! 每次获得随从后调用 [`Engine::check_triple`]，直到不再产生新的三连。
//! 合成结果是一张金色随从：基底为最早获得（实例编号最小）的副本，完整继承其状态；
//! 金色进入手牌，不立即触发发现——发现延后到金色打出后由「三连奖励」牌发起。
//! `Op::Discover` 的三选一与奖励牌发现共用同一候选抽取口径。

use std::collections::BTreeSet;

use crate::data::{Card, CardType, Catalog, Playstyle, card_in_pool};
use crate::effects::{Effect, EffectCtx};
use crate::engine::Engine;
use crate::events::GameEvent;
use crate::rng::Stream;
use crate::state::{
    GameState, HandCard, InstanceId, MinionInstance, PendingChoice, PlayerState, Seat, base_stats,
};

/// 一次发现提供的候选上限。
const DISCOVER_CANDIDATES: usize = 3;

/// 「三连奖励」效果牌 id；不进商店、不作为任何发现候选、不扣共享牌池。
pub const REWARD_CARD_ID: &str = "triple-reward";

impl Engine {
    /// 反复合成三连，直到该座位不再有三张同名非金色随从。
    ///
    /// 每合成一次推入 `TripleFormed` 事件；金色进入手牌，不再登记发现待选。
    /// 只读卡牌目录，所有状态变化都写入传入的 `state`。
    pub fn check_triple(&self, state: &mut GameState, seat: Seat) -> Vec<GameEvent> {
        let mut events = Vec::new();
        while let Some((card_id, copies)) = self.find_triple(state, seat) {
            let Some(golden) = self.form_golden(state, seat, &copies) else {
                break;
            };
            events.push(GameEvent::TripleFormed {
                seat,
                card_id,
                golden_instance_id: golden.instance_id,
                sources: copies.clone(),
            });
        }
        events
    }

    /// 把一组三张副本合成一张金色，返回金色实例。
    ///
    /// 基底为实例编号最小的副本（即最先获得的那张），完整继承其成长、繁衍、
    /// 传染、护盾与已授予关键词；攻击与生命的卡面基础值按金色 ×2 重设，
    /// 其余层数原样保留。金色作为新实例进入手牌，三张副本全部离场。
    /// 手牌已满时金色留在基底原来的位置（棋盘上或手牌里），没有经过打出，
    /// 因此不发放奖励牌（产品 §2 兜底）。
    fn form_golden(
        &self,
        state: &mut GameState,
        seat: Seat,
        copies: &[InstanceId],
    ) -> Option<MinionInstance> {
        let player = state.players.get(usize::from(seat))?;
        let base_id = *copies.iter().min()?;
        let hand_slots = self.catalog.config.hand_slots;
        let base_board_index = player
            .board
            .iter()
            .position(|minion| minion.instance_id == base_id);
        let new_id = state.next_instance_id;
        state.next_instance_id = new_id.saturating_add(1);

        let player = &mut state.players[usize::from(seat)];
        let mut golden = match take_from_board(player, base_id) {
            Some(minion) => minion,
            None => take_from_hand(player, base_id)?,
        };
        remove_copies(player, copies, base_id);

        // 三张副本已离场，再判断手牌是否真的没有空位（只有三张都在棋盘时才可能满）。
        let hand_full = player.hand.len() >= hand_slots;
        upgrade_to_golden(&self.catalog, &mut golden);
        golden.instance_id = new_id;

        if hand_full {
            // 兜底：金色留在基底原位置，不进手牌，因此没有「从手牌打出」这一步。
            let index = base_board_index
                .unwrap_or(player.board.len())
                .min(player.board.len());
            player.board.insert(index, golden);
        } else {
            player.hand.push(HandCard::Minion(golden));
        }
        player
            .board
            .iter()
            .chain(player.hand.iter().filter_map(HandCard::minion))
            .find(|minion| minion.instance_id == new_id)
            .cloned()
    }

    /// 从指定星级的合格池中无放回抽取最多三张不重复候选。
    ///
    /// 合格池按卡牌 id 顺序枚举，随机只走 [`Stream::Discover`]，且每抽一张推进一次；
    /// 三连发现与效果发现共用这一口径（关键词机制 §8 的「三选一」）。
    fn draw_discover_candidates(&self, state: &mut GameState, level: u8) -> Vec<String> {
        let mut remaining: Vec<String> = self
            .catalog
            .cards
            .values()
            .filter(|candidate| {
                is_discover_candidate(candidate, level, &state.enabled_playstyles, &state.pool)
            })
            .map(|candidate| candidate.id.clone())
            .collect();

        let mut chosen = Vec::with_capacity(DISCOVER_CANDIDATES);
        while chosen.len() < DISCOVER_CANDIDATES && !remaining.is_empty() {
            let index = state.rng.below(Stream::Discover, remaining.len() as u64) as usize;
            chosen.push(remaining.remove(index));
        }
        chosen
    }

    /// 执行招募侧 `Discover{pool, choose}`：抽取候选并登记一条发现待选。
    ///
    /// - `pool` 只认 `playstyle`；省略或其它取值等价，候选始终按本局启用玩法过滤。
    /// - 星级与玩家当前时代相同（本局时代 = 商店等级，见核心规则 §4.1）。
    /// - 没有任何合格候选时不登记待选。
    /// - 抽取时立即推进 [`Stream::Discover`] 并写入候选，重放结果确定；
    ///   被选择时才扣 1 份共享池（`choose_discover`）。
    pub(crate) fn apply_discover(
        &self,
        state: &mut GameState,
        effect: &Effect,
        ctx: &EffectCtx,
    ) -> Vec<GameEvent> {
        // 奖励牌锁定合成时的来源星级 +1；其它发现来源按玩家当前时代。
        let level = match ctx.level_override {
            Some(level) => level.min(6),
            None => match state.players.get(usize::from(ctx.seat)) {
                Some(player) => player.era as u8,
                None => return Vec::new(),
            },
        };
        if effect.params.choose.unwrap_or(1) == 0 {
            return Vec::new();
        }
        let candidates = self.draw_discover_candidates(state, level);
        if candidates.is_empty() {
            return Vec::new();
        }
        state.pending.push_back(PendingChoice::Discover {
            seat: ctx.seat,
            candidates,
            source: ctx.source.unwrap_or(0),
        });
        Vec::new()
    }

    /// 找出该座位最早出现、且至少有三张非金色副本的随从卡牌。
    ///
    /// 棋盘与手牌都要在目录中查得到、为随从且进商店卡池；效果牌、非池卡、
    /// 缺失 id 与金色随从都不参与。返回按实例编号升序的前三张副本编号，编号即获得先后。
    fn find_triple(&self, state: &GameState, seat: Seat) -> Option<(String, Vec<InstanceId>)> {
        let player = state.players.get(usize::from(seat))?;
        let is_minion = |card_id: &str| {
            self.catalog
                .cards
                .get(card_id)
                .is_some_and(|card| card.card_type == CardType::Minion && card.in_shop_pool)
        };
        let mut entries: Vec<(&str, InstanceId)> = Vec::new();
        for minion in &player.board {
            if !minion.golden && is_minion(&minion.card_id) {
                entries.push((minion.card_id.as_str(), minion.instance_id));
            }
        }
        for card in &player.hand {
            if let Some(minion) = card.minion()
                && !minion.golden
                && is_minion(&minion.card_id)
            {
                entries.push((minion.card_id.as_str(), minion.instance_id));
            }
        }

        let mut examined: BTreeSet<&str> = BTreeSet::new();
        for (card_id, _) in &entries {
            if !examined.insert(card_id) {
                continue;
            }
            let mut copies: Vec<InstanceId> = entries
                .iter()
                .filter(|(entry_id, _)| entry_id == card_id)
                .map(|(_, id)| *id)
                .collect();
            copies.sort_unstable();
            if copies.len() >= 3 {
                copies.truncate(3);
                return Some((card_id.to_string(), copies));
            }
        }
        None
    }
}

/// 从棋盘取出指定编号的随从。
fn take_from_board(player: &mut PlayerState, instance_id: InstanceId) -> Option<MinionInstance> {
    let index = player
        .board
        .iter()
        .position(|minion| minion.instance_id == instance_id)?;
    Some(player.board.remove(index))
}

/// 从手牌按实例编号取出随从牌；效果牌不会被选中。
fn take_from_hand(player: &mut PlayerState, instance_id: InstanceId) -> Option<MinionInstance> {
    let index = player
        .hand
        .iter()
        .position(|card| card.instance_id() == instance_id && card.minion().is_some())?;
    match player.hand.remove(index) {
        HandCard::Minion(minion) => Some(minion),
        HandCard::Effect { .. } => None,
    }
}

/// 移除除基底以外的三连副本；基底已由调用方取出。
fn remove_copies(player: &mut PlayerState, copies: &[InstanceId], base_id: InstanceId) {
    player
        .board
        .retain(|minion| minion.instance_id == base_id || !copies.contains(&minion.instance_id));
    player
        .hand
        .retain(|card| card.instance_id() == base_id || !copies.contains(&card.instance_id()));
}

/// 把实例升级为金色：基础值按卡面翻倍，清空本回合行动状态。
///
/// 成长、永久与回合层保留；`current` 随 `set_initial` 同步调整。
fn upgrade_to_golden(catalog: &Catalog, minion: &mut MinionInstance) {
    let (attack, health) = base_stats(catalog, &minion.card_id, true).unwrap_or((0, 0));
    minion.golden = true;
    minion.stats.attack.set_initial(attack);
    minion.stats.health.set_initial(health);
    minion.attacks_left = 0;
    minion.cannot_attack = false;
}

/// 发现候选：池内随从、指定星级、[`card_in_pool`] 通过、共享池有剩余。
fn is_discover_candidate(
    card: &Card,
    level: u8,
    enabled_playstyles: &BTreeSet<Playstyle>,
    pool: &std::collections::BTreeMap<String, u16>,
) -> bool {
    card.card_type == CardType::Minion
        && card.in_shop_pool
        && card.level == level
        && card_in_pool(card, enabled_playstyles)
        && pool.get(&card.id).copied().unwrap_or(0) > 0
}
