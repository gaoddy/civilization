//! 状态哈希：对规范化编码做 FNV-1a 64，用于确定性校验。
//!
//! 编码约定：定长整数按小端写入；字符串与集合先写长度前缀；枚举写固定标签；
//! `Option` 先写存在标记。所有遍历都依赖 `BTreeMap` / `BTreeSet` / `Vec`
//! 的确定顺序，不包含地址、时间等非确定内容。随机流状态属于确定性状态，一并计入。

use crate::data::Era;
use crate::effects::Duration;
use crate::rng::Stream;
use crate::state::{GameState, MinionInstance, Modifier, Phase, PlayerState};

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// 计算对局状态哈希。
pub fn state_hash(state: &GameState) -> u64 {
    let mut hasher = Hasher::new();

    hasher.u8(state.turn);
    hasher.u8(phase_tag(state.phase));
    hasher.u64(state.first_seat.into());
    hasher.u64(state.next_instance_id);

    hasher.len(state.players.len());
    for player in &state.players {
        hash_player(&mut hasher, player);
    }

    hasher.len(state.pool.len());
    for (card_id, count) in &state.pool {
        hasher.str(card_id);
        hasher.u16(*count);
    }

    for stream in Stream::ALL {
        for word in state.rng.words(stream) {
            hasher.u64(word);
        }
    }

    hasher.len(state.enabled_tribes.len());
    for tribe in &state.enabled_tribes {
        hasher.u8(*tribe as u8);
    }

    hasher.len(state.previous_opponent.len());
    for (seat, opponent) in &state.previous_opponent {
        hasher.u8(*seat);
        match opponent {
            Some(seat) => {
                hasher.u8(1);
                hasher.u8(*seat);
            }
            None => hasher.u8(0),
        }
    }

    hasher.len(state.previous_board.len());
    for (seat, board) in &state.previous_board {
        hasher.u8(*seat);
        hash_minions(&mut hasher, board);
    }

    match state.last_winner {
        Some(seat) => {
            hasher.u8(1);
            hasher.u8(seat);
        }
        None => hasher.u8(0),
    }

    hasher.len(state.pending.len());
    for choice in &state.pending {
        hasher.u8(0); // PendingChoice 目前只有 Discover 变体
        match choice {
            crate::state::PendingChoice::Discover {
                seat,
                candidates,
                source,
            } => {
                hasher.u8(*seat);
                hasher.len(candidates.len());
                for candidate in candidates {
                    hasher.str(candidate);
                }
                hasher.u64(*source);
            }
        }
    }

    hasher.len(state.ranking.len());
    for seat in &state.ranking {
        hasher.u8(*seat);
    }

    hasher.finish()
}

fn hash_player(hasher: &mut Hasher, player: &PlayerState) {
    hasher.u8(player.seat);
    hasher.i32(player.hp);
    hasher.u8(player.gold);
    hasher.u8(era_tag(player.era));
    hasher.u8(player.turns_in_era);

    hasher.len(player.shop.len());
    for slot in &player.shop {
        match slot {
            Some(slot) => {
                hasher.u8(1);
                hasher.str(&slot.card_id);
                hasher.bool(slot.frozen);
            }
            None => hasher.u8(0),
        }
    }

    hash_minions(hasher, &player.board);

    hasher.len(player.hand.len());
    for card in &player.hand {
        hasher.str(&card.card_id);
        hasher.bool(card.golden);
    }

    hasher.opt_str(player.hero.as_deref());

    hasher.len(player.hero_uses.len());
    for (card_id, uses) in &player.hero_uses {
        hasher.str(card_id);
        hasher.u8(*uses);
    }

    hasher.i32(player.damage_dealt);
    hasher.bool(player.eliminated);
    hasher.bool(player.first_refresh_used);
}

fn hash_minions(hasher: &mut Hasher, minions: &[MinionInstance]) {
    hasher.len(minions.len());
    for minion in minions {
        hash_minion(hasher, minion);
    }
}

fn hash_minion(hasher: &mut Hasher, minion: &MinionInstance) {
    hasher.u64(minion.instance_id);
    hasher.str(&minion.card_id);
    hasher.bool(minion.golden);
    hasher.i32(minion.growth);
    hasher.i32(minion.growth_health_only);
    hasher.i32(minion.breed);
    hasher.i32(minion.breed_applied);
    hasher.i32(minion.contagion);

    hasher.len(minion.granted.len());
    for (keyword, duration) in &minion.granted {
        hasher.str(keyword);
        hasher.u8(duration_tag(*duration));
    }

    hasher.len(minion.mods.len());
    for modifier in &minion.mods {
        hash_modifier(hasher, modifier);
    }

    hasher.bool(minion.shield_used);
    hasher.bool(minion.reborn_used);
    hasher.u8(minion.attacks_left);
    hasher.bool(minion.cannot_attack);
}

fn hash_modifier(hasher: &mut Hasher, modifier: &Modifier) {
    hasher.i32(modifier.attack);
    hasher.i32(modifier.health);
    hasher.u8(duration_tag(modifier.duration));
}

fn phase_tag(phase: Phase) -> u8 {
    match phase {
        Phase::Lobby => 0,
        Phase::Recruit => 1,
        Phase::Combat => 2,
        Phase::Settle => 3,
        Phase::GameOver => 4,
    }
}

fn era_tag(era: Era) -> u8 {
    era as u8
}

fn duration_tag(duration: Duration) -> u8 {
    match duration {
        Duration::Instant => 0,
        Duration::Permanent => 1,
        Duration::Combat => 2,
    }
}

struct Hasher {
    value: u64,
}

impl Hasher {
    fn new() -> Self {
        Self { value: FNV_OFFSET }
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.value ^= u64::from(byte);
            self.value = self.value.wrapping_mul(FNV_PRIME);
        }
    }

    fn u8(&mut self, value: u8) {
        self.write(&[value]);
    }

    fn u16(&mut self, value: u16) {
        self.write(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.write(&value.to_le_bytes());
    }

    fn i32(&mut self, value: i32) {
        self.write(&value.to_le_bytes());
    }

    fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    fn len(&mut self, value: usize) {
        self.u64(value as u64);
    }

    fn str(&mut self, value: &str) {
        self.len(value.len());
        self.write(value.as_bytes());
    }

    fn opt_str(&mut self, value: Option<&str>) {
        match value {
            Some(text) => {
                self.u8(1);
                self.str(text);
            }
            None => self.u8(0),
        }
    }

    fn finish(self) -> u64 {
        self.value
    }
}
