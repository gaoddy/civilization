//! 随机策略：只用于冒烟与确定性测试，不参与训练。

/// xorshift64* 的确定性随机数发生器。
#[derive(Debug, Clone)]
pub struct RandomPolicy {
    state: u64,
}

impl RandomPolicy {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1) | 1,
        }
    }

    /// 返回 `0..n` 内的整数；`n` 为 0 时返回 0。
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        (x.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) % n
    }
}
