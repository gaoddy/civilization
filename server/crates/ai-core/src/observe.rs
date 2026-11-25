//! 观测编码：只含玩家可见信息，训练与推理共用。
//!
//! 明确不进入观测：`state.rng`、对手手牌、对手商店、共享牌池余量、未来随机数。

use rule_engine::data::Playstyle;
use rule_engine::state::{Phase, Seat};
use rule_engine::view::{PendingChoiceView, SeatView};
use serde::{Deserialize, Serialize};

use crate::catalog::CardIndex;
use crate::history::History;

pub const SHOP_SLOTS: usize = 6;
pub const HAND_SLOTS: usize = 10;
pub const BOARD_SLOTS: usize = 7;
pub const SEATS: usize = 8;
/// 每槽位特征数：卡索引、攻击、生命、金色、冻结、有效。
pub const SLOT_FEATURES: usize = 6;
/// 每个座位摘要的特征数：存活、血量、时代、上阵数、英雄、名次。
pub const PLAYER_FEATURES: usize = 6;
/// 框头特征数：座位、回合、阶段、英雄、玩法掩码、血量、金币、时代。
pub const HEADER_FEATURES: usize = 8;

/// 单个槽位（商店/手牌/棋盘）的编码。
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Slot {
    /// 卡牌索引；空槽为 `-1`。
    pub card: i32,
    pub attack: f32,
    pub health: f32,
    pub golden: f32,
    pub frozen: f32,
    /// 1 表示该槽位存在。
    pub valid: f32,
}

/// 一个座位的公开摘要。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Opponent {
    pub alive: f32,
    pub hp: f32,
    pub era: f32,
    pub board_count: f32,
    /// 英雄索引；未选英雄为 `-1`。
    pub hero: i32,
    /// 实时名次；未进入排名为 0。
    pub rank: f32,
    /// 本人亲自见到的最近棋盘；没见过为全 `-1`。
    pub last_board: [i32; BOARD_SLOTS],
}

impl Default for Opponent {
    fn default() -> Self {
        Self {
            alive: 0.0,
            hp: 0.0,
            era: 0.0,
            board_count: 0.0,
            hero: -1,
            rank: 0.0,
            last_board: [-1; BOARD_SLOTS],
        }
    }
}

/// 一个决策时刻的完整观测。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub seat: Seat,
    pub turn: u8,
    /// 阶段编码：0 Lobby、1 Recruit、2 Combat、3 Settle、4 GameOver。
    pub phase: u8,
    pub hero: i32,
    /// 本局 5 个玩法的位掩码，位序同 [`Playstyle::ALL`]。
    pub playstyles: u16,
    pub hp: i32,
    pub gold: u8,
    pub era: u8,
    pub shop: [Slot; SHOP_SLOTS],
    pub hand: [Slot; HAND_SLOTS],
    pub board: [Slot; BOARD_SLOTS],
    /// 全部座位的公开摘要，按座位升序；本人也在其中。
    pub players: [Opponent; SEATS],
    /// 待选类型：0 无、1 发现、2 进化、3 目标。
    pub pending: u8,
}

/// 观测向量的固定长度。
pub const OBS_DIM: usize = HEADER_FEATURES
    + SHOP_SLOTS * SLOT_FEATURES
    + HAND_SLOTS * SLOT_FEATURES
    + BOARD_SLOTS * SLOT_FEATURES
    + SEATS * PLAYER_FEATURES
    + SEATS * BOARD_SLOTS;

impl Observation {
    /// 展平成固定顺序的向量；归一化常数见 [`Encoder`]。
    pub fn to_vec(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(OBS_DIM);
        out.push(f32::from(self.seat));
        out.push(f32::from(self.turn) / 20.0);
        out.push(f32::from(self.phase));
        out.push(self.hero as f32);
        out.push(f32::from(self.playstyles));
        out.push(self.hp as f32 / 40.0);
        out.push(f32::from(self.gold) / 10.0);
        out.push(f32::from(self.era) / 7.0);

        for slot in &self.shop {
            push_slot(&mut out, slot);
        }
        for slot in &self.hand {
            push_slot(&mut out, slot);
        }
        for slot in &self.board {
            push_slot(&mut out, slot);
        }
        for player in &self.players {
            out.push(player.alive);
            out.push(player.hp / 40.0);
            out.push(player.era / 7.0);
            out.push(player.board_count / 7.0);
            out.push(player.hero as f32);
            out.push(player.rank / 8.0);
        }
        for player in &self.players {
            for card in &player.last_board {
                out.push(*card as f32);
            }
        }
        debug_assert_eq!(out.len(), OBS_DIM);
        out
    }
}

