//! 下行信封序号分配。
//!
//! 每个玩家连接持有一个 [`SeatChannel`]；`seq` 从 1 起单调递增，失败的构造同样会
//! 消耗序号，保证同一连接内不会重复。

use protocol::{ProtocolError, ServerEnvelope};
use serde_json::{Map, Value};

/// 单连接的下行信封构造器。
#[derive(Debug, Clone, Default)]
pub struct SeatChannel {
    next_seq: u64,
}

impl SeatChannel {
    /// 新建通道，首个信封的 `seq` 为 1。
    pub fn new() -> Self {
        Self { next_seq: 0 }
    }

    /// 为下行载荷分配下一个序号并构造信封。
    ///
    /// 序号先自增再委托 [`ServerEnvelope::new`]；构造失败（未知类型、载荷不合法）
    /// 只透传错误，不回退序号。
    ///
    /// # Errors
    ///
    /// 同 [`ServerEnvelope::new`]：未知 `kind` 返回 `UNKNOWN_TYPE`，`seq` 为 0 返回
    /// `BAD_PAYLOAD`（本类型不会产生 0）。
    pub fn bind(
        &mut self,
        kind: &str,
        payload: Map<String, Value>,
    ) -> Result<ServerEnvelope, ProtocolError> {
        self.next_seq = self.next_seq.saturating_add(1);
        ServerEnvelope::new(kind, self.next_seq, payload)
    }

    /// 当前已分配到的序号（尚未分配时为 0）。
    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }

    /// 把已分配序号推进到 `seq`（只前进不后退）。
    ///
    /// 连接层在认证阶段用独立的通道下发 `AuthResult`，入房后要保证该座位后续
    /// 大厅/对局消息的序号继续递增，避免同一玩家看到重复 `seq`。
    pub fn set_next_seq(&mut self, seq: u64) {
        self.next_seq = self.next_seq.max(seq);
    }
}
