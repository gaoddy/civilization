//! 固定种子评测：同一权重与种子两次结果一致，且报告 95% 置信区间。

#![cfg(feature = "train")]

use ai_core::CardIndex;
use ai_model::{ModelConfig, Weights};
use ai_train::eval_full::{EvalConfig, eval_full};
use ai_train::league::{League, PolicyId};
use ai_train::ppo::TrainModel;
use rule_engine::data::Catalog;

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

fn tiny_catalog() -> Catalog {
    let mut catalog = catalog();
    catalog.config.turn_limit = 2;
    catalog
}

fn weights_for(catalog: &Catalog, seed: u64) -> Weights {
    let index = CardIndex::build(catalog);
    let model = ModelConfig {
        obs_dim: ai_core::OBS_DIM,
        card_emb: 16,
        hidden: 64,
        layers: 2,
    };
    TrainModel::new(
        &model,
        index.card_count(),
        index.hero_count(),
        index.skill_count(),
        seed,
    )
    .expect("模型")
    .weights()
    .expect("权重")
}

#[test]
fn eval_is_deterministic_and_reports_confidence_interval() {
    let catalog = tiny_catalog();
    let learner = weights_for(&catalog, 1);
    let anchor = weights_for(&catalog, 2);
    let mut league = League::new(0, 1);
    league.set_anchor(anchor);
    let cfg = EvalConfig {
        seeds: (0..8).collect(),
        learner_seats: 4,
        anchors: vec![PolicyId::Anchor],
        max_decisions_per_turn: 16,
    };

    let first = eval_full(&catalog, &learner, &league, &cfg).expect("第一次评测");
    let second = eval_full(&catalog, &learner, &league, &cfg).expect("第二次评测");

    assert_eq!(first, second, "同权重同种子必须逐字段一致");
    assert_eq!(first.games, 8, "每个种子一局");
    assert!(first.mean_placement >= 1.0 && first.mean_placement <= 8.0);
    assert!(first.ci95 > 0.0, "名次有波动时置信区间宽度应大于 0");
}
