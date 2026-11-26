//! 观测编码：只含玩家可见信息，训练与推理共用。
//!
//! 明确不进入观测：`state.rng`、对手手牌、对手商店、共享牌池余量、未来随机数。

use rule_engine::data::Playstyle;
use rule_engine::keyword::{Keyword, parse_keyword};
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
/// 商店可购买性特征数：每槽 1 位。
pub const SHOP_AFFORD_FEATURES: usize = SHOP_SLOTS;
/// 关键词基名位数，位序见 [`rule_engine::keyword::ALL`]。
pub const KEYWORD_BITS: usize = 16;
/// 关键词叠加层数特征：`breed` / `assemble` / `armor`。
pub const KEYWORD_LAYERS: usize = 3;
/// 每个棋盘槽的关键词特征数。
pub const BOARD_KEYWORD_FEATURES: usize = KEYWORD_BITS + KEYWORD_LAYERS;
/// 英雄主动技能槽数（英雄最多两个主动技能）。
pub const HERO_POWER_SLOTS: usize = 2;
/// 英雄技能特征：`(技能索引, 剩余次数, 可用)` × 槽数 + 实际数量。
pub const HERO_POWER_FEATURES: usize = HERO_POWER_SLOTS * 3 + 1;
/// 每个棋盘槽的主动技能特征：`(有, 技能索引, 剩余次数, 可用)`。
pub const ABILITY_FEATURES: usize = 4;
/// 发现候选槽数（发现固定三选一）。
pub const DISCOVER_SLOTS: usize = 3;
/// 发现特征：候选卡索引 + 数量。
pub const DISCOVER_FEATURES: usize = DISCOVER_SLOTS + 1;
/// 目标待选候选数特征。
pub const TARGET_FEATURES: usize = 1;
/// 本回合对手特征：是否有对手 + 座位。
pub const OPPONENT_SEAT_FEATURES: usize = 2;
/// 剩余次数的归一化分母：数据中 `uses_per_turn` 上限为 2。
pub const MAX_USES_PER_TURN: f32 = 2.0;
/// 目标候选数的归一化分母：棋盘 7 + 手牌 10 + 商店 6。
pub const MAX_TARGET_CANDIDATES: f32 = 23.0;
/// 关键词叠加层数的归一化分母。
pub const KEYWORD_LAYER_DIVISOR: f32 = 3.0;

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

/// 一个英雄主动技能的编码。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HeroPower {
    /// 技能索引；没有该槽时为 `-1`。
    pub skill: i32,
    /// 剩余次数 `/ 2`。
    pub uses: f32,
    /// 本回合是否可用。
    pub can_use: f32,
}

impl Default for HeroPower {
    fn default() -> Self {
        Self {
            skill: -1,
            uses: 0.0,
            can_use: 0.0,
        }
    }
}

/// 棋盘随从主动技能的编码。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Ability {
    /// 1 表示该随从有主动技能。
    pub has: f32,
    /// 技能索引；没有技能为 `-1`。
    pub skill: i32,
    /// 剩余次数 `/ 2`。
    pub uses: f32,
    /// 本回合是否可用。
    pub can_use: f32,
}

