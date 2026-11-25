//! 对手棋盘历史：只保留本人亲自对战过的对手最近一次棋盘。
//!
//! 这是玩家从战斗结果里本来就能看到的信息；引擎不保存它，由训练环境在战斗
//! 结算时记录，服务端从战斗结果重建。

use rule_engine::state::Seat;
use serde::{Deserialize, Serialize};

/// 单个对手的最近一次棋盘快照。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardSnapshot {
    /// 棋盘卡牌索引，按站位顺序；不足部分为 `-1`。
    pub cards: [i32; 7],
}

impl Default for BoardSnapshot {
    fn default() -> Self {
        Self { cards: [-1; 7] }
    }
}

/// 8 个座位的对手棋盘历史。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct History {
    opponents: [Option<BoardSnapshot>; 8],
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录某个对手在本轮被看到的棋盘；`cards` 超过 7 张时截断。
    pub fn record(&mut self, seat: Seat, cards: &[i32]) {
        let Some(slot) = self.opponents.get_mut(usize::from(seat)) else {
            return;
        };
        let mut snapshot = BoardSnapshot::default();
        for (target, card) in snapshot.cards.iter_mut().zip(cards) {
            *target = *card;
        }
        *slot = Some(snapshot);
    }

    /// 某个对手的最近棋盘；没记录过时全是 `-1`。
    pub fn board(&self, seat: Seat) -> [i32; 7] {
        self.opponents
            .get(usize::from(seat))
            .and_then(|snapshot| *snapshot)
            .map(|snapshot| snapshot.cards)
            .unwrap_or([-1; 7])
    }
}
