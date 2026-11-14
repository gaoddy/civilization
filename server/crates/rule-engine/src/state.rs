//! 对局状态模型。
//!
//! 所有集合使用 `BTreeMap` / `BTreeSet` / `Vec`，保证遍历顺序确定；
//! 状态中只含整数，不含浮点、时间或地址。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::data::{Era, Tribe};
use crate::effects::Duration;
use crate::rng::RngStreams;

pub use crate::settle::Pairing;

/// 座位号，0..8。
pub type Seat = u8;

/// 随从实例编号，由 `GameState::next_instance_id` 单调分配。
pub type InstanceId = u64;

/// 对局阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Lobby,
    Recruit,
    Combat,
    Settle,
    GameOver,
}

/// 临时属性修正；`Combat` 结束后清除。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Modifier {
    pub attack: i32,
    pub health: i32,
    pub duration: Duration,
}

/// 某个棋盘实例当前生效的光环派生值。
///
/// 光环只存在于派生缓存中，绝不写入 [`MinionInstance::mods`] 或
/// [`MinionInstance::granted`]；战斗快照折叠时才临时并入克隆。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuraEffect {
    pub attack: i32,
    pub health: i32,
    pub keywords: BTreeSet<String>,
    /// 受到任一光环作用；用于战斗快照的生命下限 1。
    pub floor_one: bool,
}

/// 棋盘上的随从实例。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinionInstance {
    pub instance_id: InstanceId,
    pub card_id: String,
    pub golden: bool,
    pub growth: i32,
    pub growth_health_only: i32,
    pub breed: i32,
    pub breed_applied: i32,
    pub contagion: i32,
    pub granted: Vec<(String, Duration)>,
    pub mods: Vec<Modifier>,
    pub shield_used: bool,
    pub reborn_used: bool,
    pub attacks_left: u8,
    pub cannot_attack: bool,
}

/// 商店中的一个卡槽。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopSlot {
    pub card_id: String,
    pub frozen: bool,
}

/// 手牌中的一张卡。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandCard {
    pub card_id: String,
    pub golden: bool,
}

/// 单个玩家的状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerState {
    pub seat: Seat,
    pub hp: i32,
    pub gold: u8,
    pub era: Era,
    pub turns_in_era: u8,
    pub shop: Vec<Option<ShopSlot>>,
    pub board: Vec<MinionInstance>,
    pub hand: Vec<HandCard>,
    pub hero: Option<String>,
    pub hero_uses: BTreeMap<String, u8>,
    pub damage_dealt: i32,
    pub eliminated: bool,
    pub first_refresh_used: bool,
    /// 本回合的首次刷新是否尚未触发 `OnFirstRefresh`。
    pub first_refresh_pending: bool,
    /// 本回合是否已提交 `EndRecruit`（本玩家结束招募）；全员结束或超时后才进入 `Combat`。
    pub recruit_done: bool,
}

/// 等待玩家做出的选择。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingChoice {
    /// 三连或效果驱动的发现：从 `candidates` 中选一张进入手牌，扣 1 份共享池。
    Discover {
        seat: Seat,
        candidates: Vec<String>,
        source: InstanceId,
    },
    /// 进化：来源随从从 `EVOLVE_CHOICES` 中选一项，`remaining` 为还需选择的次数。
    ///
    /// 进化不算发现：不扣池、不进手牌，结算在来源随从身上完成。
    Evolve {
        seat: Seat,
        source: InstanceId,
        remaining: u8,
    },
}

/// 整局对局状态，`state_hash` 的输入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameState {
    pub turn: u8,
    pub phase: Phase,
    pub players: Vec<PlayerState>,
    pub pool: BTreeMap<String, u16>,
    pub rng: RngStreams,
    pub first_seat: Seat,
    pub enabled_tribes: BTreeSet<Tribe>,
    pub previous_opponent: BTreeMap<Seat, Option<Seat>>,
    pub previous_board: BTreeMap<Seat, Vec<MinionInstance>>,
    pub last_winner: Option<Seat>,
    /// 本回合配对结果，由招募结束写入、结算结束清空。
    pub pairings: Vec<Pairing>,
    /// 本回合每名败者将受到的伤害，由战斗结算写入、结算清空。
    pub pending_settle: BTreeMap<Seat, i32>,
    /// 本回合赢下自己战斗的座位（镜像获胜计入，镜像失败不计）。
    pub last_won: BTreeSet<Seat>,
    pub pending: VecDeque<PendingChoice>,
    /// 已确定的排名（从第 1 名到末名）；出局者先进入尾部，结束时补齐。
    pub ranking: Vec<Seat>,
    pub next_instance_id: InstanceId,
    /// 按实例编号索引的光环派生值；每次区域变化后全量重算。
    pub auras: BTreeMap<InstanceId, AuraEffect>,
}