fn push_slot(out: &mut Vec<f32>, slot: &Slot) {
    out.push(slot.card as f32);
    out.push(slot.attack / 50.0);
    out.push(slot.health / 50.0);
    out.push(slot.golden);
    out.push(slot.frozen);
    out.push(slot.valid);
}

/// 把视图编码成观测。
#[derive(Debug, Clone)]
pub struct Encoder {
    index: CardIndex,
}

impl Encoder {
    pub fn new(index: CardIndex) -> Self {
        Self { index }
    }

    pub fn index(&self) -> &CardIndex {
        &self.index
    }

    pub fn encode(&self, view: &SeatView, history: &History) -> Observation {
        let me = &view.self_view;
        let mut shop = [Slot::default(); SHOP_SLOTS];
        for (target, slot) in shop.iter_mut().zip(&me.shop) {
            if let Some(card_id) = slot.card_id.as_deref() {
                *target = Slot {
                    card: self.index.card(card_id),
                    frozen: f32::from(u8::from(slot.frozen)),
                    valid: 1.0,
                    ..Slot::default()
                };
            }
        }

        let mut hand = [Slot::default(); HAND_SLOTS];
        for (target, slot) in hand.iter_mut().zip(&me.hand) {
            *target = Slot {
                card: self.index.card(&slot.card_id),
                attack: slot.attack.unwrap_or(0) as f32,
                health: slot.health.unwrap_or(0) as f32,
                golden: f32::from(u8::from(slot.golden)),
                frozen: 0.0,
                valid: 1.0,
            };
        }

        let mut board = [Slot::default(); BOARD_SLOTS];
        for (target, slot) in board.iter_mut().zip(&me.board) {
            *target = Slot {
                card: self.index.card(&slot.card_id),
                attack: slot.attack as f32,
                health: slot.health as f32,
                golden: f32::from(u8::from(slot.golden)),
                frozen: 0.0,
                valid: 1.0,
            };
        }

        let mut players = [Opponent::default(); SEATS];
        for summary in &view.players {
            let seat = usize::from(summary.seat);
            if seat >= SEATS {
                continue;
            }
            players[seat] = Opponent {
                alive: f32::from(u8::from(summary.alive)),
                hp: summary.hp as f32,
                era: f32::from(summary.era),
                board_count: f32::from(summary.board_count),
                hero: summary
                    .hero_card_id
                    .as_deref()
                    .map(|id| self.index.hero(id))
                    .unwrap_or(-1),
                rank: summary.rank.map(f32::from).unwrap_or(0.0),
                last_board: history.board(summary.seat),
            };
        }

        Observation {
            seat: view.seat,
            turn: view.turn,
            phase: phase_code(view.phase),
            hero: me
                .hero_card_id
                .as_deref()
                .map(|id| self.index.hero(id))
                .unwrap_or(-1),
            playstyles: playstyle_mask(&view.enabled_playstyles),
            hp: me.hp,
            gold: me.gold,
            era: me.era,
            shop,
            hand,
            board,
            players,
            pending: pending_code(me.pending_choices.first()),
        }
    }
}

fn phase_code(phase: Phase) -> u8 {
    match phase {
        Phase::Lobby => 0,
        Phase::Recruit => 1,
        Phase::Combat => 2,
        Phase::Settle => 3,
        Phase::GameOver => 4,
    }
}

fn pending_code(pending: Option<&PendingChoiceView>) -> u8 {
    match pending {
        None => 0,
        Some(PendingChoiceView::Discover { .. }) => 1,
        Some(PendingChoiceView::Evolve { .. }) => 2,
        Some(PendingChoiceView::Target { .. }) => 3,
    }
}

/// 本局玩法集合的位掩码；位序与 [`Playstyle::ALL`] 一致。
pub fn playstyle_mask(enabled: &[String]) -> u16 {
    let mut mask = 0u16;
    for (bit, playstyle) in Playstyle::ALL.iter().enumerate() {
        if enabled.iter().any(|id| id == playstyle.id()) {
            mask |= 1 << bit;
        }
    }
    mask
}
