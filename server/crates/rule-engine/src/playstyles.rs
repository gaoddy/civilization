//! 开局玩法抽取。
//!
//! 纯函数、独立随机源：不读取也不消费引擎的 `RngStreams`。
//! 同一 seed 必然得到同样的 5 个玩法；结果按枚举顺序排序后返回。

use crate::data::Playstyle;

/// 每局抽取的玩法数量。
pub const PLAYSTYLES_PER_GAME: usize = 5;

/// 从 13 个玩法中无放回抽取 5 个，按 [`Playstyle`] 顺序排序。
///
/// 使用 splitmix64 作独立随机源；同一个 `seed` 结果稳定，与引擎种子派生无关。
pub fn draw(seed: u64) -> [Playstyle; PLAYSTYLES_PER_GAME] {
    let mut rng = SplitMix64::new(seed);
    let mut pool = Playstyle::ALL.to_vec();
    // Fisher–Yates：从后往前与 [0, i] 中的随机位置交换。
    for index in (1..pool.len()).rev() {
        let pick = rng.below((index + 1) as u64) as usize;
        pool.swap(index, pick);
    }
    let mut chosen: [Playstyle; PLAYSTYLES_PER_GAME] = [
        Playstyle::Mammal,
        Playstyle::Bird,
        Playstyle::Reptile,
        Playstyle::Fish,
        Playstyle::Insect,
    ];
    for (slot, playstyle) in chosen.iter_mut().zip(pool.into_iter()) {
        *slot = playstyle;
    }
    chosen.sort();
    chosen
}

/// splitmix64：小、确定、无平台依赖。
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// 返回 `[0, n)` 内的均匀随机数；拒绝采样避免模偏差。
    fn below(&mut self, n: u64) -> u64 {
        if n <= 1 {
            return 0;
        }
        let threshold = n.wrapping_neg() % n;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return value % n;
            }
        }
    }
}
