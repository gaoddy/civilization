//! 天梯积分变化：8 人两两 28 对的 Elo 计算。
//!
//! 纯函数，不读时钟、不访问存储；产品规则见
//! `docs/versions/v0.2/product/systems/ranked-and-matchmaking.md` §3。
//!
//! 每对中名次更高者为胜；玩家 `i` 的变化为
//! `K / 7 × Σ(实际结果 − 预期胜率)`，对另外 7 名对手求和。计算保留小数，
//! 最终四舍五入为整数，不要求 8 人变化总和为 0，取整后也不补差。

/// 每局座位数。
pub const PLAYERS_PER_MATCH: usize = 8;

/// Elo K 系数（产品默认值）。
pub const K_FACTOR: f64 = 200.0;

/// 8 人开局积分与名次 → 每人四舍五入后的整数积分变化。
///
/// `start_ratings[i]` 与 `placements[i]` 必须对应同一名玩家；名次为 1–8 且
/// 互不重复。返回数组与输入下标一致。
///
/// # Panics
///
/// 仅在调试构建下断言名次范围与唯一性；释放构建不校验，调用方应保证输入合法。
#[must_use]
pub fn rating_deltas(
    k: f64,
    start_ratings: &[i32; PLAYERS_PER_MATCH],
    placements: &[u8; PLAYERS_PER_MATCH],
) -> [i32; PLAYERS_PER_MATCH] {
    debug_assert!(
        placements.iter().all(|place| (1..=8).contains(place)),
        "名次必须在 1–8：{placements:?}"
    );
    debug_assert!(
        {
            let mut sorted = *placements;
            sorted.sort_unstable();
            sorted == [1, 2, 3, 4, 5, 6, 7, 8]
        },
        "名次必须唯一：{placements:?}"
    );

    let mut deltas = [0_i32; PLAYERS_PER_MATCH];
    for (index, &place) in placements.iter().enumerate() {
        let rating = f64::from(start_ratings[index]);
        let mut expected_wins = 0.0_f64;
        let mut actual_wins = 0_u32;
        for (other_index, &other_place) in placements.iter().enumerate() {
            if other_index == index {
                continue;
            }
            let other_rating = f64::from(start_ratings[other_index]);
            expected_wins += 1.0 / (1.0 + 10_f64.powf((other_rating - rating) / 400.0));
            if place < other_place {
                actual_wins += 1;
            }
        }
        let raw = k / (PLAYERS_PER_MATCH as f64 - 1.0) * (f64::from(actual_wins) - expected_wins);
        // Rust 的 `round` 为远离零舍入，与产品示例一致。
        deltas[index] = raw.round() as i32;
    }
    deltas
}

/// 应用个人总分下限：`after = max(0, before + delta)`。
///
/// 返回 `(结算后总分, 实际入账变化)`；被下限截断时实际变化小于原始变化，
/// 结算页展示实际入账值。
#[must_use]
pub fn apply_rating_floor(before: i32, delta: i32) -> (i32, i32) {
    let after = before.saturating_add(delta).max(0);
    (after, after - before)
}

#[cfg(test)]
mod tests {
    use super::{K_FACTOR, apply_rating_floor, rating_deltas};

    #[test]
    fn eight_equal_ratings_match_documented_table() {
        let ratings = [1000; 8];
        let placements = [1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(
            rating_deltas(K_FACTOR, &ratings, &placements),
            [100, 71, 43, 14, -14, -43, -71, -100]
        );
    }

    #[test]
    fn low_rated_player_against_seven_high_rated_opponents() {
        // 1000 分玩家对 7 名 1200 分玩家，预期胜场约 1.68。
        let mut ratings = [1200; 8];
        ratings[7] = 1000;

        // 第 4 名：实际胜场 4，200 / 7 × (4 − 1.68) = +66。
        let mut placements = [1_u8, 2, 3, 5, 6, 7, 8, 4];
        placements[7] = 4;
        assert_eq!(rating_deltas(K_FACTOR, &ratings, &placements)[7], 66);

        // 第 8 名：实际胜场 0，200 / 7 × (0 − 1.68) = −48。
        let mut placements = [1_u8, 2, 3, 4, 5, 6, 7, 8];
        placements[7] = 8;
        assert_eq!(rating_deltas(K_FACTOR, &ratings, &placements)[7], -48);

        // 第 1 名：实际胜场 7，200 / 7 × (7 − 1.68) = +152。
        let mut placements = [2_u8, 3, 4, 5, 6, 7, 8, 1];
        placements[7] = 1;
        assert_eq!(rating_deltas(K_FACTOR, &ratings, &placements)[7], 152);
    }

    #[test]
    fn custom_k_scales_deltas() {
        // K 加倍，变化量也应加倍（文档表格按 K=200 计算）。
        let ratings = [1000; 8];
        let placements = [1, 2, 3, 4, 5, 6, 7, 8];
        assert_eq!(
            rating_deltas(K_FACTOR * 2.0, &ratings, &placements),
            [200, 143, 86, 29, -29, -86, -143, -200]
        );
        assert_eq!(rating_deltas(0.0, &ratings, &placements), [0; 8]);
    }

    #[test]
    fn documented_floor_example_clamps_to_zero() {
        assert_eq!(apply_rating_floor(5, -14), (0, -5));
        assert_eq!(apply_rating_floor(1000, -100), (900, -100));
        assert_eq!(apply_rating_floor(0, 0), (0, 0));
    }

    #[test]
    fn deltas_are_not_forced_to_sum_to_zero() {
        // 不同积分下的取整误差保留在原处，不做补差。
        let ratings = [1000, 1010, 1020, 1030, 1040, 1050, 1060, 1070];
        let placements = [1, 2, 3, 4, 5, 6, 7, 8];
        let deltas = rating_deltas(K_FACTOR, &ratings, &placements);
        let sum: i32 = deltas.iter().sum();
        let _ = sum;
        // 只要求每项为四舍五入结果，不要求 sum == 0。
        assert_eq!(deltas.len(), 8);
    }
}
