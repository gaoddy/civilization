//! 确定性随机数：xoshiro256++ 的五个命名流。
//!
//! 所有随机性都经由 [`RngStreams`]，不使用全局 RNG、时间或系统熵。
//! 每个流用 `seed` 与流名做 FNV-1a 派生种子，`below` 用拒绝采样避免模偏差。

/// 随机流。各流互相独立，同名同种子必然产生相同序列。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Shop,
    Combat,
    Discover,
    Matchmaking,
    Effects,
}

impl Stream {
    /// 全部流，按固定顺序排列，供遍历与哈希使用。
    pub const ALL: [Stream; 5] = [
        Stream::Shop,
        Stream::Combat,
        Stream::Discover,
        Stream::Matchmaking,
        Stream::Effects,
    ];

    /// 流名，参与种子派生，不得随意改动。
    pub fn name(self) -> &'static str {
        match self {
            Stream::Shop => "shop",
            Stream::Combat => "combat",
            Stream::Discover => "discover",
            Stream::Matchmaking => "matchmaking",
            Stream::Effects => "effects",
        }
    }

    fn index(self) -> usize {
        match self {
            Stream::Shop => 0,
            Stream::Combat => 1,
            Stream::Discover => 2,
            Stream::Matchmaking => 3,
            Stream::Effects => 4,
        }
    }
}

/// xoshiro256++ 的状态集合，每条流独立维护 256 位状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RngStreams {
    states: [[u64; 4]; 5],
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const SPLITMIX_GAMMA: u64 = 0x9e37_79b9_7f4a_7c15;
const NONZERO_FALLBACK: u64 = 0x9e37_79b9_7f4a_7c15;

impl RngStreams {
    /// 由 32 字节种子派生五个独立流。
    pub fn from_seed(seed: [u8; 32]) -> Self {
        let mut states = [[0u64; 4]; 5];
        for stream in Stream::ALL {
            let mut counter = fnv1a_seed(&seed, stream.name());
            let state = &mut states[stream.index()];
            for word in state.iter_mut() {
                *word = splitmix64(&mut counter);
            }
            if state.iter().all(|&word| word == 0) {
                state[0] = NONZERO_FALLBACK;
            }
        }
        Self { states }
    }

    /// 取某条流的下一个 64 位输出（xoshiro256++）。
    pub fn next_u64(&mut self, stream: Stream) -> u64 {
        let s = &mut self.states[stream.index()];
        let result = s[0].wrapping_add(s[3]).rotate_left(23).wrapping_add(s[0]);
        let t = s[1] << 17;

        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);

        result
    }

    /// 返回 `[0, n)` 内的均匀随机数；`n <= 1` 时返回 0。
    ///
    /// 使用拒绝采样，避免 `% n` 的模偏差。
    pub fn below(&mut self, stream: Stream, n: u64) -> u64 {
        if n <= 1 {
            return 0;
        }
        let threshold = n.wrapping_neg() % n;
        loop {
            let value = self.next_u64(stream);
            if value >= threshold {
                return value % n;
            }
        }
    }

    /// 某条流当前的四字状态快照，供状态哈希使用。
    pub fn words(&self, stream: Stream) -> [u64; 4] {
        self.states[stream.index()]
    }
}

/// FNV-1a：把种子字节与流名依次混入。
fn fnv1a_seed(seed: &[u8; 32], stream_name: &str) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in seed.iter().chain(stream_name.as_bytes().iter()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// splitmix64：把 64 位计数器扩展为高质量伪随机字。
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(SPLITMIX_GAMMA);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// 引擎 32 字节种子与日志 64 位种子之间的对应：前 8 字节按小端写日志种子，其余三组
/// 8 字节由该种子经固定混合常数派生。日志只记录 64 位种子，重放时由它重建同样的
/// 32 字节，因此重放与原始对局使用完全相同的随机流。
pub fn seed_from_u64(seed: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&seed.to_le_bytes());
    bytes[8..16].copy_from_slice(&seed.wrapping_mul(0x9e37_79b9_7f4a_7c15).to_le_bytes());
    bytes[16..24].copy_from_slice(&seed.wrapping_add(0xd1b5_4a32_d192_ed03).to_le_bytes());
    bytes[24..32].copy_from_slice(&seed.rotate_left(29).to_le_bytes());
    bytes
}
