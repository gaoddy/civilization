use std::collections::HashSet;

use ai_train::data_c0::{BoardSample, generate_pairs, pair_seed};
use rule_engine::combat;
use rule_engine::data::Catalog;
use rule_engine::rng::{RngStreams, seed_from_u64};
use rule_engine::state::MinionInstance;

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

/// 与生成器一致的实例编号重排：攻击方从 1 起，防守方从 1001 起。
fn renumber(sample: &BoardSample, base: u64) -> Vec<MinionInstance> {
    sample
        .board
        .iter()
        .enumerate()
        .map(|(index, minion)| {
            let mut minion = minion.clone();
            minion.instance_id = base + index as u64;
            minion
        })
        .collect()
}

#[test]
fn labels_match_combat_simulation() {
    let catalog = catalog();
    let pairs = generate_pairs(&catalog, 4, 2026);
    assert!(pairs.len() >= 100, "样本太少：{}", pairs.len());
    for (index, pair) in pairs.iter().take(100).enumerate() {
        let attacker = renumber(&pair.a, 1);
        let defender = renumber(&pair.b, 1001);
        let mut rng = RngStreams::from_seed(seed_from_u64(pair_seed(2026, index)));
        let log = combat::simulate(&catalog, &attacker, &defender, 0, 1, 0, &mut rng);
        let expected = log.winner.map(|winner| winner == 0);
        assert_eq!(expected, Some(pair.a_wins), "第 {index} 对");
    }
}

#[test]
fn coverage_includes_multiple_heroes_and_playstyles() {
    let catalog = catalog();
    let pairs = generate_pairs(&catalog, 16, 7);
    let heroes: HashSet<&str> = pairs
        .iter()
        .flat_map(|pair| [pair.a.hero.as_str(), pair.b.hero.as_str()])
        .collect();
    let playstyles: HashSet<u16> = pairs
        .iter()
        .flat_map(|pair| [pair.a.playstyles, pair.b.playstyles])
        .collect();
    assert!(heroes.len() >= 8, "英雄覆盖不足：{heroes:?}");
    assert!(
        playstyles.len() >= 10,
        "玩法组合覆盖不足：{} 种",
        playstyles.len()
    );
}

#[test]
fn same_seed_generates_same_pairs() {
    let catalog = catalog();
    let first = generate_pairs(&catalog, 2, 11);
    let second = generate_pairs(&catalog, 2, 11);
    assert_eq!(first, second);
    assert!(!first.is_empty());
}
