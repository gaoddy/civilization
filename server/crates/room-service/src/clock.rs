//! 时钟抽象：引擎与网络服务不直接读系统时间，统一经 [`Clock`] 注入。
//!
//! 生产使用 [`SystemClock`]；测试使用 [`FixedClock`]，可显式设定与推进时间。

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};

/// 毫秒级时钟。`Send + Sync + 'static` 便于在 tokio 任务与测试间共享。
pub trait Clock: Send + Sync + 'static {
    /// 当前 Unix epoch 毫秒。
    fn now_ms(&self) -> u64;

    /// 当前时间的 RFC3339 文本，由 [`Clock::now_ms`] 派生，保证两者始终一致。
    fn now_rfc3339(&self) -> String {
        DateTime::<Utc>::from_timestamp_millis(self.now_ms() as i64)
            .map(|time| time.to_rfc3339())
            .unwrap_or_default()
    }
}

/// 系统时钟：读取 `std::time::SystemTime`。
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as u64)
            .unwrap_or(0)
    }
}

/// 固定/可推进的测试时钟，内部为 `Arc<AtomicU64>`，克隆后共享同一时间。
#[derive(Debug, Clone)]
pub struct FixedClock {
    now_ms: Arc<AtomicU64>,
}

impl FixedClock {
    /// 以给定 Unix epoch 毫秒创建时钟。
    pub fn new(ms: u64) -> Self {
        Self {
            now_ms: Arc::new(AtomicU64::new(ms)),
        }
    }

    /// 直接设定当前时间。
    pub fn set(&self, ms: u64) {
        self.now_ms.store(ms, Ordering::SeqCst);
    }

    /// 在当前时间上累加 `delta` 毫秒（饱和处理，不回绕）。
    pub fn advance(&self, delta: u64) {
        self.now_ms
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                Some(current.saturating_add(delta))
            })
            .ok();
    }
}

impl Clock for FixedClock {
    fn now_ms(&self) -> u64 {
        self.now_ms.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::{Clock, FixedClock};

    #[test]
    fn clone_shares_time() {
        let clock = FixedClock::new(10);
        let clone = clock.clone();
        clone.advance(5);
        assert_eq!(clock.now_ms(), 15);
    }
}
