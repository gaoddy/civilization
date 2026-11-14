//! 三连合成与发现待选。
//!
//! 三连只统计棋盘与手牌中的同名非金色随从；效果牌、金色随从与共享池都不参与。
//! 每次获得随从后调用 [`Engine::check_triple`]，直到不再产生新的三连。

use std::collections::BTreeSet;

use crate::data::{Card, CardType, Tribe};
use crate::engine::Engine;
use crate::events::GameEvent;
use crate::recruit::new_minion;
use crate::rng::Stream;
use crate::state::{GameState, InstanceId, MinionInstance, PendingChoice, Seat};

/// 一次发现提供的候选上限。
const DISCOVER_CANDIDATES: usize = 3;

/// 一次三连中某张副本的原始位置。
#[derive(Debug, Clone, Copy)]
enum Copy {
    Board(InstanceId),
    Hand(usize),
}

impl Engine {
    /// 反复合成三连，直到该座位不再有三张同名非金色随从。
    ///
    /// 每合成一次推入 `TripleFormed` 事件，并在存在候选时为该座位追加一个发现待选。
    /// 只读卡牌目录，所有状态变化都写入传入的 `state`。
    pub fn check_triple(&self, state: &mut GameState, seat: Seat) -> Vec<GameEvent> {
        let mut events = Vec::new();
        while let Some((card_id, copies)) = find_triple(state, seat) {
            let Some(source) = self.form_golden(state, seat, &card_id, &copies) else {
                break;
            };
            events.push(GameEvent::TripleFormed {
                seat,
                card_id: card_id.clone(),
            });
            let candidates = self.discover_candidates(state, &card_id);
            if let Some(candidates) = candidates
                && !candidates.is_empty()
            {
                state.pending.push_back(PendingChoice::Discover {
                    seat,
                    candidates,
                    source,
                });
            }
        }
        events
    }

    /// 把一组三张副本合成一张金色，返回金色随从的实例编号。
    ///
    /// 最早副本作为基底，保留其成长、增殖、护盾等全部字段；棋盘上的基底留在原位置。
    /// 三张都在手牌时，棋盘有空位则把金色放到棋盘末尾，否则留在最早的手牌位置。
    fn form_golden(
        &self,
        state: &mut GameState,
        seat: Seat,
        card_id: &str,
        copies: &[Copy],
    ) -> Option<InstanceId> {
        let base = *copies.first()?;
        let board_slots = self.catalog.config.board_slots;
        let player = state.players.get_mut(usize::from(seat))?;
        let mut hand_removals: Vec<usize> = Vec::new();

        let source = match base {
            Copy::Board(base_id) => {
                if let Some(minion) = player
                    .board
                    .iter_mut()
                    .find(|minion| minion.instance_id == base_id)
                {
                    make_golden(minion);
                }
                for copy in &copies[1..] {
                    match copy {
                        Copy::Board(instance_id) => {
                            player
                                .board
                                .retain(|minion| minion.instance_id != *instance_id);
                        }
                        Copy::Hand(index) => hand_removals.push(*index),
                    }
                }
                base_id
            }
            Copy::Hand(base_index) => {
                if player.board.len() < board_slots {
                    let new_id = state.next_instance_id;
                    state.next_instance_id = new_id.saturating_add(1);
                    player.board.push(new_minion(new_id, card_id, true));
                    for copy in copies {
                        if let Copy::Hand(index) = copy {
                            hand_removals.push(*index);
                        }
                    }
                    new_id
                } else {
                    if let Some(card) = player.hand.get_mut(base_index) {
                        card.golden = true;
                    }
                    for copy in &copies[1..] {
                        if let Copy::Hand(index) = copy {
                            hand_removals.push(*index);
                        }
                    }
                    // 金色留在手牌，没有随从实例；为待选来源分配一个编号。
                    let source = state.next_instance_id;
                    state.next_instance_id = source.saturating_add(1);
                    source
                }
            }
        };

        hand_removals.sort_unstable_by(|left, right| right.cmp(left));
        for index in hand_removals {
            if index < player.hand.len() {
                player.hand.remove(index);
            }
        }
        Some(source)
    }

    /// 为该卡面星级 +1（6 星封顶）抽取最多三张不重复的发现候选。
    fn discover_candidates(&self, state: &mut GameState, card_id: &str) -> Option<Vec<String>> {
        let card = self.catalog.cards.get(card_id)?;
        let level = card.level.saturating_add(1).min(6);
        let mut remaining: Vec<String> = self
            .catalog
            .cards
            .values()
            .filter(|candidate| {
                is_discover_candidate(candidate, level, &state.enabled_tribes, &state.pool)
            })
            .map(|candidate| candidate.id.clone())
            .collect();

        let mut chosen = Vec::with_capacity(DISCOVER_CANDIDATES);
        while chosen.len() < DISCOVER_CANDIDATES && !remaining.is_empty() {
            let index = state.rng.below(Stream::Discover, remaining.len() as u64) as usize;
            chosen.push(remaining.remove(index));
        }
        Some(chosen)
    }
}

/// 把实例原地升级为金色：保留全部成长与来源字段，清空本回合行动状态。
fn make_golden(minion: &mut MinionInstance) {
    minion.golden = true;
    minion.attacks_left = 0;
    minion.cannot_attack = false;
}

/// 找出该座位最早出现、且至少有三张非金色副本的卡牌。
fn find_triple(state: &GameState, seat: Seat) -> Option<(String, Vec<Copy>)> {
    let player = state.players.get(usize::from(seat))?;
    let mut entries: Vec<(&str, Copy)> = Vec::new();
    for minion in &player.board {
        if !minion.golden {
            entries.push((minion.card_id.as_str(), Copy::Board(minion.instance_id)));
        }
    }
    for (index, card) in player.hand.iter().enumerate() {
        if !card.golden {
            entries.push((card.card_id.as_str(), Copy::Hand(index)));
        }
    }

    let mut examined: BTreeSet<&str> = BTreeSet::new();
    for (card_id, _) in &entries {
        if !examined.insert(card_id) {
            continue;
        }
        let copies: Vec<Copy> = entries
            .iter()
            .filter(|(entry_id, _)| entry_id == card_id)
            .take(3)
            .map(|(_, copy)| *copy)
            .collect();
        if copies.len() == 3 {
            return Some((card_id.to_string(), copies));
        }
    }
    None
}

/// 发现候选：随从、非英雄、指定星级、流派启用或通用、共享池有剩余。
fn is_discover_candidate(
    card: &Card,
    level: u8,
    enabled_tribes: &BTreeSet<Tribe>,
    pool: &std::collections::BTreeMap<String, u16>,
) -> bool {
    card.card_type == CardType::Minion
        && !card.is_hero
        && card.level == level
        && (card.tribe == Tribe::General || enabled_tribes.contains(&card.tribe))
        && pool.get(&card.id).copied().unwrap_or(0) > 0
}
