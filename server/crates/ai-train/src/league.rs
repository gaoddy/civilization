//! 自博弈联盟：当前策略 + 最近 K 个快照 + 启发式锚点 + 随机兜底。
//!
//! 采样配比按设计 D7 的默认值：50% 当前策略、30% 最近 8 个快照、20% 锚点。
//! 快照与锚点只存权重；当前策略由训练循环自己持有，联盟只记录「轮到它」。

use std::collections::VecDeque;

use ai_model::Weights;
use rule_engine::candidates::SEAT_COUNT;
use rule_engine::state::Seat;

use crate::random_policy::RandomPolicy;

/// 默认保留的快照数。
pub const SNAPSHOT_CAPACITY: usize = 8;
/// 当前策略的采样权重（百分比）。
pub const CURRENT_WEIGHT: u64 = 50;
/// 快照池的采样权重（百分比）。
pub const SNAPSHOT_WEIGHT: u64 = 30;
/// 锚点的采样权重（百分比）；其余落到随机兜底。
pub const ANCHOR_WEIGHT: u64 = 20;

/// 联盟给出的对手身份。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyId {
    /// 正在训练的当前策略。
    Current,
    /// 历史快照，`0` 是最早保留的，越大越新。
    Snapshot(usize),
    /// 固定的启发式锚点。
    Anchor,
    /// 锚点缺席时的随机兜底。
    Random,
}

/// 对手池。
pub struct League {
    /// 每个座位一条独立随机流，采样结果与座位决策顺序无关。
    rngs: Vec<RandomPolicy>,
    capacity: usize,
    snapshots: VecDeque<Weights>,
    anchor: Option<Weights>,
}

impl League {
    pub fn new(seed: u64, capacity: usize) -> Self {
        Self {
            rngs: (0..SEAT_COUNT)
                .map(|seat| RandomPolicy::new(seed ^ (seat as u64).wrapping_mul(0x9E37_79B9)))
                .collect(),
            capacity,
            snapshots: VecDeque::new(),
            anchor: None,
        }
    }

    /// 加入一份快照；超出容量时丢弃最早的。
    pub fn add_snapshot(&mut self, weights: Weights) {
        if self.capacity == 0 {
            return;
        }
        while self.snapshots.len() >= self.capacity {
            self.snapshots.pop_front();
        }
        self.snapshots.push_back(weights);
    }

    /// 保留中的快照，从最早到最新。
    pub fn snapshots(&self) -> &VecDeque<Weights> {
        &self.snapshots
    }

    pub fn anchor(&self) -> Option<&Weights> {
        self.anchor.as_ref()
    }

    pub fn set_anchor(&mut self, weights: Weights) {
        self.anchor = Some(weights);
    }

    /// 按设计 D7 的配比给该座位抽一个对手。
    pub fn sample(&mut self, seat: Seat) -> PolicyId {
        let roll = self.rngs[usize::from(seat)].below(100);
        if roll < CURRENT_WEIGHT || self.snapshots.is_empty() {
            return PolicyId::Current;
        }
        if roll < CURRENT_WEIGHT + SNAPSHOT_WEIGHT {
            let index = self.rngs[usize::from(seat)].below(self.snapshots.len() as u64) as usize;
            return PolicyId::Snapshot(index);
        }
        if self.anchor.is_some() {
            PolicyId::Anchor
        } else {
            PolicyId::Random
        }
    }

    /// 快照或锚点的权重；当前策略与随机兜底由调用方自己解决。
    pub fn weights(&self, policy: PolicyId) -> Option<&Weights> {
        match policy {
            PolicyId::Snapshot(index) => self.snapshots.get(index),
            PolicyId::Anchor => self.anchor.as_ref(),
            PolicyId::Current | PolicyId::Random => None,
        }
    }
}
