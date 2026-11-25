//! 开局候选英雄分配。
//!
//! 训练与房间服务共用同一套口径：候选只取 [`hero_in_pool`] 通过的英雄，
//! 洗牌与切段都由对局种子决定，段号即座位号。

use std::collections::{BTreeMap, BTreeSet};

use crate::data::{Catalog, Playstyle, hero_in_pool};
use crate::state::Seat;

/// 满座人数。
pub const SEAT_COUNT: usize = 8;

/// 每名玩家分到的候选英雄数量。
pub const CANDIDATE_COUNT: usize = 4;

/// 按对局种子为 8 个座位分配候选英雄。
///
/// - 池 ≥ 满座总需求时每座位 4 张、房间内互不重复；
/// - 池不足时按 `(seat * 4 + i) % len` 取 `min(4, len)` 张，允许跨座位重复；
/// - 池小于 4 时每座位取整个池并至少 1 张。
pub fn allocate_candidates(
    seed: u64,
    playstyles: &[Playstyle; 5],
    catalog: &Catalog,
) -> BTreeMap<Seat, Vec<String>> {
    let enabled: BTreeSet<Playstyle> = playstyles.iter().copied().collect();
    let mut heroes: Vec<String> = catalog
        .heroes
        .values()
        .filter(|hero| hero_in_pool(hero, &enabled))
        .map(|hero| hero.id.clone())
        .collect();
    // 产品兜底：本局玩法过滤后一张英雄都没有时，回退到全部英雄卡。
    if heroes.is_empty() {
        heroes = catalog.heroes.keys().cloned().collect();
    }
    heroes.sort();
    shuffle(&mut heroes, seed);

    let mut candidates = BTreeMap::new();
    for seat in 0..SEAT_COUNT as Seat {
        let picked = if heroes.is_empty() {
            Vec::new()
        } else if heroes.len() >= SEAT_COUNT * CANDIDATE_COUNT {
            let start = usize::from(seat) * CANDIDATE_COUNT;
            let end = (start + CANDIDATE_COUNT).min(heroes.len());
            heroes
                .get(start..end)
                .map(<[_]>::to_vec)
                .unwrap_or_default()
        } else {
            let take = CANDIDATE_COUNT.min(heroes.len());
            (0..take)
                .map(|offset| {
                    heroes[(usize::from(seat) * CANDIDATE_COUNT + offset) % heroes.len()].clone()
                })
                .collect()
        };
        candidates.insert(seat, picked);
    }
    candidates
}

/// 固定种子的 Fisher–Yates 洗牌。
fn shuffle(items: &mut [String], seed: u64) {
    let mut state = seed | 1;
    for index in (1..items.len()).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let swap_with = (state >> 33) as usize % (index + 1);
        items.swap(index, swap_with);
    }
}