impl Default for Ability {
    fn default() -> Self {
        Self {
            has: 0.0,
            skill: -1,
            uses: 0.0,
            can_use: 0.0,
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
    /// 商店各槽是否可购买（引擎 affordance 口径）。
    pub shop_buyable: [f32; SHOP_SLOTS],
    /// 棋盘各槽的关键词 multi-hot，位序见 `rule_engine::keyword::ALL`。
    pub board_keywords: [[f32; KEYWORD_BITS]; BOARD_SLOTS],
    /// 棋盘各槽的 `breed` / `assemble` / `armor` 层数，按 `/3` 归一。
    pub board_keyword_layers: [[f32; KEYWORD_LAYERS]; BOARD_SLOTS],
    /// 英雄主动技能。
    pub hero_powers: [HeroPower; HERO_POWER_SLOTS],
    /// 实际拥有的英雄主动技能数。
    pub hero_power_count: u8,
    /// 棋盘各槽的主动技能。
    pub board_abilities: [Ability; BOARD_SLOTS],
    /// 发现待选的候选卡索引；非发现待选为 `-1`。
    pub discover: [i32; DISCOVER_SLOTS],
    /// 发现候选数量。
    pub discover_count: u8,
    /// 目标待选的候选数量。
    pub target_candidates: u8,
    /// 本回合对手座位；未知为 `-1`。
    pub opponent_seat: i8,
}

impl Default for Observation {
    fn default() -> Self {
        Self {
            seat: 0,
            turn: 0,
            phase: 0,
            hero: -1,
            playstyles: 0,
            hp: 0,
            gold: 0,
            era: 0,
            shop: [Slot::default(); SHOP_SLOTS],
            hand: [Slot::default(); HAND_SLOTS],
            board: [Slot::default(); BOARD_SLOTS],
            players: [Opponent::default(); SEATS],
            pending: 0,
            shop_buyable: [0.0; SHOP_SLOTS],
            board_keywords: [[0.0; KEYWORD_BITS]; BOARD_SLOTS],
            board_keyword_layers: [[0.0; KEYWORD_LAYERS]; BOARD_SLOTS],
            hero_powers: [HeroPower::default(); HERO_POWER_SLOTS],
            hero_power_count: 0,
            board_abilities: [Ability::default(); BOARD_SLOTS],
            discover: [-1; DISCOVER_SLOTS],
            discover_count: 0,
            target_candidates: 0,
            opponent_seat: -1,
        }
    }
}

/// 观测向量的固定长度。
pub const OBS_DIM: usize = HEADER_FEATURES
    + SHOP_SLOTS * SLOT_FEATURES
    + HAND_SLOTS * SLOT_FEATURES
    + BOARD_SLOTS * SLOT_FEATURES
    + SEATS * PLAYER_FEATURES
    + SEATS * BOARD_SLOTS
    + SHOP_AFFORD_FEATURES
    + BOARD_SLOTS * BOARD_KEYWORD_FEATURES
    + HERO_POWER_FEATURES
    + BOARD_SLOTS * ABILITY_FEATURES
    + DISCOVER_FEATURES
    + TARGET_FEATURES
    + OPPONENT_SEAT_FEATURES;

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
        for buyable in &self.shop_buyable {
            out.push(*buyable);
        }
        for (keywords, layers) in self.board_keywords.iter().zip(&self.board_keyword_layers) {
            out.extend_from_slice(keywords);
            out.extend_from_slice(layers);
        }
        for power in &self.hero_powers {
            out.push(power.skill as f32);
            out.push(power.uses);
            out.push(power.can_use);
        }
        out.push(f32::from(self.hero_power_count));
        for ability in &self.board_abilities {
            out.push(ability.has);
            out.push(ability.skill as f32);
            out.push(ability.uses);
            out.push(ability.can_use);
        }
        for card in &self.discover {
            out.push(*card as f32);
        }
        out.push(f32::from(self.discover_count));
        out.push(f32::from(self.target_candidates) / MAX_TARGET_CANDIDATES);
        out.push(f32::from(u8::from(self.opponent_seat >= 0)));
        out.push(if self.opponent_seat >= 0 {
            f32::from(self.opponent_seat) / 7.0
        } else {
            -1.0
        });
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

        // 可购买性取引擎 affordance，不按金币自行推算，避免与引擎判定分叉。
        let mut shop_buyable = [0.0f32; SHOP_SLOTS];
        for (target, slot) in shop_buyable.iter_mut().zip(&me.shop) {
            let buyable = me
                .affordances
                .shop_slots
                .iter()
                .find(|affordance| affordance.index == slot.index)
                .is_some_and(|affordance| affordance.buyable);
            *target = f32::from(u8::from(buyable));
        }

        let mut board_keywords = [[0.0f32; KEYWORD_BITS]; BOARD_SLOTS];
        let mut board_keyword_layers = [[0.0f32; KEYWORD_LAYERS]; BOARD_SLOTS];
        for (index, minion) in me.board.iter().enumerate().take(BOARD_SLOTS) {
            let (bits, layers) = encode_keywords(&minion.keywords);
            board_keywords[index] = bits;
            board_keyword_layers[index] = layers;
        }

        let mut hero_powers = [HeroPower::default(); HERO_POWER_SLOTS];
        for (target, power) in hero_powers.iter_mut().zip(&me.hero_powers) {
            *target = HeroPower {
                skill: self.index.skill(&power.skill_id),
                uses: f32::from(power.uses_left) / MAX_USES_PER_TURN,
                can_use: f32::from(u8::from(power.can_use)),
            };
        }
        let hero_power_count = me.hero_powers.len().min(HERO_POWER_SLOTS) as u8;

        let mut board_abilities = [Ability::default(); BOARD_SLOTS];
        for (target, minion) in board_abilities.iter_mut().zip(&me.board) {
            if let Some(ability) = minion.abilities.first() {
                *target = Ability {
                    has: 1.0,
                    skill: self.index.skill(&ability.skill_id),
                    uses: f32::from(ability.uses_left) / MAX_USES_PER_TURN,
                    can_use: f32::from(u8::from(ability.can_use)),
                };
            }
        }

        let mut discover = [-1i32; DISCOVER_SLOTS];
        let mut discover_count = 0u8;
        let mut target_candidates = 0u8;
        match me.pending_choices.first() {
            Some(PendingChoiceView::Discover { candidates }) => {
                for (target, card_id) in discover.iter_mut().zip(candidates) {
                    *target = self.index.card(card_id);
                }
                discover_count = candidates.len().min(usize::from(u8::MAX)) as u8;
            }
            Some(PendingChoiceView::Target { candidates, .. }) => {
                target_candidates = candidates.len().min(usize::from(u8::MAX)) as u8;
            }
            Some(PendingChoiceView::Evolve { .. }) | None => {}
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
            shop_buyable,
            board_keywords,
            board_keyword_layers,
            hero_powers,
            hero_power_count,
            board_abilities,
            discover,
            discover_count,
            target_candidates,
            opponent_seat: view.opponent_seat.map(|seat| seat as i8).unwrap_or(-1),
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

/// 把视图里的关键词字符串编码成「基名位 + 叠加层数」。
///
/// `KeywordSet::names()` 对 `breed` / `assemble` / `armor` 输出带层数的串；
/// 这里按 [`parse_keyword`] 的口径拆开，未知串直接忽略（没有对应位）。
pub fn encode_keywords(names: &[String]) -> ([f32; KEYWORD_BITS], [f32; KEYWORD_LAYERS]) {
    let mut bits = [0.0f32; KEYWORD_BITS];
    let mut layers = [0.0f32; KEYWORD_LAYERS];
    for raw in names {
        let Some((keyword, layer)) = parse_keyword(raw) else {
            continue;
        };
        bits[keyword.index()] = 1.0;
        let layer_slot = match keyword {
            Keyword::Breed => 0,
            Keyword::Assemble => 1,
            Keyword::Armor => 2,
            _ => continue,
        };
        layers[layer_slot] = f32::from(layer) / KEYWORD_LAYER_DIVISOR;
    }
    (bits, layers)
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
