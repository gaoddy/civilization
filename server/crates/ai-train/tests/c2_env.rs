//! C2 完整对局环境：英雄选择前置步骤、联盟采样与候选分配口径。

use std::collections::{BTreeMap, BTreeSet};

use ai_core::Action;
use ai_model::Weights;
use ai_train::env_full::FullEnv;
use ai_train::league::{League, PolicyId};
use rule_engine::candidates::{CANDIDATE_COUNT, SEAT_COUNT, allocate_candidates};
use rule_engine::data::{Catalog, Playstyle, hero_in_pool};
use rule_engine::playstyles;
use rule_engine::state::Seat;

fn catalog() -> Catalog {
    Catalog::from_json(
        include_str!("../../../data/v0.1/cards.json"),
        include_str!("../../../data/v0.1/heroes.json"),
        include_str!("../../../data/v0.1/skills.json"),
        include_str!("../../../data/v0.1/config.json"),
        include_str!("../../../data/v0.1/taxonomy.json"),
    )
    .expect("卡表")
}

fn weights(tag: usize) -> Weights {
    Weights {
        tensors: BTreeMap::from([("tag".to_string(), vec![tag as f32])]),
        card_count: 0,
        hero_count: 0,
        skill_count: 0,
    }
}

#[test]
fn hero_pick_is_the_first_decision_and_has_four_options() {
    let catalog = catalog();
    let seed = 7;
    let candidates = allocate_candidates(seed, &playstyles::draw(seed), &catalog);
    let mut env = FullEnv::new(&catalog, League::new(1, 8));
    env.reset(seed, &candidates).expect("重置");

    for seat in 0..SEAT_COUNT as Seat {
        assert_eq!(env.current_seat(), Some(seat), "按座位升序发放英雄选择");
        assert_eq!(env.observe(seat).hero, -1, "选英雄前观测里没有英雄");
        let legal = env.legal(seat);
        assert_eq!(
            legal.len(),
            CANDIDATE_COUNT,
            "每个座位 {CANDIDATE_COUNT} 个候选"
        );
        let heroes: Vec<String> = legal
            .iter()
            .map(|action| match action {
                Action::PickHero { hero } => hero.clone(),
                other => panic!("选英雄阶段出现非 PickHero 动作：{other:?}"),
            })
            .collect();
        assert_eq!(heroes, candidates[&seat], "候选顺序与分配一致");
        env.step(Action::PickHero {
            hero: heroes[0].clone(),
        })
        .expect("选英雄");
        assert_ne!(env.observe(seat).hero, -1, "选完后观测里带上英雄");
    }

    // 全员选完：回到座位 0 的招募，PickHero 不再合法。
    assert!(env.started(), "选完英雄后对局开始");
    assert_eq!(env.current_seat(), Some(0));
    let legal = env.legal(0);
    assert!(
        legal
            .iter()
            .all(|action| !matches!(action, Action::PickHero { .. })),
        "开局后不再出现 PickHero"
    );
    assert!(
        legal.iter().any(|action| matches!(action, Action::End)),
        "开局后进入招募，可以结束回合"
    );
}

#[test]
fn pick_hero_outside_candidates_is_rejected() {
    let catalog = catalog();
    let seed = 11;
    let candidates = allocate_candidates(seed, &playstyles::draw(seed), &catalog);
    let mut env = FullEnv::new(&catalog, League::new(2, 8));
    env.reset(seed, &candidates).expect("重置");

    let outside = catalog
        .heroes
        .keys()
        .find(|hero| !candidates[&0].contains(hero))
        .expect("存在候选外英雄")
        .clone();
    assert!(env.step(Action::PickHero { hero: outside }).is_err());
    assert_eq!(env.current_seat(), Some(0), "拒绝后状态不变");
}

#[test]
fn league_sample_covers_anchor_and_snapshots() {
    let mut league = League::new(42, 8);
    assert_eq!(league.anchor(), None, "初始没有锚点");
    // 容量 8：写入 10 个快照只保留最近 8 个。
    for index in 0..10 {
        league.add_snapshot(weights(index));
    }
    assert_eq!(league.snapshots().len(), 8);
    league.set_anchor(weights(99));

    let mut seen_snapshots = BTreeSet::new();
    let mut seen_anchor = 0;
    let mut seen_current = 0;
    for _ in 0..1000 {
        match league.sample(0) {
            PolicyId::Current => seen_current += 1,
            PolicyId::Snapshot(index) => {
                assert!(index < 8, "快照下标在容量内");
                seen_snapshots.insert(index);
            }
            PolicyId::Anchor => seen_anchor += 1,
            PolicyId::Random => panic!("已有锚点与快照时不该落到随机兜底"),
        }
    }
    assert_eq!(seen_snapshots.len(), 8, "每个快照都被采样到");
    assert!(seen_anchor > 0, "锚点被采样到");
    assert!(seen_current > 0, "当前策略被采样到");
}

#[test]
fn candidate_allocation_matches_room_service() {
    let catalog = catalog();
    for seed in [0u64, 1, 2, 12_345, 9_876_543_210] {
        let playstyles = playstyles::draw(seed);
        assert_eq!(
            allocate_candidates(seed, &playstyles, &catalog),
            reference_allocate(seed, &playstyles, &catalog),
            "seed {seed} 的候选分配与房间服务原实现一致"
        );
    }
}

/// 房间服务在抽取共享实现之前的原始算法，作为口径基准保留在测试里。
fn reference_allocate(
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
    if heroes.is_empty() {
        heroes = catalog.heroes.keys().cloned().collect();
    }
    heroes.sort();
    reference_shuffle(&mut heroes, seed);

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

fn reference_shuffle(items: &mut [String], seed: u64) {
    let mut state = seed | 1;
    for index in (1..items.len()).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let swap_with = (state >> 33) as usize % (index + 1);
        items.swap(index, swap_with);
    }
}
