//! 对局事件：引擎把状态变化以事件形式返回，供协议层同步。

use crate::data::Era;
use crate::state::{InstanceId, Phase, Seat};

/// 一次引擎调用产生的事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEvent {
    TurnStarted {
        turn: u8,
    },
    PhaseChanged {
        phase: Phase,
        turn: u8,
    },
    MinionBought {
        seat: Seat,
        slot: u8,
        card_id: String,
    },
    MinionSold {
        seat: Seat,
        minion: InstanceId,
    },
    ShopRolled {
        seat: Seat,
    },
    EraUpgraded {
        seat: Seat,
        era: Era,
    },
    TripleFormed {
        seat: Seat,
        card_id: String,
    },
    CardDiscovered {
        seat: Seat,
        card_id: String,
    },
    DamageTaken {
        seat: Seat,
        amount: i32,
    },
    PlayerEliminated {
        seat: Seat,
        place: u8,
    },
    CombatResolved {
        winner: Seat,
        loser: Seat,
        damage: i32,
    },
    GameOver {
        ranking: Vec<Seat>,
    },
}
